use crate::common::*;
use crate::domain;
use crate::domain::*;
use crate::infra::network::{AddFriendError, AddFriendEvent, CaptchaEvent, ChatConnError, ChatMessageRecv, ChatMessageSent, ChatMetaData, FetchConversationHistoryError, FetchConversationHistoryEvent, FetchFriendListError, FetchFriendListEvent, Identity, LoginEvent, MessageEvent, Network, NetworkError, SessionEvent, SignupEvent, StreamMessage, WithGeneration};
use crate::state::conversation_store::ConversationStore;
use crate::state::conversation_store_impl::ConversationStoreImpl;
use crate::state::key_provider::*;
use crate::state::user_store::UserStore;
use crate::state::user_store_impl::UserStoreImpl;
use anyhow::{anyhow, Error, Result};
use crossbeam_channel::Sender;
use futures_util::sink::With;
use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;
use tracing::{error, event, warn};

const TIMEOUT_MS: u64 = 1000_000;

pub struct RealAppState {
    key_provider: KeyProvider,
    captcha_registry: HashMap<SemanticKey, MaybeWithGen<AsyncValue<CaptchaData, CaptchaError>>>,
    signup_registry: Option<MaybeWithGen<AsyncValue<SignupSuccess, SignupError>>>,
    login_registry: Option<MaybeWithGen<AsyncValue<LoginSuccess, LoginError>>>,
    identity: Option<Identity>,
    connection_request_registry: Option<MaybeWithGen<AsyncValue<Connected, EstablishError>>>,
    connection: Option<Generation>,

    friend_list_registry:
        Option<MaybeWithGen<AsyncValue<Vec<FriendSummary>, FetchFriendListError>>>,
    add_friend_registry: Option<MaybeWithGen<AsyncValue<ConversationId, AddFriendError>>>,
    conversation_store: Option<Box<dyn ConversationStore>>,
    user_store: Option<Box<dyn UserStore>>,
    message_tx: Sender<AppMessage>,
    network: Rc<RefCell<dyn Network>>,
}

impl RealAppState {
    pub fn new(message_tx: Sender<AppMessage>, network: Rc<RefCell<dyn Network>>) -> Self {
        Self {
            key_provider: KeyProvider::new(),
            captcha_registry: HashMap::new(),
            signup_registry: None,
            login_registry: None,
            identity: None,
            connection_request_registry: None,
            connection: None,

            friend_list_registry: None,
            add_friend_registry: None,
            conversation_store: None,
            user_store: None,
            message_tx,
            network,
        }
    }

    fn fetch_captcha(&mut self, key: SemanticKey) {
        debug_assert!(
            self.captcha_registry.contains_key(&key),
            "captcha key not registered: {:?}",
            key
        );

        let message_tx_clone = self.message_tx.clone();
        let map = move |event: WithGeneration<CaptchaEvent>| {
            let generation = event.generation;
            match event.result.result {
                Ok(data) => {
                    let _ = message_tx_clone.send(AppMessage::CaptchaEvent(WithGenAndKey::new(
                        generation,
                        key,
                        Ok(domain::CaptchaData {
                            id: CaptchaId(data.id),
                            image: Image(data.image_base64),
                        }),
                    )));
                }
                Err(_) => {}
            }
        };
        let message_tx_clone = self.message_tx.clone();
        let map_err = move |error: WithGeneration<NetworkError>| {
            let generation = error.generation;
            let _ = message_tx_clone.send(AppMessage::CaptchaEvent(WithGenAndKey::new(
                generation,
                key,
                Err(domain::CaptchaError),
            )));
        };
        let tagged = self.captcha_registry.get_mut(&key).unwrap();
        tagged.generation = self
            .network
            .borrow_mut()
            .fetch_captcha(TIMEOUT_MS, Box::new(map), Box::new(map_err))
            .ok();
        tagged.slot = AsyncValue::Pending;
    }

    fn receive_captcha(&mut self, event: WithGenAndKey<Result<CaptchaData, CaptchaError>>) {
        let Some(tagged) = self.captcha_registry.get_mut(&event.key) else {
            warn!("drop captcha: no such key");
            return;
        };
        let Some(generation) = tagged.generation else {
            warn!("drop captcha: no generation stored");
            return;
        };
        if generation != event.generation {
            warn!("drop captcha: generation mismatch");
            return;
        }

        tagged.generation = None;
        tagged.slot = AsyncValue::Ready(event.body);
    }

    fn send_signup_request(&mut self, input: SignupInput) {
        debug_assert!(self.signup_registry.is_some(), "signup state not present");
        let message_tx_clone = self.message_tx.clone();
        let map = move |event: WithGeneration<SignupEvent>| {
            let generation = event.generation;
            match event.result.result {
                Ok(_) => {
                    let _ = message_tx_clone.send(AppMessage::SignupEvent(WithGen::new(
                        Generation(generation),
                        Ok(SignupSuccess),
                    )));
                }
                Err(_e) => {
                    let _ = message_tx_clone.send(AppMessage::SignupEvent(WithGen::new(
                        Generation(generation),
                        Err(SignupError::Failed),
                    )));
                }
            }
        };
        let message_tx_clone = self.message_tx.clone();
        let map_err = move |error: WithGeneration<NetworkError>| {
            let generation = error.generation;
            let _ = message_tx_clone.send(AppMessage::SignupEvent(WithGen::new(
                Generation(generation),
                Err(SignupError::Failed),
            )));
        };
        let tagged = self.signup_registry.as_mut().unwrap();
        tagged.generation = self
            .network
            .borrow_mut()
            .signup(
                input.username,
                input.password,
                input.captcha_id.0,
                input.captcha_answer,
                TIMEOUT_MS,
                Box::new(map),
                Box::new(map_err),
            )
            .ok();
        tagged.slot = AsyncValue::Pending;
    }

    fn receive_signup_event(&mut self, event: WithGen<Result<SignupSuccess, SignupError>>) {
        if self.signup_registry.is_none() {
            warn!("drop signup state: no available slot");
            return;
        }
        if self.signup_registry.as_ref().unwrap().generation.is_none() {
            warn!("drop signup state: no pending request");
            return;
        }

        let tagged = self.signup_registry.as_mut().unwrap();
        if tagged.generation.unwrap() != event.generation.value() {
            warn!("drop signup event: generation mismatch");
            return;
        }

        tagged.generation = None;
        tagged.slot = AsyncValue::Ready(event.body);
    }

    fn send_login_request(&mut self, input: LoginInput) {
        let message_tx_clone = self.message_tx.clone();
        let map = move |event: WithGeneration<LoginEvent>| {
            let generation = event.generation;
            match event.result.result {
                Ok(identity) => {
                    let _ = message_tx_clone.send(AppMessage::LoginEvent(WithGen::new(
                        Generation(generation),
                        Ok(identity),
                    )));
                }
                Err(_e) => {
                    let _ = message_tx_clone.send(AppMessage::LoginEvent(WithGen::new(
                        Generation(generation),
                        Err(LoginError::AuthenticationFailed),
                    )));
                }
            }
        };
        let message_tx_clone = self.message_tx.clone();
        let map_err = move |error: WithGeneration<NetworkError>| {
            let generation = error.generation;
            let _ = message_tx_clone.send(AppMessage::LoginEvent(WithGen::new(
                Generation(generation),
                Err(LoginError::AuthenticationFailed),
            )));
        };
        let tagged = self.login_registry.as_mut().unwrap();
        tagged.generation = self
            .network
            .borrow_mut()
            .login(
                input.username,
                input.password,
                input.captcha_id.0,
                input.captcha_answer,
                TIMEOUT_MS,
                Box::new(map),
                Box::new(map_err),
            )
            .ok();
        tagged.slot = AsyncValue::Pending;
    }

    fn receive_login_event(&mut self, event: WithGen<Result<Identity, LoginError>>) {
        if self.login_registry.is_none() {
            warn!("drop login state: no available slot");
            return;
        }
        if self.login_registry.as_ref().unwrap().generation.is_none() {
            warn!("drop login state: no pending request");
            return;
        }

        let tagged = self.login_registry.as_mut().unwrap();
        if tagged.generation.unwrap() != event.generation.value() {
            warn!("drop login event: generation mismatch");
            return;
        }

        tagged.generation = None;
        tagged.slot = match event.body {
            Ok(identity) => {
                self.identity = Some(identity);
                AsyncValue::Ready(Ok(LoginSuccess))
            }
            Err(error) => AsyncValue::Ready(Err(error)),
        };
    }

    fn open_conversation(&mut self, conversation_id: ConversationId) {
        if self
            .conversation_store
            .as_ref()
            .unwrap()
            .get_conversation_is_fully_synced(conversation_id)
        {
            return;
        }
        let message_tx_clone = self.message_tx.clone();
        let map = move |event: WithGeneration<FetchConversationHistoryEvent>| {
            let generation = event.generation;
            match event.result.result {
                Ok(conversation_history) => {
                    let _ = message_tx_clone.send(AppMessage::ConversationHistory(WithGen::new(
                        Generation(generation),
                        (conversation_id, Ok(conversation_history)),
                    )));
                }
                Err(_e) => {
                    let _ = message_tx_clone.send(AppMessage::ConversationHistory(WithGen::new(
                        Generation(generation),
                        (conversation_id, Err(FetchConversationHistoryError::InternalError)),
                    )));
                }
            }
        };
        let message_tx_clone = self.message_tx.clone();
        let map_err = move |error: WithGeneration<NetworkError>| {
            let generation = error.generation;
            let _ = message_tx_clone.send(AppMessage::ConversationHistory(WithGen::new(
                Generation(generation),
                (conversation_id, Err(FetchConversationHistoryError::InternalError)),
            )));
        };

        let access_token = self
            .identity
            .as_ref()
            .unwrap()
            .auth_tokens
            .access_token
            .clone();
        let generation = self
            .network
            .borrow_mut()
            .fetch_conversation_history(
                access_token,
                conversation_id,
                PageSize(100),
                None,
                TIMEOUT_MS,
                Box::new(map),
                Box::new(map_err),
            )
            .ok();
        self.conversation_store
            .as_mut()
            .unwrap()
            .reconcile_fetch_request_state(
                conversation_id,
                Some(Generation(generation.unwrap())),
                AsyncValue::Pending,
            );
    }

    fn send_friend_list_request(&mut self) {
        let message_tx_clone = self.message_tx.clone();
        let map = move |event: WithGeneration<FetchFriendListEvent>| {
            let generation = event.generation;
            match event.result.result {
                Ok(friend_list) => {
                    let _ = message_tx_clone.send(AppMessage::FriendListEvent(WithGen::new(
                        Generation(generation),
                        Ok(friend_list),
                    )));
                }
                Err(_e) => {
                    let _ = message_tx_clone.send(AppMessage::FriendListEvent(WithGen::new(
                        Generation(generation),
                        Err(FetchFriendListError::InternalError),
                    )));
                }
            }
        };
        let message_tx_clone = self.message_tx.clone();
        let map_err = move |error: WithGeneration<NetworkError>| {
            let generation = error.generation;
            let _ = message_tx_clone.send(AppMessage::FriendListEvent(WithGen::new(
                Generation(generation),
                Err(FetchFriendListError::InternalError),
            )));
        };

        let access_token = self
            .identity
            .as_ref()
            .unwrap()
            .auth_tokens
            .access_token
            .clone();
        let tagged = self.friend_list_registry.as_mut().unwrap();
        tagged.generation = self
            .network
            .borrow_mut()
            .fetch_friend_list(
                access_token,
                PageSize(100),
                None,
                TIMEOUT_MS,
                Box::new(map),
                Box::new(map_err),
            )
            .ok();
        tagged.slot = AsyncValue::Pending;
    }

    fn receive_friend_list_event(&mut self, event: WithGen<Result<Vec<FriendSummary>, FetchFriendListError>>) {
        if self.friend_list_registry.is_none() {
            warn!("drop friend list: no available slot");
            return;
        }
        if self
            .friend_list_registry
            .as_ref()
            .unwrap()
            .generation
            .is_none()
        {
            warn!("drop friend list: no pending request");
            return;
        }

        let tagged = self.friend_list_registry.as_mut().unwrap();
        if tagged.generation.unwrap() != event.generation.value() {
            warn!("drop friend list: generation mismatch");
            return;
        }

        tagged.generation = None;
        tagged.slot = match event.body {
            Ok(friend_list) => {
                for friend in friend_list.iter() {
                    self.user_store
                        .as_mut()
                        .unwrap()
                        .update_user(friend.user_id, &friend.username);
                    self.conversation_store.as_mut().unwrap().reconcile_from_friend(friend.conversation_id, &friend.username);
                }
                AsyncValue::Ready(Ok(friend_list))
            }
            Err(error) => AsyncValue::Ready(Err(error)),
        };
    }

    fn receive_conv_history_event(&mut self, event: WithGen<(ConversationId, Result<Vec<MessageRecord>, FetchConversationHistoryError>)>) {
        let generation = event.generation;
        match event.body.1 {
            Ok(records) => {
                let conversation_store = self.conversation_store.as_mut().unwrap();
                let _ = conversation_store.reconcile_from_fetch(records, true);
                let _ = conversation_store.reconcile_fetch_request_state(
                    event.body.0,
                    Some(generation),
                    AsyncValue::Ready(Ok(())),
                );
            }
            Err(error) => {
                let conversation_store = self.conversation_store.as_mut().unwrap();
                let _ = conversation_store.reconcile_fetch_request_state(
                    event.body.0,
                    Some(generation),
                    AsyncValue::Ready(Err(anyhow::anyhow!("failed to fetch history"))),
                );
            }
        }
    }

    fn send_friendship_request(&mut self, username: String) {
        let message_tx_clone = self.message_tx.clone();
        let map = move |event: WithGeneration<AddFriendEvent>| {
            let generation = event.generation;
            match event.result.result {
                Ok(conversation_id) => {
                    let _ = message_tx_clone.send(AppMessage::AddFriendEvent(
                        WithGen::new(Generation(generation), Ok(conversation_id)),
                    ));
                }
                Err(_e) => {
                    let _ =
                        message_tx_clone.send(AppMessage::AddFriendEvent(WithGen::new(
                            Generation(generation),
                            Err(AddFriendError::InternalError),
                        )));
                }
            }
        };
        let message_tx_clone = self.message_tx.clone();
        let map_err = move |error: WithGeneration<NetworkError>| {
            let generation = error.generation;
            let _ = message_tx_clone.send(AppMessage::AddFriendEvent(WithGen::new(
                Generation(generation),
                Err(AddFriendError::InternalError),
            )));
        };

        let access_token = self
            .identity
            .as_ref()
            .unwrap()
            .auth_tokens
            .access_token
            .clone();
        let tagged = self.add_friend_registry.as_mut().unwrap();
        tagged.generation = self
            .network
            .borrow_mut()
            .add_friend(
                access_token,
                username,
                IdempotencyKey(uuid::Uuid::new_v4()),
                TIMEOUT_MS,
                Box::new(map),
                Box::new(map_err),
            )
            .ok();
        tagged.slot = AsyncValue::Pending;
    }

    fn receive_friendship_event(&mut self, event: WithGen<Result<ConversationId, AddFriendError>>) {
        if self.add_friend_registry.is_none() {
            warn!("drop add friend event: no available slot");
            return;
        }
        if self
            .add_friend_registry
            .as_ref()
            .unwrap()
            .generation
            .is_none()
        {
            warn!("drop add friend event: no pending request");
            return;
        }

        let tagged = self.add_friend_registry.as_mut().unwrap();
        if tagged.generation.unwrap() != event.generation.value() {
            warn!("drop add friend event: generation mismatch");
            return;
        }

        tagged.generation = None;
        tagged.slot = AsyncValue::Idle; // todo!()
        let _ = self.message_tx.try_send(AppMessage::FriendListRequest);
    }

    fn send_connection_request(&mut self) {
        let message_tx_clone = self.message_tx.clone();
        let map = move |event: WithGeneration<SessionEvent>| {
            let generation = event.generation;
            let message = match event.result.result {
                Ok(metadata) => AppMessage::EstablishConnectionEvent(WithGen::new(
                    Generation(generation),
                    Ok(metadata),
                )),
                Err(_e) => AppMessage::EstablishConnectionEvent(WithGen::new(
                    Generation(generation),
                    Err(EstablishError::InternalError),
                )),
            };
            let _ = message_tx_clone.send(message);
        };
        let message_tx_clone = self.message_tx.clone();
        let map_err = move |error: WithGeneration<NetworkError>| {
            let generation = error.generation;
            let _ = message_tx_clone.send(AppMessage::EstablishConnectionEvent(
                WithGen::new(Generation(generation), Err(EstablishError::InternalError)),
            ));
        };

        let message_tx_clone = self.message_tx.clone();
        let access_token = self
            .identity
            .as_ref()
            .unwrap()
            .auth_tokens
            .access_token
            .clone();
        let tagged = self.connection_request_registry.as_mut().unwrap();
        tagged.generation = self
            .network
            .borrow_mut()
            .connect_chat(
                "".to_owned(),
                access_token.0,
                Box::new(move |message| {
                    let _ = message_tx_clone.send(AppMessage::Stream(message));
                }),
                TIMEOUT_MS,
                Box::new(map),
                Box::new(map_err),
            )
            .ok();
    }

    fn receive_connection_event(&mut self, event: WithGen<Result<ChatMetaData, EstablishError>>) {
        if self.connection_request_registry.is_none() {
            warn!("drop connection event: no available slot");
            return;
        }

        if self
            .connection_request_registry
            .as_ref()
            .unwrap()
            .generation
            .is_none()
        {
            warn!("drop connection event: no pending request");
            return;
        }

        let tagged = self.connection_request_registry.as_mut().unwrap();
        if tagged.generation.unwrap() != event.generation.value() {
            warn!("drop connection event: generation mismatch");
        }

        tagged.generation = None;
        tagged.slot = match event.body {
            Ok(metadata) => {
                self.connection = Some(event.generation);
                AsyncValue::Ready(Ok(Connected))
            }
            Err(error) => AsyncValue::Ready(Err(error)),
        };
    }

    fn send_chat_message_request(&mut self, input: ChatMessageInput) {
        let me = self.identity.as_ref().unwrap().user_id;
        let conversation_id = input.conversation_id;
        let message_tx_clone = self.message_tx.clone();
        let map = move |event: WithGeneration<MessageEvent>| {
            let generation = event.generation;
            match event.result.result {
                Ok(ack) => {
                    let _ =
                        message_tx_clone.send(AppMessage::ChatMessageEvent(WithGen::new(
                            Generation(generation),
                            Ok(ChatMessageOk {
                                me,
                                conversation_id: ack.conversation_id,
                                message_id: ack.message_id,
                                message_offset: ack.message_offset,
                                created_at: ack.created_at,
                            }),
                        )));
                }
                Err(_e) => {
                    let _ =
                        message_tx_clone.send(AppMessage::ChatMessageEvent(WithGen::new(
                            Generation(generation),
                            Err(MessageError {
                                conversation_id,
                                kind: MessageErrorKind::InternalError,
                            }),
                        )));
                }
            }
        };
        let message_tx_clone = self.message_tx.clone();
        let map_err = move |error: WithGeneration<NetworkError>| {
            let generation = error.generation;
            let _ = message_tx_clone.send(AppMessage::ChatMessageEvent(WithGen::new(
                Generation(generation),
                Err(MessageError {
                    conversation_id,
                    kind: MessageErrorKind::InternalError,
                }),
            )));
        };

        let task_result = self.network.borrow_mut().send_chat_message(
            input.conversation_id,
            input.message_id,
            input.content.clone(),
            TIMEOUT_MS,
            Box::new(map),
            Box::new(map_err),
        );
        if let Ok(generation) = task_result {
            let conversation_store = self.conversation_store.as_mut().unwrap();
            conversation_store.reconcile_message_request(Generation(generation), input);
        }
    }

    fn receive_chat_message_from_push(&mut self, r: ChatMessageRecv) {
        let record = MessageRecord {
            message_id: r.message_id,
            conversation_id: r.conversation_id,
            message_offset: r.message_offset,
            sender: r.sender,
            content: r.content,
            created_at: r.created_at,
        };
        let conversation_stroe = self.conversation_store.as_mut().unwrap();
        let _ = conversation_stroe.reconcile_from_push(record);
    }

    fn receive_chat_message_event(&mut self, event: WithGen<Result<ChatMessageOk, MessageError>>) {
        let conversation_store = self.conversation_store.as_mut().unwrap();
        conversation_store.reconcile_message_result(event.generation, event.body);
    }
}

impl AppState for RealAppState {
    fn prepare_captcha(&mut self) -> SemanticKey {
        let key = self.key_provider.next();
        self.captcha_registry
            .insert(key, MaybeWithGen::new(None, AsyncValue::Idle));
        key
    }

    fn drop_captcha(&mut self, key: SemanticKey) {
        self.captcha_registry
            .remove(&key)
            .expect(format!("key {} not found", key.value()).as_str());
    }

    fn get_captcha(&self, key: SemanticKey) -> &AsyncValue<CaptchaData, CaptchaError> {
        &self
            .captcha_registry
            .get(&key)
            .expect(format!("key {} not found", key.value()).as_str())
            .slot
    }

    fn prepare_signup_state(&mut self) {
        self.signup_registry = Some(MaybeWithGen::new(None, AsyncValue::Idle));
    }

    fn drop_signup_state(&mut self) {
        self.signup_registry = None;
    }

    fn get_signup_state(&self) -> &AsyncValue<SignupSuccess, SignupError> {
        &self
            .signup_registry
            .as_ref()
            .expect("signup state not initialized")
            .slot
    }

    fn prepare_login_state(&mut self) {
        self.login_registry = Some(MaybeWithGen::new(None, AsyncValue::Idle));
    }

    fn drop_login_state(&mut self) {
        self.login_registry = None;
    }

    fn get_login_state(&self) -> &AsyncValue<LoginSuccess, LoginError> {
        &self
            .login_registry
            .as_ref()
            .expect("login state not initialized")
            .slot
    }

    fn prepare_friend_list(&mut self) {
        self.friend_list_registry = Some(MaybeWithGen::new(None, AsyncValue::Idle));
    }

    fn drop_friend_list(&mut self) {
        self.friend_list_registry = None;
    }

    fn get_friend_list(&self) -> &AsyncValue<Vec<FriendSummary>, FetchFriendListError> {
        &self
            .friend_list_registry
            .as_ref()
            .expect("friend list not initialized")
            .slot
    }

    fn prepare_add_friend_state(&mut self) {
        self.add_friend_registry = Some(MaybeWithGen::new(None, AsyncValue::Idle));
    }

    fn drop_add_friend_state(&mut self) {
        self.add_friend_registry = None;
    }

    fn get_add_friend_state(&self) -> &AsyncValue<ConversationId, AddFriendError> {
        &self
            .add_friend_registry
            .as_ref()
            .expect("add friend state not initialized")
            .slot
    }

    fn prepare_connection(&mut self) {
        self.connection_request_registry = Some(MaybeWithGen::new(None, AsyncValue::Idle));
    }

    fn drop_connection(&mut self) {
        self.connection_request_registry = None;
    }

    fn get_connection_request_state(&self) -> &AsyncValue<Connected, EstablishError> {
        &self
            .connection_request_registry
            .as_ref()
            .expect("connection request state not initialized")
            .slot
    }

    fn get_connection_state(&self) -> &Generation {
        self.connection
            .as_ref()
            .expect("connection not initialized")
    }

    fn try_get_connection_state(&self) -> &Option<Generation> {
        &self.connection
    }

    fn prepare_conversation(&mut self) {
        self.conversation_store = Some(Box::new(ConversationStoreImpl::new()));
        self.user_store = Some(Box::new(UserStoreImpl::new()));
    }

    fn drop_conversation(&mut self) {
        self.conversation_store = None;
        self.user_store = None;
    }

    fn get_conversation_history(&self, conversation_id: ConversationId) -> Vec<HistoryMessage> {
        self.conversation_store
            .as_ref()
            .unwrap()
            .get_conversation_history(conversation_id)
    }

    fn get_conversation_history_version(&self, conversation_id: ConversationId) -> u64 {
        self.conversation_store
            .as_ref()
            .unwrap()
            .get_conversation_history_version(conversation_id)
    }

    fn get_auth_tokens(&self) -> &AuthTokens {
        &self
            .identity
            .as_ref()
            .expect("auth tokens not initialized")
            .auth_tokens
    }

    fn try_get_auth_tokens(&self) -> Option<&AuthTokens> {
        self.identity.as_ref().map(|i| &i.auth_tokens)
    }

    fn update(&mut self, message: AppMessage) {
        match message {
            AppMessage::CaptchaRequest(key) => self.fetch_captcha(key),
            AppMessage::CaptchaEvent(event) => self.receive_captcha(event),
            AppMessage::SignupRequest(input) => self.send_signup_request(input),
            AppMessage::SignupEvent(event) => self.receive_signup_event(event),
            AppMessage::LoginRequest(input) => self.send_login_request(input),
            AppMessage::LoginEvent(event) => self.receive_login_event(event),
            AppMessage::FriendListRequest => self.send_friend_list_request(),
            AppMessage::FriendListEvent(event) => self.receive_friend_list_event(event),
            AppMessage::OpenConversation(conv_id) => self.open_conversation(conv_id),
            AppMessage::ConversationHistory(event) => self.receive_conv_history_event(event),
            AppMessage::AddFriendRequest(username) => self.send_friendship_request(username),
            AppMessage::AddFriendEvent(event) => self.receive_friendship_event(event),
            AppMessage::EstablishConnectionRequest => self.send_connection_request(),
            AppMessage::EstablishConnectionEvent(event) => self.receive_connection_event(event),
            AppMessage::CreateGroup(_) => {}
            AppMessage::ChatMessageRequest(input) => self.send_chat_message_request(input),
            AppMessage::ChatMessageEvent(event) => self.receive_chat_message_event(event),
            AppMessage::Stream(stream_message) => match stream_message {
                StreamMessage::ChatMessageRecv(r) => self.receive_chat_message_from_push(r),
                StreamMessage::FriendshipRecv(_) => self.send_friend_list_request(),
                StreamMessage::Distribute(_) => {}
            },
        }
    }
}

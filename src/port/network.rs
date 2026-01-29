use crate::domain::*;
use chrono::{DateTime, Utc};
use std::fmt::Debug;
use uuid::Uuid;

// NOTE: timeout unit in this file is ms

pub trait Network {
    fn fetch_captcha(
        &mut self,
        timeout: u64,
        map_function: Box<dyn FnOnce(WithGeneration<CaptchaEvent>) + Send + Sync>,
        err_function: Box<dyn FnOnce(WithGeneration<NetworkError>) + Send + Sync>,
    ) -> anyhow::Result<u64>;
    fn signup(
        &mut self,
        username: String,
        password: String,
        captcha_id: Uuid,
        captcha_answer: String,
        timeout: u64,
        map_function: Box<dyn FnOnce(WithGeneration<SignupEvent>) + Send + Sync>,
        err_function: Box<dyn FnOnce(WithGeneration<NetworkError>) + Send + Sync>,
    ) -> anyhow::Result<u64>;
    fn login(
        &mut self,
        username: String,
        password: String,
        captcha_id: Uuid,
        captcha_answer: String,
        timeout: u64,
        map_function: Box<dyn FnOnce(WithGeneration<LoginEvent>) + Send + Sync>,
        err_function: Box<dyn FnOnce(WithGeneration<NetworkError>) + Send + Sync>,
    ) -> anyhow::Result<u64>;
    fn fetch_friend_list(
        &mut self,
        token: AccessToken,
        page_size: PageSize,
        cursor: Option<FriendCursor>,
        timeout: u64,
        map_function: Box<dyn FnOnce(WithGeneration<FetchFriendListEvent>) + Send + Sync>,
        err_function: Box<dyn FnOnce(WithGeneration<NetworkError>) + Send + Sync>,
    ) -> anyhow::Result<u64>;
    fn add_friend(
        &mut self,
        token: AccessToken,
        other: String,
        key: IdempotencyKey,
        timeout: u64,
        map_function: Box<dyn FnOnce(WithGeneration<AddFriendEvent>) + Send + Sync>,
        err_function: Box<dyn FnOnce(WithGeneration<NetworkError>) + Send + Sync>,
    ) -> anyhow::Result<u64>;
    fn fetch_conversation_history(
        &mut self,
        token: AccessToken,
        conversation_id: ConversationId,
        page_size: PageSize,
        cursor: Option<OffsetCursor>,
        timeout: u64,
        map_function: Box<dyn FnOnce(WithGeneration<FetchConversationHistoryEvent>) + Send + Sync>,
        err_function: Box<dyn FnOnce(WithGeneration<NetworkError>) + Send + Sync>,
    ) -> anyhow::Result<u64>;
    fn cancel(&mut self, generation: u64) -> anyhow::Result<()>;
    fn connect_chat(
        &mut self,
        address: String,
        jwt: String,
        msg_function: Box<dyn Fn(StreamMessage) + Send + Sync>,
        timeout: u64,
        map_function: Box<dyn FnOnce(WithGeneration<SessionEvent>) + Send + Sync>,
        err_function: Box<dyn FnOnce(WithGeneration<NetworkError>) + Send + Sync>,
    ) -> anyhow::Result<u64>;
    fn send_chat_message(
        &mut self,
        conversation_id: ConversationId,
        message_id: MessageId,
        content: String,
        timeout: u64,
        map_function: Box<dyn FnOnce(WithGeneration<MessageEvent>) + Send + Sync>,
        err_function: Box<dyn FnOnce(WithGeneration<NetworkError>) + Send + Sync>,
    ) -> anyhow::Result<u64>;
}

pub type NetworkResult = Result<NetworkEvent, NetworkError>;

#[derive(Debug)]
pub struct WithGeneration<T> {
    pub generation: u64,
    pub result: T,
}

#[derive(Debug)]
pub enum NetworkError {
    Aborted,
    SysCancelled,
    UsrCancelled,
    Timeout,
}

#[derive(Debug)]
pub enum NetworkEvent {
    Captcha(CaptchaEvent),
    Signup(SignupEvent),
    Login(LoginEvent),
    FetchFriendList(FetchFriendListEvent),
    AddFriend(AddFriendEvent),
    FetchConversationHistory(FetchConversationHistoryEvent),
    EstablishEvent(SessionEvent),
    Session(SessionEvent),
    ChatMessageSent(MessageEvent),
}

#[derive(Debug)]
pub struct CaptchaEvent {
    pub result: Result<CaptchaData, CaptchaError>,
}

pub struct CaptchaData {
    pub id: Uuid,
    pub image_base64: String,
}

impl Debug for CaptchaData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CaptchaData")
            .field("id", &self.id)
            .field(
                "image_base64",
                &self.image_base64.chars().take(64).collect::<String>(),
            )
            .finish()
    }
}

#[derive(Debug)]
pub enum CaptchaError {
    FallbackError,
}

#[derive(Debug)]
pub struct SignupEvent {
    pub result: Result<(), SignupError>,
}

#[derive(Debug)]
pub enum SignupError {
    DuplicateName,
    WeakPassword,
    WrongCaptcha,
    FallbackError,
}

#[derive(Debug)]
pub struct LoginEvent {
    pub result: Result<Identity, LoginError>,
}

#[derive(Debug)]
pub struct Identity {
    pub user_id: UserId,
    pub auth_tokens: AuthTokens,
}

#[derive(Debug)]
pub enum LoginError {
    Unauthorized,
    WrongCaptcha,
    FallbackError,
}

#[derive(Debug)]
pub struct FetchFriendListEvent {
    pub result: Result<Vec<FriendSummary>, FetchFriendListError>,
}

#[derive(Debug)]
pub enum FetchFriendListError {
    InternalError,
}

#[derive(Debug)]
pub struct AddFriendEvent {
    pub result: Result<ConversationId, AddFriendError>,
}

#[derive(Debug)]
pub enum AddFriendError {
    InternalError,
}

#[derive(Debug)]
pub struct FetchConversationHistoryEvent {
    pub result: Result<Vec<MessageRecord>, FetchConversationHistoryError>,
}

#[derive(Debug)]
pub enum FetchConversationHistoryError {
    InternalError,
}

#[derive(Debug)]
pub struct SessionEvent {
    pub result: Result<ChatMetaData, ChatConnError>,
}

#[derive(Debug)]
pub struct ChatMetaData;

#[derive(Debug)]
pub enum ChatConnError {
    FallbackError,
}

#[derive(Debug)]
pub struct MessageEvent {
    pub result: Result<ChatMessageSent, MessageError>,
}

#[derive(Debug)]
pub struct ChatMessageSent {
    pub conversation_id: ConversationId,
    pub message_id: MessageId,
    pub message_offset: MessageOffset,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct MessageSent;

#[derive(Debug)]
pub enum MessageError {
    MissingSession,
    FallbackError,
}

#[derive(Debug)]
pub enum StreamMessage {
    ChatMessageRecv(ChatMessageRecv),
    FriendshipRecv(FriendshipRecv),
    Distribute(ChatMessage),
}

#[derive(Debug)]
pub struct ChatMessageRecv {
    pub conversation_id: ConversationId,
    pub message_id: MessageId,
    pub message_offset: MessageOffset,
    pub content: String,
    pub sender: UserId,
    pub username: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct FriendshipRecv {
    pub conversation_id: ConversationId,
    pub other: UserId,
    pub username: String,
}

#[derive(Debug)]
pub struct ChatMessage {
    pub sender: UserId,
    pub conversation_id: ConversationId,
    pub content: String,
}

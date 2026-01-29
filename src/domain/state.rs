use std::cell::Ref;
use crate::common::*;
use crate::domain::*;
use crate::infra::network::{AddFriendError, ChatMetaData, FetchFriendListError, Identity};

pub trait AppState {
    fn prepare_captcha(&mut self) -> SemanticKey;
    fn drop_captcha(&mut self, key: SemanticKey);
    fn get_captcha(&self, key: SemanticKey) -> &AsyncValue<CaptchaData, CaptchaError>;

    fn prepare_signup_state(&mut self);
    fn drop_signup_state(&mut self);
    fn get_signup_state(&self) -> &AsyncValue<SignupSuccess, SignupError>;

    fn prepare_login_state(&mut self);
    fn drop_login_state(&mut self);
    fn get_login_state(&self) -> &AsyncValue<LoginSuccess, LoginError>;

    fn prepare_friend_list(&mut self);
    fn drop_friend_list(&mut self);
    fn get_friend_list(&self) -> &AsyncValue<Vec<FriendSummary>, FetchFriendListError>;

    fn prepare_add_friend_state(&mut self);
    fn drop_add_friend_state(&mut self);
    fn get_add_friend_state(&self) -> &AsyncValue<ConversationId, AddFriendError>;

    fn prepare_connection(&mut self);
    fn drop_connection(&mut self);
    fn get_connection_request_state(&self) -> &AsyncValue<Connected, EstablishError>;

    fn get_connection_state(&self) -> &Generation;
    fn try_get_connection_state(&self) -> &Option<Generation>;

    fn prepare_conversation(&mut self);
    fn drop_conversation(&mut self);
    fn get_conversation_history(&self, conversation_id: ConversationId) -> Vec<HistoryMessage>;
    fn get_conversation_history_version(&self, conversation_id: ConversationId) -> u64;

    fn get_auth_tokens(&self) -> &AuthTokens;
    fn try_get_auth_tokens(&self) -> Option<&AuthTokens>;

    fn update(&mut self, message: AppMessage);
}

pub trait DebugState {
    fn set_captcha(&mut self, base64: &str);
}

#[derive(Debug)]
pub struct LoginSuccess;

#[derive(Debug)]
pub enum LoginError {
    AuthenticationFailed,
    ConnectionFailed,
    SyncFailed,
}

#[derive(Debug, Clone)]
pub struct SignupSuccess;

#[derive(Debug, Clone)]
pub enum SignupError {
    Failed,
}

#[derive(Debug, Clone)]
pub struct Connected;

#[derive(Debug, Clone)]
pub enum EstablishError {
    InternalError,
}

#[derive(Debug)]
pub enum HistoryMessage {
    Concrete(MessageRecord),
    Request(ChatMessageInput),
}


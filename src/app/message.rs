use crate::common::*;
use crate::domain::*;
use crate::port::network::{
    AddFriendError, ChatMetaData, FetchConversationHistoryError, FetchFriendListError, Identity,
    StreamMessage,
};

#[derive(Debug)]
pub enum AppMessage {
    CaptchaRequest(SemanticKey),
    CaptchaEvent(WithGenAndKey<Result<CaptchaData, CaptchaError>>),
    SignupRequest(SignupInput),
    SignupEvent(WithGen<Result<SignupSuccess, SignupError>>),
    LoginRequest(LoginInput),
    LoginEvent(WithGen<Result<Identity, LoginError>>),
    FriendListRequest,
    FriendListEvent(WithGen<Result<Vec<FriendSummary>, FetchFriendListError>>),
    OpenConversation(ConversationId),
    ConversationHistory(
        WithGen<(
            ConversationId,
            Result<Vec<MessageRecord>, FetchConversationHistoryError>,
        )>,
    ),
    AddFriendRequest(String),
    AddFriendEvent(WithGen<Result<ConversationId, AddFriendError>>),
    EstablishConnectionRequest,
    EstablishConnectionEvent(WithGen<Result<ChatMetaData, EstablishError>>),
    CreateGroup(String),
    ChatMessageRequest(ChatMessageInput),
    ChatMessageEvent(WithGen<Result<ChatMessageOk, MessageError>>),
    Stream(StreamMessage),
}

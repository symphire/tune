use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize)]
pub struct IdempotencyKey(pub uuid::Uuid);

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct CaptchaId(pub uuid::Uuid);
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ConversationId(pub uuid::Uuid);

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct MessageId(pub uuid::Uuid);

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct UserId(pub uuid::Uuid);

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct Image(pub String);

impl Image {
    pub fn new(base_64: &str) -> Image {
        Image(base_64.to_string())
    }
    pub fn to_base64(&self) -> &str {
        &*self.0
    }
}

impl std::fmt::Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0.chars().take(16).collect::<String>())
    }
}

pub struct Conversation {
    pub id: ConversationId,
    pub name: String,
    pub kind: ConversationKind,
}

pub enum ConversationKind {
    Group,
    Direct,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AccessToken(pub String);
#[derive(Debug, Clone, Deserialize)]
pub struct RefreshToken(pub String);

#[derive(Debug, Deserialize)]
pub struct AuthTokens {
    pub access_token: AccessToken,
    pub refresh_token: RefreshToken,
    pub access_token_expires_at: DateTime<Utc>,
    pub refresh_token_expires_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct CaptchaData {
    pub id: CaptchaId,
    pub image: Image,
}

#[derive(Debug)]
pub struct CaptchaError;

#[derive(Debug)]
pub struct SignupInput {
    pub username: String,
    pub password: String,
    pub captcha_id: CaptchaId,
    pub captcha_answer: String,
}

#[derive(Debug)]
pub struct LoginInput {
    pub username: String,
    pub password: String,
    pub captcha_id: CaptchaId,
    pub captcha_answer: String,
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PageSize(pub u16);

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct FriendCursor {
    pub since: DateTime<Utc>,
    pub other_user: UserId, // tiebreaker
}

impl std::fmt::Display for FriendCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}~{}", self.since.to_rfc3339(), &self.other_user)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct FriendSummary {
    pub user_id: UserId,
    pub username: String,
    pub conversation_id: ConversationId,
    pub since: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct MessageOffset(pub u64);

impl std::fmt::Display for MessageOffset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct OffsetCursor {
    pub offset: MessageOffset,
}

impl std::fmt::Display for OffsetCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.offset)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageRecord {
    pub message_id: MessageId,
    pub conversation_id: ConversationId,
    pub message_offset: MessageOffset,
    pub sender: UserId,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageInput {
    pub conversation_id: ConversationId,
    pub message_id: MessageId,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct ChatMessageOk {
    pub me: UserId,
    pub conversation_id: ConversationId,
    pub message_id: MessageId,
    pub message_offset: MessageOffset,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct MessageError {
    pub conversation_id: ConversationId,
    pub kind: MessageErrorKind,
}

#[derive(Debug)]
pub enum MessageErrorKind {
    InternalError,
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

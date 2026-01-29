use crate::domain;
use crate::domain::{AuthTokens, ConversationId, IdempotencyKey, PageSize, UserId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub const API_BASE_URL: &str = "https://127.0.0.1:8443/api/v1";
pub const CAPTCHA_SUFFIX: &str = "captcha";
pub const SIGNUP_SUFFIX: &str = "signup";
pub const LOGIN_SUFFIX: &str = "login";
pub const FRIEND_LIST_SUFFIX: &str = "friend_list";
pub const ADD_FRIEND_SUFFIX: &str = "add_friend";
pub const CONVERSATION_HISTORY_SUFFIX: &str = "conversation_history";

#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ApiError>,
}

#[derive(Debug, Deserialize)]
pub struct ApiError {
    pub code: ApiErrorCode,
    pub message: String,
}

#[derive(Debug, Clone, Error, Deserialize)]
pub enum ApiErrorCode {
    #[error("Invalid captcha ID or answer")]
    InvalidCaptcha,
    #[error("Invalid username or password")]
    InvalidCredentials,
    #[error("Username already taken")]
    UsernameTaken,
    #[error("Token is not valid")]
    InvalidToken,
    #[error("Internal error")]
    InternalError,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptchaResponse {
    pub id: Uuid,
    pub image_base64: String,
    pub expire_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct SignupRequest {
    pub username: String,
    pub password: String,
    pub captcha_id: Uuid,
    pub captcha_answer: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignupResponse;

#[derive(Debug, Serialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub captcha_id: Uuid,
    pub captcha_answer: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoginResponse {
    pub user_id: UserId,
    pub auth_tokens: AuthTokens,
}

#[derive(Debug, Serialize)]
pub struct FriendListQuery {
    pub page_size: PageSize,
    pub after: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AddFriendRequest {
    pub other: String,
    pub key: IdempotencyKey,
}

#[derive(Debug, Serialize)]
pub struct ConversationHistoryQuery {
    pub conversation_id: ConversationId,
    pub page_size: PageSize,
    pub before: Option<String>,
}

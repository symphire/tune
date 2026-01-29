use crate::domain::{
    AccessToken, ConversationId, FriendCursor, FriendSummary, IdempotencyKey, MessageRecord,
    OffsetCursor, PageSize, UserId,
};
use crate::infra::network::http_api_v1::{
    AddFriendRequest, ApiResponse, CaptchaResponse, ConversationHistoryQuery, FriendListQuery,
    LoginRequest, LoginResponse, SignupRequest, SignupResponse, ADD_FRIEND_SUFFIX, API_BASE_URL,
    CAPTCHA_SUFFIX, CONVERSATION_HISTORY_SUFFIX, FRIEND_LIST_SUFFIX, LOGIN_SUFFIX, SIGNUP_SUFFIX,
};
use crate::infra::network::HttpWorker;
use crate::port::network::{CaptchaData, Identity};
use reqwest::Client;
use std::fs;
use uuid::Uuid;

fn endpoint_url(suffix: &str) -> String {
    format!(
        "{}/{}",
        API_BASE_URL.trim_end_matches('/'),
        suffix.trim_start_matches('/')
    )
}

#[derive(Clone)]
pub struct RealHttpWorker {
    client: Client,
}

impl RealHttpWorker {
    pub fn new() -> Self {
        let cert = fs::read("certs/dev_cert.pem").expect("Failed to read certificate");
        let cert = reqwest::Certificate::from_pem(&cert).expect("Failed to parse cert");

        let client = Client::builder()
            .add_root_certificate(cert)
            .no_proxy()
            .build()
            .expect("Failed to build http client");
        Self { client }
    }
}

#[async_trait::async_trait]
impl HttpWorker for RealHttpWorker {
    async fn fetch_captcha(&self) -> anyhow::Result<CaptchaData> {
        let response = self.client.get(endpoint_url(CAPTCHA_SUFFIX)).send().await?;
        let response: CaptchaResponse = response.json().await?;
        let captcha_data = CaptchaData {
            id: response.id,
            image_base64: response.image_base64,
        };

        Ok(captcha_data)
    }

    async fn signup(
        &self,
        username: String,
        password: String,
        captcha_id: Uuid,
        captcha_answer: String,
    ) -> anyhow::Result<()> {
        let request = SignupRequest {
            username,
            password,
            captcha_id,
            captcha_answer,
        };

        let response = self
            .client
            .post(endpoint_url(SIGNUP_SUFFIX))
            .json(&request)
            .send()
            .await?;

        let body_bytes = response.bytes().await?;
        tracing::trace!(
            "signup response body: {:?}",
            String::from_utf8_lossy(&body_bytes)
        );

        let response: ApiResponse<SignupResponse> = serde_json::from_slice(&body_bytes)?;

        Ok(())
    }

    async fn login(
        &self,
        username: String,
        password: String,
        captcha_id: Uuid,
        captcha_answer: String,
    ) -> anyhow::Result<Identity> {
        let request = LoginRequest {
            username,
            password,
            captcha_id,
            captcha_answer,
        };

        let response = self
            .client
            .post(endpoint_url(LOGIN_SUFFIX))
            .json(&request)
            .send()
            .await?;

        let body_bytes = response.bytes().await?;
        tracing::trace!(
            "login response body: {:?}",
            String::from_utf8_lossy(&body_bytes)
        );

        let response: ApiResponse<LoginResponse> = serde_json::from_slice(&body_bytes)?;

        let identity = response
            .data
            .map(|data| Identity {
                user_id: data.user_id,
                auth_tokens: data.auth_tokens,
            })
            .ok_or(anyhow::anyhow!("failed to extract login data"))?;

        Ok(identity)
    }

    async fn fetch_friend_list(
        &self,
        token: AccessToken,
        page_size: PageSize,
        cursor: Option<FriendCursor>,
    ) -> anyhow::Result<Vec<FriendSummary>> {
        let cursor = match cursor {
            Some(cursor) => Some(cursor.to_string()),
            None => None,
        };
        let query = FriendListQuery {
            page_size,
            after: cursor,
        };

        let response = self
            .client
            .get(endpoint_url(FRIEND_LIST_SUFFIX))
            .bearer_auth(&token.0)
            .query(&query)
            .send()
            .await?;

        let data = response.json::<ApiResponse<Vec<FriendSummary>>>().await?;
        Ok(data
            .data
            .ok_or(anyhow::anyhow!("fetch_friend_list data parse error"))?)
    }

    async fn add_friend(
        &self,
        token: AccessToken,
        other: &str,
        key: IdempotencyKey,
    ) -> anyhow::Result<ConversationId> {
        let request = AddFriendRequest {
            other: other.to_owned(),
            key,
        };

        let response = self
            .client
            .post(endpoint_url(ADD_FRIEND_SUFFIX))
            .bearer_auth(&token.0)
            .json(&request)
            .send()
            .await?;
        let body_bytes = response.bytes().await?;
        tracing::trace!(
            "add_friend response body: {:?}",
            String::from_utf8_lossy(&body_bytes)
        );

        let response: ApiResponse<ConversationId> = serde_json::from_slice(&body_bytes)?;
        Ok(response
            .data
            .ok_or(anyhow::anyhow!("add_friend data parse error"))?)
    }

    async fn fetch_conversation_history(
        &self,
        token: AccessToken,
        conversation_id: ConversationId,
        page_size: PageSize,
        cursor: Option<OffsetCursor>,
    ) -> anyhow::Result<Vec<MessageRecord>> {
        let cursor = match cursor {
            Some(cursor) => Some(cursor.to_string()),
            None => None,
        };
        let request = ConversationHistoryQuery {
            conversation_id,
            page_size,
            before: cursor,
        };

        let response = self
            .client
            .get(endpoint_url(CONVERSATION_HISTORY_SUFFIX))
            .bearer_auth(&token.0)
            .query(&request)
            .send()
            .await?;

        let body_bytes = response.bytes().await?;
        tracing::trace!(
            "conversation_history response: {:?}",
            String::from_utf8_lossy(&body_bytes)
        );

        let response: ApiResponse<Vec<MessageRecord>> = serde_json::from_slice(&body_bytes)?;
        Ok(response
            .data
            .ok_or(anyhow::anyhow!("fetch_conversation_history data error"))?)
    }

    fn clone_box(&self) -> Box<dyn HttpWorker> {
        Box::new(self.clone())
    }
}

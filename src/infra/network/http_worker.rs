use crate::domain::{
    AccessToken, ConversationId, FriendCursor, FriendSummary, IdempotencyKey, MessageRecord,
    OffsetCursor, PageSize, UserId,
};
use crate::port::network::{CaptchaData, Identity};
use uuid::Uuid;

#[async_trait::async_trait]
pub trait HttpWorker: Send + Sync {
    async fn fetch_captcha(&self) -> anyhow::Result<CaptchaData>;
    async fn signup(
        &self,
        username: String,
        password: String,
        captcha_id: Uuid,
        captcha_answer: String,
    ) -> anyhow::Result<()>;
    async fn login(
        &self,
        username: String,
        password: String,
        captcha_id: Uuid,
        captcha_answer: String,
    ) -> anyhow::Result<Identity>;
    async fn fetch_friend_list(
        &self,
        token: AccessToken,
        page_size: PageSize,
        cursor: Option<FriendCursor>,
    ) -> anyhow::Result<Vec<FriendSummary>>;
    async fn add_friend(
        &self,
        token: AccessToken,
        other: &str,
        key: IdempotencyKey,
    ) -> anyhow::Result<ConversationId>;
    async fn fetch_conversation_history(
        &self,
        token: AccessToken,
        conversation_id: ConversationId,
        page_size: PageSize,
        cursor: Option<OffsetCursor>,
    ) -> anyhow::Result<Vec<MessageRecord>>;

    fn clone_box(&self) -> Box<dyn HttpWorker>;
}

impl Clone for Box<dyn HttpWorker> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

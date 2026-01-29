use crate::domain::{ConversationId, MessageId};

#[async_trait::async_trait]
pub trait WsWorker: Send + Sync {
    async fn send_message(&self, message_id: MessageId, conversation_id: ConversationId, content: String) -> anyhow::Result<()>;
}
use crate::common::{AsyncValue, Generation, MaybeWithGen};
use crate::domain::HistoryMessage;
use crate::domain::{
    ChatMessageInput, ChatMessageOk, ConversationId, MessageError, MessageOffset, MessageRecord,
    UserId,
};

pub trait ConversationStore {
    fn reconcile_from_friend(&mut self, conversation_id: ConversationId, title: &str);
    fn get_conversation_is_need_sync(&self, conversation_id: ConversationId) -> bool;
    fn get_conversation_is_fully_synced(&self, conversation_id: ConversationId) -> bool;
    fn reconcile_fetch_request_state(
        &mut self,
        conversation_id: ConversationId,
        generation: Option<Generation>,
        value: AsyncValue<(), anyhow::Error>,
    );
    fn reconcile_from_fetch(
        &mut self,
        trunk: Vec<MessageRecord>,
        is_fully_synced: bool,
    ) -> anyhow::Result<()>;
    fn reconcile_from_push(&mut self, message: MessageRecord) -> anyhow::Result<()>;
    fn reconcile_message_request(&mut self, generation: Generation, input: ChatMessageInput);
    fn reconcile_message_result(
        &mut self,
        generation: Generation,
        result: Result<ChatMessageOk, MessageError>,
    );
    fn get_conversation_history(&self, conversation_id: ConversationId) -> Vec<HistoryMessage>;
    fn get_conversation_history_version(&self, conversation_id: ConversationId) -> u64;
}

#[derive(Debug)]
pub enum MetadataKind {
    Active,
    Placeholder,
}

#[derive(Debug)]
pub struct ConversationMetadata {
    pub id: ConversationId,
    pub title: String,
    pub kind: MetadataKind,
}

#[derive(Debug)]
pub struct SyncState {
    pub version: u64,
    pub last_server_message_offset: Option<MessageOffset>, // checkpoint for "newer"
    pub oldest_local_message_offset: Option<MessageOffset>, // checkpoint for "older"
    pub fully_synced_to_oldest: bool, // whether full history loaded (oldest - last)
    pub need_sync: bool,              // signals missing messages
    pub sync_request_state: MaybeWithGen<AsyncValue<(), anyhow::Error>>, // whether a request is sent
}

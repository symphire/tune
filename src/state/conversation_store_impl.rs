use super::conversation_store::*;
use crate::common::{AsyncValue, Generation, MaybeWithGen};
use crate::domain::HistoryMessage;
use crate::domain::{
    ChatMessageInput, ChatMessageOk, ConversationId, MessageError, MessageId, MessageOffset,
    MessageRecord,
};
use chrono::{DateTime, Utc};
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::AtomicU64;

#[derive(Debug)]
struct ChatMessageRequest {
    input: ChatMessageInput,
    state: AsyncValue<(), anyhow::Error>,
}

struct RegistryItem {
    pub metadata: ConversationMetadata,
    pub sync_state: SyncState,
    pub messages: BTreeMap<MessageOffset, MessageRecord>,
    pub requests: HashMap<Generation, ChatMessageRequest>,
}

pub struct ConversationStoreImpl {
    version: AtomicU64,
    registry: HashMap<ConversationId, RegistryItem>,
    metadata_registry: HashMap<ConversationId, ConversationMetadata>,
    sync_tracker: HashMap<ConversationId, SyncState>,
    message_registry: HashMap<ConversationId, BTreeMap<MessageOffset, MessageRecord>>,
    request_registry: HashMap<ConversationId, HashMap<Generation, ChatMessageRequest>>,
}

impl ConversationStoreImpl {
    pub fn new() -> ConversationStoreImpl {
        ConversationStoreImpl {
            version: AtomicU64::new(1),
            registry: HashMap::new(),
            metadata_registry: HashMap::new(),
            sync_tracker: HashMap::new(),
            message_registry: HashMap::new(),
            request_registry: HashMap::new(),
        }
    }

    fn ensure_active(&mut self, id: ConversationId) {
        self.ensure_item(id, MetadataKind::Active);
    }

    fn ensure_placeholder(&mut self, id: ConversationId) {
        self.ensure_item(id, MetadataKind::Placeholder);
    }

    fn ensure_item(&mut self, id: ConversationId, kind: MetadataKind) {
        if !self.registry.contains_key(&id) {
            let item = RegistryItem {
                metadata: ConversationMetadata {
                    id,
                    title: "".to_string(),
                    kind,
                },
                sync_state: SyncState {
                    version: 0,
                    last_server_message_offset: None,
                    oldest_local_message_offset: None,
                    fully_synced_to_oldest: false,
                    need_sync: false,
                    sync_request_state: MaybeWithGen {
                        generation: None,
                        slot: AsyncValue::Idle,
                    },
                },
                messages: BTreeMap::new(),
                requests: HashMap::new(),
            };
            self.registry.insert(id, item);
        } else {
            self.registry.get_mut(&id).unwrap().metadata.kind = kind;
        }
    }

    fn create_conv_metadata(&mut self, conversation_id: ConversationId, title: &str) {
        self.metadata_registry.insert(
            conversation_id,
            ConversationMetadata {
                id: conversation_id,
                title: title.to_owned(),
                kind: MetadataKind::Active,
            },
        );
    }

    fn create_conv_metadata_placeholder(&mut self, conversation_id: ConversationId) {
        self.metadata_registry.insert(
            conversation_id,
            ConversationMetadata {
                id: conversation_id,
                title: "".to_owned(),
                kind: MetadataKind::Placeholder,
            },
        );
    }

    fn create_conv_sync_state(&mut self, conversation_id: ConversationId) {
        self.sync_tracker.insert(
            conversation_id,
            SyncState {
                version: 0,
                last_server_message_offset: None,
                oldest_local_message_offset: None,
                fully_synced_to_oldest: false,
                need_sync: true,
                sync_request_state: MaybeWithGen::new(None, AsyncValue::Idle),
            },
        );
    }

    fn update_conv_metadata(&mut self, conversation_id: &ConversationId, title: &str) {
        if let Some(metadata) = self.metadata_registry.get_mut(&conversation_id) {
            match metadata.kind {
                MetadataKind::Active => metadata.title = title.to_owned(),
                MetadataKind::Placeholder => {
                    metadata.title = title.to_owned();
                    metadata.kind = MetadataKind::Active;
                }
            }
        }
    }
}

impl ConversationStore for ConversationStoreImpl {
    fn reconcile_from_friend(&mut self, conversation_id: ConversationId, title: &str) {
        self.ensure_active(conversation_id);
        let item = self.registry.get_mut(&conversation_id).unwrap();

        item.metadata.title = title.to_owned();
    }

    fn get_conversation_is_need_sync(&self, conversation_id: ConversationId) -> bool {
        if let Some(item) = self.registry.get(&conversation_id) {
            return item.sync_state.need_sync;
        }
        unreachable!();
    }

    fn get_conversation_is_fully_synced(&self, conversation_id: ConversationId) -> bool {
        if let Some(item) = self.registry.get(&conversation_id) {
            return item.sync_state.fully_synced_to_oldest;
        }
        unreachable!();
    }

    fn reconcile_fetch_request_state(
        &mut self,
        conversation_id: ConversationId,
        generation: Option<Generation>,
        value: AsyncValue<(), anyhow::Error>,
    ) {
        self.ensure_active(conversation_id);
        let item = self.registry.get_mut(&conversation_id).unwrap();

        item.sync_state.sync_request_state.generation = generation.map(|g| g.value());
        item.sync_state.sync_request_state.slot = value;
    }

    fn reconcile_from_fetch(
        &mut self,
        trunk: Vec<MessageRecord>,
        is_fully_synced: bool,
    ) -> anyhow::Result<()> {
        if trunk.is_empty() {
            return Ok(());
        }

        let conversation_id = trunk[0].conversation_id;
        self.ensure_active(conversation_id);
        let item = self.registry.get_mut(&conversation_id).unwrap();

        let last = trunk.last().unwrap();
        let max_offset = last.message_offset;

        let sync_state = &mut item.sync_state;
        if !(sync_state.last_server_message_offset.is_some()
            && sync_state.last_server_message_offset.unwrap() > max_offset)
        {
            sync_state.last_server_message_offset = Some(max_offset);
        }
        sync_state.fully_synced_to_oldest = is_fully_synced;
        sync_state.version = self
            .version
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        for record in trunk {
            item.messages.insert(record.message_offset, record);
        }

        Ok(())
    }

    fn reconcile_from_push(&mut self, message: MessageRecord) -> anyhow::Result<()> {
        self.ensure_active(message.conversation_id);
        let item = self.registry.get_mut(&message.conversation_id).unwrap();

        let messages = &mut item.messages;
        let sync_state = &mut item.sync_state;
        if (sync_state.last_server_message_offset.is_some()
            && sync_state.last_server_message_offset.unwrap() < message.message_offset)
            || sync_state.last_server_message_offset.is_none()
        {
            sync_state.last_server_message_offset = Some(message.message_offset);
            sync_state.version = self
                .version
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            messages.insert(message.message_offset, message);
        } else {
            tracing::debug!(
                "drop push chat message: already exists [{:?}-{:?}]",
                message.conversation_id,
                message.message_offset
            );
        }

        Ok(())
    }

    fn reconcile_message_request(&mut self, generation: Generation, input: ChatMessageInput) {
        self.ensure_active(input.conversation_id);
        let item = self.registry.get_mut(&input.conversation_id).unwrap();

        item.requests.insert(
            generation,
            ChatMessageRequest {
                input,
                state: AsyncValue::Pending,
            },
        );
        item.sync_state.version = self
            .version
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    fn reconcile_message_result(
        &mut self,
        generation: Generation,
        result: Result<ChatMessageOk, MessageError>,
    ) {
        let conversation_id = match &result {
            Ok(o) => o.conversation_id,
            Err(e) => e.conversation_id,
        };
        self.ensure_active(conversation_id);
        let item = self.registry.get_mut(&conversation_id).unwrap();

        let requests = &mut item.requests;
        if !requests.contains_key(&generation) {
            tracing::warn!(
                "drop chat message ok: generation not found [{:?}]",
                generation
            );
            return;
        };

        let messages = &mut item.messages;
        let sync_state = &mut item.sync_state;
        match result {
            Ok(message_ok) => {
                let request = requests.remove(&generation).unwrap();
                if (sync_state.last_server_message_offset.is_some()
                    && sync_state.last_server_message_offset.unwrap() < message_ok.message_offset)
                    || sync_state.last_server_message_offset.is_none()
                {
                    sync_state.last_server_message_offset = Some(message_ok.message_offset);
                    messages.insert(
                        message_ok.message_offset,
                        MessageRecord {
                            message_id: message_ok.message_id,
                            conversation_id: message_ok.conversation_id,
                            message_offset: message_ok.message_offset,
                            sender: message_ok.me,
                            content: request.input.content,
                            created_at: message_ok.created_at,
                        },
                    );
                } else {
                    tracing::debug!(
                        "drop chat message ok: already exists [{:?}-{:?}]",
                        message_ok.conversation_id,
                        message_ok.message_offset
                    );
                }
            }
            Err(error) => {
                let request = requests.get_mut(&generation).unwrap();
                request.state = AsyncValue::Ready(Err(anyhow::anyhow!("{:?}", error.kind)));
            }
        }
        sync_state.version = self
            .version
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    fn get_conversation_history(&self, conversation_id: ConversationId) -> Vec<HistoryMessage> {
        #[derive(Debug)]
        enum Sortable<'a> {
            Concrete(&'a MessageRecord),
            Request(&'a ChatMessageInput),
        }

        let mut sorted: Vec<(DateTime<Utc>, Sortable)> = Vec::new();

        let item = self.registry.get(&conversation_id).unwrap();

        // ---- 1. Collect confirmed messages ----
        for record in item.messages.values() {
            sorted.push((record.created_at, Sortable::Concrete(record)));
        }

        // ---- 2. Collect pending requests ----
        for request in item.requests.values() {
            sorted.push((request.input.created_at, Sortable::Request(&request.input)));
        }

        // ---- 3. Sort with custom ordering rules ----
        sorted.sort_by(|(time_a, a), (time_b, b)| {
            match time_a.cmp(time_b) {
                std::cmp::Ordering::Equal => match (a, b) {
                    // ---- History always placed before request if equal timestamp ----
                    (Sortable::Concrete(_), Sortable::Request(_)) => std::cmp::Ordering::Less,
                    (Sortable::Request(_), Sortable::Concrete(_)) => std::cmp::Ordering::Greater,

                    // ---- Ordering between requests: use message_id ----
                    (Sortable::Request(r1), Sortable::Request(r2)) => {
                        r1.message_id.cmp(&r2.message_id)
                    }

                    // ---- Ordering between confirmed messages: use offset ----
                    (Sortable::Concrete(m1), Sortable::Concrete(m2)) => {
                        m1.message_offset.cmp(&m2.message_offset)
                    }
                },

                // Default: sort by created_at ascending
                ord => ord,
            }
        });

        // ---- 4. Convert result ----
        sorted
            .into_iter()
            .map(|(_, entry)| match entry {
                Sortable::Concrete(record) => HistoryMessage::Concrete(record.clone()),
                Sortable::Request(input) => HistoryMessage::Request(input.clone()),
            })
            .collect()
    }

    fn get_conversation_history_version(&self, conversation_id: ConversationId) -> u64 {
        match self.registry.get(&conversation_id) {
            None => 0,
            Some(item) => item.sync_state.version,
        }
    }
}

use crate::domain::{ConversationId, MessageId, MessageOffset, UserId};
use crate::infra::network::{
    ChatMessage, ChatMessageRecv, ChatMessageSent, FriendshipRecv, StreamMessage,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const WS_CHAT_URL: &str = "wss://127.0.0.1:8443/api/v1/chat";

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "content", rename_all = "lowercase")]
pub enum C2SCommand {
    ChatMessageSend(ChatMessageSend),
    HistoryFetched,
    Send(SendMessage),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessageSend {
    pub conversation_id: ConversationId,
    pub message_id: MessageId,
    pub content: String,
}

#[derive(Debug)]
pub enum ClientMessage {
    Control(ControlMessage),
    Stream(StreamMessage),
}

#[derive(Debug)]
pub enum ControlMessage {
    ChatMessageACK(ChatMessageACK),
}

impl From<S2CEvent> for ClientMessage {
    fn from(event: S2CEvent) -> Self {
        match event {
            S2CEvent::ChatMessageACK(e) => {
                ClientMessage::Control(ControlMessage::ChatMessageACK(e))
            }
            S2CEvent::ChatMessageNew(e) => {
                ClientMessage::Stream(StreamMessage::ChatMessageRecv(ChatMessageRecv {
                    conversation_id: e.conversation_id,
                    message_id: e.message_id,
                    message_offset: e.message_offset,
                    content: e.content,
                    sender: e.sender,
                    username: e.username,
                    created_at: e.created_at,
                }))
            }
            S2CEvent::FriendshipNew(e) => {
                ClientMessage::Stream(StreamMessage::FriendshipRecv(FriendshipRecv {
                    conversation_id: e.conversation_id,
                    other: e.other,
                    username: e.username,
                }))
            }
            _ => {
                ClientMessage::Stream(StreamMessage::Distribute(ChatMessage {
                    sender: UserId(uuid::Uuid::nil()),
                    conversation_id: ConversationId(uuid::Uuid::nil()),
                    content: "placeholder content".to_owned(),
                }))
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessage {
    pub message_seq: u64,
    #[serde(flatten)]
    pub content: ChatContent,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "content", rename_all = "lowercase")]
pub enum S2CEvent {
    ChatMessageACK(ChatMessageACK),
    ChatMessageNew(ChatMessageNew),
    FriendshipNew(FriendshipNew),
    Distribute(DistributeMessage),
    ACK(ACK),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessageACK {
    pub conversation_id: ConversationId,
    pub message_id: MessageId,
    pub message_offset: MessageOffset,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessageNew {
    pub conversation_id: ConversationId,
    pub message_id: MessageId,
    pub message_offset: MessageOffset,
    pub content: String,
    pub sender: UserId,
    pub username: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FriendshipNew {
    pub conversation_id: ConversationId,
    pub other: UserId,
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DistributeMessage {
    pub sender: UserId,
    #[serde(flatten)]
    pub content: ChatContent,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatContent {
    pub conversation_id: ConversationId,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ACK {
    pub message_seq: u64,
}

//! Realtime event envelope and payloads pushed over the `/ws` gateway.
//!
//! Wire format: `{ "type": "<EventType>", "payload": { ... }, "ts": "<ISO-8601>" }`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::datetime_format;
use crate::models::{DirectMessage, DmConversation, Message, Notification, Post};

/// A channel message together with the community it belongs to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessageEvent {
    pub message: Message,
    pub community_id: Uuid,
}

/// A newly created DM request: the pending conversation plus its first message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmRequestEvent {
    pub conversation: DmConversation,
    pub message: DirectMessage,
}

/// Typed realtime event, tagged by `type` with its DTO under `payload`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum Event {
    MessageCreated(ChannelMessageEvent),
    ThreadCreated(Post),
    ThreadPendingApproval(Post),
    DmRequestCreated(DmRequestEvent),
    DmMessageCreated(DirectMessage),
    NotificationCreated(Notification),
}

/// Envelope pushed to clients: the tagged event flattened next to a timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    #[serde(flatten)]
    pub event: Event,
    #[serde(with = "datetime_format")]
    pub ts: DateTime<Utc>,
}

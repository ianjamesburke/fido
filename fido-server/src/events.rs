use std::sync::Arc;

use anyhow::Result;
use fido_types::{DirectMessage, DmConversation, Message, Notification, Post};

#[derive(Debug, Clone)]
pub enum ServerEvent {
    DmRequestCreated {
        conversation: DmConversation,
        message: DirectMessage,
    },
    DmMessageCreated(DirectMessage),
    MessageCreated(Message),
    NotificationCreated(Notification),
    ThreadCreated(Post),
    ThreadPendingApproval(Post),
}

pub trait EventBus: Send + Sync {
    fn emit(&self, event: ServerEvent) -> Result<()>;
}

pub type SharedEventBus = Arc<dyn EventBus>;

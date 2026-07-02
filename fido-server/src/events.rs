use std::sync::Arc;

use anyhow::Result;
use fido_types::{Message, Post};

#[derive(Debug, Clone)]
pub enum ServerEvent {
    MessageCreated(Message),
    ThreadCreated(Post),
    ThreadPendingApproval(Post),
}

pub trait EventBus: Send + Sync {
    fn emit(&self, event: ServerEvent) -> Result<()>;
}

#[derive(Debug, Default)]
pub struct NoopEventBus;

impl EventBus for NoopEventBus {
    fn emit(&self, event: ServerEvent) -> Result<()> {
        match event {
            ServerEvent::MessageCreated(message) => {
                let _ = message.id;
            }
            ServerEvent::ThreadCreated(post) | ServerEvent::ThreadPendingApproval(post) => {
                let _ = post.id;
            }
        }
        Ok(())
    }
}

pub type SharedEventBus = Arc<dyn EventBus>;

use std::sync::Arc;

use anyhow::Result;
use fido_types::Message;

#[derive(Debug, Clone)]
pub enum ServerEvent {
    MessageCreated(Message),
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
        }
        Ok(())
    }
}

pub type SharedEventBus = Arc<dyn EventBus>;

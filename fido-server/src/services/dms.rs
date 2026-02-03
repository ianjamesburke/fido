//! Direct message related business logic.

use chrono::Utc;
use uuid::Uuid;

use crate::api::{ApiError, ApiResult};
use crate::db::repositories::{DirectMessageRepository, UserRepository};
use crate::db::DbPool;
use fido_types::DirectMessage;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ConversationSummary {
    pub other_user_id: String,
    pub other_username: String,
    pub last_message: String,
    pub last_message_time: String,
    pub unread_count: usize,
}

pub struct DMService {
    dm_repo: DirectMessageRepository,
    user_repo: UserRepository,
}

impl DMService {
    pub fn new(pool: DbPool) -> Self {
        Self {
            dm_repo: DirectMessageRepository::new(pool.clone()),
            user_repo: UserRepository::new(pool),
        }
    }

    pub fn get_conversations(&self, user_id: &Uuid) -> ApiResult<Vec<ConversationSummary>> {
        let conversation_user_ids = self.dm_repo.get_conversations_list(user_id)?;

        let mut conversations = Vec::new();
        for other_user_id in conversation_user_ids {
            let user = self
                .user_repo
                .get_by_id(&other_user_id)?
                .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

            let messages = self.dm_repo.get_conversation(user_id, &other_user_id)?;

            let unread_count = messages
                .iter()
                .filter(|m| m.to_user_id == *user_id && !m.is_read)
                .count();

            if messages.is_empty() {
                continue;
            }

            let (last_message, last_message_time) = if let Some(last_msg) = messages.last() {
                (last_msg.content.clone(), last_msg.created_at.to_rfc3339())
            } else {
                ("No messages yet".to_string(), Utc::now().to_rfc3339())
            };

            conversations.push(ConversationSummary {
                other_user_id: other_user_id.to_string(),
                other_username: user.username,
                last_message,
                last_message_time,
                unread_count,
            });
        }

        Ok(conversations)
    }

    pub fn get_conversation(
        &self,
        user_id: &Uuid,
        other_user_id: &Uuid,
    ) -> ApiResult<Vec<DirectMessage>> {
        self.user_repo
            .get_by_id(other_user_id)?
            .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

        let mut messages = self.dm_repo.get_conversation(user_id, other_user_id)?;

        for msg in &mut messages {
            let from_user = self
                .user_repo
                .get_by_id(&msg.from_user_id)?
                .ok_or_else(|| ApiError::NotFound("Sender not found".to_string()))?;

            let to_user = self
                .user_repo
                .get_by_id(&msg.to_user_id)?
                .ok_or_else(|| ApiError::NotFound("Recipient not found".to_string()))?;

            msg.from_username = from_user.username;
            msg.to_username = to_user.username;
        }

        self.dm_repo.mark_as_read(user_id, other_user_id)?;

        Ok(messages)
    }

    pub fn mark_messages_read(&self, user_id: &Uuid, other_user_id: &Uuid) -> ApiResult<()> {
        self.dm_repo.mark_as_read(user_id, other_user_id)?;
        Ok(())
    }

    pub fn send_message(
        &self,
        from_user_id: &Uuid,
        to_username: &str,
        content: &str,
    ) -> ApiResult<DirectMessage> {
        if content.is_empty() {
            return Err(ApiError::BadRequest(
                "Message content cannot be empty".to_string(),
            ));
        }

        let to_user = self
            .user_repo
            .get_by_username(to_username)?
            .ok_or_else(|| ApiError::NotFound(format!("User '{}' not found", to_username)))?;

        if *from_user_id == to_user.id {
            return Err(ApiError::BadRequest(
                "Cannot send message to yourself".to_string(),
            ));
        }

        let from_user = self
            .user_repo
            .get_by_id(from_user_id)?
            .ok_or_else(|| ApiError::NotFound("Sender not found".to_string()))?;

        let message = DirectMessage {
            id: Uuid::new_v4(),
            from_user_id: *from_user_id,
            to_user_id: to_user.id,
            from_username: from_user.username,
            to_username: to_user.username,
            content: content.to_string(),
            created_at: Utc::now(),
            is_read: false,
        };

        self.dm_repo.create(&message)?;

        Ok(message)
    }

    pub fn delete_conversation(&self, user_id: &Uuid, other_user_id: &Uuid) -> ApiResult<()> {
        self.dm_repo.delete_conversation(user_id, other_user_id)?;
        Ok(())
    }
}

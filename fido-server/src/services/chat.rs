//! Channel chat business logic.

use chrono::Utc;
use uuid::Uuid;

use crate::{
    api::{ApiError, ApiResult},
    db::repositories::Repositories,
    events::{ServerEvent, SharedEventBus},
};
use fido_types::{Channel, Message};

pub const DEFAULT_MESSAGE_LIMIT: i32 = 50;
pub const MAX_MESSAGE_LIMIT: i32 = 100;

pub struct ChatService {
    repos: Repositories,
    event_bus: SharedEventBus,
}

impl ChatService {
    pub fn new(repos: Repositories, event_bus: SharedEventBus) -> Self {
        Self { repos, event_bus }
    }

    pub fn list_channels(&self, user_id: Uuid, community_id: Uuid) -> ApiResult<Vec<Channel>> {
        self.repos
            .communities
            .get_by_id(&community_id)?
            .ok_or_else(|| ApiError::NotFound("Community not found".to_string()))?;
        self.require_membership(user_id, community_id)?;
        Ok(self.repos.channels.list_by_community(&community_id)?)
    }

    pub fn list_messages(
        &self,
        user_id: Uuid,
        channel_id: Uuid,
        before: Option<Uuid>,
        after: Option<Uuid>,
        limit: i32,
    ) -> ApiResult<Vec<Message>> {
        let channel = self.require_channel(channel_id)?;
        self.require_membership(user_id, channel.community_id)?;
        let limit = normalize_limit(limit)?;
        Ok(self
            .repos
            .messages
            .list(&channel_id, before.as_ref(), after.as_ref(), limit)?)
    }

    pub fn send_message(
        &self,
        user_id: Uuid,
        channel_id: Uuid,
        content: String,
    ) -> ApiResult<Message> {
        let channel = self.require_channel(channel_id)?;
        self.require_membership(user_id, channel.community_id)?;

        let message = Message {
            id: Uuid::new_v4(),
            channel_id,
            author_id: user_id,
            content,
            created_at: Utc::now(),
        };
        self.repos.messages.create(&message)?;
        self.event_bus
            .emit(ServerEvent::MessageCreated(message.clone()))?;

        Ok(message)
    }

    fn require_channel(&self, channel_id: Uuid) -> ApiResult<Channel> {
        self.repos
            .channels
            .get_by_id(&channel_id)?
            .ok_or_else(|| ApiError::NotFound("Channel not found".to_string()))
    }

    fn require_membership(&self, user_id: Uuid, community_id: Uuid) -> ApiResult<()> {
        self.repos
            .memberships
            .get(&community_id, &user_id)?
            .ok_or_else(|| ApiError::Forbidden("Community membership required".to_string()))?;
        Ok(())
    }
}

pub fn normalize_limit(limit: i32) -> ApiResult<i32> {
    if !(1..=MAX_MESSAGE_LIMIT).contains(&limit) {
        return Err(ApiError::BadRequest(format!(
            "limit must be between 1 and {}",
            MAX_MESSAGE_LIMIT
        )));
    }
    Ok(limit)
}

#[cfg(all(test, feature = "sqlite-tests"))]
mod tests {
    use super::*;
    use crate::{
        db::Database,
        events::{EventBus, ServerEvent},
    };
    use anyhow::Result;
    use fido_types::{Community, Membership, MembershipRole, User};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct RecordingEventBus {
        events: Mutex<Vec<ServerEvent>>,
    }

    impl EventBus for RecordingEventBus {
        fn emit(&self, event: ServerEvent) -> Result<()> {
            self.events.lock().unwrap().push(event);
            Ok(())
        }
    }

    fn setup() -> Result<(
        Database,
        Repositories,
        Arc<RecordingEventBus>,
        Uuid,
        Uuid,
        Uuid,
    )> {
        let db = Database::in_memory()?;
        db.initialize()?;
        let repos = Repositories::new(db.pool.clone());
        let user = User {
            id: Uuid::new_v4(),
            username: "member".to_string(),
            bio: None,
            join_date: Utc::now(),
            is_test_user: true,
            is_admin: false,
        };
        repos.users.create(&user)?;

        let community = Community {
            id: Uuid::new_v4(),
            github_repo_id: 42,
            owner: "octocat".to_string(),
            name: "hello".to_string(),
            claimed_by: None,
            require_thread_approval: false,
            created_at: Utc::now(),
        };
        repos.communities.create(&community)?;

        let channel = Channel {
            id: Uuid::new_v4(),
            community_id: community.id,
            name: "general".to_string(),
            created_at: Utc::now(),
        };
        repos.channels.create(&channel)?;
        repos.memberships.insert(&Membership {
            community_id: community.id,
            user_id: user.id,
            role: MembershipRole::Member,
            created_at: Utc::now(),
        })?;

        let event_bus = Arc::new(RecordingEventBus::default());
        Ok((db, repos, event_bus, user.id, community.id, channel.id))
    }

    #[test]
    fn send_message_persists_and_emits_event() -> Result<()> {
        let (_db, repos, event_bus, user_id, _community_id, channel_id) = setup()?;
        let service = ChatService::new(repos.clone(), event_bus.clone());

        let message = service
            .send_message(user_id, channel_id, "hello channel".to_string())
            .expect("member can send message");

        let messages = repos.messages.list(&channel_id, None, None, 10)?;
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].id, message.id);

        let events = event_bus.events.lock().unwrap();
        assert_eq!(events.len(), 1);
        match &events[0] {
            ServerEvent::MessageCreated(created) => assert_eq!(created.id, message.id),
            ServerEvent::ThreadCreated(_) | ServerEvent::ThreadPendingApproval(_) => {
                panic!("expected message event")
            }
        }
        Ok(())
    }

    #[test]
    fn rejects_non_member_send() -> Result<()> {
        let (_db, repos, event_bus, _user_id, _community_id, channel_id) = setup()?;
        let service = ChatService::new(repos, event_bus);

        let result = service.send_message(Uuid::new_v4(), channel_id, "nope".to_string());
        assert!(matches!(result, Err(ApiError::Forbidden(_))));
        Ok(())
    }
}

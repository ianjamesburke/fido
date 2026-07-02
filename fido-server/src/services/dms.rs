//! Direct message related business logic.

use uuid::Uuid;

use chrono::Utc;

use crate::api::{ApiError, ApiResult};
use crate::db::repositories::Repositories;
use crate::events::{ServerEvent, SharedEventBus};
use crate::services::notifications::NotificationService;
use fido_types::{DirectMessage, DmConversationState, NotificationType};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ConversationSummary {
    pub other_user_id: String,
    pub other_username: String,
    pub last_message: String,
    pub last_message_time: String,
    pub unread_count: usize,
}

#[derive(Debug, Serialize)]
pub struct DmRequestSummary {
    pub from_user_id: String,
    pub from_username: String,
    pub created_at: String,
}

pub struct DMService {
    repos: Repositories,
    event_bus: SharedEventBus,
}

impl DMService {
    pub fn new(repos: Repositories, event_bus: SharedEventBus) -> Self {
        Self { repos, event_bus }
    }

    pub fn get_conversations(&self, user_id: &Uuid) -> ApiResult<Vec<ConversationSummary>> {
        let summaries = self.repos.dms.get_conversation_summaries(user_id)?;

        let mut conversations = Vec::with_capacity(summaries.len());
        for summary in summaries {
            let user = self
                .repos
                .users
                .get_by_id(&summary.other_user_id)?
                .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

            conversations.push(ConversationSummary {
                other_user_id: summary.other_user_id.to_string(),
                other_username: user.username,
                last_message: summary.last_message,
                last_message_time: summary.last_message_time.to_rfc3339(),
                unread_count: summary.unread_count,
            });
        }

        Ok(conversations)
    }

    pub fn get_conversation(
        &self,
        user_id: &Uuid,
        other_user_id: &Uuid,
    ) -> ApiResult<Vec<DirectMessage>> {
        self.repos
            .users
            .get_by_id(other_user_id)?
            .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

        let mut messages = self.repos.dms.get_conversation(user_id, other_user_id)?;

        for msg in &mut messages {
            let from_user = self
                .repos
                .users
                .get_by_id(&msg.from_user_id)?
                .ok_or_else(|| ApiError::NotFound("Sender not found".to_string()))?;

            let to_user = self
                .repos
                .users
                .get_by_id(&msg.to_user_id)?
                .ok_or_else(|| ApiError::NotFound("Recipient not found".to_string()))?;

            msg.from_username = from_user.username;
            msg.to_username = to_user.username;
        }

        self.repos.dms.mark_as_read(user_id, other_user_id)?;

        Ok(messages)
    }

    pub fn mark_messages_read(&self, user_id: &Uuid, other_user_id: &Uuid) -> ApiResult<()> {
        self.repos.dms.mark_as_read(user_id, other_user_id)?;
        Ok(())
    }

    pub fn get_pending_requests(&self, user_id: &Uuid) -> ApiResult<Vec<DmRequestSummary>> {
        let conversations = self
            .repos
            .dm_conversations
            .list_pending_for_recipient(user_id)?;
        let mut requests = Vec::with_capacity(conversations.len());

        for conversation in conversations {
            let from_user = self
                .repos
                .users
                .get_by_id(&conversation.initiator_id)?
                .ok_or_else(|| ApiError::NotFound("Request sender not found".to_string()))?;

            requests.push(DmRequestSummary {
                from_user_id: from_user.id.to_string(),
                from_username: from_user.username,
                created_at: conversation.created_at.to_rfc3339(),
            });
        }

        Ok(requests)
    }

    pub fn accept_request(&self, recipient_id: &Uuid, requester_id: &Uuid) -> ApiResult<()> {
        self.update_request_state(recipient_id, requester_id, DmConversationState::Accepted)
    }

    pub fn decline_request(&self, recipient_id: &Uuid, requester_id: &Uuid) -> ApiResult<()> {
        self.update_request_state(recipient_id, requester_id, DmConversationState::Declined)
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
            .repos
            .users
            .get_by_username(to_username)?
            .ok_or_else(|| ApiError::NotFound(format!("User '{}' not found", to_username)))?;

        if *from_user_id == to_user.id {
            return Err(ApiError::BadRequest(
                "Cannot send message to yourself".to_string(),
            ));
        }

        let from_user = self
            .repos
            .users
            .get_by_id(from_user_id)?
            .ok_or_else(|| ApiError::NotFound("Sender not found".to_string()))?;

        let conversation_state = self.ensure_send_allowed(from_user_id, &to_user.id)?;

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

        self.repos.dms.create(&message)?;
        if conversation_state == DmConversationState::Pending {
            if let Some(conversation) =
                self.repos.dm_conversations.get(from_user_id, &to_user.id)?
            {
                NotificationService::new(self.repos.clone(), self.event_bus.clone()).create(
                    to_user.id,
                    NotificationType::DmRequest,
                    *from_user_id,
                    "dm_conversation",
                    dm_subject_id(from_user_id, &to_user.id),
                )?;
                self.event_bus.emit(ServerEvent::DmRequestCreated {
                    conversation,
                    message: message.clone(),
                })?;
            }
        }
        self.event_bus
            .emit(ServerEvent::DmMessageCreated(message.clone()))?;

        Ok(message)
    }

    pub fn delete_conversation(&self, user_id: &Uuid, other_user_id: &Uuid) -> ApiResult<()> {
        self.repos.dms.delete_conversation(user_id, other_user_id)?;
        Ok(())
    }

    fn update_request_state(
        &self,
        recipient_id: &Uuid,
        requester_id: &Uuid,
        state: DmConversationState,
    ) -> ApiResult<()> {
        let conversation = self
            .repos
            .dm_conversations
            .get(recipient_id, requester_id)?
            .ok_or_else(|| ApiError::NotFound("DM request not found".to_string()))?;

        if conversation.state != DmConversationState::Pending {
            return Err(ApiError::BadRequest(
                "DM request is not pending".to_string(),
            ));
        }

        if conversation.initiator_id != *requester_id {
            return Err(ApiError::Forbidden(
                "Only the request recipient can update this DM request".to_string(),
            ));
        }

        self.repos
            .dm_conversations
            .set_state(recipient_id, requester_id, state)?;
        Ok(())
    }

    fn ensure_send_allowed(
        &self,
        from_user_id: &Uuid,
        to_user_id: &Uuid,
    ) -> ApiResult<DmConversationState> {
        if let Some(conversation) = self.repos.dm_conversations.get(from_user_id, to_user_id)? {
            return match conversation.state {
                DmConversationState::Accepted => Ok(DmConversationState::Accepted),
                DmConversationState::Declined => {
                    Err(ApiError::Forbidden("DM request was declined".to_string()))
                }
                DmConversationState::Pending => {
                    let existing_messages =
                        self.repos.dms.count_between(from_user_id, to_user_id)?;
                    if conversation.initiator_id == *from_user_id && existing_messages == 0 {
                        Ok(DmConversationState::Pending)
                    } else {
                        Err(ApiError::Forbidden(
                            "DM request is pending acceptance".to_string(),
                        ))
                    }
                }
            };
        }

        let state = if self.should_auto_accept(from_user_id, to_user_id)? {
            DmConversationState::Accepted
        } else {
            DmConversationState::Pending
        };
        self.repos
            .dm_conversations
            .create(from_user_id, to_user_id, from_user_id, state)?;
        Ok(state)
    }

    fn should_auto_accept(&self, from_user_id: &Uuid, to_user_id: &Uuid) -> ApiResult<bool> {
        Ok(self
            .repos
            .friends
            .are_mutual_friends(from_user_id, to_user_id)?
            || self
                .repos
                .memberships
                .users_share_community(from_user_id, to_user_id)?)
    }
}

fn dm_subject_id(user_a: &Uuid, user_b: &Uuid) -> String {
    let mut ids = [user_a.to_string(), user_b.to_string()];
    ids.sort();
    ids.join(":")
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

    fn test_user(username: &str) -> User {
        User {
            id: Uuid::new_v4(),
            username: username.to_string(),
            bio: None,
            join_date: Utc::now(),
            is_test_user: true,
            is_admin: false,
        }
    }

    fn setup() -> Result<(Database, Repositories, Arc<RecordingEventBus>, User, User)> {
        let db = Database::in_memory()?;
        db.initialize()?;
        let repos = Repositories::new(db.pool.clone());
        let sender = test_user("sender");
        let recipient = test_user("recipient");
        repos.users.create(&sender)?;
        repos.users.create(&recipient)?;
        let event_bus = Arc::new(RecordingEventBus::default());
        Ok((db, repos, event_bus, sender, recipient))
    }

    #[test]
    fn send_message_creates_pending_request_for_stranger() -> Result<()> {
        let (_db, repos, event_bus, sender, recipient) = setup()?;
        let service = DMService::new(repos.clone(), event_bus.clone());

        service
            .send_message(&sender.id, &recipient.username, "hello")
            .expect("first stranger message creates request");

        let conversation = repos
            .dm_conversations
            .get(&sender.id, &recipient.id)?
            .expect("conversation exists");
        assert_eq!(conversation.state, DmConversationState::Pending);
        assert_eq!(conversation.initiator_id, sender.id);

        let requests = service
            .get_pending_requests(&recipient.id)
            .expect("recipient can list pending request");
        assert_eq!(requests.len(), 1);

        let events = event_bus.events.lock().unwrap();
        assert_eq!(events.len(), 3);
        assert!(matches!(events[0], ServerEvent::NotificationCreated(_)));
        assert!(matches!(events[1], ServerEvent::DmRequestCreated { .. }));
        assert!(matches!(events[2], ServerEvent::DmMessageCreated(_)));

        let notifications = repos.notifications.list_for_user(&recipient.id, 10, 0)?;
        assert_eq!(notifications.len(), 1);
        assert_eq!(
            notifications[0].notification_type,
            NotificationType::DmRequest
        );
        Ok(())
    }

    #[test]
    fn send_message_rejects_second_pending_message() -> Result<()> {
        let (_db, repos, event_bus, sender, recipient) = setup()?;
        let service = DMService::new(repos, event_bus);

        service
            .send_message(&sender.id, &recipient.username, "hello")
            .expect("first stranger message creates request");
        let result = service.send_message(&sender.id, &recipient.username, "again");

        assert!(matches!(result, Err(ApiError::Forbidden(_))));
        Ok(())
    }

    #[test]
    fn accept_request_allows_followup_messages() -> Result<()> {
        let (_db, repos, event_bus, sender, recipient) = setup()?;
        let service = DMService::new(repos.clone(), event_bus);

        service
            .send_message(&sender.id, &recipient.username, "hello")
            .expect("first stranger message creates request");
        service
            .accept_request(&recipient.id, &sender.id)
            .expect("recipient can accept request");
        service
            .send_message(&sender.id, &recipient.username, "thanks")
            .expect("accepted request allows followup");

        let conversation = repos
            .dm_conversations
            .get(&sender.id, &recipient.id)?
            .expect("conversation exists");
        assert_eq!(conversation.state, DmConversationState::Accepted);
        assert_eq!(repos.dms.count_between(&sender.id, &recipient.id)?, 2);
        Ok(())
    }

    #[test]
    fn decline_request_rejects_future_messages() -> Result<()> {
        let (_db, repos, event_bus, sender, recipient) = setup()?;
        let service = DMService::new(repos, event_bus);

        service
            .send_message(&sender.id, &recipient.username, "hello")
            .expect("first stranger message creates request");
        service
            .decline_request(&recipient.id, &sender.id)
            .expect("recipient can decline request");
        let result = service.send_message(&sender.id, &recipient.username, "again");

        assert!(matches!(result, Err(ApiError::Forbidden(_))));
        Ok(())
    }

    #[test]
    fn send_message_auto_accepts_mutual_follows() -> Result<()> {
        let (_db, repos, event_bus, sender, recipient) = setup()?;
        repos.friends.follow_user(&sender.id, &recipient.id)?;
        repos.friends.follow_user(&recipient.id, &sender.id)?;
        let service = DMService::new(repos.clone(), event_bus);

        service
            .send_message(&sender.id, &recipient.username, "hello")
            .expect("mutual follows auto-accept dm");

        let conversation = repos
            .dm_conversations
            .get(&sender.id, &recipient.id)?
            .expect("conversation exists");
        assert_eq!(conversation.state, DmConversationState::Accepted);
        Ok(())
    }

    #[test]
    fn send_message_auto_accepts_shared_community_members() -> Result<()> {
        let (_db, repos, event_bus, sender, recipient) = setup()?;
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
        repos.memberships.insert(&Membership {
            community_id: community.id,
            user_id: sender.id,
            role: MembershipRole::Member,
            created_at: Utc::now(),
        })?;
        repos.memberships.insert(&Membership {
            community_id: community.id,
            user_id: recipient.id,
            role: MembershipRole::Member,
            created_at: Utc::now(),
        })?;
        let service = DMService::new(repos.clone(), event_bus);

        service
            .send_message(&sender.id, &recipient.username, "hello")
            .expect("shared community auto-accepts dm");

        let conversation = repos
            .dm_conversations
            .get(&sender.id, &recipient.id)?
            .expect("conversation exists");
        assert_eq!(conversation.state, DmConversationState::Accepted);
        Ok(())
    }
}

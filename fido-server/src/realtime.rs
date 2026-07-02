//! Realtime WebSocket gateway: the in-process event bus backing `GET /ws`.
//!
//! API handlers publish [`ServerEvent`]s through the [`EventBus`] trait; the
//! gateway resolves recipients at publish time (one DB lookup per event),
//! serializes the wire envelope once, and fans the result out over a
//! `tokio::sync::broadcast` channel. Each connection task only checks whether
//! its own user id is in the recipient set.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use chrono::Utc;
use fido_types::{ChannelMessageEvent, DmRequestEvent, Event, EventEnvelope};
use tokio::sync::{broadcast, watch};
use uuid::Uuid;

use crate::db::repositories::Repositories;
use crate::events::{EventBus, ServerEvent};

/// Broadcast channel capacity. A connection that lags this far behind is
/// closed so its client falls back to refetching (polling fallback contract).
const BROADCAST_CAPACITY: usize = 256;

/// A serialized event plus the set of user ids that may receive it.
#[derive(Debug, Clone)]
pub struct OutboundEvent {
    recipients: Arc<HashSet<Uuid>>,
    json: Arc<str>,
}

impl OutboundEvent {
    pub fn is_for(&self, user_id: &Uuid) -> bool {
        self.recipients.contains(user_id)
    }

    pub fn json(&self) -> &str {
        &self.json
    }
}

/// Tracks how many live WebSocket connections each user holds.
#[derive(Debug, Default)]
pub struct ConnectionRegistry {
    connections: Mutex<HashMap<Uuid, usize>>,
}

impl ConnectionRegistry {
    /// Record a new connection; returns the user's connection count after it.
    pub fn connect(&self, user_id: Uuid) -> usize {
        let mut connections = self.connections.lock().unwrap_or_else(|e| e.into_inner());
        let count = connections.entry(user_id).or_insert(0);
        *count += 1;
        *count
    }

    /// Record a closed connection; returns the user's remaining count.
    pub fn disconnect(&self, user_id: Uuid) -> usize {
        let mut connections = self.connections.lock().unwrap_or_else(|e| e.into_inner());
        match connections.get_mut(&user_id) {
            Some(count) if *count > 1 => {
                *count -= 1;
                *count
            }
            Some(_) => {
                connections.remove(&user_id);
                0
            }
            None => 0,
        }
    }
}

/// The realtime gateway: publish-time recipient resolution + broadcast fan-out.
pub struct RealtimeGateway {
    repos: Repositories,
    sender: broadcast::Sender<OutboundEvent>,
    registry: ConnectionRegistry,
    shutdown: watch::Sender<bool>,
}

impl RealtimeGateway {
    pub fn new(repos: Repositories) -> Self {
        let (sender, _) = broadcast::channel(BROADCAST_CAPACITY);
        let (shutdown, _) = watch::channel(false);
        Self {
            repos,
            sender,
            registry: ConnectionRegistry::default(),
            shutdown,
        }
    }

    /// Subscribe a new connection task to the event stream.
    pub fn subscribe(&self) -> broadcast::Receiver<OutboundEvent> {
        self.sender.subscribe()
    }

    /// Watch channel that flips to `true` when the server is shutting down.
    pub fn shutdown_signal(&self) -> watch::Receiver<bool> {
        self.shutdown.subscribe()
    }

    /// Signal all connection tasks to close with a going-away frame.
    pub fn shutdown(&self) {
        let _ = self.shutdown.send(true);
    }

    pub fn registry(&self) -> &ConnectionRegistry {
        &self.registry
    }

    /// Resolve the recipient set for an event via a single repo lookup.
    fn resolve_recipients(&self, event: &Event) -> Result<HashSet<Uuid>> {
        let recipients = match event {
            Event::MessageCreated(payload) => self
                .repos
                .memberships
                .list_members(&payload.community_id)
                .context("Failed to resolve MessageCreated recipients")?
                .into_iter()
                .map(|m| m.user_id)
                .collect(),
            Event::ThreadCreated(post) => self
                .repos
                .memberships
                .list_members(&post.community_id)
                .context("Failed to resolve ThreadCreated recipients")?
                .into_iter()
                .map(|m| m.user_id)
                .collect(),
            Event::ThreadPendingApproval(post) => self
                .repos
                .memberships
                .list_admins(&post.community_id)
                .context("Failed to resolve ThreadPendingApproval recipients")?
                .into_iter()
                .map(|m| m.user_id)
                .collect(),
            Event::DmRequestCreated(payload) => {
                let conversation = &payload.conversation;
                let recipient = if conversation.initiator_id == conversation.user_a {
                    conversation.user_b
                } else {
                    conversation.user_a
                };
                HashSet::from([recipient])
            }
            Event::DmMessageCreated(message) => {
                HashSet::from([message.from_user_id, message.to_user_id])
            }
            Event::NotificationCreated(notification) => HashSet::from([notification.user_id]),
        };
        Ok(recipients)
    }
}

impl EventBus for RealtimeGateway {
    fn emit(&self, event: ServerEvent) -> Result<()> {
        let event = match event {
            ServerEvent::MessageCreated(message) => {
                let channel = self
                    .repos
                    .channels
                    .get_by_id(&message.channel_id)
                    .context("Failed to look up channel for MessageCreated event")?
                    .with_context(|| {
                        format!(
                            "Channel {} not found for MessageCreated event",
                            message.channel_id
                        )
                    })?;
                Event::MessageCreated(ChannelMessageEvent {
                    message,
                    community_id: channel.community_id,
                })
            }
            ServerEvent::ThreadCreated(post) => Event::ThreadCreated(post),
            ServerEvent::ThreadPendingApproval(post) => Event::ThreadPendingApproval(post),
            ServerEvent::DmRequestCreated {
                conversation,
                message,
            } => Event::DmRequestCreated(DmRequestEvent {
                conversation,
                message,
            }),
            ServerEvent::DmMessageCreated(message) => Event::DmMessageCreated(message),
            ServerEvent::NotificationCreated(notification) => {
                Event::NotificationCreated(notification)
            }
        };

        let recipients = self.resolve_recipients(&event)?;
        let envelope = EventEnvelope {
            event,
            ts: Utc::now(),
        };
        let json =
            serde_json::to_string(&envelope).context("Failed to serialize event envelope")?;

        // Err means no live subscribers, which is fine: events are also
        // observable via the REST endpoints (polling fallback contract).
        let _ = self.sender.send(OutboundEvent {
            recipients: Arc::new(recipients),
            json: Arc::from(json),
        });
        Ok(())
    }
}

#[cfg(all(test, feature = "sqlite-tests"))]
mod tests {
    use super::*;
    use crate::db::Database;
    use chrono::Utc;
    use fido_types::{
        Channel, Community, Membership, MembershipRole, Message, Notification, NotificationType,
        User,
    };

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

    fn setup() -> Result<(Database, RealtimeGateway, Community, Channel, User, User)> {
        let db = Database::in_memory()?;
        db.initialize()?;
        let repos = Repositories::new(db.pool.clone());

        let member = test_user("member");
        let outsider = test_user("outsider");
        repos.users.create(&member)?;
        repos.users.create(&outsider)?;

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
            user_id: member.id,
            role: MembershipRole::Member,
            created_at: Utc::now(),
        })?;

        let gateway = RealtimeGateway::new(repos);
        Ok((db, gateway, community, channel, member, outsider))
    }

    #[test]
    fn message_created_routes_to_members_only() -> Result<()> {
        let (_db, gateway, community, channel, member, outsider) = setup()?;
        let mut rx = gateway.subscribe();

        gateway.emit(ServerEvent::MessageCreated(Message {
            id: Uuid::new_v4(),
            channel_id: channel.id,
            author_id: member.id,
            content: "hello".to_string(),
            created_at: Utc::now(),
        }))?;

        let outbound = rx.try_recv()?;
        assert!(outbound.is_for(&member.id));
        assert!(!outbound.is_for(&outsider.id));

        let envelope: serde_json::Value = serde_json::from_str(outbound.json())?;
        assert_eq!(envelope["type"], "MessageCreated");
        assert_eq!(
            envelope["payload"]["community_id"],
            community.id.to_string()
        );
        assert!(envelope["ts"].is_string());
        Ok(())
    }

    #[test]
    fn notification_created_routes_to_recipient_only() -> Result<()> {
        let (_db, gateway, _community, _channel, member, outsider) = setup()?;
        let mut rx = gateway.subscribe();

        gateway.emit(ServerEvent::NotificationCreated(Notification {
            id: Uuid::new_v4(),
            user_id: member.id,
            notification_type: NotificationType::Mention,
            actor_id: outsider.id,
            subject_type: "post".to_string(),
            subject_id: Uuid::new_v4().to_string(),
            read: false,
            created_at: Utc::now(),
        }))?;

        let outbound = rx.try_recv()?;
        assert!(outbound.is_for(&member.id));
        assert!(!outbound.is_for(&outsider.id));
        Ok(())
    }

    #[test]
    fn registry_tracks_multiple_connections_per_user() {
        let registry = ConnectionRegistry::default();
        let user = Uuid::new_v4();
        assert_eq!(registry.connect(user), 1);
        assert_eq!(registry.connect(user), 2);
        assert_eq!(registry.disconnect(user), 1);
        assert_eq!(registry.disconnect(user), 0);
        assert_eq!(registry.disconnect(user), 0);
    }
}

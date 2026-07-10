//! Realtime WebSocket gateway: the in-process event bus backing `GET /ws`.
//!
//! API handlers publish [`ServerEvent`]s through the [`EventBus`] trait; the
//! gateway resolves recipients at publish time, caches community recipient sets
//! until membership changes, serializes the wire envelope once, and fans the
//! result out over a `tokio::sync::broadcast` channel. Each connection task
//! only checks whether its own user id is in the recipient set.

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

#[derive(Debug, Default)]
struct RecipientCache {
    members: Mutex<HashMap<Uuid, Arc<HashSet<Uuid>>>>,
    admins: Mutex<HashMap<Uuid, Arc<HashSet<Uuid>>>>,
}

impl RecipientCache {
    fn members(&self, repos: &Repositories, community_id: &Uuid) -> Result<Arc<HashSet<Uuid>>> {
        if let Some(cached) = self
            .members
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(community_id)
            .cloned()
        {
            return Ok(cached);
        }

        let loaded: Arc<HashSet<Uuid>> = Arc::new(
            repos
                .memberships
                .list_members(community_id)
                .context("Failed to resolve community members")?
                .into_iter()
                .map(|m| m.user_id)
                .collect(),
        );
        let mut cache = self.members.lock().unwrap_or_else(|e| e.into_inner());
        Ok(cache
            .entry(*community_id)
            .or_insert_with(|| loaded.clone())
            .clone())
    }

    fn admins(&self, repos: &Repositories, community_id: &Uuid) -> Result<Arc<HashSet<Uuid>>> {
        if let Some(cached) = self
            .admins
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(community_id)
            .cloned()
        {
            return Ok(cached);
        }

        let loaded: Arc<HashSet<Uuid>> = Arc::new(
            repos
                .memberships
                .list_admins(community_id)
                .context("Failed to resolve community admins")?
                .into_iter()
                .map(|m| m.user_id)
                .collect(),
        );
        let mut cache = self.admins.lock().unwrap_or_else(|e| e.into_inner());
        Ok(cache
            .entry(*community_id)
            .or_insert_with(|| loaded.clone())
            .clone())
    }

    fn invalidate_community(&self, community_id: &Uuid) {
        self.members
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(community_id);
        self.admins
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(community_id);
    }
}

/// The realtime gateway: publish-time recipient resolution + broadcast fan-out.
pub struct RealtimeGateway {
    repos: Repositories,
    sender: broadcast::Sender<OutboundEvent>,
    registry: ConnectionRegistry,
    recipient_cache: RecipientCache,
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
            recipient_cache: RecipientCache::default(),
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

    /// Drop cached community recipients after membership or role changes.
    pub fn invalidate_community_recipients(&self, community_id: &Uuid) {
        self.recipient_cache.invalidate_community(community_id);
    }

    /// Resolve the recipient set for an event. Community-scoped events use a
    /// cache that is explicitly invalidated by membership mutations.
    fn resolve_recipients(&self, event: &Event) -> Result<Arc<HashSet<Uuid>>> {
        let recipients = match event {
            Event::MessageCreated(payload) => self
                .recipient_cache
                .members(&self.repos, &payload.community_id)
                .context("Failed to resolve MessageCreated recipients")?,
            Event::ThreadCreated(post) => self
                .recipient_cache
                .members(&self.repos, &post.community_id)
                .context("Failed to resolve ThreadCreated recipients")?,
            Event::ThreadPendingApproval(post) => self
                .recipient_cache
                .admins(&self.repos, &post.community_id)
                .context("Failed to resolve ThreadPendingApproval recipients")?,
            Event::DmRequestCreated(payload) => {
                let conversation = &payload.conversation;
                let recipient = if conversation.initiator_id == conversation.user_a {
                    conversation.user_b
                } else {
                    conversation.user_a
                };
                Arc::new(HashSet::from([recipient]))
            }
            Event::DmMessageCreated(message) => {
                Arc::new(HashSet::from([message.from_user_id, message.to_user_id]))
            }
            Event::NotificationCreated(notification) => {
                Arc::new(HashSet::from([notification.user_id]))
            }
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
            recipients,
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
    use std::time::Instant;
    use tokio::sync::broadcast::error::TryRecvError;

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

    #[test]
    fn community_recipient_cache_refreshes_after_invalidation() -> Result<()> {
        let (_db, gateway, community, channel, member, outsider) = setup()?;
        let mut rx = gateway.subscribe();

        gateway.emit(ServerEvent::MessageCreated(Message {
            id: Uuid::new_v4(),
            channel_id: channel.id,
            author_id: member.id,
            content: "warm cache".to_string(),
            created_at: Utc::now(),
        }))?;
        let warmed = rx.try_recv()?;
        assert!(warmed.is_for(&member.id));
        assert!(!warmed.is_for(&outsider.id));

        gateway.repos.memberships.insert(&Membership {
            community_id: community.id,
            user_id: outsider.id,
            role: MembershipRole::Member,
            created_at: Utc::now(),
        })?;

        gateway.emit(ServerEvent::MessageCreated(Message {
            id: Uuid::new_v4(),
            channel_id: channel.id,
            author_id: member.id,
            content: "still cached".to_string(),
            created_at: Utc::now(),
        }))?;
        let stale = rx.try_recv()?;
        assert!(!stale.is_for(&outsider.id));

        gateway.invalidate_community_recipients(&community.id);
        gateway.emit(ServerEvent::MessageCreated(Message {
            id: Uuid::new_v4(),
            channel_id: channel.id,
            author_id: member.id,
            content: "refreshed".to_string(),
            created_at: Utc::now(),
        }))?;
        let refreshed = rx.try_recv()?;
        assert!(refreshed.is_for(&outsider.id));
        Ok(())
    }

    #[test]
    fn large_room_fanout_keeps_broadcast_lag_bounded() -> Result<()> {
        let (_db, gateway, community, channel, member, _outsider) = setup()?;

        for i in 0..1_000 {
            let user = test_user(&format!("member-{i}"));
            gateway.repos.users.create(&user)?;
            gateway.repos.memberships.insert(&Membership {
                community_id: community.id,
                user_id: user.id,
                role: MembershipRole::Member,
                created_at: Utc::now(),
            })?;
        }
        gateway.invalidate_community_recipients(&community.id);

        let mut receivers = (0..128).map(|_| gateway.subscribe()).collect::<Vec<_>>();
        let started = Instant::now();
        for i in 0..64 {
            gateway.emit(ServerEvent::MessageCreated(Message {
                id: Uuid::new_v4(),
                channel_id: channel.id,
                author_id: member.id,
                content: format!("stress {i}"),
                created_at: Utc::now(),
            }))?;
        }
        eprintln!(
            "realtime fanout stress: 64 emits to 1001 members and 128 subscribers in {:?}",
            started.elapsed()
        );

        let outbound = receivers[0].try_recv()?;
        assert!(outbound.is_for(&member.id));

        let mut lagging = gateway.subscribe();
        for i in 0..(BROADCAST_CAPACITY + 5) {
            gateway.emit(ServerEvent::MessageCreated(Message {
                id: Uuid::new_v4(),
                channel_id: channel.id,
                author_id: member.id,
                content: format!("lag {i}"),
                created_at: Utc::now(),
            }))?;
        }
        assert!(
            matches!(lagging.try_recv(), Err(TryRecvError::Lagged(_))),
            "lagging receivers must observe bounded broadcast overflow so /ws can close and refetch"
        );

        Ok(())
    }
}

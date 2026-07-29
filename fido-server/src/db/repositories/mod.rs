mod activity_repository;
mod audit_repository;
mod channel_repository;
mod community_repository;
mod config_repository;
mod dm_conversation_repository;
mod dm_repository;
mod friend_repository;
mod github_token_repository;
mod membership_repository;
mod message_repository;
mod notification_repository;
mod post_repository;
mod rate_limit_repository;
mod session_repository;
mod user_repository;
mod vote_repository;

pub use activity_repository::ActivityRepository;
pub use audit_repository::AuditRepository;
pub use channel_repository::ChannelRepository;
pub use community_repository::CommunityRepository;
pub use config_repository::ConfigRepository;
pub use dm_conversation_repository::DmConversationRepository;
pub use dm_repository::DirectMessageRepository;
pub use friend_repository::FriendRepository;
pub use github_token_repository::GitHubTokenRepository;
pub use membership_repository::MembershipRepository;
pub use message_repository::MessageRepository;
pub use notification_repository::NotificationRepository;
pub use post_repository::PostRepository;
pub use rate_limit_repository::RateLimitRepository;
pub use session_repository::SessionRepository;
pub use user_repository::UserRepository;
pub use vote_repository::VoteRepository;

use crate::db::DbPool;

/// Bundle of every repository, wired to a single database pool.
///
/// Repositories are cheap `DbPool` wrappers, so this holds them by value and
/// is cloned freely into services and handlers.
#[derive(Clone)]
pub struct Repositories {
    pub activity: ActivityRepository,
    pub posts: PostRepository,
    pub votes: VoteRepository,
    pub users: UserRepository,
    pub friends: FriendRepository,
    pub github_tokens: GitHubTokenRepository,
    pub config: ConfigRepository,
    pub rate_limits: RateLimitRepository,
    pub dms: DirectMessageRepository,
    pub sessions: SessionRepository,
    pub audit: AuditRepository,
    pub communities: CommunityRepository,
    pub channels: ChannelRepository,
    pub messages: MessageRepository,
    pub memberships: MembershipRepository,
    pub notifications: NotificationRepository,
    pub dm_conversations: DmConversationRepository,
}

impl Repositories {
    pub fn new(pool: DbPool) -> Self {
        Self {
            activity: ActivityRepository::new(pool.clone()),
            posts: PostRepository::new(pool.clone()),
            votes: VoteRepository::new(pool.clone()),
            users: UserRepository::new(pool.clone()),
            friends: FriendRepository::new(pool.clone()),
            github_tokens: GitHubTokenRepository::new(pool.clone()),
            config: ConfigRepository::new(pool.clone()),
            rate_limits: RateLimitRepository::new(pool.clone()),
            dms: DirectMessageRepository::new(pool.clone()),
            sessions: SessionRepository::new(pool.clone()),
            audit: AuditRepository::new(pool.clone()),
            communities: CommunityRepository::new(pool.clone()),
            channels: ChannelRepository::new(pool.clone()),
            messages: MessageRepository::new(pool.clone()),
            memberships: MembershipRepository::new(pool.clone()),
            notifications: NotificationRepository::new(pool.clone()),
            dm_conversations: DmConversationRepository::new(pool),
        }
    }
}

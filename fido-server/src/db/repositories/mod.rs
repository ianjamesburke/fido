mod audit_repository;
mod config_repository;
mod dm_repository;
mod friend_repository;
mod hashtag_repository;
mod post_repository;
mod rate_limit_repository;
mod session_repository;
mod user_repository;
mod vote_repository;

pub use audit_repository::AuditRepository;
pub use config_repository::ConfigRepository;
pub use dm_repository::DirectMessageRepository;
pub use friend_repository::FriendRepository;
pub use hashtag_repository::HashtagRepository;
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
    pub posts: PostRepository,
    pub hashtags: HashtagRepository,
    pub votes: VoteRepository,
    pub users: UserRepository,
    pub friends: FriendRepository,
    pub config: ConfigRepository,
    pub rate_limits: RateLimitRepository,
    pub dms: DirectMessageRepository,
    pub sessions: SessionRepository,
    pub audit: AuditRepository,
}

impl Repositories {
    pub fn new(pool: DbPool) -> Self {
        Self {
            posts: PostRepository::new(pool.clone()),
            hashtags: HashtagRepository::new(pool.clone()),
            votes: VoteRepository::new(pool.clone()),
            users: UserRepository::new(pool.clone()),
            friends: FriendRepository::new(pool.clone()),
            config: ConfigRepository::new(pool.clone()),
            rate_limits: RateLimitRepository::new(pool.clone()),
            dms: DirectMessageRepository::new(pool.clone()),
            sessions: SessionRepository::new(pool.clone()),
            audit: AuditRepository::new(pool),
        }
    }
}

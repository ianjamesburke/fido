//! Storage trait definitions and store wiring.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::db::DbPool;
use fido_types::{DirectMessage, Post, SortOrder, User, UserConfig, Vote, VoteDirection};

pub mod sqlite;

#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub last_activity: Option<DateTime<Utc>>,
}

pub trait PostStore: Send + Sync {
    fn get_posts(&self, sort_order: SortOrder, limit: i32) -> anyhow::Result<Vec<Post>>;
    fn get_posts_by_hashtag(
        &self,
        hashtag: &str,
        sort_order: SortOrder,
        limit: i32,
    ) -> anyhow::Result<Vec<Post>>;
    fn get_posts_by_username(
        &self,
        username: &str,
        sort_order: SortOrder,
        limit: i32,
    ) -> anyhow::Result<Vec<Post>>;
    fn get_posts_by_hashtag_and_username(
        &self,
        hashtag: &str,
        username: &str,
        sort_order: SortOrder,
        limit: i32,
    ) -> anyhow::Result<Vec<Post>>;
    fn get_by_id(&self, post_id: &Uuid) -> anyhow::Result<Option<Post>>;
    fn get_replies(&self, post_id: &Uuid) -> anyhow::Result<Vec<Post>>;
    fn create(&self, post: &Post) -> anyhow::Result<()>;
    fn update_content(&self, post_id: &Uuid, content: &str) -> anyhow::Result<()>;
    fn delete(&self, post_id: &Uuid) -> anyhow::Result<()>;
    fn update_vote_counts(&self, post_id: &Uuid) -> anyhow::Result<()>;
    fn get_post_count(&self, user_id: &Uuid) -> anyhow::Result<i32>;
}

pub trait HashtagStore: Send + Sync {
    fn get_by_post(&self, post_id: &Uuid) -> anyhow::Result<Vec<String>>;
    fn store_hashtags(&self, post_id: &Uuid, hashtags: &[String]) -> anyhow::Result<()>;
    fn delete_by_post(&self, post_id: &Uuid) -> anyhow::Result<()>;
    fn increment_activity(&self, user_id: &Uuid, hashtag: &str) -> anyhow::Result<()>;
    fn get_active_by_user(
        &self,
        user_id: &Uuid,
        limit: usize,
    ) -> anyhow::Result<Vec<(String, i64)>>;
    fn get_followed_by_user(&self, user_id: &Uuid) -> anyhow::Result<Vec<String>>;
    fn get_post_count(&self, name: &str) -> anyhow::Result<i32>;
    fn follow_hashtag(&self, user_id: &Uuid, name: &str) -> anyhow::Result<()>;
    fn unfollow_hashtag(&self, user_id: &Uuid, name: &str) -> anyhow::Result<()>;
    fn search(&self, query: &str, limit: usize) -> anyhow::Result<Vec<String>>;
}

pub trait VoteStore: Send + Sync {
    fn upsert_vote(
        &self,
        user_id: &Uuid,
        post_id: &Uuid,
        direction: VoteDirection,
    ) -> anyhow::Result<()>;
    fn get_vote(&self, user_id: &Uuid, post_id: &Uuid) -> anyhow::Result<Option<Vote>>;
    fn calculate_karma(&self, user_id: &Uuid) -> anyhow::Result<i32>;
}

pub trait UserStore: Send + Sync {
    fn get_by_id(&self, user_id: &Uuid) -> anyhow::Result<Option<User>>;
    fn get_by_username(&self, username: &str) -> anyhow::Result<Option<User>>;
    fn get_test_users(&self) -> anyhow::Result<Vec<User>>;
    fn list_all(&self) -> anyhow::Result<Vec<User>>;
    fn update_bio(&self, user_id: &Uuid, bio: &str) -> anyhow::Result<()>;
    fn create_or_update_from_github(
        &self,
        github_id: i64,
        github_login: &str,
        name: Option<&str>,
    ) -> anyhow::Result<User>;
}

pub trait FriendStore: Send + Sync {
    fn is_following(&self, follower_id: &Uuid, following_id: &Uuid) -> anyhow::Result<bool>;
    fn are_mutual_friends(&self, user_a: &Uuid, user_b: &Uuid) -> anyhow::Result<bool>;
    fn follow_user(&self, follower_id: &Uuid, following_id: &Uuid) -> anyhow::Result<()>;
    fn unfollow_user(&self, follower_id: &Uuid, following_id: &Uuid) -> anyhow::Result<usize>;
    fn get_following(&self, user_id: &Uuid) -> anyhow::Result<Vec<Uuid>>;
    fn get_followers(&self, user_id: &Uuid) -> anyhow::Result<Vec<Uuid>>;
    fn get_mutual_friends(&self, user_id: &Uuid) -> anyhow::Result<Vec<Uuid>>;
    fn get_follower_count(&self, user_id: &Uuid) -> anyhow::Result<usize>;
    fn get_following_count(&self, user_id: &Uuid) -> anyhow::Result<usize>;
}

pub trait ConfigStore: Send + Sync {
    fn get(&self, user_id: &Uuid) -> anyhow::Result<UserConfig>;
    fn update(&self, config: &UserConfig) -> anyhow::Result<()>;
}

pub trait RateLimitStore: Send + Sync {
    fn get_last_post_at(&self, user_id: &Uuid) -> anyhow::Result<Option<DateTime<Utc>>>;
    fn update_last_post_at(&self, user_id: &Uuid, at: DateTime<Utc>) -> anyhow::Result<()>;
    fn get_last_dm_at(&self, user_id: &Uuid) -> anyhow::Result<Option<DateTime<Utc>>>;
    fn update_last_dm_at(&self, user_id: &Uuid, at: DateTime<Utc>) -> anyhow::Result<()>;
}

pub trait DirectMessageStore: Send + Sync {
    fn create(&self, dm: &DirectMessage) -> anyhow::Result<()>;
    fn get_conversation_summaries(
        &self,
        user_id: &Uuid,
    ) -> anyhow::Result<Vec<DirectMessageConversationSummary>>;
    fn get_conversation(
        &self,
        user1_id: &Uuid,
        user2_id: &Uuid,
    ) -> anyhow::Result<Vec<DirectMessage>>;
    fn mark_as_read(&self, user_id: &Uuid, other_user_id: &Uuid) -> anyhow::Result<()>;
    fn delete_conversation(&self, user_id: &Uuid, other_user_id: &Uuid) -> anyhow::Result<()>;
}

#[derive(Debug, Clone)]
pub struct DirectMessageConversationSummary {
    pub other_user_id: Uuid,
    pub last_message: String,
    pub last_message_time: DateTime<Utc>,
    pub unread_count: usize,
}

pub trait SessionStore: Send + Sync {
    fn create_session(
        &self,
        token: &str,
        user_id: Uuid,
        created_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
        last_activity: DateTime<Utc>,
    ) -> anyhow::Result<()>;
    fn get_session(&self, token: &str) -> anyhow::Result<Option<SessionRecord>>;
    fn update_activity(&self, token: &str, at: DateTime<Utc>) -> anyhow::Result<()>;
    fn delete_session(&self, token: &str) -> anyhow::Result<usize>;
    fn cleanup_expired_sessions(&self, now: DateTime<Utc>) -> anyhow::Result<usize>;
    #[allow(dead_code)]
    fn invalidate_user_sessions(&self, user_id: Uuid) -> anyhow::Result<usize>;
}

pub trait AuditStore: Send + Sync {
    fn log_event(
        &self,
        event_type: &str,
        user_id: Option<Uuid>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
        details: Option<&str>,
        timestamp: DateTime<Utc>,
    ) -> anyhow::Result<Uuid>;
}

#[derive(Clone)]
pub struct Stores {
    pub posts: Arc<dyn PostStore>,
    pub hashtags: Arc<dyn HashtagStore>,
    pub votes: Arc<dyn VoteStore>,
    pub users: Arc<dyn UserStore>,
    pub friends: Arc<dyn FriendStore>,
    pub config: Arc<dyn ConfigStore>,
    pub rate_limits: Arc<dyn RateLimitStore>,
    pub dms: Arc<dyn DirectMessageStore>,
    pub sessions: Arc<dyn SessionStore>,
    pub audit: Arc<dyn AuditStore>,
}

impl Stores {
    pub fn sqlite(pool: DbPool) -> Self {
        Self {
            posts: Arc::new(sqlite::SqlitePostStore::new(pool.clone())),
            hashtags: Arc::new(sqlite::SqliteHashtagStore::new(pool.clone())),
            votes: Arc::new(sqlite::SqliteVoteStore::new(pool.clone())),
            users: Arc::new(sqlite::SqliteUserStore::new(pool.clone())),
            friends: Arc::new(sqlite::SqliteFriendStore::new(pool.clone())),
            config: Arc::new(sqlite::SqliteConfigStore::new(pool.clone())),
            rate_limits: Arc::new(sqlite::SqliteRateLimitStore::new(pool.clone())),
            dms: Arc::new(sqlite::SqliteDirectMessageStore::new(pool.clone())),
            sessions: Arc::new(sqlite::SqliteSessionStore::new(pool.clone())),
            audit: Arc::new(sqlite::SqliteAuditStore::new(pool)),
        }
    }
}

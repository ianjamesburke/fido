//! SQLite store implementations.

use chrono::{DateTime, Utc};
use rusqlite::OptionalExtension;
use uuid::Uuid;

use crate::db::repositories::{
    ConfigRepository, DirectMessageRepository, FriendRepository, HashtagRepository, PostRepository,
    UserRepository, VoteRepository,
};
use crate::db::DbPool;
use crate::stores::{
    AuditStore, ConfigStore, DirectMessageConversationSummary, DirectMessageStore, FriendStore,
    HashtagStore, PostStore, RateLimitStore, SessionRecord, SessionStore, UserStore, VoteStore,
};
use fido_types::{Post, SortOrder, User, UserConfig, Vote, VoteDirection};

pub struct SqlitePostStore {
    repo: PostRepository,
}

impl SqlitePostStore {
    pub fn new(pool: DbPool) -> Self {
        Self {
            repo: PostRepository::new(pool),
        }
    }
}

impl PostStore for SqlitePostStore {
    fn get_posts(&self, sort_order: SortOrder, limit: i32) -> anyhow::Result<Vec<Post>> {
        self.repo.get_posts(sort_order, limit)
    }

    fn get_posts_by_hashtag(
        &self,
        hashtag: &str,
        sort_order: SortOrder,
        limit: i32,
    ) -> anyhow::Result<Vec<Post>> {
        self.repo.get_posts_by_hashtag(hashtag, sort_order, limit)
    }

    fn get_posts_by_username(
        &self,
        username: &str,
        sort_order: SortOrder,
        limit: i32,
    ) -> anyhow::Result<Vec<Post>> {
        self.repo.get_posts_by_username(username, sort_order, limit)
    }

    fn get_posts_by_hashtag_and_username(
        &self,
        hashtag: &str,
        username: &str,
        sort_order: SortOrder,
        limit: i32,
    ) -> anyhow::Result<Vec<Post>> {
        self.repo
            .get_posts_by_hashtag_and_username(hashtag, username, sort_order, limit)
    }

    fn get_by_id(&self, post_id: &Uuid) -> anyhow::Result<Option<Post>> {
        self.repo.get_by_id(post_id)
    }

    fn get_replies(&self, post_id: &Uuid) -> anyhow::Result<Vec<Post>> {
        self.repo.get_replies(post_id)
    }

    fn create(&self, post: &Post) -> anyhow::Result<()> {
        self.repo.create(post)
    }

    fn update_content(&self, post_id: &Uuid, content: &str) -> anyhow::Result<()> {
        self.repo.update_content(post_id, content)
    }

    fn delete(&self, post_id: &Uuid) -> anyhow::Result<()> {
        self.repo.delete(post_id)
    }

    fn update_vote_counts(&self, post_id: &Uuid) -> anyhow::Result<()> {
        self.repo.update_vote_counts(post_id)
    }

    fn get_post_count(&self, user_id: &Uuid) -> anyhow::Result<i32> {
        self.repo.get_post_count(user_id)
    }
}

pub struct SqliteHashtagStore {
    repo: HashtagRepository,
}

impl SqliteHashtagStore {
    pub fn new(pool: DbPool) -> Self {
        Self {
            repo: HashtagRepository::new(pool),
        }
    }
}

impl HashtagStore for SqliteHashtagStore {
    fn get_by_post(&self, post_id: &Uuid) -> anyhow::Result<Vec<String>> {
        self.repo.get_by_post(post_id)
    }

    fn store_hashtags(&self, post_id: &Uuid, hashtags: &[String]) -> anyhow::Result<()> {
        self.repo.store_hashtags(post_id, hashtags)
    }

    fn delete_by_post(&self, post_id: &Uuid) -> anyhow::Result<()> {
        self.repo.delete_by_post(post_id)
    }

    fn increment_activity(&self, user_id: &Uuid, hashtag: &str) -> anyhow::Result<()> {
        self.repo.increment_activity(user_id, hashtag)
    }

    fn get_active_by_user(
        &self,
        user_id: &Uuid,
        limit: usize,
    ) -> anyhow::Result<Vec<(String, i64)>> {
        self.repo.get_active_by_user(user_id, limit)
    }

    fn get_followed_by_user(&self, user_id: &Uuid) -> anyhow::Result<Vec<String>> {
        self.repo.get_followed_by_user(user_id)
    }

    fn get_post_count(&self, name: &str) -> anyhow::Result<i32> {
        self.repo.get_post_count(name)
    }

    fn follow_hashtag(&self, user_id: &Uuid, name: &str) -> anyhow::Result<()> {
        self.repo.follow_hashtag(user_id, name)
    }

    fn unfollow_hashtag(&self, user_id: &Uuid, name: &str) -> anyhow::Result<()> {
        self.repo.unfollow_hashtag(user_id, name)
    }

    fn search(&self, query: &str, limit: usize) -> anyhow::Result<Vec<String>> {
        self.repo.search(query, limit)
    }
}

pub struct SqliteVoteStore {
    repo: VoteRepository,
}

impl SqliteVoteStore {
    pub fn new(pool: DbPool) -> Self {
        Self {
            repo: VoteRepository::new(pool),
        }
    }
}

impl VoteStore for SqliteVoteStore {
    fn upsert_vote(
        &self,
        user_id: &Uuid,
        post_id: &Uuid,
        direction: VoteDirection,
    ) -> anyhow::Result<()> {
        self.repo.upsert_vote(user_id, post_id, direction)
    }

    fn get_vote(&self, user_id: &Uuid, post_id: &Uuid) -> anyhow::Result<Option<Vote>> {
        self.repo.get_vote(user_id, post_id)
    }

    fn calculate_karma(&self, user_id: &Uuid) -> anyhow::Result<i32> {
        self.repo.calculate_karma(user_id)
    }
}

pub struct SqliteUserStore {
    repo: UserRepository,
}

impl SqliteUserStore {
    pub fn new(pool: DbPool) -> Self {
        Self {
            repo: UserRepository::new(pool),
        }
    }
}

impl UserStore for SqliteUserStore {
    fn get_by_id(&self, user_id: &Uuid) -> anyhow::Result<Option<User>> {
        self.repo.get_by_id(user_id)
    }

    fn get_by_username(&self, username: &str) -> anyhow::Result<Option<User>> {
        self.repo.get_by_username(username)
    }

    fn get_test_users(&self) -> anyhow::Result<Vec<User>> {
        self.repo.get_test_users()
    }

    fn list_all(&self) -> anyhow::Result<Vec<User>> {
        self.repo.list_all()
    }

    fn update_bio(&self, user_id: &Uuid, bio: &str) -> anyhow::Result<()> {
        self.repo.update_bio(user_id, bio)
    }

    fn create_or_update_from_github(
        &self,
        github_id: i64,
        github_login: &str,
        name: Option<&str>,
    ) -> anyhow::Result<User> {
        self.repo
            .create_or_update_from_github(github_id, github_login, name)
    }
}

pub struct SqliteFriendStore {
    repo: FriendRepository,
}

impl SqliteFriendStore {
    pub fn new(pool: DbPool) -> Self {
        Self {
            repo: FriendRepository::new(pool),
        }
    }
}

impl FriendStore for SqliteFriendStore {
    fn is_following(&self, follower_id: &Uuid, following_id: &Uuid) -> anyhow::Result<bool> {
        self.repo.is_following(follower_id, following_id)
    }

    fn are_mutual_friends(&self, user_a: &Uuid, user_b: &Uuid) -> anyhow::Result<bool> {
        self.repo.are_mutual_friends(user_a, user_b)
    }

    fn follow_user(&self, follower_id: &Uuid, following_id: &Uuid) -> anyhow::Result<()> {
        self.repo.follow_user(follower_id, following_id)
    }

    fn unfollow_user(&self, follower_id: &Uuid, following_id: &Uuid) -> anyhow::Result<usize> {
        self.repo.unfollow_user(follower_id, following_id)
    }

    fn get_following(&self, user_id: &Uuid) -> anyhow::Result<Vec<Uuid>> {
        self.repo.get_following(user_id)
    }

    fn get_followers(&self, user_id: &Uuid) -> anyhow::Result<Vec<Uuid>> {
        self.repo.get_followers(user_id)
    }

    fn get_mutual_friends(&self, user_id: &Uuid) -> anyhow::Result<Vec<Uuid>> {
        self.repo.get_mutual_friends(user_id)
    }

    fn get_follower_count(&self, user_id: &Uuid) -> anyhow::Result<usize> {
        self.repo.get_follower_count(user_id)
    }

    fn get_following_count(&self, user_id: &Uuid) -> anyhow::Result<usize> {
        self.repo.get_following_count(user_id)
    }
}

pub struct SqliteConfigStore {
    repo: ConfigRepository,
}

impl SqliteConfigStore {
    pub fn new(pool: DbPool) -> Self {
        Self {
            repo: ConfigRepository::new(pool),
        }
    }
}

impl ConfigStore for SqliteConfigStore {
    fn get(&self, user_id: &Uuid) -> anyhow::Result<UserConfig> {
        self.repo.get(user_id)
    }

    fn update(&self, config: &UserConfig) -> anyhow::Result<()> {
        self.repo.update(config)
    }
}

pub struct SqliteRateLimitStore {
    pool: DbPool,
}

impl SqliteRateLimitStore {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

impl RateLimitStore for SqliteRateLimitStore {
    fn get_last_post_at(&self, user_id: &Uuid) -> anyhow::Result<Option<DateTime<Utc>>> {
        let conn = self.pool.get()?;
        let last_post_at: Option<String> = conn
            .query_row(
                "SELECT last_post_at FROM post_rate_limits WHERE user_id = ?",
                [user_id.to_string()],
                |row| row.get(0),
            )
            .optional()?;

        match last_post_at {
            Some(value) => {
                let parsed = DateTime::parse_from_rfc3339(&value)?.with_timezone(&Utc);
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }

    fn update_last_post_at(&self, user_id: &Uuid, at: DateTime<Utc>) -> anyhow::Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO post_rate_limits (user_id, last_post_at) VALUES (?, ?)
             ON CONFLICT(user_id) DO UPDATE SET last_post_at = excluded.last_post_at",
            (user_id.to_string(), at.to_rfc3339()),
        )?;
        Ok(())
    }

    fn get_last_dm_at(&self, user_id: &Uuid) -> anyhow::Result<Option<DateTime<Utc>>> {
        let conn = self.pool.get()?;
        let last_dm_at: Option<String> = conn
            .query_row(
                "SELECT last_dm_at FROM dm_rate_limits WHERE user_id = ?",
                [user_id.to_string()],
                |row| row.get(0),
            )
            .optional()?;

        match last_dm_at {
            Some(value) => {
                let parsed = DateTime::parse_from_rfc3339(&value)?.with_timezone(&Utc);
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }

    fn update_last_dm_at(&self, user_id: &Uuid, at: DateTime<Utc>) -> anyhow::Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO dm_rate_limits (user_id, last_dm_at) VALUES (?, ?)
             ON CONFLICT(user_id) DO UPDATE SET last_dm_at = excluded.last_dm_at",
            (user_id.to_string(), at.to_rfc3339()),
        )?;
        Ok(())
    }
}

pub struct SqliteDirectMessageStore {
    repo: DirectMessageRepository,
}

impl SqliteDirectMessageStore {
    pub fn new(pool: DbPool) -> Self {
        Self {
            repo: DirectMessageRepository::new(pool),
        }
    }
}

impl DirectMessageStore for SqliteDirectMessageStore {
    fn create(&self, dm: &fido_types::DirectMessage) -> anyhow::Result<()> {
        self.repo.create(dm)
    }

    fn get_conversation_summaries(
        &self,
        user_id: &Uuid,
    ) -> anyhow::Result<Vec<DirectMessageConversationSummary>> {
        self.repo.get_conversation_summaries(user_id)
    }

    fn get_conversation(
        &self,
        user1_id: &Uuid,
        user2_id: &Uuid,
    ) -> anyhow::Result<Vec<fido_types::DirectMessage>> {
        self.repo.get_conversation(user1_id, user2_id)
    }

    fn mark_as_read(&self, user_id: &Uuid, other_user_id: &Uuid) -> anyhow::Result<()> {
        self.repo.mark_as_read(user_id, other_user_id)
    }

    fn delete_conversation(&self, user_id: &Uuid, other_user_id: &Uuid) -> anyhow::Result<()> {
        self.repo.delete_conversation(user_id, other_user_id)
    }
}

pub struct SqliteSessionStore {
    pool: DbPool,
}

impl SqliteSessionStore {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

impl SessionStore for SqliteSessionStore {
    fn create_session(
        &self,
        token: &str,
        user_id: Uuid,
        created_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
        last_activity: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO sessions (token, user_id, created_at, expires_at, last_activity) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                token,
                user_id.to_string(),
                created_at.to_rfc3339(),
                expires_at.to_rfc3339(),
                last_activity.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    fn get_session(&self, token: &str) -> anyhow::Result<Option<SessionRecord>> {
        let conn = self.pool.get()?;
        let row: Option<(String, String, Option<String>)> = conn
            .query_row(
                "SELECT user_id, expires_at, last_activity FROM sessions WHERE token = ?1",
                rusqlite::params![token],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;

        match row {
            Some((user_id_str, expires_at_str, last_activity_str)) => {
                let user_id = Uuid::parse_str(&user_id_str)?;
                let expires_at = DateTime::parse_from_rfc3339(&expires_at_str)?.with_timezone(&Utc);
                let last_activity = match last_activity_str {
                    Some(v) => Some(DateTime::parse_from_rfc3339(&v)?.with_timezone(&Utc)),
                    None => None,
                };
                Ok(Some(SessionRecord {
                    user_id,
                    expires_at,
                    last_activity,
                }))
            }
            None => Ok(None),
        }
    }

    fn update_activity(&self, token: &str, at: DateTime<Utc>) -> anyhow::Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE sessions SET last_activity = ?1 WHERE token = ?2",
            rusqlite::params![at.to_rfc3339(), token],
        )?;
        Ok(())
    }

    fn delete_session(&self, token: &str) -> anyhow::Result<usize> {
        let conn = self.pool.get()?;
        Ok(conn.execute(
            "DELETE FROM sessions WHERE token = ?1",
            rusqlite::params![token],
        )?)
    }

    fn cleanup_expired_sessions(&self, now: DateTime<Utc>) -> anyhow::Result<usize> {
        let conn = self.pool.get()?;
        Ok(conn.execute(
            "DELETE FROM sessions WHERE expires_at < ?1",
            rusqlite::params![now.to_rfc3339()],
        )?)
    }

    fn invalidate_user_sessions(&self, user_id: Uuid) -> anyhow::Result<usize> {
        let conn = self.pool.get()?;
        Ok(conn.execute(
            "DELETE FROM sessions WHERE user_id = ?1",
            rusqlite::params![user_id.to_string()],
        )?)
    }
}

pub struct SqliteAuditStore {
    pool: DbPool,
}

impl SqliteAuditStore {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

impl AuditStore for SqliteAuditStore {
    fn log_event(
        &self,
        event_type: &str,
        user_id: Option<Uuid>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
        details: Option<&str>,
        timestamp: DateTime<Utc>,
    ) -> anyhow::Result<Uuid> {
        let conn = self.pool.get()?;
        let id = Uuid::new_v4();
        conn.execute(
            "INSERT INTO audit_logs (id, event_type, user_id, ip_address, user_agent, details, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                id.to_string(),
                event_type,
                user_id.map(|u| u.to_string()),
                ip_address,
                user_agent,
                details,
                timestamp.to_rfc3339(),
            ],
        )?;
        Ok(id)
    }
}

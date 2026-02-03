//! Storage trait definitions and store wiring.

use std::sync::Arc;

use uuid::Uuid;

use crate::db::DbPool;
use fido_types::{Post, SortOrder, User, Vote, VoteDirection};

pub mod sqlite;

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
    fn get_active_by_user(&self, user_id: &Uuid, limit: usize) -> anyhow::Result<Vec<(String, i64)>>;
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
    fn update_bio(&self, user_id: &Uuid, bio: &str) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub struct Stores {
    pub posts: Arc<dyn PostStore>,
    pub hashtags: Arc<dyn HashtagStore>,
    pub votes: Arc<dyn VoteStore>,
    pub users: Arc<dyn UserStore>,
}

impl Stores {
    pub fn sqlite(pool: DbPool) -> Self {
        Self {
            posts: Arc::new(sqlite::SqlitePostStore::new(pool.clone())),
            hashtags: Arc::new(sqlite::SqliteHashtagStore::new(pool.clone())),
            votes: Arc::new(sqlite::SqliteVoteStore::new(pool.clone())),
            users: Arc::new(sqlite::SqliteUserStore::new(pool)),
        }
    }
}

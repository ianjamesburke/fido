//! SQLite store implementations.

use uuid::Uuid;

use crate::db::repositories::{HashtagRepository, PostRepository, UserRepository, VoteRepository};
use crate::db::DbPool;
use crate::stores::{HashtagStore, PostStore, UserStore, VoteStore};
use fido_types::{Post, SortOrder, User, Vote, VoteDirection};

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
        self.repo
            .get_posts_by_hashtag(hashtag, sort_order, limit)
    }

    fn get_posts_by_username(
        &self,
        username: &str,
        sort_order: SortOrder,
        limit: i32,
    ) -> anyhow::Result<Vec<Post>> {
        self.repo
            .get_posts_by_username(username, sort_order, limit)
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

    fn get_active_by_user(&self, user_id: &Uuid, limit: usize) -> anyhow::Result<Vec<(String, i64)>> {
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

    fn update_bio(&self, user_id: &Uuid, bio: &str) -> anyhow::Result<()> {
        self.repo.update_bio(user_id, bio)
    }
}

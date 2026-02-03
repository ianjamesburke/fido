//! Post-related business logic.

use uuid::Uuid;

use crate::api::{ApiError, ApiResult};
use crate::db::DbPool;
use crate::stores::Stores;
use fido_types::{Post, SortOrder, User, VoteDirection};

pub struct PostService {
    stores: Stores,
}

impl PostService {
    pub fn new(stores: Stores) -> Self {
        Self { stores }
    }

    pub fn sqlite(pool: DbPool) -> Self {
        Self::new(Stores::sqlite(pool))
    }

    pub fn get_posts(
        &self,
        sort_order: SortOrder,
        limit: i32,
        hashtag: Option<&str>,
        username: Option<&str>,
        user_id: Option<Uuid>,
    ) -> ApiResult<Vec<Post>> {
        let mut posts = match (hashtag, username) {
            (Some(tag), Some(user)) => self
                .stores
                .posts
                .get_posts_by_hashtag_and_username(tag, user, sort_order, limit)?,
            (Some(tag), None) => self
                .stores
                .posts
                .get_posts_by_hashtag(tag, sort_order, limit)?,
            (None, Some(user)) => self
                .stores
                .posts
                .get_posts_by_username(user, sort_order, limit)?,
            (None, None) => self.stores.posts.get_posts(sort_order, limit)?,
        };

        if let (Some(tag), Some(uid)) = (hashtag, user_id) {
            let _ = self.stores.hashtags.increment_activity(&uid, tag);
        }

        self.populate_posts(&mut posts, user_id)?;

        Ok(posts)
    }

    pub fn get_replies(&self, post_id: &Uuid, user_id: Option<Uuid>) -> ApiResult<Vec<Post>> {
        self.stores
            .posts
            .get_by_id(post_id)?
            .ok_or_else(|| ApiError::NotFound("Post not found".to_string()))?;

        let mut replies = self.stores.posts.get_replies(post_id)?;
        self.populate_posts(&mut replies, user_id)?;

        Ok(replies)
    }

    pub fn get_post(&self, post_id: &Uuid, user_id: Option<Uuid>) -> ApiResult<Post> {
        let mut post = self
            .stores
            .posts
            .get_by_id(post_id)?
            .ok_or_else(|| ApiError::NotFound("Post not found".to_string()))?;

        self.populate_post(&mut post, user_id)?;

        Ok(post)
    }

    pub fn get_thread_parts(
        &self,
        post_id: &Uuid,
        user_id: Option<Uuid>,
    ) -> ApiResult<(Post, Vec<Post>)> {
        let mut root_post = self
            .stores
            .posts
            .get_by_id(post_id)?
            .ok_or_else(|| ApiError::NotFound("Post not found".to_string()))?;
        self.populate_post(&mut root_post, user_id)?;

        let mut replies = self.stores.posts.get_replies(post_id)?;
        self.populate_posts(&mut replies, user_id)?;

        Ok((root_post, replies))
    }

    pub fn get_user_by_id(&self, user_id: &Uuid) -> ApiResult<User> {
        self.stores
            .users
            .get_by_id(user_id)?
            .ok_or_else(|| ApiError::NotFound("User not found".to_string()))
    }

    pub fn create_post(&self, post: &Post) -> ApiResult<()> {
        self.stores.posts.create(post)?;
        Ok(())
    }

    pub fn store_hashtags(&self, post_id: &Uuid, hashtags: &[String]) -> ApiResult<()> {
        self.stores.hashtags.store_hashtags(post_id, hashtags)?;
        Ok(())
    }

    pub fn delete_post_hashtags(&self, post_id: &Uuid) -> ApiResult<()> {
        self.stores.hashtags.delete_by_post(post_id)?;
        Ok(())
    }

    pub fn update_post_content(&self, post_id: &Uuid, content: &str) -> ApiResult<()> {
        self.stores.posts.update_content(post_id, content)?;
        Ok(())
    }

    pub fn delete_post(&self, post_id: &Uuid) -> ApiResult<()> {
        self.stores.posts.delete(post_id)?;
        Ok(())
    }

    pub fn record_vote(
        &self,
        user_id: &Uuid,
        post_id: &Uuid,
        direction: VoteDirection,
    ) -> ApiResult<()> {
        self.stores
            .posts
            .get_by_id(post_id)?
            .ok_or_else(|| ApiError::NotFound("Post not found".to_string()))?;

        self.stores.votes.upsert_vote(user_id, post_id, direction)?;
        self.stores.posts.update_vote_counts(post_id)?;

        let hashtags = self.stores.hashtags.get_by_post(post_id)?;
        for hashtag in hashtags {
            let _ = self.stores.hashtags.increment_activity(user_id, &hashtag);
        }

        Ok(())
    }

    pub fn increment_hashtag_activity(&self, user_id: &Uuid, hashtag: &str) {
        let _ = self.stores.hashtags.increment_activity(user_id, hashtag);
    }

    pub fn verify_ownership(&self, user_id: &Uuid, post_id: &Uuid) -> ApiResult<()> {
        let post = self
            .stores
            .posts
            .get_by_id(post_id)?
            .ok_or_else(|| ApiError::NotFound("Post not found".to_string()))?;

        if &post.author_id != user_id {
            return Err(ApiError::Forbidden(
                "You don't have permission to modify this post".to_string(),
            ));
        }

        Ok(())
    }

    fn populate_posts(&self, posts: &mut [Post], user_id: Option<Uuid>) -> ApiResult<()> {
        for post in posts {
            self.populate_post(post, user_id)?;
        }
        Ok(())
    }

    fn populate_post(&self, post: &mut Post, user_id: Option<Uuid>) -> ApiResult<()> {
        post.hashtags = self.stores.hashtags.get_by_post(&post.id)?;

        if let Some(uid) = user_id {
            if let Ok(Some(vote)) = self.stores.votes.get_vote(&uid, &post.id) {
                post.user_vote = Some(vote.direction.as_str().to_string());
            }
        }

        Ok(())
    }
}

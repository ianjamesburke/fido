//! Post-related business logic.

use uuid::Uuid;

use crate::api::{ApiError, ApiResult};
use crate::db::repositories::{HashtagRepository, PostRepository, VoteRepository};
use crate::db::DbPool;
use fido_types::{Post, SortOrder};

pub struct PostService {
    post_repo: PostRepository,
    hashtag_repo: HashtagRepository,
    vote_repo: VoteRepository,
}

impl PostService {
    pub fn new(pool: DbPool) -> Self {
        Self {
            post_repo: PostRepository::new(pool.clone()),
            hashtag_repo: HashtagRepository::new(pool.clone()),
            vote_repo: VoteRepository::new(pool.clone()),
        }
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
                .post_repo
                .get_posts_by_hashtag_and_username(tag, user, sort_order, limit)?,
            (Some(tag), None) => self.post_repo.get_posts_by_hashtag(tag, sort_order, limit)?,
            (None, Some(user)) => self.post_repo.get_posts_by_username(user, sort_order, limit)?,
            (None, None) => self.post_repo.get_posts(sort_order, limit)?,
        };

        if let (Some(tag), Some(uid)) = (hashtag, user_id) {
            let _ = self.hashtag_repo.increment_activity(&uid, tag);
        }

        self.populate_posts(&mut posts, user_id)?;

        Ok(posts)
    }

    pub fn get_replies(&self, post_id: &Uuid, user_id: Option<Uuid>) -> ApiResult<Vec<Post>> {
        self.post_repo
            .get_by_id(post_id)?
            .ok_or_else(|| ApiError::NotFound("Post not found".to_string()))?;

        let mut replies = self.post_repo.get_replies(post_id)?;
        self.populate_posts(&mut replies, user_id)?;

        Ok(replies)
    }

    pub fn get_post(&self, post_id: &Uuid, user_id: Option<Uuid>) -> ApiResult<Post> {
        let mut post = self
            .post_repo
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
            .post_repo
            .get_by_id(post_id)?
            .ok_or_else(|| ApiError::NotFound("Post not found".to_string()))?;
        self.populate_post(&mut root_post, user_id)?;

        let mut replies = self.post_repo.get_replies(post_id)?;
        self.populate_posts(&mut replies, user_id)?;

        Ok((root_post, replies))
    }

    fn populate_posts(&self, posts: &mut [Post], user_id: Option<Uuid>) -> ApiResult<()> {
        for post in posts {
            self.populate_post(post, user_id)?;
        }
        Ok(())
    }

    fn populate_post(&self, post: &mut Post, user_id: Option<Uuid>) -> ApiResult<()> {
        post.hashtags = self.hashtag_repo.get_by_post(&post.id)?;

        if let Some(uid) = user_id {
            if let Ok(Some(vote)) = self.vote_repo.get_vote(&uid, &post.id) {
                post.user_vote = Some(vote.direction.as_str().to_string());
            }
        }

        Ok(())
    }
}

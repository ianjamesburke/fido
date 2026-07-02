//! Post-related business logic.

use uuid::Uuid;

use crate::{
    api::{ApiError, ApiResult},
    db::repositories::Repositories,
    events::{ServerEvent, SharedEventBus},
};
use fido_types::{MembershipRole, Post, SortOrder, User, VoteDirection};

pub struct PostService {
    repos: Repositories,
    event_bus: SharedEventBus,
}

impl PostService {
    pub fn new(repos: Repositories, event_bus: SharedEventBus) -> Self {
        Self { repos, event_bus }
    }

    pub fn get_posts(
        &self,
        community_id: Uuid,
        sort_order: SortOrder,
        limit: i32,
        username: Option<&str>,
        user_id: Uuid,
    ) -> ApiResult<Vec<Post>> {
        self.require_membership(user_id, community_id)?;
        let mut posts = match username {
            Some(user) => {
                self.repos
                    .posts
                    .get_posts_by_username(&community_id, user, sort_order, limit)?
            }
            None => self
                .repos
                .posts
                .get_posts(&community_id, sort_order, limit)?,
        };

        self.populate_posts(&mut posts, Some(user_id))?;

        Ok(posts)
    }

    pub fn get_pending_posts(
        &self,
        community_id: Uuid,
        user_id: Uuid,
        limit: i32,
    ) -> ApiResult<Vec<Post>> {
        self.require_admin(user_id, community_id)?;
        let mut posts = self.repos.posts.get_pending_posts(&community_id, limit)?;
        self.populate_posts(&mut posts, Some(user_id))?;
        Ok(posts)
    }

    pub fn get_replies(&self, post_id: &Uuid, user_id: Uuid) -> ApiResult<Vec<Post>> {
        let parent = self
            .repos
            .posts
            .get_by_id(post_id)?
            .ok_or_else(|| ApiError::NotFound("Post not found".to_string()))?;
        self.ensure_visible(&parent, user_id)?;

        let mut replies = self.repos.posts.get_replies(post_id)?;
        self.populate_posts(&mut replies, Some(user_id))?;

        Ok(replies)
    }

    pub fn get_post(&self, post_id: &Uuid, user_id: Uuid) -> ApiResult<Post> {
        let mut post = self
            .repos
            .posts
            .get_by_id(post_id)?
            .ok_or_else(|| ApiError::NotFound("Post not found".to_string()))?;

        self.ensure_visible(&post, user_id)?;
        self.populate_post(&mut post, Some(user_id))?;

        Ok(post)
    }

    pub fn get_thread_parts(&self, post_id: &Uuid, user_id: Uuid) -> ApiResult<(Post, Vec<Post>)> {
        let mut root_post = self
            .repos
            .posts
            .get_by_id(post_id)?
            .ok_or_else(|| ApiError::NotFound("Post not found".to_string()))?;
        self.ensure_visible(&root_post, user_id)?;
        self.populate_post(&mut root_post, Some(user_id))?;

        let mut replies = self.repos.posts.get_replies(post_id)?;
        self.populate_posts(&mut replies, Some(user_id))?;

        Ok((root_post, replies))
    }

    pub fn get_user_by_id(&self, user_id: &Uuid) -> ApiResult<User> {
        self.repos
            .users
            .get_by_id(user_id)?
            .ok_or_else(|| ApiError::NotFound("User not found".to_string()))
    }

    pub fn create_post(&self, post: &Post) -> ApiResult<()> {
        self.require_membership(post.author_id, post.community_id)?;
        self.repos.posts.create(post)?;
        if post.parent_post_id.is_none() {
            let event = if post.approved {
                ServerEvent::ThreadCreated(post.clone())
            } else {
                ServerEvent::ThreadPendingApproval(post.clone())
            };
            self.event_bus.emit(event)?;
        }
        Ok(())
    }

    pub fn update_post_content(&self, post_id: &Uuid, content: &str) -> ApiResult<()> {
        self.repos.posts.update_content(post_id, content)?;
        Ok(())
    }

    pub fn delete_post(&self, post_id: &Uuid) -> ApiResult<()> {
        self.repos.posts.delete(post_id)?;
        Ok(())
    }

    pub fn record_vote(
        &self,
        user_id: &Uuid,
        post_id: &Uuid,
        direction: VoteDirection,
    ) -> ApiResult<()> {
        let post = self
            .repos
            .posts
            .get_by_id(post_id)?
            .ok_or_else(|| ApiError::NotFound("Post not found".to_string()))?;
        self.ensure_visible(&post, *user_id)?;

        self.repos.votes.upsert_vote(user_id, post_id, direction)?;
        self.repos.posts.update_vote_counts(post_id)?;

        Ok(())
    }

    pub fn approve_post(&self, user_id: Uuid, post_id: &Uuid) -> ApiResult<Post> {
        let mut post = self
            .repos
            .posts
            .get_by_id(post_id)?
            .ok_or_else(|| ApiError::NotFound("Post not found".to_string()))?;
        if post.parent_post_id.is_some() {
            return Err(ApiError::BadRequest(
                "Replies do not require approval".to_string(),
            ));
        }
        self.require_admin(user_id, post.community_id)?;
        self.repos.posts.approve(post_id)?;
        post.approved = true;
        self.populate_post(&mut post, Some(user_id))?;
        Ok(post)
    }

    pub fn verify_ownership(&self, user_id: &Uuid, post_id: &Uuid) -> ApiResult<()> {
        let post = self
            .repos
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
        if let Some(uid) = user_id {
            if let Ok(Some(vote)) = self.repos.votes.get_vote(&uid, &post.id) {
                post.user_vote = Some(vote.direction.as_str().to_string());
            }
        }

        Ok(())
    }

    fn ensure_visible(&self, post: &Post, user_id: Uuid) -> ApiResult<()> {
        self.require_membership(user_id, post.community_id)?;
        if !post.approved && post.parent_post_id.is_none() {
            self.require_admin(user_id, post.community_id)?;
        }
        Ok(())
    }

    fn require_membership(&self, user_id: Uuid, community_id: Uuid) -> ApiResult<()> {
        self.repos
            .memberships
            .get(&community_id, &user_id)?
            .ok_or_else(|| ApiError::Forbidden("Community membership required".to_string()))?;
        Ok(())
    }

    fn require_admin(&self, user_id: Uuid, community_id: Uuid) -> ApiResult<()> {
        let membership = self
            .repos
            .memberships
            .get(&community_id, &user_id)?
            .ok_or_else(|| ApiError::Forbidden("Community membership required".to_string()))?;
        if membership.role != MembershipRole::Admin {
            return Err(ApiError::Forbidden(
                "Community admin role required".to_string(),
            ));
        }
        Ok(())
    }
}

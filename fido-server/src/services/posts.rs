//! Post-related business logic.

use uuid::Uuid;

use crate::{
    api::{ApiError, ApiResult},
    db::repositories::Repositories,
    events::{ServerEvent, SharedEventBus},
    mention::extract_mentions,
    services::notifications::NotificationService,
};
use fido_types::{MembershipRole, NotificationType, Post, SortOrder, User, VoteDirection};

pub struct PostService {
    repos: Repositories,
    event_bus: SharedEventBus,
    activity: Option<crate::services::activity::ActivityService>,
}

#[cfg(all(test, feature = "sqlite-tests"))]
mod tests {
    use super::*;
    use crate::{
        db::Database,
        events::{EventBus, ServerEvent},
    };
    use anyhow::Result;
    use chrono::Utc;
    use fido_types::{Community, Membership};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct RecordingEventBus {
        events: Mutex<Vec<ServerEvent>>,
    }

    impl EventBus for RecordingEventBus {
        fn emit(&self, event: ServerEvent) -> Result<()> {
            self.events.lock().unwrap().push(event);
            Ok(())
        }
    }

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

    fn setup() -> Result<(
        Repositories,
        Arc<RecordingEventBus>,
        Community,
        User,
        User,
        User,
    )> {
        let db = Database::in_memory()?;
        db.initialize()?;
        let repos = Repositories::new(db.pool);
        let admin = test_user("admin");
        let author = test_user("author");
        let mentioned = test_user("mentioned");
        repos.users.create(&admin)?;
        repos.users.create(&author)?;
        repos.users.create(&mentioned)?;

        let community = Community {
            id: Uuid::new_v4(),
            github_repo_id: 9009,
            owner: "octocat".to_string(),
            name: "notifications".to_string(),
            claimed_by: Some(admin.id),
            require_thread_approval: false,
            created_at: Utc::now(),
        };
        repos.communities.create(&community)?;
        for (user, role) in [
            (&admin, MembershipRole::Admin),
            (&author, MembershipRole::Member),
            (&mentioned, MembershipRole::Member),
        ] {
            repos.memberships.insert(&Membership {
                community_id: community.id,
                user_id: user.id,
                role,
                created_at: Utc::now(),
            })?;
        }
        Ok((
            repos,
            Arc::new(RecordingEventBus::default()),
            community,
            admin,
            author,
            mentioned,
        ))
    }

    fn post(author: &User, community_id: Uuid, content: &str) -> Post {
        Post {
            id: Uuid::new_v4(),
            author_id: author.id,
            author_username: author.username.clone(),
            community_id,
            content: content.to_string(),
            created_at: Utc::now(),
            upvotes: 0,
            downvotes: 0,
            approved: true,
            hashtags: Vec::new(),
            user_vote: None,
            parent_post_id: None,
            reply_count: 0,
            reply_to_user_id: None,
            reply_to_username: None,
            github_id: None,
            github_kind: None,
            github_state: None,
            github_html_url: None,
        }
    }

    #[test]
    fn create_reply_notifies_parent_author_and_mentions() -> Result<()> {
        let (repos, event_bus, community, _admin, author, mentioned) = setup()?;
        let service = PostService::new(repos.clone(), event_bus.clone());
        let root = post(&author, community.id, "root");
        service.create_post(&root).expect("root creates");

        let reply_author = mentioned.clone();
        let reply = Post {
            id: Uuid::new_v4(),
            author_id: reply_author.id,
            author_username: reply_author.username,
            community_id: community.id,
            content: "@author @mentioned hi".to_string(),
            created_at: Utc::now(),
            upvotes: 0,
            downvotes: 0,
            approved: true,
            hashtags: Vec::new(),
            user_vote: None,
            parent_post_id: Some(root.id),
            reply_count: 0,
            reply_to_user_id: Some(root.author_id),
            reply_to_username: Some(root.author_username),
            github_id: None,
            github_kind: None,
            github_state: None,
            github_html_url: None,
        };
        service.create_post(&reply).expect("reply creates");

        let author_notifications = repos.notifications.list_for_user(&author.id, 10, 0)?;
        assert_eq!(author_notifications.len(), 2);
        assert!(author_notifications
            .iter()
            .any(|n| n.notification_type == NotificationType::Reply));
        assert!(author_notifications
            .iter()
            .any(|n| n.notification_type == NotificationType::Mention));

        let self_notifications = repos.notifications.list_for_user(&mentioned.id, 10, 0)?;
        assert!(self_notifications.is_empty());
        assert!(event_bus
            .events
            .lock()
            .unwrap()
            .iter()
            .any(|event| matches!(event, ServerEvent::NotificationCreated(_))));
        Ok(())
    }

    #[test]
    fn approving_thread_notifies_author() -> Result<()> {
        let (repos, _event_bus, community, admin, author, _mentioned) = setup()?;
        let service = PostService::new(repos.clone(), Arc::new(RecordingEventBus::default()));
        let pending = Post {
            approved: false,
            ..post(&author, community.id, "pending")
        };
        service.create_post(&pending).expect("pending creates");
        service
            .approve_post(admin.id, &pending.id)
            .expect("admin approves");

        let notifications = repos.notifications.list_for_user(&author.id, 10, 0)?;
        assert_eq!(notifications.len(), 1);
        assert_eq!(
            notifications[0].notification_type,
            NotificationType::ThreadApproved
        );
        Ok(())
    }

    #[test]
    fn rejecting_thread_deletes_and_notifies_author() -> Result<()> {
        let (repos, _event_bus, community, admin, author, _mentioned) = setup()?;
        let service = PostService::new(repos.clone(), Arc::new(RecordingEventBus::default()));
        let pending = Post {
            approved: false,
            ..post(&author, community.id, "pending")
        };
        service.create_post(&pending).expect("pending creates");
        service
            .reject_post(admin.id, &pending.id)
            .expect("admin rejects");

        assert!(repos.posts.get_by_id(&pending.id)?.is_none());
        let notifications = repos.notifications.list_for_user(&author.id, 10, 0)?;
        assert_eq!(notifications.len(), 1);
        assert_eq!(
            notifications[0].notification_type,
            NotificationType::ThreadRejected
        );
        Ok(())
    }
}

impl PostService {
    pub fn new(repos: Repositories, event_bus: SharedEventBus) -> Self {
        Self {
            repos,
            event_bus,
            activity: None,
        }
    }

    pub fn with_activity(mut self, activity: crate::services::activity::ActivityService) -> Self {
        self.activity = Some(activity);
        self
    }

    pub async fn get_posts(
        &self,
        community_id: Uuid,
        sort_order: SortOrder,
        limit: i32,
        username: Option<&str>,
        user_id: Uuid,
    ) -> ApiResult<Vec<Post>> {
        self.require_membership(user_id, community_id)?;
        if let Some(activity) = &self.activity {
            activity.sync_activity(community_id).await?;
        }
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
        } else {
            self.notify_reply_targets(post)?;
        }
        self.notify_mentions(post)?;
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
        NotificationService::new(self.repos.clone(), self.event_bus.clone()).create(
            post.author_id,
            NotificationType::ThreadApproved,
            user_id,
            "community",
            post.community_id.to_string(),
        )?;
        self.populate_post(&mut post, Some(user_id))?;
        Ok(post)
    }

    pub fn reject_post(&self, user_id: Uuid, post_id: &Uuid) -> ApiResult<Post> {
        let post = self
            .repos
            .posts
            .get_by_id(post_id)?
            .ok_or_else(|| ApiError::NotFound("Post not found".to_string()))?;
        if post.parent_post_id.is_some() {
            return Err(ApiError::BadRequest(
                "Replies do not require approval".to_string(),
            ));
        }
        if post.approved {
            return Err(ApiError::BadRequest(
                "Thread is already approved".to_string(),
            ));
        }
        self.require_admin(user_id, post.community_id)?;
        self.repos.posts.delete(post_id)?;
        NotificationService::new(self.repos.clone(), self.event_bus.clone()).create(
            post.author_id,
            NotificationType::ThreadRejected,
            user_id,
            "community",
            post.community_id.to_string(),
        )?;
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

    fn notify_reply_targets(&self, post: &Post) -> ApiResult<()> {
        let Some(reply_to_user_id) = post.reply_to_user_id else {
            return Ok(());
        };

        NotificationService::new(self.repos.clone(), self.event_bus.clone()).create(
            reply_to_user_id,
            NotificationType::Reply,
            post.author_id,
            "community",
            post.community_id.to_string(),
        )?;
        Ok(())
    }

    fn notify_mentions(&self, post: &Post) -> ApiResult<()> {
        let notifications = NotificationService::new(self.repos.clone(), self.event_bus.clone());
        for username in extract_mentions(&post.content) {
            if let Some(user) = self.repos.users.get_by_username(&username)? {
                notifications.create(
                    user.id,
                    NotificationType::Mention,
                    post.author_id,
                    "community",
                    post.community_id.to_string(),
                )?;
            }
        }
        Ok(())
    }
}

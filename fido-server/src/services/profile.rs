//! Profile-related business logic.

use uuid::Uuid;

use crate::api::{ApiError, ApiResult};
use crate::db::repositories::Repositories;
use fido_types::UserProfile;

pub struct ProfileService {
    repos: Repositories,
}

impl ProfileService {
    pub fn new(repos: Repositories) -> Self {
        Self { repos }
    }

    pub fn get_profile(&self, user_id: &Uuid) -> ApiResult<UserProfile> {
        let user = self
            .repos
            .users
            .get_by_id(user_id)?
            .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

        let karma = self.repos.votes.calculate_karma(user_id)?;
        let post_count = self.repos.posts.get_post_count(user_id)?;
        let active_hashtags = self.repos.hashtags.get_active_by_user(user_id, 5)?;

        let recent_hashtags: Vec<String> = active_hashtags
            .into_iter()
            .map(|(name, _count)| name)
            .collect();

        Ok(UserProfile {
            user_id: user.id,
            username: user.username,
            bio: user.bio,
            karma,
            post_count,
            join_date: user.join_date,
            recent_hashtags,
        })
    }

    pub fn update_bio(&self, user_id: &Uuid, bio: &str) -> ApiResult<()> {
        self.repos
            .users
            .get_by_id(user_id)?
            .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

        self.repos.users.update_bio(user_id, bio)?;
        Ok(())
    }

    pub fn get_user_hashtags(&self, user_id: &Uuid, limit: usize) -> ApiResult<Vec<String>> {
        let active_hashtags = self.repos.hashtags.get_active_by_user(user_id, limit)?;
        Ok(active_hashtags
            .into_iter()
            .map(|(name, _count)| name)
            .collect())
    }
}

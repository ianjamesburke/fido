//! Profile-related business logic.

use uuid::Uuid;

use crate::api::{ApiError, ApiResult};
use crate::db::DbPool;
use crate::stores::Stores;
use fido_types::UserProfile;

pub struct ProfileService {
    stores: Stores,
}

impl ProfileService {
    pub fn new(stores: Stores) -> Self {
        Self { stores }
    }

    pub fn sqlite(pool: DbPool) -> Self {
        Self::new(Stores::sqlite(pool))
    }

    pub fn get_profile(&self, user_id: &Uuid) -> ApiResult<UserProfile> {
        let user = self
            .stores
            .users
            .get_by_id(user_id)?
            .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

        let karma = self.stores.votes.calculate_karma(user_id)?;
        let post_count = self.stores.posts.get_post_count(user_id)?;
        let active_hashtags = self.stores.hashtags.get_active_by_user(user_id, 5)?;

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
        self.stores
            .users
            .get_by_id(user_id)?
            .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

        self.stores.users.update_bio(user_id, bio)?;
        Ok(())
    }

    pub fn get_user_hashtags(&self, user_id: &Uuid, limit: usize) -> ApiResult<Vec<String>> {
        let active_hashtags = self.stores.hashtags.get_active_by_user(user_id, limit)?;
        Ok(active_hashtags
            .into_iter()
            .map(|(name, _count)| name)
            .collect())
    }
}

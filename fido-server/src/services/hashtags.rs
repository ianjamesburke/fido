//! Hashtag-related business logic.

use uuid::Uuid;

use crate::api::ApiResult;
use crate::stores::HashtagStore;
use std::sync::Arc;

#[derive(Debug)]
pub struct HashtagWithCount {
    pub name: String,
    pub post_count: i32,
}

#[derive(Debug)]
pub struct ActiveHashtag {
    pub name: String,
    pub interaction_count: i64,
}

pub struct HashtagService {
    store: Arc<dyn HashtagStore>,
}

impl HashtagService {
    pub fn new(store: Arc<dyn HashtagStore>) -> Self {
        Self { store }
    }

    pub fn get_followed_hashtags(&self, user_id: &Uuid) -> ApiResult<Vec<HashtagWithCount>> {
        let hashtags = self.store.get_followed_by_user(user_id)?;

        let mut response = Vec::new();
        for name in hashtags {
            let post_count = self.store.get_post_count(&name)?;
            response.push(HashtagWithCount { name, post_count });
        }

        Ok(response)
    }

    pub fn follow_hashtag(&self, user_id: &Uuid, name: &str) -> ApiResult<()> {
        self.store.follow_hashtag(user_id, name)?;
        Ok(())
    }

    pub fn unfollow_hashtag(&self, user_id: &Uuid, name: &str) -> ApiResult<()> {
        self.store.unfollow_hashtag(user_id, name)?;
        Ok(())
    }

    pub fn search_hashtags(&self, query: &str, limit: usize) -> ApiResult<Vec<HashtagWithCount>> {
        let hashtags = self.store.search(query, limit)?;

        let mut response = Vec::new();
        for name in hashtags {
            let post_count = self.store.get_post_count(&name)?;
            response.push(HashtagWithCount { name, post_count });
        }

        Ok(response)
    }

    pub fn get_active_hashtags(&self, user_id: &Uuid, limit: usize) -> ApiResult<Vec<ActiveHashtag>> {
        let hashtags = self.store.get_active_by_user(user_id, limit)?;
        Ok(hashtags
            .into_iter()
            .map(|(name, interaction_count)| ActiveHashtag {
                name,
                interaction_count,
            })
            .collect())
    }

}

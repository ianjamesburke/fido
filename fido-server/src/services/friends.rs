//! Social and friends related business logic.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::api::{ApiError, ApiResult};
use crate::stores::Stores;

#[derive(Debug)]
pub struct UserSearchResult {
    pub id: Uuid,
    pub username: String,
}

#[derive(Debug)]
pub struct SocialUser {
    pub id: Uuid,
    pub username: String,
    pub follower_count: usize,
    pub following_count: usize,
}

#[derive(Debug)]
pub struct RelationshipInfo {
    pub is_self: bool,
    pub is_mutual: bool,
    pub is_following: bool,
    pub follows_you: bool,
}

#[derive(Debug)]
pub struct UserProfileData {
    pub id: Uuid,
    pub username: String,
    pub bio: Option<String>,
    pub join_date: DateTime<Utc>,
    pub follower_count: usize,
    pub following_count: usize,
    pub post_count: usize,
    pub relationship: RelationshipInfo,
}

pub struct FriendsService {
    stores: Stores,
}

impl FriendsService {
    pub fn new(stores: Stores) -> Self {
        Self { stores }
    }

    pub fn search_users(&self, query: &str, limit: usize) -> ApiResult<Vec<UserSearchResult>> {
        let all_users = self.stores.users.list_all()?;
        let search_lower = query.to_lowercase();

        let mut results: Vec<_> = all_users
            .into_iter()
            .filter(|u| u.username.to_lowercase().contains(&search_lower))
            .map(|u| UserSearchResult {
                id: u.id,
                username: u.username,
            })
            .collect();

        results.sort_by(|a, b| {
            let a_exact = a.username.to_lowercase() == search_lower;
            let b_exact = b.username.to_lowercase() == search_lower;

            match (a_exact, b_exact) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.username.cmp(&b.username),
            }
        });

        results.truncate(limit);

        Ok(results)
    }

    pub fn get_user_profile(
        &self,
        viewer_id: Option<Uuid>,
        profile_user_id: &Uuid,
    ) -> ApiResult<UserProfileData> {
        let user = self
            .stores
            .users
            .get_by_id(profile_user_id)?
            .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

        let follower_count = self
            .stores
            .friends
            .get_follower_count(profile_user_id)
            .unwrap_or(0);
        let following_count = self
            .stores
            .friends
            .get_following_count(profile_user_id)
            .unwrap_or(0);

        let post_count = self.stores.posts.get_post_count(profile_user_id).unwrap_or(0) as usize;

        let relationship = if let Some(viewer) = viewer_id {
            if viewer == *profile_user_id {
                RelationshipInfo {
                    is_self: true,
                    is_mutual: false,
                    is_following: false,
                    follows_you: false,
                }
            } else {
                RelationshipInfo {
                    is_self: false,
                    is_mutual: self
                        .stores
                        .friends
                        .are_mutual_friends(&viewer, profile_user_id)
                        .unwrap_or(false),
                    is_following: self
                        .stores
                        .friends
                        .is_following(&viewer, profile_user_id)
                        .unwrap_or(false),
                    follows_you: self
                        .stores
                        .friends
                        .is_following(profile_user_id, &viewer)
                        .unwrap_or(false),
                }
            }
        } else {
            RelationshipInfo {
                is_self: false,
                is_mutual: false,
                is_following: false,
                follows_you: false,
            }
        };

        Ok(UserProfileData {
            id: user.id,
            username: user.username,
            bio: user.bio,
            join_date: user.join_date,
            follower_count,
            following_count,
            post_count,
            relationship,
        })
    }

    pub fn follow_user(&self, follower_id: &Uuid, following_id: &Uuid) -> ApiResult<()> {
        if follower_id == following_id {
            return Err(ApiError::BadRequest("Cannot follow yourself".to_string()));
        }

        self.stores
            .users
            .get_by_id(following_id)?
            .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

        self.stores.friends.follow_user(follower_id, following_id)?;
        Ok(())
    }

    pub fn unfollow_user(&self, follower_id: &Uuid, following_id: &Uuid) -> ApiResult<bool> {
        let rows_deleted = self.stores.friends.unfollow_user(follower_id, following_id)?;
        Ok(rows_deleted > 0)
    }

    pub fn get_following_list(&self, user_id: &Uuid) -> ApiResult<Vec<SocialUser>> {
        let following_ids = self.stores.friends.get_following(user_id)?;

        let mut users = Vec::new();
        for id in following_ids {
            if let Ok(Some(user)) = self.stores.users.get_by_id(&id) {
                let follower_count = self.stores.friends.get_follower_count(&id).unwrap_or(0);
                let following_count = self.stores.friends.get_following_count(&id).unwrap_or(0);

                users.push(SocialUser {
                    id: user.id,
                    username: user.username,
                    follower_count,
                    following_count,
                });
            }
        }

        Ok(users)
    }

    pub fn get_followers_list(&self, user_id: &Uuid) -> ApiResult<Vec<SocialUser>> {
        let follower_ids = self.stores.friends.get_followers(user_id)?;

        let mut users = Vec::new();
        for id in follower_ids {
            if let Ok(Some(user)) = self.stores.users.get_by_id(&id) {
                let follower_count = self.stores.friends.get_follower_count(&id).unwrap_or(0);
                let following_count = self.stores.friends.get_following_count(&id).unwrap_or(0);

                users.push(SocialUser {
                    id: user.id,
                    username: user.username,
                    follower_count,
                    following_count,
                });
            }
        }

        Ok(users)
    }

    pub fn get_mutual_friends_list(&self, user_id: &Uuid) -> ApiResult<Vec<SocialUser>> {
        let friend_ids = self.stores.friends.get_mutual_friends(user_id)?;

        let mut users = Vec::new();
        for id in friend_ids {
            if let Ok(Some(user)) = self.stores.users.get_by_id(&id) {
                let follower_count = self.stores.friends.get_follower_count(&id).unwrap_or(0);
                let following_count = self.stores.friends.get_following_count(&id).unwrap_or(0);

                users.push(SocialUser {
                    id: user.id,
                    username: user.username,
                    follower_count,
                    following_count,
                });
            }
        }

        Ok(users)
    }
}

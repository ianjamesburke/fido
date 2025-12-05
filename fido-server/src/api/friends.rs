use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::{ApiError, ApiResult},
    db::repositories::{FriendRepository, PostRepository, UserRepository},
    state::AppState,
};

/// Extract user ID from session token header
fn get_user_from_headers(state: &AppState, headers: &HeaderMap) -> Result<Uuid, ApiError> {
    let token = headers
        .get("X-Session-Token")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::Unauthorized("Missing session token".to_string()))?;

    state
        .get_authenticated_user_id_from_token(token)
        .ok_or_else(|| ApiError::Unauthorized("Invalid session token".to_string()))
}

/// Extract optional user ID from session token header (for public endpoints)
fn get_optional_user_from_headers(state: &AppState, headers: &HeaderMap) -> Option<Uuid> {
    let token = headers.get("X-Session-Token")?.to_str().ok()?;
    state.get_authenticated_user_id_from_token(token)
}

/// GET /users/search?q=query - Search users by username
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

#[derive(Debug, Serialize)]
pub struct UserSearchResponse {
    pub id: String,
    pub username: String,
}

pub async fn search_users(
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<SearchQuery>,
) -> ApiResult<Json<Vec<UserSearchResponse>>> {
    let user_repo = UserRepository::new(state.db.pool.clone());

    // Simple search: get all users and filter by username substring
    let all_users = user_repo
        .list_all()
        .map_err(|e| ApiError::InternalError(format!("Failed to search users: {}", e)))?;

    let search_lower = query.q.to_lowercase();
    let mut results: Vec<_> = all_users
        .into_iter()
        .filter(|u| u.username.to_lowercase().contains(&search_lower))
        .map(|u| UserSearchResponse {
            id: u.id.to_string(),
            username: u.username,
        })
        .collect();

    // Sort by username similarity (exact match first, then alphabetical)
    results.sort_by(|a, b| {
        let a_exact = a.username.to_lowercase() == search_lower;
        let b_exact = b.username.to_lowercase() == search_lower;

        match (a_exact, b_exact) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.username.cmp(&b.username),
        }
    });

    // Limit to 20 results
    results.truncate(20);

    Ok(Json(results))
}

/// GET /users/:id/profile - Get user profile with relationship status
#[derive(Debug, Serialize)]
pub struct UserProfileResponse {
    pub id: String,
    pub username: String,
    pub bio: Option<String>,
    pub join_date: String,
    pub follower_count: usize,
    pub following_count: usize,
    pub post_count: usize,
    pub relationship: RelationshipStatus,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum RelationshipStatus {
    #[serde(rename = "self")]
    Self_,
    #[serde(rename = "mutual_friends")]
    MutualFriends,
    #[serde(rename = "following")]
    Following,
    #[serde(rename = "follows_you")]
    FollowsYou,
    #[serde(rename = "none")]
    None,
}

pub async fn get_user_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id_str): Path<String>,
) -> ApiResult<Json<UserProfileResponse>> {
    let viewer_id = get_optional_user_from_headers(&state, &headers);

    let profile_user_id = Uuid::parse_str(&user_id_str)
        .map_err(|_| ApiError::BadRequest("Invalid user ID".to_string()))?;

    let user_repo = UserRepository::new(state.db.pool.clone());
    let user = user_repo
        .find_by_id(&profile_user_id)
        .map_err(|e| ApiError::InternalError(format!("Failed to find user: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    let friend_repo = FriendRepository::new(state.db.pool.clone());
    let post_repo = PostRepository::new(state.db.pool.clone());

    let follower_count = friend_repo
        .get_follower_count(&profile_user_id)
        .unwrap_or(0);
    let following_count = friend_repo
        .get_following_count(&profile_user_id)
        .unwrap_or(0);

    // Count posts by this user
    let post_count = post_repo
        .get_posts(fido_types::SortOrder::Newest, 1000)
        .map(|posts| {
            posts
                .iter()
                .filter(|p| p.author_id == profile_user_id)
                .count()
        })
        .unwrap_or(0);

    // Determine relationship status
    let relationship = if let Some(viewer) = viewer_id {
        if viewer == profile_user_id {
            RelationshipStatus::Self_
        } else if friend_repo
            .are_mutual_friends(&viewer, &profile_user_id)
            .unwrap_or(false)
        {
            RelationshipStatus::MutualFriends
        } else if friend_repo
            .is_following(&viewer, &profile_user_id)
            .unwrap_or(false)
        {
            RelationshipStatus::Following
        } else if friend_repo
            .is_following(&profile_user_id, &viewer)
            .unwrap_or(false)
        {
            RelationshipStatus::FollowsYou
        } else {
            RelationshipStatus::None
        }
    } else {
        RelationshipStatus::None
    };

    Ok(Json(UserProfileResponse {
        id: user.id.to_string(),
        username: user.username,
        bio: user.bio,
        join_date: user.join_date.to_rfc3339(),
        follower_count,
        following_count,
        post_count,
        relationship,
    }))
}

/// POST /users/:id/follow - Follow a user
pub async fn follow_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id_str): Path<String>,
) -> ApiResult<StatusCode> {
    let follower_id = get_user_from_headers(&state, &headers)?;

    let following_id = Uuid::parse_str(&user_id_str)
        .map_err(|_| ApiError::BadRequest("Invalid user ID".to_string()))?;

    if follower_id == following_id {
        return Err(ApiError::BadRequest("Cannot follow yourself".to_string()));
    }

    // Verify user exists
    let user_repo = UserRepository::new(state.db.pool.clone());
    user_repo
        .find_by_id(&following_id)
        .map_err(|e| ApiError::InternalError(format!("Failed to find user: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    let friend_repo = FriendRepository::new(state.db.pool.clone());
    friend_repo
        .follow_user(&follower_id, &following_id)
        .map_err(|e| ApiError::InternalError(format!("Failed to follow user: {}", e)))?;

    Ok(StatusCode::OK)
}

/// DELETE /users/:id/follow - Unfollow a user
pub async fn unfollow_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id_str): Path<String>,
) -> ApiResult<StatusCode> {
    let follower_id = get_user_from_headers(&state, &headers)?;

    let following_id = Uuid::parse_str(&user_id_str)
        .map_err(|_| ApiError::BadRequest("Invalid user ID".to_string()))?;

    let friend_repo = FriendRepository::new(state.db.pool.clone());
    let rows_deleted = friend_repo
        .unfollow_user(&follower_id, &following_id)
        .map_err(|e| ApiError::InternalError(format!("Failed to unfollow user: {}", e)))?;

    if rows_deleted == 0 {
        return Err(ApiError::NotFound("Not following this user".to_string()));
    }

    Ok(StatusCode::OK)
}

/// GET /social/following - Get list of users the current user is following
#[derive(Debug, Serialize)]
pub struct SocialUserResponse {
    pub id: String,
    pub username: String,
    pub follower_count: usize,
    pub following_count: usize,
}

pub async fn get_following_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<SocialUserResponse>>> {
    let user_id = get_user_from_headers(&state, &headers)?;
    
    let friend_repo = FriendRepository::new(state.db.pool.clone());
    let user_repo = UserRepository::new(state.db.pool.clone());
    
    let following_ids = friend_repo.get_following(&user_id)
        .map_err(|e| ApiError::InternalError(format!("Failed to get following: {}", e)))?;
    
    let mut users = Vec::new();
    for user_id in following_ids {
        if let Ok(Some(user)) = user_repo.find_by_id(&user_id) {
            let follower_count = friend_repo.get_follower_count(&user_id).unwrap_or(0);
            let following_count = friend_repo.get_following_count(&user_id).unwrap_or(0);
            
            users.push(SocialUserResponse {
                id: user.id.to_string(),
                username: user.username,
                follower_count,
                following_count,
            });
        }
    }
    
    Ok(Json(users))
}

/// GET /social/followers - Get list of users following the current user
pub async fn get_followers_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<SocialUserResponse>>> {
    let user_id = get_user_from_headers(&state, &headers)?;
    
    let friend_repo = FriendRepository::new(state.db.pool.clone());
    let user_repo = UserRepository::new(state.db.pool.clone());
    
    let follower_ids = friend_repo.get_followers(&user_id)
        .map_err(|e| ApiError::InternalError(format!("Failed to get followers: {}", e)))?;
    
    let mut users = Vec::new();
    for user_id in follower_ids {
        if let Ok(Some(user)) = user_repo.find_by_id(&user_id) {
            let follower_count = friend_repo.get_follower_count(&user_id).unwrap_or(0);
            let following_count = friend_repo.get_following_count(&user_id).unwrap_or(0);
            
            users.push(SocialUserResponse {
                id: user.id.to_string(),
                username: user.username,
                follower_count,
                following_count,
            });
        }
    }
    
    Ok(Json(users))
}

/// GET /social/mutual - Get list of mutual friends
pub async fn get_mutual_friends_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<SocialUserResponse>>> {
    let user_id = get_user_from_headers(&state, &headers)?;
    
    let friend_repo = FriendRepository::new(state.db.pool.clone());
    let user_repo = UserRepository::new(state.db.pool.clone());
    
    let friend_ids = friend_repo.get_mutual_friends(&user_id)
        .map_err(|e| ApiError::InternalError(format!("Failed to get mutual friends: {}", e)))?;
    
    let mut users = Vec::new();
    for user_id in friend_ids {
        if let Ok(Some(user)) = user_repo.find_by_id(&user_id) {
            let follower_count = friend_repo.get_follower_count(&user_id).unwrap_or(0);
            let following_count = friend_repo.get_following_count(&user_id).unwrap_or(0);
            
            users.push(SocialUserResponse {
                id: user.id.to_string(),
                username: user.username,
                follower_count,
                following_count,
            });
        }
    }
    
    Ok(Json(users))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::state::AppState;
    use axum::http::HeaderValue;

    fn setup_test_state() -> (AppState, Uuid, String) {
        let db = Database::in_memory().expect("Failed to create test database");
        db.seed_test_data().expect("Failed to seed test data");

        let state = AppState::new(db);

        // Create a test user and get session token
        let user_repo = UserRepository::new(state.db.pool.clone());
        let test_user = user_repo
            .find_by_username("alice")
            .expect("Failed to find user")
            .expect("Alice user not found");

        let session_token = state
            .session_manager
            .create_session(test_user.id)
            .expect("Failed to create session");

        (state, test_user.id, session_token)
    }

    fn create_headers(token: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("X-Session-Token", HeaderValue::from_str(token).unwrap());
        headers
    }

}

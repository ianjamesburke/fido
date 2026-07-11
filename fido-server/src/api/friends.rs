use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::{ApiError, ApiResult},
    http::{AuthenticatedUser, OptionalUser},
    services::friends::{FriendsService, RelationshipInfo},
    state::AppState,
};

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
    AuthenticatedUser(_user_id): AuthenticatedUser,
    axum::extract::Query(query): axum::extract::Query<SearchQuery>,
) -> ApiResult<Json<Vec<UserSearchResponse>>> {
    let service = FriendsService::new(state.repos);
    let results = service.search_users(&query.q, 20)?;

    Ok(Json(
        results
            .into_iter()
            .map(|u| UserSearchResponse {
                id: u.id.to_string(),
                username: u.username,
            })
            .collect(),
    ))
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
    OptionalUser(viewer_id): OptionalUser,
    Path(user_id_str): Path<String>,
) -> ApiResult<Json<UserProfileResponse>> {
    let profile_user_id = Uuid::parse_str(&user_id_str)
        .map_err(|_| ApiError::BadRequest("Invalid user ID".to_string()))?;

    let service = FriendsService::new(state.repos);
    let profile = service.get_user_profile(viewer_id, &profile_user_id)?;
    let relationship = map_relationship(&profile.relationship);

    Ok(Json(UserProfileResponse {
        id: profile.id.to_string(),
        username: profile.username,
        bio: profile.bio,
        join_date: profile.join_date.to_rfc3339(),
        follower_count: profile.follower_count,
        following_count: profile.following_count,
        post_count: profile.post_count,
        relationship,
    }))
}

/// POST /users/:id/follow - Follow a user
pub async fn follow_user(
    State(state): State<AppState>,
    AuthenticatedUser(follower_id): AuthenticatedUser,
    Path(user_id_str): Path<String>,
) -> ApiResult<StatusCode> {
    let following_id = Uuid::parse_str(&user_id_str)
        .map_err(|_| ApiError::BadRequest("Invalid user ID".to_string()))?;

    let service = FriendsService::new(state.repos);
    service.follow_user(&follower_id, &following_id)?;

    Ok(StatusCode::OK)
}

/// DELETE /users/:id/follow - Unfollow a user
pub async fn unfollow_user(
    State(state): State<AppState>,
    AuthenticatedUser(follower_id): AuthenticatedUser,
    Path(user_id_str): Path<String>,
) -> ApiResult<StatusCode> {
    let following_id = Uuid::parse_str(&user_id_str)
        .map_err(|_| ApiError::BadRequest("Invalid user ID".to_string()))?;

    let service = FriendsService::new(state.repos);
    let deleted = service.unfollow_user(&follower_id, &following_id)?;

    if !deleted {
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
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> ApiResult<Json<Vec<SocialUserResponse>>> {
    let service = FriendsService::new(state.repos);
    let users = service.get_following_list(&user_id)?;

    Ok(Json(
        users
            .into_iter()
            .map(|u| SocialUserResponse {
                id: u.id.to_string(),
                username: u.username,
                follower_count: u.follower_count,
                following_count: u.following_count,
            })
            .collect(),
    ))
}

/// GET /social/followers - Get list of users following the current user
pub async fn get_followers_list(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> ApiResult<Json<Vec<SocialUserResponse>>> {
    let service = FriendsService::new(state.repos);
    let users = service.get_followers_list(&user_id)?;

    Ok(Json(
        users
            .into_iter()
            .map(|u| SocialUserResponse {
                id: u.id.to_string(),
                username: u.username,
                follower_count: u.follower_count,
                following_count: u.following_count,
            })
            .collect(),
    ))
}

/// GET /social/mutual - Get list of mutual friends
pub async fn get_mutual_friends_list(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> ApiResult<Json<Vec<SocialUserResponse>>> {
    let service = FriendsService::new(state.repos);
    let users = service.get_mutual_friends_list(&user_id)?;

    Ok(Json(
        users
            .into_iter()
            .map(|u| SocialUserResponse {
                id: u.id.to_string(),
                username: u.username,
                follower_count: u.follower_count,
                following_count: u.following_count,
            })
            .collect(),
    ))
}

fn map_relationship(info: &RelationshipInfo) -> RelationshipStatus {
    if info.is_self {
        RelationshipStatus::Self_
    } else if info.is_mutual {
        RelationshipStatus::MutualFriends
    } else if info.is_following {
        RelationshipStatus::Following
    } else if info.follows_you {
        RelationshipStatus::FollowsYou
    } else {
        RelationshipStatus::None
    }
}

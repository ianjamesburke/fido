use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::{ApiError, ApiResult},
    db::repositories::HashtagRepository,
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

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

#[derive(Debug, Serialize)]
pub struct HashtagResponse {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_count: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct ActiveHashtagResponse {
    pub name: String,
    pub interaction_count: i64,
}

/// GET /hashtags/followed - Get user's followed hashtags
pub async fn get_followed_hashtags(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<HashtagResponse>>> {
    let user_id = get_user_from_headers(&state, &headers)?;

    let hashtag_repo = HashtagRepository::new(state.db.pool.clone());
    let hashtags = hashtag_repo
        .get_followed_by_user(&user_id)
        .map_err(|e| ApiError::InternalError(format!("Failed to get followed hashtags: {}", e)))?;

    let mut response = Vec::new();
    for name in hashtags {
        let post_count = hashtag_repo
            .get_post_count(&name)
            .map_err(|e| ApiError::InternalError(format!("Failed to get post count: {}", e)))?;
        response.push(HashtagResponse {
            name,
            post_count: Some(post_count),
        });
    }

    Ok(Json(response))
}

/// POST /hashtags/follow - Follow a hashtag
#[derive(Debug, Deserialize)]
pub struct FollowHashtagRequest {
    pub name: String,
}

pub async fn follow_hashtag(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<FollowHashtagRequest>,
) -> ApiResult<StatusCode> {
    let user_id = get_user_from_headers(&state, &headers)?;

    let hashtag_repo = HashtagRepository::new(state.db.pool.clone());
    hashtag_repo
        .follow_hashtag(&user_id, &req.name)
        .map_err(|e| ApiError::InternalError(format!("Failed to follow hashtag: {}", e)))?;

    Ok(StatusCode::OK)
}

/// DELETE /hashtags/follow/:name - Unfollow a hashtag
pub async fn unfollow_hashtag(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> ApiResult<StatusCode> {
    let user_id = get_user_from_headers(&state, &headers)?;

    let hashtag_repo = HashtagRepository::new(state.db.pool.clone());
    hashtag_repo
        .unfollow_hashtag(&user_id, &name)
        .map_err(|e| ApiError::InternalError(format!("Failed to unfollow hashtag: {}", e)))?;

    Ok(StatusCode::OK)
}

/// GET /hashtags/search?q=query - Search hashtags
pub async fn search_hashtags(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> ApiResult<Json<Vec<HashtagResponse>>> {
    let hashtag_repo = HashtagRepository::new(state.db.pool.clone());
    let hashtags = hashtag_repo
        .search(&query.q, 20)
        .map_err(|e| ApiError::InternalError(format!("Failed to search hashtags: {}", e)))?;

    let mut response = Vec::new();
    for name in hashtags {
        let post_count = hashtag_repo
            .get_post_count(&name)
            .map_err(|e| ApiError::InternalError(format!("Failed to get post count: {}", e)))?;
        response.push(HashtagResponse {
            name,
            post_count: Some(post_count),
        });
    }

    Ok(Json(response))
}

/// GET /hashtags/active - Get user's most active hashtags
pub async fn get_active_hashtags(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<ActiveHashtagResponse>>> {
    let user_id = get_user_from_headers(&state, &headers)?;

    let hashtag_repo = HashtagRepository::new(state.db.pool.clone());
    let hashtags = hashtag_repo
        .get_active_by_user(&user_id, 5)
        .map_err(|e| ApiError::InternalError(format!("Failed to get active hashtags: {}", e)))?;

    let response = hashtags
        .into_iter()
        .map(|(name, count)| ActiveHashtagResponse {
            name,
            interaction_count: count,
        })
        .collect();

    Ok(Json(response))
}

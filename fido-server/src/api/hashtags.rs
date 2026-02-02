use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use crate::{
    api::{ApiError, ApiResult},
    db::repositories::HashtagRepository,
    http::{extract_client_ip, extract_user_agent, AuthenticatedUser},
    security::validation::validate_hashtag,
    security::{AuditEvent, AuditEventType},
    state::AppState,
};

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
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> ApiResult<Json<Vec<HashtagResponse>>> {
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
    AuthenticatedUser(user_id): AuthenticatedUser,
    headers: HeaderMap,
    Json(req): Json<FollowHashtagRequest>,
) -> ApiResult<StatusCode> {
    let client_ip = extract_client_ip(&headers);
    let user_agent = extract_user_agent(&headers);

    // Validate hashtag name
    if let Err(e) = validate_hashtag(&req.name) {
        // Log validation failure
        let _ = state.audit_logger.log(
            AuditEvent::new(AuditEventType::ValidationFailure)
                .with_optional_ip_address(client_ip)
                .with_optional_user_agent(user_agent)
                .with_details(format!("Hashtag follow validation failed: {}", e)),
        );
        return Err(ApiError::BadRequest(e.to_string()));
    }

    let hashtag_repo = HashtagRepository::new(state.db.pool.clone());
    hashtag_repo
        .follow_hashtag(&user_id, &req.name)
        .map_err(|e| ApiError::InternalError(format!("Failed to follow hashtag: {}", e)))?;

    Ok(StatusCode::OK)
}

/// DELETE /hashtags/follow/:name - Unfollow a hashtag
pub async fn unfollow_hashtag(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> ApiResult<StatusCode> {
    let client_ip = extract_client_ip(&headers);
    let user_agent = extract_user_agent(&headers);

    // Validate hashtag name
    if let Err(e) = validate_hashtag(&name) {
        // Log validation failure
        let _ = state.audit_logger.log(
            AuditEvent::new(AuditEventType::ValidationFailure)
                .with_optional_ip_address(client_ip)
                .with_optional_user_agent(user_agent)
                .with_details(format!("Hashtag unfollow validation failed: {}", e)),
        );
        return Err(ApiError::BadRequest(e.to_string()));
    }

    let hashtag_repo = HashtagRepository::new(state.db.pool.clone());
    hashtag_repo
        .unfollow_hashtag(&user_id, &name)
        .map_err(|e| ApiError::InternalError(format!("Failed to unfollow hashtag: {}", e)))?;

    Ok(StatusCode::OK)
}

/// GET /hashtags/search?q=query - Search hashtags
pub async fn search_hashtags(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<SearchQuery>,
) -> ApiResult<Json<Vec<HashtagResponse>>> {
    let client_ip = extract_client_ip(&headers);
    let user_agent = extract_user_agent(&headers);

    // Validate search query as a hashtag (if not empty)
    if !query.q.is_empty() {
        if let Err(e) = validate_hashtag(&query.q) {
            // Log validation failure
            let _ = state.audit_logger.log(
                AuditEvent::new(AuditEventType::ValidationFailure)
                    .with_optional_ip_address(client_ip)
                    .with_optional_user_agent(user_agent)
                    .with_details(format!("Hashtag search validation failed: {}", e)),
            );
            return Err(ApiError::BadRequest(e.to_string()));
        }
    }

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
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> ApiResult<Json<Vec<ActiveHashtagResponse>>> {
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

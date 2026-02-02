use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use uuid::Uuid;

use crate::{
    api::{ApiError, ApiResult},
    db::repositories::{HashtagRepository, PostRepository, UserRepository, VoteRepository},
    security::validation::validate_bio,
    security::{AuditEvent, AuditEventType},
    state::AppState,
};
use fido_types::{UpdateBioRequest, UserProfile};

/// Helper function to extract client IP address from headers
fn extract_client_ip(headers: &HeaderMap) -> Option<String> {
    // Check X-Forwarded-For first (for proxied requests)
    headers
        .get("X-Forwarded-For")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
        .or_else(|| {
            // Fall back to X-Real-IP
            headers
                .get("X-Real-IP")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
}

/// Helper function to extract User-Agent from headers
fn extract_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get("User-Agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

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

/// GET /users/:id/profile - Get user profile with stats
pub async fn get_profile(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> ApiResult<Json<UserProfile>> {
    // Parse user ID
    let user_id = Uuid::parse_str(&user_id)
        .map_err(|_| ApiError::BadRequest("Invalid user ID".to_string()))?;

    let pool = state.db.pool.clone();
    let user_repo = UserRepository::new(pool.clone());
    let vote_repo = VoteRepository::new(pool.clone());
    let post_repo = PostRepository::new(pool.clone());
    let hashtag_repo = HashtagRepository::new(pool);

    // Get user
    let user = user_repo
        .get_by_id(&user_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    // Calculate karma (sum of upvotes on user's posts)
    let karma = vote_repo
        .calculate_karma(&user_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Get post count
    let post_count = post_repo
        .get_post_count(&user_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Get most active hashtags (top 5)
    let active_hashtags = hashtag_repo
        .get_active_by_user(&user_id, 5)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Extract just the hashtag names for the profile
    let recent_hashtags: Vec<String> = active_hashtags
        .into_iter()
        .map(|(name, _count)| name)
        .collect();

    let profile = UserProfile {
        user_id: user.id,
        username: user.username,
        bio: user.bio,
        karma,
        post_count,
        join_date: user.join_date,
        recent_hashtags,
    };

    Ok(Json(profile))
}

/// PUT /users/:id/profile - Update user bio
pub async fn update_profile(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<UpdateBioRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let client_ip = extract_client_ip(&headers);
    let user_agent = extract_user_agent(&headers);

    // Parse user ID from path
    let user_id = Uuid::parse_str(&user_id)
        .map_err(|_| ApiError::BadRequest("Invalid user ID".to_string()))?;

    // Validate bio content
    if let Err(e) = validate_bio(&payload.bio) {
        // Log validation failure
        let _ = state.audit_logger.log(
            AuditEvent::new(AuditEventType::ValidationFailure)
                .with_optional_ip_address(client_ip)
                .with_optional_user_agent(user_agent)
                .with_details(format!("Bio validation failed: {}", e)),
        );
        return Err(ApiError::BadRequest(e.to_string()));
    }

    // Get authenticated user from session token
    let authenticated_user_id = get_user_from_headers(&state, &headers)?;

    // Authorization check: users can only edit their own bio
    if user_id != authenticated_user_id {
        return Err(ApiError::Unauthorized(
            "You can only edit your own profile".to_string(),
        ));
    }

    let pool = state.db.pool.clone();
    let user_repo = UserRepository::new(pool);

    // Verify user exists
    user_repo
        .get_by_id(&user_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    // Update bio
    user_repo
        .update_bio(&user_id, &payload.bio)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "message": "Profile updated successfully",
        "user_id": user_id
    })))
}

/// GET /users/:id/hashtags - Get recent hashtags for user
pub async fn get_user_hashtags(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> ApiResult<Json<Vec<String>>> {
    // Parse user ID
    let user_id = Uuid::parse_str(&user_id)
        .map_err(|_| ApiError::BadRequest("Invalid user ID".to_string()))?;

    let pool = state.db.pool.clone();
    let hashtag_repo = HashtagRepository::new(pool);

    // Get most active hashtags (top 10)
    let active_hashtags = hashtag_repo
        .get_active_by_user(&user_id, 10)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Extract just the hashtag names
    let hashtags: Vec<String> = active_hashtags
        .into_iter()
        .map(|(name, _count)| name)
        .collect();

    Ok(Json(hashtags))
}

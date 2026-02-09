use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use uuid::Uuid;

use crate::{
    api::{ApiError, ApiResult},
    http::{extract_client_ip, extract_user_agent, AuthenticatedUser},
    security::validation::validate_bio,
    security::{AuditEvent, AuditEventType},
    services::profile::ProfileService,
    state::AppState,
};
use fido_types::{UpdateBioRequest, UserProfile};

/// GET /users/:id/profile - Get user profile with stats
pub async fn get_profile(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> ApiResult<Json<UserProfile>> {
    // Parse user ID
    let user_id = Uuid::parse_str(&user_id)
        .map_err(|_| ApiError::BadRequest("Invalid user ID".to_string()))?;
    let service = ProfileService::new(state.stores.clone());
    let profile = service.get_profile(&user_id)?;

    Ok(Json(profile))
}

/// PUT /users/:id/profile - Update user bio
pub async fn update_profile(
    State(state): State<AppState>,
    AuthenticatedUser(authenticated_user_id): AuthenticatedUser,
    Path(user_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<UpdateBioRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let service = ProfileService::new(state.stores.clone());
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

    // Authorization check: users can only edit their own bio
    if user_id != authenticated_user_id {
        return Err(ApiError::Unauthorized(
            "You can only edit your own profile".to_string(),
        ));
    }

    service.update_bio(&user_id, &payload.bio)?;

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

    let service = ProfileService::new(state.stores.clone());
    let hashtags = service.get_user_hashtags(&user_id, 10)?;

    Ok(Json(hashtags))
}

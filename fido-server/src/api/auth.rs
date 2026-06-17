use axum::{
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
use fido_types::{LoginRequest, LoginResponse, User};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::{ApiError, ApiResult};
use crate::http::{extract_client_ip, extract_user_agent};
use crate::oauth::GitHubOAuthConfig;
use crate::security::validation::validate_username;
use crate::security::{create_session_cookie, is_https, AuditEvent, AuditEventType};
use crate::state::AppState;

// In-memory store for active device codes: device_code -> unix timestamp of creation
lazy_static::lazy_static! {
    static ref DEVICE_CODES: Arc<Mutex<HashMap<String, i64>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

/// Response for GitHub Device Flow initiation
#[derive(Serialize)]
pub struct GitHubDeviceFlowResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: i64,
    pub interval: i64,
}

/// Request to poll for device authorization
#[derive(Deserialize)]
pub struct DevicePollRequest {
    pub device_code: String,
}

/// Response for session validation
#[derive(Serialize)]
pub struct ValidateSessionResponse {
    pub user: User,
    pub valid: bool,
}

/// GET /users/test - List all test users
pub async fn list_test_users(State(state): State<AppState>) -> ApiResult<Json<Vec<User>>> {
    let users = state
        .stores
        .users
        .get_test_users()
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(Json(users))
}

/// POST /auth/login - Login with test user
pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> ApiResult<Response> {
    let client_ip = extract_client_ip(&headers);
    let user_agent = extract_user_agent(&headers);
    let https = is_https(&headers);

    // Validate username format
    if let Err(e) = validate_username(&payload.username) {
        // Log login failure due to invalid username
        let _ = state.audit_logger.log(
            AuditEvent::new(AuditEventType::LoginFailure)
                .with_optional_ip_address(client_ip)
                .with_optional_user_agent(user_agent)
                .with_details(format!("Invalid username format: {}", payload.username)),
        );
        return Err(ApiError::BadRequest(e.to_string()));
    }

    // Find user by username
    let user = match state.stores.users.get_by_username(&payload.username) {
        Ok(Some(u)) => u,
        Ok(None) => {
            // Log login failure - user not found
            let _ = state.audit_logger.log(
                AuditEvent::new(AuditEventType::LoginFailure)
                    .with_optional_ip_address(client_ip)
                    .with_optional_user_agent(user_agent)
                    .with_details(format!("User not found: {}", payload.username)),
            );
            return Err(ApiError::NotFound(format!(
                "User '{}' not found",
                payload.username
            )));
        }
        Err(e) => {
            // Log login failure - database error
            let _ = state.audit_logger.log(
                AuditEvent::new(AuditEventType::LoginFailure)
                    .with_optional_ip_address(client_ip.clone())
                    .with_optional_user_agent(user_agent)
                    .with_details(format!(
                        "Database error during login for: {}",
                        payload.username
                    )),
            );
            return Err(ApiError::InternalError(e.to_string()));
        }
    };

    // Verify it's a test user
    if !user.is_test_user {
        // Log login failure - not a test user
        let _ = state.audit_logger.log(
            AuditEvent::new(AuditEventType::LoginFailure)
                .with_user_id(user.id)
                .with_optional_ip_address(client_ip)
                .with_optional_user_agent(user_agent)
                .with_details("Attempted login via test endpoint with non-test user"),
        );
        return Err(ApiError::BadRequest(
            "Only test users can login via this endpoint".to_string(),
        ));
    }

    // Create session
    let session_token = state
        .session_manager
        .create_session(user.id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Log successful login
    let _ = state.audit_logger.log(
        AuditEvent::new(AuditEventType::LoginSuccess)
            .with_user_id(user.id)
            .with_optional_ip_address(client_ip)
            .with_optional_user_agent(user_agent)
            .with_details(format!("Test user login: {}", user.username)),
    );

    // Create response with session cookie
    let response_body = LoginResponse {
        user,
        session_token: session_token.clone(),
    };

    let mut response = Json(response_body).into_response();

    // Set session cookie with Secure flag if HTTPS
    let cookie = create_session_cookie(&session_token, https);
    response.headers_mut().insert("Set-Cookie", cookie);

    Ok(response)
}

/// POST /auth/logout - Logout current user
pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(session_token): Json<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let client_ip = extract_client_ip(&headers);
    let user_agent = extract_user_agent(&headers);

    // Try to get user_id before deleting session for audit logging
    let user_id = state.session_manager.validate_session(&session_token).ok();

    // Delete session
    state
        .session_manager
        .delete_session(&session_token)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Log session revocation
    let _ = state.audit_logger.log(
        AuditEvent::new(AuditEventType::SessionRevoked)
            .with_optional_user_id(user_id)
            .with_optional_ip_address(client_ip)
            .with_optional_user_agent(user_agent)
            .with_details("User logged out"),
    );

    Ok(Json(serde_json::json!({
        "message": "Logged out successfully"
    })))
}

/// POST /auth/cleanup-sessions - Manually trigger session cleanup (admin endpoint)
///
/// This endpoint removes all expired sessions from the database.
/// Useful for manual cleanup or testing purposes.
pub async fn cleanup_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    let client_ip = extract_client_ip(&headers);
    let user_agent = extract_user_agent(&headers);

    // Get admin user ID from session token for audit logging
    let admin_user_id = headers
        .get("X-Session-Token")
        .and_then(|v| v.to_str().ok())
        .and_then(|token| state.session_manager.validate_session(token).ok());

    let count = state
        .session_manager
        .cleanup_expired_sessions()
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Log admin action
    let _ = state.audit_logger.log(
        AuditEvent::new(AuditEventType::AdminAction)
            .with_optional_user_id(admin_user_id)
            .with_optional_ip_address(client_ip)
            .with_optional_user_agent(user_agent)
            .with_details(format!(
                "Session cleanup: removed {} expired sessions",
                count
            )),
    );

    Ok(Json(serde_json::json!({
        "message": "Session cleanup completed",
        "sessions_removed": count
    })))
}

/// POST /auth/github/device - Initiate GitHub Device Flow
///
/// Requests a device code from GitHub and returns the user code and verification URI.
/// The client should display the user code and direct the user to the verification URI.
pub async fn github_device_flow(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<GitHubDeviceFlowResponse>> {
    let client_ip = extract_client_ip(&headers);
    let user_agent = extract_user_agent(&headers);

    // Load GitHub OAuth configuration
    let oauth_config = GitHubOAuthConfig::from_env()
        .map_err(|e| ApiError::BadRequest(format!("OAuth configuration error: {}", e)))?;

    // Request device code from GitHub
    let device_response = oauth_config
        .request_device_code()
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to request device code: {}", e)))?;

    // Store device code with current timestamp
    let now = chrono::Utc::now().timestamp();
    {
        let mut codes = DEVICE_CODES.lock().unwrap_or_else(|e| e.into_inner());
        codes.insert(device_response.device_code.clone(), now);

        // Clean up expired codes (older than 15 minutes)
        let expired_threshold = now - 900;
        codes.retain(|_, timestamp| *timestamp > expired_threshold);
    }

    // Log device code generation
    let _ = state.audit_logger.log(
        AuditEvent::new(AuditEventType::DeviceCodeGenerated)
            .with_optional_ip_address(client_ip)
            .with_optional_user_agent(user_agent)
            .with_details(format!("User code: {}", device_response.user_code)),
    );

    tracing::info!(
        "Generated GitHub device code: {}",
        device_response.user_code
    );

    Ok(Json(GitHubDeviceFlowResponse {
        device_code: device_response.device_code,
        user_code: device_response.user_code,
        verification_uri: device_response.verification_uri,
        expires_in: device_response.expires_in,
        interval: device_response.interval,
    }))
}

/// POST /auth/github/device/poll - Poll for Device Flow completion
///
/// Polls GitHub to check if the user has authorized the device.
/// Returns the session token if authorized, or an error if still pending/failed.
pub async fn github_device_poll(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<DevicePollRequest>,
) -> ApiResult<Response> {
    let client_ip = extract_client_ip(&headers);
    let user_agent = extract_user_agent(&headers);
    let https = is_https(&headers);

    // Check if device code exists and is not expired
    let is_valid = {
        let codes = DEVICE_CODES.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(timestamp) = codes.get(&payload.device_code) {
            let now = chrono::Utc::now().timestamp();
            now - timestamp < 900 // 15 minutes
        } else {
            false
        }
    };

    if !is_valid {
        // Log login failure - invalid/expired device code
        let _ = state.audit_logger.log(
            AuditEvent::new(AuditEventType::LoginFailure)
                .with_optional_ip_address(client_ip)
                .with_optional_user_agent(user_agent)
                .with_details("Invalid or expired device code"),
        );
        return Err(ApiError::BadRequest(
            "Invalid or expired device code".to_string(),
        ));
    }

    // Load GitHub OAuth configuration
    let oauth_config = GitHubOAuthConfig::from_env()
        .map_err(|e| ApiError::InternalError(format!("OAuth configuration error: {}", e)))?;

    // Poll GitHub for access token
    let access_token = match oauth_config.poll_device_token(&payload.device_code).await {
        Ok(Some(token)) => token,
        Ok(None) => {
            // Still pending - return a specific error that the client can handle
            return Err(ApiError::BadRequest("authorization_pending".to_string()));
        }
        Err(e) => {
            // Remove device code on error
            DEVICE_CODES.lock().unwrap_or_else(|e| e.into_inner()).remove(&payload.device_code);
            // Log login failure
            let _ = state.audit_logger.log(
                AuditEvent::new(AuditEventType::LoginFailure)
                    .with_optional_ip_address(client_ip)
                    .with_optional_user_agent(user_agent)
                    .with_details(format!("Device authorization failed: {}", e)),
            );
            return Err(ApiError::InternalError(format!(
                "Device authorization failed: {}",
                e
            )));
        }
    };

    // Fetch GitHub user profile
    let github_user = oauth_config
        .get_user(access_token)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to fetch GitHub user: {}", e)))?;

    tracing::info!(
        "GitHub user authenticated via device flow: {} (ID: {})",
        github_user.login,
        github_user.id
    );

    // Create or update user in database
    let user = state
        .stores
        .users
        .create_or_update_from_github(
            github_user.id,
            &github_user.login,
            github_user.name.as_deref(),
        )
        .map_err(|e| ApiError::InternalError(format!("Failed to create/update user: {}", e)))?;

    // Create session
    let session_token = state
        .session_manager
        .create_session(user.id)
        .map_err(|e| ApiError::InternalError(format!("Failed to create session: {}", e)))?;

    // Remove device code after successful authentication
    DEVICE_CODES.lock().unwrap_or_else(|e| e.into_inner()).remove(&payload.device_code);

    // Log successful GitHub login
    let _ = state.audit_logger.log(
        AuditEvent::new(AuditEventType::LoginSuccess)
            .with_user_id(user.id)
            .with_optional_ip_address(client_ip.clone())
            .with_optional_user_agent(user_agent.clone())
            .with_details(format!("GitHub OAuth login: {}", user.username)),
    );

    // Log device code used
    let _ = state.audit_logger.log(
        AuditEvent::new(AuditEventType::DeviceCodeUsed)
            .with_user_id(user.id)
            .with_optional_ip_address(client_ip)
            .with_optional_user_agent(user_agent),
    );

    tracing::info!("Created session for user {} ({})", user.username, user.id);

    // Create response with session cookie
    let response_body = LoginResponse {
        user,
        session_token: session_token.clone(),
    };

    let mut response = Json(response_body).into_response();

    // Set session cookie with Secure flag if HTTPS
    let cookie = create_session_cookie(&session_token, https);
    response.headers_mut().insert("Set-Cookie", cookie);

    Ok(response)
}

/// GET /auth/validate - Validate session token
///
/// Validates the session token from the X-Session-Token header and returns
/// the associated user information if valid.
pub async fn validate_session(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> ApiResult<Json<ValidateSessionResponse>> {
    // Extract session token from header
    let token = headers
        .get("X-Session-Token")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::Unauthorized("Missing session token".to_string()))?;

    // Validate session token
    let user_id = state
        .session_manager
        .validate_session(token)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired session".to_string()))?;

    // Get user information
    let user = state
        .stores
        .users
        .get_by_id(&user_id)
        .map_err(|e| ApiError::InternalError(format!("Failed to get user: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    Ok(Json(ValidateSessionResponse { user, valid: true }))
}

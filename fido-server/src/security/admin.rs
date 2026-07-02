//! Admin authorization middleware for Fido server
//!
//! This module provides middleware to protect admin-only endpoints by verifying
//! that the authenticated user has admin privileges.

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::security::{AuditEvent, AuditEventType};
use crate::state::AppState;

/// Helper function to extract client IP address from headers
fn extract_client_ip(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get("X-Forwarded-For")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
        .or_else(|| {
            headers
                .get("X-Real-IP")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
}

/// Helper function to extract User-Agent from headers
fn extract_user_agent(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get("User-Agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// Middleware that requires the authenticated user to be an admin.
///
/// This middleware:
/// 1. Extracts the X-Session-Token header
/// 2. Validates the session using the session manager
/// 3. Retrieves the user from the database
/// 4. Checks if the user has admin privileges (is_admin = true)
///
/// Returns 401 Unauthorized if:
/// - No session token is provided
/// - The session token is invalid or expired
/// - The user cannot be found
///
/// Returns 403 Forbidden if:
/// - The user is not an admin
///
/// # Example
///
/// ```rust,ignore
/// use axum::{Router, routing::post, middleware};
/// use fido_server::security::admin::require_admin;
///
/// let app = Router::new()
///     .route("/admin/cleanup", post(cleanup_handler))
///     .route_layer(middleware::from_fn_with_state(state.clone(), require_admin));
/// ```
pub async fn require_admin(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    let client_ip = extract_client_ip(request.headers());
    let user_agent = extract_user_agent(request.headers());
    let request_path = request.uri().path().to_string();

    // Extract session token from X-Session-Token header
    let session_token = request
        .headers()
        .get("X-Session-Token")
        .and_then(|v| v.to_str().ok());

    let token = match session_token {
        Some(t) if !t.is_empty() => t,
        _ => {
            tracing::warn!("Admin endpoint accessed without session token");
            // Log unauthorized access attempt
            let _ = state.audit_logger.log(
                AuditEvent::new(AuditEventType::AdminAction)
                    .with_optional_ip_address(client_ip)
                    .with_optional_user_agent(user_agent)
                    .with_details(format!(
                        "Unauthorized admin access attempt (no token): {}",
                        request_path
                    )),
            );
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "Unauthorized",
                    "details": "Authentication required"
                })),
            )
                .into_response());
        }
    };

    // Validate the session and get user ID
    let user_id = match state.session_manager.validate_session(token) {
        Ok(id) => id,
        Err(e) => {
            tracing::warn!("Admin endpoint accessed with invalid session: {}", e);
            // Log unauthorized access attempt
            let _ = state.audit_logger.log(
                AuditEvent::new(AuditEventType::AdminAction)
                    .with_optional_ip_address(client_ip)
                    .with_optional_user_agent(user_agent)
                    .with_details(format!(
                        "Unauthorized admin access attempt (invalid session): {}",
                        request_path
                    )),
            );
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "Unauthorized",
                    "details": "Invalid or expired session"
                })),
            )
                .into_response());
        }
    };

    // Get the user from the database
    let user = match state.repos.users.get_by_id(&user_id) {
        Ok(Some(u)) => u,
        Ok(None) => {
            tracing::error!("Admin check failed: user {} not found in database", user_id);
            // Log unauthorized access attempt
            let _ = state.audit_logger.log(
                AuditEvent::new(AuditEventType::AdminAction)
                    .with_user_id(user_id)
                    .with_optional_ip_address(client_ip)
                    .with_optional_user_agent(user_agent)
                    .with_details(format!(
                        "Admin access attempt (user not found): {}",
                        request_path
                    )),
            );
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "Unauthorized",
                    "details": "User not found"
                })),
            )
                .into_response());
        }
        Err(e) => {
            tracing::error!("Admin check failed: database error: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal Server Error",
                    "details": "Failed to verify admin status"
                })),
            )
                .into_response());
        }
    };

    // Check if user is an admin
    if !user.is_admin {
        tracing::warn!(
            "Non-admin user {} ({}) attempted to access admin endpoint",
            user.username,
            user_id
        );
        // Log forbidden access attempt
        let _ = state.audit_logger.log(
            AuditEvent::new(AuditEventType::AdminAction)
                .with_user_id(user_id)
                .with_optional_ip_address(client_ip)
                .with_optional_user_agent(user_agent)
                .with_details(format!(
                    "Forbidden admin access attempt by non-admin user {}: {}",
                    user.username, request_path
                )),
        );
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "Forbidden",
                "details": "Admin privileges required"
            })),
        )
            .into_response());
    }

    tracing::debug!(
        "Admin access granted for user {} ({})",
        user.username,
        user_id
    );

    // User is admin, proceed with the request
    Ok(next.run(request).await)
}

#[cfg(all(test, feature = "sqlite-tests"))]
mod tests {
    use crate::db::repositories::UserRepository;
    use crate::db::Database;
    use crate::state::AppState;
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use chrono::Utc;
    use uuid::Uuid;

    fn setup_test_db() -> Database {
        let db = Database::in_memory().expect("Failed to create test database");
        db.initialize().expect("Failed to initialize database");
        db
    }

    fn setup_test_state(db: Database) -> AppState {
        let _guard = crate::test_utils::env_lock().lock().unwrap();
        std::env::set_var("FIDO_TOKEN_KEY", STANDARD.encode([11u8; 32]));
        AppState::new(db).expect("Failed to create app state")
    }

    fn create_test_user(db: &Database, username: &str, is_admin: bool) -> Uuid {
        let user_id = Uuid::new_v4();
        let conn = db.connection().expect("Failed to get connection");
        conn.execute(
            "INSERT INTO users (id, username, bio, join_date, is_test_user, is_admin) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                user_id.to_string(),
                username,
                "Test user",
                Utc::now().to_rfc3339(),
                1,
                if is_admin { 1 } else { 0 },
            ],
        )
        .expect("Failed to create test user");
        user_id
    }

    #[test]
    fn test_admin_user_has_is_admin_flag() {
        let db = setup_test_db();

        // Create an admin user
        let admin_id = create_test_user(&db, "admin_user", true);

        // Verify the user has is_admin = true
        let user_repo = UserRepository::new(db.pool.clone());
        let user = user_repo
            .get_by_id(&admin_id)
            .expect("Failed to get user")
            .expect("User not found");

        assert!(user.is_admin, "Admin user should have is_admin = true");
    }

    #[test]
    fn test_regular_user_has_no_admin_flag() {
        let db = setup_test_db();

        // Create a regular user
        let user_id = create_test_user(&db, "regular_user", false);

        // Verify the user has is_admin = false
        let user_repo = UserRepository::new(db.pool.clone());
        let user = user_repo
            .get_by_id(&user_id)
            .expect("Failed to get user")
            .expect("User not found");

        assert!(!user.is_admin, "Regular user should have is_admin = false");
    }

    #[test]
    fn test_session_manager_creates_valid_session() {
        let db = setup_test_db();
        let state = setup_test_state(db.clone());

        // Create a user
        let user_id = create_test_user(&db, "test_user", false);

        // Create a session
        let token = state
            .session_manager
            .create_session(user_id)
            .expect("Failed to create session");

        // Validate the session
        let validated_user_id = state
            .session_manager
            .validate_session(&token)
            .expect("Failed to validate session");

        assert_eq!(
            user_id, validated_user_id,
            "Session should return the correct user ID"
        );
    }

    #[test]
    fn test_invalid_session_fails_validation() {
        let db = setup_test_db();
        let state = setup_test_state(db);

        // Try to validate an invalid token
        let result = state.session_manager.validate_session("invalid-token");

        assert!(result.is_err(), "Invalid session should fail validation");
    }

    #[test]
    fn test_admin_check_logic() {
        let db = setup_test_db();
        let state = setup_test_state(db.clone());

        // Create an admin user
        let admin_id = create_test_user(&db, "admin_user", true);

        // Create a session for the admin
        let token = state
            .session_manager
            .create_session(admin_id)
            .expect("Failed to create session");

        // Validate session and get user
        let user_id = state
            .session_manager
            .validate_session(&token)
            .expect("Failed to validate session");
        let user_repo = UserRepository::new(state.db.pool.clone());
        let user = user_repo
            .get_by_id(&user_id)
            .expect("Failed to get user")
            .expect("User not found");

        // Check admin status
        assert!(user.is_admin, "User should be an admin");
    }

    #[test]
    fn test_non_admin_check_logic() {
        let db = setup_test_db();
        let state = setup_test_state(db.clone());

        // Create a regular user
        let user_id = create_test_user(&db, "regular_user", false);

        // Create a session for the user
        let token = state
            .session_manager
            .create_session(user_id)
            .expect("Failed to create session");

        // Validate session and get user
        let validated_user_id = state
            .session_manager
            .validate_session(&token)
            .expect("Failed to validate session");
        let user_repo = UserRepository::new(state.db.pool.clone());
        let user = user_repo
            .get_by_id(&validated_user_id)
            .expect("Failed to get user")
            .expect("User not found");

        // Check admin status
        assert!(!user.is_admin, "User should not be an admin");
    }
}

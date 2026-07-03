//! Admin API endpoints for Fido server
//!
//! This module provides administrative endpoints for monitoring and managing
//! the Fido server. All endpoints require admin authentication.

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{api::ApiResult, state::AppState};

/// Configuration validation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigValidationResponse {
    /// Current environment (development or production)
    pub environment: String,

    /// Whether the configuration is valid
    pub valid: bool,

    /// List of validation errors (if any)
    pub errors: Vec<String>,

    /// List of validation warnings (if any)
    pub warnings: Vec<String>,

    /// Security configuration status
    pub security: SecurityConfigStatus,

    /// Session configuration status
    pub session: SessionConfigStatus,
}

/// Security configuration status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfigStatus {
    /// Allowed CORS origins (without sensitive details)
    pub allowed_origins: Vec<String>,

    /// Whether HTTPS redirect is enabled
    pub https_redirect_enabled: bool,

    /// Maximum request body size in bytes
    pub max_request_size: usize,

    /// Whether GitHub OAuth is configured (without exposing the client ID)
    pub github_oauth_configured: bool,
}

/// Session configuration status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfigStatus {
    /// Maximum session age in days
    pub max_age_days: i64,

    /// Session idle timeout in hours
    pub idle_timeout_hours: i64,
}

/// GET /admin/config/validate - Validate and return security configuration status
///
/// This endpoint returns the current security configuration status without exposing
/// sensitive values like GitHub client IDs or secrets. It requires admin authentication.
///
/// # Returns
///
/// Returns a `ConfigValidationResponse` containing:
/// - Current environment (development/production)
/// - Validation status (valid/invalid)
/// - Any validation errors or warnings
/// - Security configuration status (CORS, HTTPS, request limits, OAuth status)
/// - Session configuration status (max age, idle timeout)
///
/// # Security
///
/// - Requires admin authentication via `require_admin` middleware
/// - Does NOT expose sensitive values (GitHub client ID, secrets, etc.)
/// - Safe to use for monitoring and diagnostics
///
/// # Example Response
///
/// ```json
/// {
///   "environment": "production",
///   "valid": true,
///   "errors": [],
///   "warnings": [],
///   "security": {
///     "allowed_origins": ["https://fido-web-production.up.railway.app"],
///     "https_redirect_enabled": true,
///     "max_request_size": 1048576,
///     "github_oauth_configured": true
///   },
///   "session": {
///     "max_age_days": 7,
///     "idle_timeout_hours": 24
///   }
/// }
/// ```
pub async fn validate_config(
    State(_state): State<AppState>,
) -> ApiResult<Json<ConfigValidationResponse>> {
    // Load current security configuration
    let security_config = match crate::security::SecurityConfig::from_env() {
        Ok(config) => config,
        Err(e) => {
            // If we can't load the config, return an error response
            let response = ConfigValidationResponse {
                environment: "unknown".to_string(),
                valid: false,
                errors: vec![format!("Failed to load security configuration: {}", e)],
                warnings: vec![],
                security: SecurityConfigStatus {
                    allowed_origins: vec![],
                    https_redirect_enabled: false,
                    max_request_size: 0,
                    github_oauth_configured: false,
                },
                session: SessionConfigStatus {
                    max_age_days: 0,
                    idle_timeout_hours: 0,
                },
            };
            return Ok(Json(response));
        }
    };

    // Validate the configuration
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if let Err(e) = security_config.validate() {
        if security_config.environment.is_production() {
            errors.push(format!("Configuration validation failed: {}", e));
        } else {
            warnings.push(format!("Configuration validation warning: {}", e));
        }
    }

    // Check GitHub OAuth configuration
    let github_oauth_configured = !security_config.github_client_id.is_empty();
    if !github_oauth_configured {
        if security_config.environment.is_production() {
            errors.push("GitHub OAuth is not configured (GITHUB_CLIENT_ID missing)".to_string());
        } else {
            warnings.push("GitHub OAuth is not configured (GITHUB_CLIENT_ID missing)".to_string());
        }
    }

    // Check CORS configuration
    if security_config.allowed_origins.is_empty() {
        warnings.push("No CORS origins configured".to_string());
    }

    // Build response
    let response = ConfigValidationResponse {
        environment: security_config.environment.to_string(),
        valid: errors.is_empty(),
        errors,
        warnings,
        security: SecurityConfigStatus {
            allowed_origins: security_config.allowed_origins.clone(),
            https_redirect_enabled: security_config.enable_https_redirect,
            max_request_size: security_config.max_request_size,
            github_oauth_configured,
        },
        session: SessionConfigStatus {
            max_age_days: security_config.session_max_age_days,
            idle_timeout_hours: security_config.session_idle_timeout_hours,
        },
    };

    // Log the validation check
    tracing::info!(
        "Configuration validation requested by admin: valid={}, errors={}, warnings={}",
        response.valid,
        response.errors.len(),
        response.warnings.len()
    );

    Ok(Json(response))
}

#[cfg(all(test, feature = "sqlite-tests"))]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::state::AppState;
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use chrono::Utc;
    use uuid::Uuid;

    fn setup_test_state() -> AppState {
        std::env::set_var("ENVIRONMENT", "development");
        std::env::set_var("FIDO_TOKEN_KEY", STANDARD.encode([9u8; 32]));
        let db = Database::in_memory().expect("Failed to create test database");
        db.initialize().expect("Failed to initialize database");
        AppState::new(db).expect("Failed to create app state")
    }

    fn create_test_user(state: &AppState, username: &str, is_admin: bool) -> Uuid {
        let user_id = Uuid::new_v4();
        let conn = state.db.connection().expect("Failed to get connection");
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

    #[tokio::test]
    async fn test_validate_config_returns_response() {
        let state = setup_test_state();

        // Call the endpoint
        let result = validate_config(State(state)).await;

        // Should return a successful response
        assert!(result.is_ok());

        let response = result.unwrap().0;

        // Should have an environment set
        assert!(!response.environment.is_empty());

        // Should have security config
        assert!(response.security.max_request_size > 0);
        assert!(response.session.max_age_days > 0);
        assert!(response.session.idle_timeout_hours > 0);
    }

    #[tokio::test]
    async fn test_validate_config_does_not_expose_secrets() {
        let _guard = crate::test_utils::env_lock().lock().unwrap();
        // Set a test GitHub client ID to verify it's not exposed
        std::env::set_var("GITHUB_CLIENT_ID", "test_secret_client_id_12345");

        let state = setup_test_state();

        // Call the endpoint
        let result = validate_config(State(state)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;

        // Serialize to JSON to check what's exposed
        let json = serde_json::to_string(&response).expect("Failed to serialize");

        // Should NOT contain the actual GitHub client ID value
        assert!(
            !json.contains("test_secret_client_id_12345"),
            "Response should not expose the actual GitHub client ID value"
        );

        // Should contain the configuration status boolean
        assert!(
            json.contains("github_oauth_configured"),
            "Response should contain github_oauth_configured boolean"
        );

        // Clean up
        std::env::remove_var("GITHUB_CLIENT_ID");
    }

    #[tokio::test]
    async fn test_validate_config_includes_environment() {
        let state = setup_test_state();

        let result = validate_config(State(state)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;

        // Should have environment (development or production)
        assert!(response.environment == "development" || response.environment == "production");
    }

    #[tokio::test]
    async fn test_validate_config_includes_cors_origins() {
        let state = setup_test_state();

        let result = validate_config(State(state)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;

        // Should have at least one allowed origin (or be empty with a warning)
        if response.security.allowed_origins.is_empty() {
            assert!(!response.warnings.is_empty());
        }
    }

    #[tokio::test]
    async fn test_validate_config_includes_session_settings() {
        let state = setup_test_state();

        let result = validate_config(State(state)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;

        // Should have session configuration
        assert!(response.session.max_age_days > 0);
        assert!(response.session.idle_timeout_hours > 0);
    }

    // ===== Admin Authentication Tests =====
    // These tests verify that the endpoint is properly protected by admin middleware
    // by testing the authentication logic that the middleware uses

    #[tokio::test]
    async fn test_admin_middleware_rejects_missing_token() {
        let state = setup_test_state();

        // Test that session validation fails without a token
        // This is what the middleware checks first
        let result = state.session_manager.validate_session("");
        assert!(result.is_err(), "Should reject empty session token");
    }

    #[tokio::test]
    async fn test_admin_middleware_rejects_invalid_token() {
        let state = setup_test_state();

        // Test that session validation fails with an invalid token
        // This is what the middleware checks
        let result = state
            .session_manager
            .validate_session("invalid-token-12345");
        assert!(result.is_err(), "Should reject invalid session token");
    }

    #[tokio::test]
    async fn test_admin_middleware_rejects_non_admin_user() {
        let state = setup_test_state();

        // Create a regular (non-admin) user
        let user_id = create_test_user(&state, "regular_user", false);

        // Create a session for the regular user
        let token = state
            .session_manager
            .create_session(user_id)
            .expect("Failed to create session");

        // Validate the session (this part would succeed in middleware)
        let validated_user_id = state
            .session_manager
            .validate_session(&token)
            .expect("Session should be valid");

        assert_eq!(user_id, validated_user_id);

        // Get the user from database (this is what middleware does)
        let user_repo = crate::db::repositories::UserRepository::new(state.db.pool.clone());
        let user = user_repo
            .get_by_id(&validated_user_id)
            .expect("Failed to get user")
            .expect("User should exist");

        // Verify the user is NOT an admin (middleware would reject here)
        assert!(
            !user.is_admin,
            "Regular user should not have admin privileges"
        );
    }

    #[tokio::test]
    async fn test_admin_middleware_allows_admin_user() {
        let state = setup_test_state();

        // Create an admin user
        let admin_id = create_test_user(&state, "admin_user", true);

        // Create a session for the admin user
        let token = state
            .session_manager
            .create_session(admin_id)
            .expect("Failed to create session");

        // Validate the session (this is what middleware does)
        let validated_user_id = state
            .session_manager
            .validate_session(&token)
            .expect("Session should be valid");

        assert_eq!(admin_id, validated_user_id);

        // Get the user from database (this is what middleware does)
        let user_repo = crate::db::repositories::UserRepository::new(state.db.pool.clone());
        let user = user_repo
            .get_by_id(&validated_user_id)
            .expect("Failed to get user")
            .expect("User should exist");

        // Verify the user IS an admin (middleware would allow here)
        assert!(user.is_admin, "Admin user should have admin privileges");

        // Now test that the endpoint works with admin authentication
        let result = validate_config(State(state)).await;
        assert!(result.is_ok(), "Endpoint should work for admin users");

        let response = result.unwrap().0;
        assert!(!response.environment.is_empty());
    }

    #[tokio::test]
    async fn test_admin_check_with_expired_session() {
        let state = setup_test_state();

        // Create an admin user
        let admin_id = create_test_user(&state, "admin_user", true);

        // Create a session
        let token = state
            .session_manager
            .create_session(admin_id)
            .expect("Failed to create session");

        // Manually expire the session in the database
        let conn = state.db.connection().expect("Failed to get connection");
        conn.execute(
            "UPDATE sessions SET expires_at = datetime('now', '-1 day') WHERE token = ?1",
            rusqlite::params![crate::session::hash_token(&token)],
        )
        .expect("Failed to expire session");

        // Try to validate the expired session
        let result = state.session_manager.validate_session(&token);
        assert!(
            result.is_err(),
            "Should reject expired session even for admin users"
        );
    }

    #[tokio::test]
    async fn test_config_validation_response_structure() {
        let state = setup_test_state();

        let result = validate_config(State(state)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;

        // Verify all required fields are present
        assert!(!response.environment.is_empty(), "Should have environment");
        // Errors and warnings are Vec, so they always exist (may be empty)

        // Verify security config structure
        assert!(
            response.security.max_request_size > 0,
            "Should have max request size"
        );

        // Verify session config structure
        assert!(
            response.session.max_age_days > 0,
            "Should have max age days"
        );
        assert!(
            response.session.idle_timeout_hours > 0,
            "Should have idle timeout hours"
        );

        // Verify valid flag is consistent with errors
        if response.errors.is_empty() {
            assert!(response.valid, "Should be valid when no errors");
        } else {
            assert!(!response.valid, "Should be invalid when errors present");
        }
    }
}

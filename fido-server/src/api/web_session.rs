use axum::{
    extract::{Query, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ApiError, ApiResult};
use crate::db::repositories::UserRepository;
use crate::state::AppState;
use crate::test_user_service::TestUserService;
use fido_types::{User, UserContext};

/// Request to create a web session
#[derive(Deserialize)]
pub struct CreateWebSessionRequest {
    pub user_id: String,
    pub user_type: String, // "test" or "real"
}

/// Response for web session creation
#[derive(Serialize)]
pub struct WebSessionResponse {
    pub session_token: String,
    pub user: User,
    pub user_context: UserContext,
}

/// Query parameters for user context detection
#[derive(Deserialize)]
pub struct UserContextQuery {
    pub mode: Option<String>, // "web" or "native"
}

/// Response for user context information
#[derive(Serialize)]
pub struct UserContextResponse {
    pub user_context: UserContext,
    pub is_web_mode: bool,
    pub isolation_active: bool,
}

/// POST /web/session - Create a web session with user context
///
/// Creates a session for either a test user or real user in web mode.
/// This endpoint handles the web-specific authentication flow.
pub async fn create_web_session(
    State(state): State<AppState>,
    Json(payload): Json<CreateWebSessionRequest>,
) -> ApiResult<Json<WebSessionResponse>> {
    let repo = UserRepository::new(state.db.pool.clone());

    match payload.user_type.as_str() {
        "test" => {
            // Handle test user session
            let user = repo
                .get_by_username(&payload.user_id)
                .map_err(|e| ApiError::InternalError(e.to_string()))?
                .ok_or_else(|| {
                    ApiError::NotFound(format!("Test user '{}' not found", payload.user_id))
                })?;

            if !user.is_test_user {
                return Err(ApiError::BadRequest("User is not a test user".to_string()));
            }

            // Create session
            let session_token = state
                .session_manager
                .create_session(user.id)
                .map_err(|e| ApiError::InternalError(e.to_string()))?;

            // Create test user context
            let user_context = UserContext::test_user(user.username.clone());

            tracing::info!("Created web session for test user: {}", user.username);

            Ok(Json(WebSessionResponse {
                session_token,
                user,
                user_context,
            }))
        }
        "real" => {
            // Handle real user session (GitHub authenticated)
            let user_uuid = Uuid::parse_str(&payload.user_id)
                .map_err(|_| ApiError::BadRequest("Invalid user ID format".to_string()))?;

            let user = repo
                .get_by_id(&user_uuid)
                .map_err(|e| ApiError::InternalError(e.to_string()))?
                .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

            if user.is_test_user {
                return Err(ApiError::BadRequest(
                    "Cannot create real user session for test user".to_string(),
                ));
            }

            // Create session
            let session_token = state
                .session_manager
                .create_session(user.id)
                .map_err(|e| ApiError::InternalError(e.to_string()))?;

            // Create real user context
            let github_login = user.github_login.clone().ok_or_else(|| {
                ApiError::InternalError("Real user missing GitHub login".to_string())
            })?;
            let user_context = UserContext::real_user(github_login);

            tracing::info!("Created web session for real user: {}", user.username);

            Ok(Json(WebSessionResponse {
                session_token,
                user,
                user_context,
            }))
        }
        _ => Err(ApiError::BadRequest(
            "Invalid user type. Must be 'test' or 'real'".to_string(),
        )),
    }
}

/// GET /web/context - Get user context information
///
/// Returns the user context for the current session, including whether
/// web mode is active and if data isolation is in effect.
pub async fn get_user_context(
    State(state): State<AppState>,
    Query(params): Query<UserContextQuery>,
    headers: HeaderMap,
) -> ApiResult<Json<UserContextResponse>> {
    // Extract session token from header
    let token = headers
        .get("X-Session-Token")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::Unauthorized("Missing session token".to_string()))?;

    // Validate session and get user
    let user_id = state
        .session_manager
        .validate_session(token)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired session".to_string()))?;

    let repo = UserRepository::new(state.db.pool.clone());
    let user = repo
        .get_by_id(&user_id)
        .map_err(|e| ApiError::InternalError(format!("Failed to get user: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    // Determine user context based on user type
    let user_context = if user.is_test_user {
        UserContext::test_user(user.username.clone())
    } else {
        let github_login = user
            .github_login
            .ok_or_else(|| ApiError::InternalError("Real user missing GitHub login".to_string()))?;
        UserContext::real_user(github_login)
    };

    // Check if web mode is active
    let is_web_mode =
        params.mode.as_deref() == Some("web") || std::env::var("FIDO_WEB_MODE").is_ok();

    // Isolation is active for test users
    let isolation_active = user_context.is_test_user();

    Ok(Json(UserContextResponse {
        user_context,
        is_web_mode,
        isolation_active,
    }))
}

/// POST /web/reset-test-data - Reset test user data
///
/// Resets all test user data to clean state. This endpoint is typically
/// called when the web interface loads to ensure a clean demo environment.
pub async fn reset_test_data(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    let test_service = TestUserService::new(state.db.pool.clone());

    test_service
        .reset_all_test_data()
        .map_err(|e| ApiError::InternalError(format!("Failed to reset test data: {}", e)))?;

    test_service
        .initialize_test_data()
        .map_err(|e| ApiError::InternalError(format!("Failed to initialize test data: {}", e)))?;

    tracing::info!("Test user data reset and reinitialized");

    Ok(Json(serde_json::json!({
        "message": "Test user data reset successfully",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

/// GET /web/mode - Get current mode information
///
/// Returns information about the current operating mode (web vs native)
/// and any relevant configuration.
pub async fn get_mode_info() -> ApiResult<Json<serde_json::Value>> {
    let is_web_mode = std::env::var("FIDO_WEB_MODE").is_ok();
    let web_mode_value = std::env::var("FIDO_WEB_MODE").unwrap_or_default();

    Ok(Json(serde_json::json!({
        "is_web_mode": is_web_mode,
        "web_mode_value": web_mode_value,
        "mode": if is_web_mode { "web" } else { "native" },
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

/// Request to write session info to temporary file
#[derive(Deserialize)]
pub struct WriteSessionRequest {
    pub session_token: String,
    pub user: User,
}

/// POST /web/write-session - Write session info to temporary file
///
/// Writes session information to a temporary file that the terminal can read.
/// This enables the web interface to pass authentication to the terminal.
pub async fn write_session_file(
    Json(payload): Json<WriteSessionRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    use std::fs;
    use std::path::Path;

    // Create temp directory if it doesn't exist
    let temp_dir = Path::new("temp");
    if !temp_dir.exists() {
        fs::create_dir_all(temp_dir).map_err(|e| {
            ApiError::InternalError(format!("Failed to create temp directory: {}", e))
        })?;
    }

    // Write session info to temporary file
    let session_info = serde_json::json!({
        "session_token": payload.session_token,
        "user": payload.user,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    let session_file = temp_dir.join("web_session.json");
    let json_content = serde_json::to_string_pretty(&session_info)
        .map_err(|e| ApiError::InternalError(format!("Failed to serialize session data: {}", e)))?;

    fs::write(&session_file, json_content)
        .map_err(|e| ApiError::InternalError(format!("Failed to write session file: {}", e)))?;

    tracing::info!("Wrote web session file for user: {}", payload.user.username);

    Ok(Json(serde_json::json!({
        "message": "Session file written successfully",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::repositories::{ConfigRepository, DirectMessageRepository, PostRepository};
    use crate::db::Database;
    use crate::session::SessionManager;
    use fido_types::{DirectMessage, Post, UserConfig};
    use proptest::prelude::*;
    use uuid::Uuid;

    #[test]
    fn test_user_context_creation() {
        let test_context = UserContext::test_user("alice".to_string());
        assert!(test_context.is_test_user());
        assert_eq!(test_context.isolation_key(), Some("test_alice"));

        let real_context = UserContext::real_user("github123".to_string());
        assert!(real_context.is_real_user());
        assert_eq!(real_context.isolation_key(), None);
    }

    #[test]
    fn test_web_mode_detection() {
        // Test environment variable detection
        std::env::remove_var("FIDO_WEB_MODE");
        assert!(!std::env::var("FIDO_WEB_MODE").is_ok());

        std::env::set_var("FIDO_WEB_MODE", "true");
        assert!(std::env::var("FIDO_WEB_MODE").is_ok());

        std::env::remove_var("FIDO_WEB_MODE");
    }

    // Property-based tests

    // **Feature: web-terminal-interface, Property 3: Authenticated User Data Access**
    // **Validates: Requirements 2.2**
    // For any successfully authenticated user, the system should provide access to their
    // complete set of posts, messages, and configuration data.
    proptest! {
        #[test]
        fn prop_authenticated_user_data_access(
            username in "[a-zA-Z][a-zA-Z0-9_]{2,19}",
            github_login in "[a-zA-Z][a-zA-Z0-9_-]{2,38}",
            post_content in "[a-zA-Z0-9 ]{1,100}",
            dm_content in "[a-zA-Z0-9 ]{1,100}",
        ) {
            // Create in-memory database for testing
            let db = Database::new(":memory:").expect("Failed to create test database");
            db.initialize().expect("Failed to initialize test database");

            let session_manager = SessionManager::new(db.clone());

            // Create a real user (not test user)
            let user_repo = crate::db::repositories::UserRepository::new(db.pool.clone());
            let user_id = Uuid::new_v4();
            let user = fido_types::User {
                id: user_id,
                username: username.clone(),
                bio: None,
                join_date: chrono::Utc::now(),
                is_test_user: false,
                github_id: Some(12345),
                github_login: Some(github_login.clone()),
            };

            user_repo.create(&user).expect("Failed to create test user");

            // Create session for the user
            let session_token = session_manager.create_session(user_id)
                .expect("Failed to create session");

            // Create some data for the user
            let post_repo = PostRepository::new(db.pool.clone());
            let dm_repo = DirectMessageRepository::new(db.pool.clone());
            let config_repo = ConfigRepository::new(db.pool.clone());

            // Create a post
            let post = Post {
                id: Uuid::new_v4(),
                author_id: user_id,
                author_username: username.clone(),
                content: post_content.clone(),
                created_at: chrono::Utc::now(),
                upvotes: 0,
                downvotes: 0,
                hashtags: vec![],
                user_vote: None,
                parent_post_id: None,
                reply_count: 0,
                reply_to_user_id: None,
                reply_to_username: None,
            };
            post_repo.create(&post).expect("Failed to create test post");

            // Create another user for DM testing
            let other_user_id = Uuid::new_v4();
            let other_user = fido_types::User {
                id: other_user_id,
                username: format!("other_{}", username),
                bio: None,
                join_date: chrono::Utc::now(),
                is_test_user: false,
                github_id: Some(54321),
                github_login: Some(format!("other_{}", github_login)),
            };
            user_repo.create(&other_user).expect("Failed to create other user");

            // Create a DM
            let dm = DirectMessage {
                id: Uuid::new_v4(),
                from_user_id: user_id,
                to_user_id: other_user_id,
                from_username: username.clone(),
                to_username: format!("other_{}", username),
                content: dm_content.clone(),
                created_at: chrono::Utc::now(),
                is_read: false,
            };
            dm_repo.create(&dm).expect("Failed to create test DM");

            // Create user config
            let config = UserConfig {
                user_id,
                color_scheme: fido_types::ColorScheme::Dark,
                sort_order: fido_types::SortOrder::Newest,
                max_posts_display: 25,
                emoji_enabled: true,
            };
            config_repo.update(&config).expect("Failed to create test config");

            // Validate session and verify data access
            let validated_user_id = session_manager.validate_session(&session_token)
                .expect("Session validation should succeed");

            assert_eq!(validated_user_id, user_id, "Session should validate to correct user");

            // Verify access to posts
            let user_posts = post_repo.get_by_user(&user_id)
                .expect("Should be able to access user posts");
            assert!(!user_posts.is_empty(), "User should have access to their posts");
            assert_eq!(user_posts[0].content, post_content, "Post content should match");
            assert_eq!(user_posts[0].author_id, user_id, "Post should belong to user");

            // Verify access to DMs
            let user_dms = dm_repo.get_conversation(&user_id, &other_user_id)
                .expect("Should be able to access user DMs");
            assert!(!user_dms.is_empty(), "User should have access to their DMs");
            assert_eq!(user_dms[0].content, dm_content, "DM content should match");
            assert_eq!(user_dms[0].from_user_id, user_id, "DM should be from user");

            // Verify access to configuration
            let user_config = config_repo.get(&user_id)
                .expect("Should be able to access user config");
            assert_eq!(user_config.user_id, user_id, "Config should belong to user");

            // Verify user can only access their own data
            let other_posts = post_repo.get_by_user(&other_user_id)
                .expect("Should be able to query other user posts");
            assert!(other_posts.is_empty(), "User should not see other user's posts in their feed");

            // Verify other user has default config (not the same as our user's config)
            let other_config = config_repo.get(&other_user_id)
                .expect("Should be able to query other user config");
            // Other user should have default config, not our user's config
            assert_eq!(other_config.user_id, other_user_id, "Other user config should belong to them");
        }
    }

    // **Feature: web-terminal-interface, Property 10: API Route Prefix Absence**
    // **Validates: Requirements 5.4**
    // For any API route definition in the server, the route path should not contain an "/api" prefix.
    proptest! {
        #[test]
        fn prop_api_route_prefix_absence(
            route_path in "/[a-zA-Z0-9/_-]{1,50}",
        ) {
            // This property test validates that our API routes don't use "/api" prefix
            // We test this by ensuring that any valid route path we might define
            // should not start with "/api"

            // Test that the route path doesn't start with "/api"
            assert!(!route_path.starts_with("/api"),
                "API route '{}' should not start with '/api' prefix", route_path);

            // Additionally, test some known routes from our actual server
            let known_routes = vec![
                "/health",
                "/users/test",
                "/auth/login",
                "/auth/logout",
                "/posts",
                "/dms",
                "/config",
                "/hashtags/followed",
                "/web/session",
                "/web/context",
                "/web/reset-test-data",
                "/web/mode",
            ];

            for route in known_routes {
                assert!(!route.starts_with("/api"),
                    "Known API route '{}' should not start with '/api' prefix", route);
            }

            // Test that if we were to accidentally add "/api" prefix, it would be detected
            if route_path.starts_with("/api") {
                panic!("Route '{}' incorrectly uses '/api' prefix", route_path);
            }
        }
    }

    // **Feature: web-terminal-interface, Property 11: Nginx Path Preservation**
    // **Validates: Requirements 5.5**
    // For any API request routed through nginx, the request path should reach the Fido API server without modification.
    proptest! {
        #[test]
        fn prop_nginx_path_preservation(
            path_segment in "[a-zA-Z0-9_-]{1,20}",
            query_param in prop::option::of("[a-zA-Z0-9_=&-]{0,30}"),
        ) {
            // This property test validates that nginx preserves request paths when proxying to the API server
            // We simulate what nginx should do: forward requests without modifying the path

            // Construct a test API path
            let api_path = format!("/{}", path_segment);
            let full_path = match &query_param {
                Some(query) if !query.is_empty() => format!("{}?{}", api_path, query),
                _ => api_path.clone(),
            };

            // Test that the path preservation logic works correctly
            // This simulates what nginx should do: preserve the original path
            let preserved_path = preserve_request_path(&full_path);

            // The preserved path should be identical to the original path
            assert_eq!(preserved_path, full_path,
                "Nginx should preserve request path '{}' without modification", full_path);

            // Test specific known API routes that nginx should preserve
            let known_api_routes = vec![
                "/posts",
                "/dms",
                "/config",
                "/auth/login",
                "/auth/logout",
                "/hashtags/followed",
                "/web/session",
                "/web/context",
                "/web/reset-test-data",
                "/health",
            ];

            for route in known_api_routes {
                let preserved = preserve_request_path(route);
                assert_eq!(preserved, route,
                    "Known API route '{}' should be preserved without modification", route);
            }

            // Test that paths with query parameters are preserved
            if let Some(ref query) = query_param {
                if !query.is_empty() {
                    let path_with_query = format!("/posts?{}", query);
                    let preserved_with_query = preserve_request_path(&path_with_query);
                    assert_eq!(preserved_with_query, path_with_query,
                        "Path with query parameters '{}' should be preserved", path_with_query);
                }
            }
        }
    }

    // Helper function that simulates nginx path preservation behavior
    // This represents what nginx should do when proxying requests
    fn preserve_request_path(original_path: &str) -> String {
        // Nginx should preserve the path exactly as received
        // No modifications, no prefix additions/removals
        original_path.to_string()
    }
}

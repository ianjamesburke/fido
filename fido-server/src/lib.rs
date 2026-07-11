// Library exports for fido-server
// This allows other crates in the workspace to use fido-server modules

pub mod api;
pub mod config;
pub mod db;
pub mod events;
pub mod http;
pub mod mention;
pub mod oauth;
pub mod rate_limit;
pub mod realtime;
pub mod security;
pub mod services;
pub mod session;
pub mod state;
#[cfg(test)]
pub mod test_utils;

use axum::{
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use rate_limit::RateLimiter;
use state::AppState;
use tower_http::limit::RequestBodyLimitLayer;

/// Create the shared API router used by production startup and tests.
///
/// This intentionally excludes production-only static file fallback routing;
/// callers that serve the web terminal can attach that fallback after the API
/// router is built.
pub fn create_router_with_security_config(
    state: AppState,
    security_config: &security::SecurityConfig,
) -> Router {
    let cors_config = security::CorsConfig::new(
        security_config.environment,
        security_config.allowed_origins.clone(),
    );
    let cors = cors_config.to_cors_layer();

    // Create global rate limiter: 100 requests per minute per user
    let rate_limiter = RateLimiter::new(100, 60);

    // Build router
    let mut router = Router::new()
        // Health check
        .route("/health", get(health_check))
        // Public community badge SVG
        .route("/badge/:owner/:repo_svg", get(api::badge::community_badge))
        // Realtime WebSocket gateway
        .route("/ws", get(api::ws::ws_handler));

    // Passwordless test-only auth routes. These allow logging in as any test
    // user with no credentials, so they MUST NEVER be mounted in production.
    if !security_config.environment.is_production() {
        router = router
            .route("/users/test", get(api::auth::list_test_users))
            .route("/auth/login", post(api::auth::login));
    }

    router
        // Authentication routes
        .route("/auth/logout", post(api::auth::logout))
        // Admin-only authentication routes (protected by require_admin middleware)
        .merge(
            Router::new()
                .route("/auth/cleanup-sessions", post(api::auth::cleanup_sessions))
                .route_layer(middleware::from_fn_with_state(
                    state.clone(),
                    security::admin::require_admin,
                )),
        )
        // Admin-only configuration routes (protected by require_admin middleware)
        .merge(
            Router::new()
                .route("/admin/config/validate", get(api::admin::validate_config))
                .route_layer(middleware::from_fn_with_state(
                    state.clone(),
                    security::admin::require_admin,
                )),
        )
        // GitHub Device Flow routes
        .route("/auth/github/device", post(api::auth::github_device_flow))
        .route(
            "/auth/github/device/poll",
            post(api::auth::github_device_poll),
        )
        .route("/auth/validate", get(api::auth::validate_session))
        // Notification routes
        .route(
            "/notifications",
            get(api::notifications::list_notifications),
        )
        .route(
            "/notifications/unread-count",
            get(api::notifications::unread_count),
        )
        .route(
            "/notifications/mark-read",
            post(api::notifications::mark_read),
        )
        // Post routes
        .route("/posts", get(api::posts::get_posts))
        .route("/posts", post(api::posts::create_post))
        .route("/posts/:id/approve", post(api::posts::approve_post))
        .route("/posts/:id/reject", post(api::posts::reject_post))
        .route("/posts/:id/vote", post(api::posts::vote_on_post))
        .route("/posts/:id/replies", get(api::posts::get_replies))
        .route("/posts/:id/reply", post(api::posts::create_reply))
        .route("/posts/:id/thread", get(api::posts::get_thread))
        .route("/posts/:id", get(api::posts::get_post))
        .route("/posts/:id", put(api::posts::update_post))
        .route("/posts/:id", delete(api::posts::delete_post))
        // Profile routes
        .route("/users/:id/profile", get(api::profile::get_profile))
        .route("/users/:id/profile", put(api::profile::update_profile))
        // DM routes
        .route("/dms/conversations", get(api::dms::get_conversations))
        .route(
            "/dms/conversations/:user_id",
            get(api::dms::get_conversation),
        )
        .route(
            "/dms/conversations/:user_id",
            delete(api::dms::delete_conversation),
        )
        .route(
            "/dms/mark-read/:user_id",
            post(api::dms::mark_messages_read),
        )
        .route("/dms/requests", get(api::dms::get_pending_requests))
        .route(
            "/dms/requests/:user_id/accept",
            post(api::dms::accept_request),
        )
        .route(
            "/dms/requests/:user_id/decline",
            post(api::dms::decline_request),
        )
        .route("/dms", post(api::dms::send_message))
        // Config routes
        .route("/config", get(api::config::get_config))
        .route("/config", put(api::config::update_config))
        // Community routes
        .route(
            "/communities/browse",
            get(api::communities::browse_communities),
        )
        .route("/communities/join", post(api::communities::join_community))
        .route("/communities", get(api::communities::list_communities))
        .route("/communities/:id", get(api::communities::get_community))
        .route(
            "/communities/:id/channels",
            get(api::chat::list_community_channels),
        )
        .route(
            "/communities/:id/posts/pending",
            get(api::posts::get_pending_posts),
        )
        .route(
            "/communities/:id/claim",
            post(api::communities::claim_community),
        )
        .route(
            "/communities/:id/members",
            get(api::communities::list_members),
        )
        .route(
            "/communities/:id/membership",
            delete(api::communities::leave_community),
        )
        .route(
            "/channels/:id/messages",
            get(api::chat::get_channel_messages).post(api::chat::send_channel_message),
        )
        // User routes
        .route("/users/search", get(api::friends::search_users))
        .route(
            "/users/:id/profile-view",
            get(api::friends::get_user_profile),
        )
        .route(
            "/users/:id/follow",
            post(api::friends::follow_user).delete(api::friends::unfollow_user),
        )
        // Social routes
        .route("/social/following", get(api::friends::get_following_list))
        .route("/social/followers", get(api::friends::get_followers_list))
        .route("/social/mutual", get(api::friends::get_mutual_friends_list))
        .with_state(state.clone())
        .layer(middleware::from_fn_with_state(
            state,
            rate_limit::rate_limit_middleware,
        ))
        .layer(axum::Extension(rate_limiter))
        .layer(cors)
        // Security headers middleware - adds X-Content-Type-Options, X-Frame-Options, etc.
        .layer(middleware::from_fn(
            security::headers::create_security_headers_layer(security_config.environment),
        ))
        // Request body size limit: 1MB (1024 * 1024 bytes)
        .layer(RequestBodyLimitLayer::new(1024 * 1024))
}

async fn health_check() -> &'static str {
    "OK"
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{Context, Result};
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use reqwest::StatusCode;

    const TEST_TOKEN_KEY: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

    fn test_state() -> Result<AppState> {
        let db = db::Database::in_memory().context("create in-memory database")?;
        db.initialize().context("initialize schema")?;
        let repos = db::repositories::Repositories::new(db.pool.clone());

        let _guard = test_utils::env_lock().lock().unwrap();
        std::env::set_var("FIDO_TOKEN_KEY", TEST_TOKEN_KEY);
        let state = AppState::new_with_repos(db, repos).context("build app state")?;
        std::env::remove_var("FIDO_TOKEN_KEY");

        Ok(state)
    }

    async fn status_for(config: security::SecurityConfig, path: &str) -> Result<StatusCode> {
        let router = create_router_with_security_config(test_state()?, &config);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .context("bind test listener")?;
        let addr = listener.local_addr().context("read listener addr")?;

        tokio::spawn(async move {
            axum::serve(listener, router)
                .await
                .expect("test server crashed");
        });

        let response = reqwest::get(format!("http://{addr}{path}"))
            .await
            .context("send test request")?;
        Ok(response.status())
    }

    #[tokio::test]
    async fn mounts_test_login_routes_in_development() -> Result<()> {
        let config = security::SecurityConfig {
            environment: security::Environment::Development,
            ..Default::default()
        };

        assert_eq!(status_for(config, "/users/test").await?, StatusCode::OK);
        Ok(())
    }

    #[tokio::test]
    async fn omits_test_login_routes_in_production() -> Result<()> {
        let config = security::SecurityConfig {
            environment: security::Environment::Production,
            ..Default::default()
        };

        assert_eq!(
            status_for(config, "/users/test").await?,
            StatusCode::NOT_FOUND
        );
        Ok(())
    }

    #[test]
    fn token_key_fixture_is_valid() -> Result<()> {
        assert_eq!(STANDARD.decode(TEST_TOKEN_KEY)?.len(), 32);
        Ok(())
    }
}

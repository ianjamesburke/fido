// Library exports for fido-server
// This allows other crates in the workspace to use fido-server modules

pub mod api;
pub mod config;
pub mod db;
pub mod hashtag;
pub mod http;
pub mod mention;
pub mod oauth;
pub mod rate_limit;
pub mod security;
pub mod services;
pub mod stores;
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

/// Create the application router for testing
///
/// This function creates the same router used by the main server,
/// but allows tests to create it with a custom AppState.
pub fn create_router(state: AppState) -> Router {
    // Configure CORS using environment-aware configuration
    let security_config = security::SecurityConfig::from_env()
        .unwrap_or_else(|_| security::SecurityConfig::default());
    let cors_config = security::CorsConfig::for_environment(security_config.environment);
    let cors = cors_config.to_cors_layer();

    // Create global rate limiter: 100 requests per minute per user
    let rate_limiter = RateLimiter::new(100, 60);

    // Build router
    Router::new()
        // Health check
        .route("/health", get(health_check))
        // Authentication routes
        .route("/users/test", get(api::auth::list_test_users))
        .route("/auth/login", post(api::auth::login))
        .route("/auth/logout", post(api::auth::logout))
        // Admin-only authentication routes (protected by require_admin middleware)
        .merge(
            Router::new()
                .route("/auth/cleanup-sessions", post(api::auth::cleanup_sessions))
                .route_layer(middleware::from_fn_with_state(state.clone(), security::admin::require_admin))
        )
        // Admin-only configuration routes (protected by require_admin middleware)
        .merge(
            Router::new()
                .route("/admin/config/validate", get(api::admin::validate_config))
                .route_layer(middleware::from_fn_with_state(state.clone(), security::admin::require_admin))
        )
        // GitHub Device Flow routes
        .route("/auth/github/device", post(api::auth::github_device_flow))
        .route(
            "/auth/github/device/poll",
            post(api::auth::github_device_poll),
        )
        .route("/auth/validate", get(api::auth::validate_session))
        // Post routes
        .route("/posts", get(api::posts::get_posts))
        .route("/posts", post(api::posts::create_post))
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
        .route("/users/:id/hashtags", get(api::profile::get_user_hashtags))
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
        .route("/dms", post(api::dms::send_message))
        // Config routes
        .route("/config", get(api::config::get_config))
        .route("/config", put(api::config::update_config))
        // Hashtag routes
        .route(
            "/hashtags/followed",
            get(api::hashtags::get_followed_hashtags),
        )
        .route("/hashtags/follow", post(api::hashtags::follow_hashtag))
        .route(
            "/hashtags/follow/:name",
            delete(api::hashtags::unfollow_hashtag),
        )
        .route("/hashtags/search", get(api::hashtags::search_hashtags))
        .route("/hashtags/active", get(api::hashtags::get_active_hashtags))
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
        .layer(middleware::from_fn_with_state(state, rate_limit::rate_limit_middleware))
        .layer(axum::Extension(rate_limiter))
        .layer(cors)
        // Security headers middleware - adds X-Content-Type-Options, X-Frame-Options, etc.
        .layer(middleware::from_fn(security::headers::create_security_headers_layer(security_config.environment)))
        // Request body size limit: 1MB (1024 * 1024 bytes)
        .layer(RequestBodyLimitLayer::new(1024 * 1024))
}

async fn health_check() -> &'static str {
    "OK"
}

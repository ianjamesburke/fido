mod api;
mod config;
mod db;
mod hashtag;
mod mention;
mod oauth;
mod rate_limit;
mod session;
mod state;

use axum::{
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use rate_limit::RateLimiter;
use state::AppState;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "fido_server=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load settings
    let settings = config::Settings::new().expect("Failed to load settings");

    // Initialize database
    let db = db::Database::new(&settings.database.path)
        .expect("Failed to create database");
    
    db.initialize()
        .expect("Failed to initialize database schema");
    
    // Always seed test data for development
    db.seed_test_data()
        .expect("Failed to seed test data");
    tracing::info!("Test data seeded successfully");

    tracing::info!("Database initialized successfully");

    // Create application state
    let state = AppState::new(db);

    // Run initial session cleanup on startup
    tracing::info!("Running initial session cleanup...");
    match state.session_manager.cleanup_expired_sessions() {
        Ok(count) => {
            if count > 0 {
                tracing::info!("Cleaned up {} expired sessions on startup", count);
            } else {
                tracing::info!("No expired sessions to clean up");
            }
        }
        Err(e) => {
            tracing::error!("Failed to cleanup expired sessions on startup: {}", e);
        }
    }

    // Start background task for periodic session cleanup
    let cleanup_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600)); // Run every hour
        loop {
            interval.tick().await;
            tracing::debug!("Running periodic session cleanup...");
            match cleanup_state.session_manager.cleanup_expired_sessions() {
                Ok(count) => {
                    if count > 0 {
                        tracing::info!("Periodic cleanup: removed {} expired sessions", count);
                    }
                }
                Err(e) => {
                    tracing::error!("Periodic session cleanup failed: {}", e);
                }
            }
        }
    });

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Create global rate limiter: 100 requests per minute per user
    let rate_limiter = RateLimiter::new(100, 60);

    // Build router
    let app = Router::new()
        // Health check
        .route("/health", get(health_check))
        // Authentication routes
        .route("/users/test", get(api::auth::list_test_users))
        .route("/auth/login", post(api::auth::login))
        .route("/auth/logout", post(api::auth::logout))
        .route("/auth/cleanup-sessions", post(api::auth::cleanup_sessions))
        // GitHub Device Flow routes
        .route("/auth/github/device", post(api::auth::github_device_flow))
        .route("/auth/github/device/poll", post(api::auth::github_device_poll))
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
        .route("/dms/conversations/:user_id", get(api::dms::get_conversation))
        .route("/dms/conversations/:user_id", delete(api::dms::delete_conversation))
        .route("/dms/mark-read/:user_id", post(api::dms::mark_messages_read))
        .route("/dms", post(api::dms::send_message))
        // Config routes
        .route("/config", get(api::config::get_config))
        .route("/config", put(api::config::update_config))
        // Hashtag routes
        .route("/hashtags/followed", get(api::hashtags::get_followed_hashtags))
        .route("/hashtags/follow", post(api::hashtags::follow_hashtag))
        .route("/hashtags/follow/:name", delete(api::hashtags::unfollow_hashtag))
        .route("/hashtags/search", get(api::hashtags::search_hashtags))
        .route("/hashtags/active", get(api::hashtags::get_active_hashtags))
        // User routes
        .route("/users/search", get(api::friends::search_users))
        .route("/users/:id/profile-view", get(api::friends::get_user_profile))
        .route("/users/:id/follow", post(api::friends::follow_user).delete(api::friends::unfollow_user))
        // Social routes
        .route("/social/following", get(api::friends::get_following_list))
        .route("/social/followers", get(api::friends::get_followers_list))
        .route("/social/mutual", get(api::friends::get_mutual_friends_list))
        .with_state(state)
        .layer(middleware::from_fn(rate_limit::rate_limit_middleware))
        .layer(axum::Extension(rate_limiter))
        .layer(cors);

    // Start server
    let addr: SocketAddr = format!("{}:{}", settings.server.host, settings.server.port)
        .parse()
        .expect("Failed to parse server address");
    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app)
        .await
        .expect("Server error");
}

async fn health_check() -> &'static str {
    "OK"
}

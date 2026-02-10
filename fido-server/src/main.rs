mod api;
mod config;
mod db;
mod hashtag;
mod http;
mod mention;
mod oauth;
mod rate_limit;
mod security;
mod services;
mod session;
mod state;
mod stores;
#[cfg(test)]
mod test_utils;

use axum::{
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use clap::Parser;
use rate_limit::RateLimiter;
use state::AppState;
use stores::Stores;
use std::net::SocketAddr;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Fido Server - Terminal social platform API
#[derive(Parser, Debug)]
#[command(name = "fido-server")]
#[command(about = "Fido API server", long_about = None)]
struct Args {
    /// Reset the database (delete and recreate with fresh test data)
    #[arg(long)]
    reset_db: bool,
}

#[tokio::main]
async fn main() {
    // Parse command-line arguments
    let args = Args::parse();

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

    tracing::info!("Starting Fido server v1.0.1...");

    // Load and validate security configuration
    let security_config = match security::SecurityConfig::from_env() {
        Ok(config) => {
            tracing::info!("Successfully loaded security configuration");
            config
        }
        Err(e) => {
            tracing::error!("Failed to load security configuration: {}", e);
            eprintln!("FATAL: Failed to load security configuration: {}", e);
            std::process::exit(1);
        }
    };

    // Validate security configuration (fail fast in production)
    if let Err(e) = security_config.validate() {
        if security_config.environment.is_production() {
            tracing::error!("Security configuration validation failed: {}", e);
            eprintln!("FATAL: Security configuration validation failed: {}", e);
            std::process::exit(1);
        } else {
            tracing::warn!(
                "Security configuration validation warning (development mode): {}",
                e
            );
        }
    }

    // Log security configuration (without secrets)
    security_config.log_config();

    // Load settings with detailed error handling
    let settings = match config::Settings::new() {
        Ok(settings) => {
            tracing::info!(
                "Successfully loaded configuration: host={}, port={}, db_path={}",
                settings.server.host,
                settings.server.port,
                settings.database.path
            );
            settings
        }
        Err(e) => {
            tracing::error!("Failed to load settings: {}", e);
            eprintln!("FATAL: Failed to load settings: {}", e);
            std::process::exit(1);
        }
    };

    // Validate settings
    if let Err(e) = settings.validate() {
        tracing::error!("Invalid configuration: {}", e);
        eprintln!("FATAL: Invalid configuration: {}", e);
        std::process::exit(1);
    }

    let backend = std::env::var("DB_BACKEND").unwrap_or_else(|_| "sqlite".to_string());
    let backend = backend.to_ascii_lowercase();

    let db = if backend == "firestore" {
        tracing::info!(
            "DB_BACKEND=firestore selected; skipping SQLite file initialization and using Firestore-backed stores"
        );
        match db::Database::in_memory() {
            Ok(db) => db,
            Err(e) => {
                tracing::error!("Failed to create placeholder in-memory database: {}", e);
                eprintln!("FATAL: Failed to create placeholder in-memory database: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        // Check database directory permissions
        let db_path = std::path::Path::new(&settings.database.path);
        if let Some(parent) = db_path.parent() {
            tracing::info!("Checking database directory: {}", parent.display());
            match std::fs::metadata(parent) {
                Ok(metadata) => {
                    tracing::info!(
                        "Database directory exists, permissions: {:?}",
                        metadata.permissions()
                    );
                }
                Err(e) => {
                    tracing::warn!("Database directory check failed: {}", e);
                    if let Err(create_err) = std::fs::create_dir_all(parent) {
                        tracing::error!("Failed to create database directory: {}", create_err);
                        eprintln!(
                            "FATAL: Failed to create database directory {}: {}",
                            parent.display(),
                            create_err
                        );
                        std::process::exit(1);
                    } else {
                        tracing::info!("Created database directory: {}", parent.display());
                    }
                }
            }
        }

        if args.reset_db {
            tracing::warn!("Database reset requested - deleting existing database");
            if std::path::Path::new(&settings.database.path).exists() {
                if let Err(e) = std::fs::remove_file(&settings.database.path) {
                    tracing::error!("Failed to delete database file: {}", e);
                    eprintln!("FATAL: Failed to delete database file: {}", e);
                    std::process::exit(1);
                }
                tracing::info!("Deleted existing database: {}", settings.database.path);
            } else {
                tracing::info!("No existing database to delete");
            }
        }

        tracing::info!("Creating database connection...");
        let db = match db::Database::new(&settings.database.path) {
            Ok(db) => {
                tracing::info!("Successfully created database connection");
                db
            }
            Err(e) => {
                tracing::error!("Failed to create database: {}", e);
                eprintln!("FATAL: Failed to create database: {}", e);
                std::process::exit(1);
            }
        };

        tracing::info!("Initializing database schema...");
        if let Err(e) = db.initialize() {
            tracing::error!("Failed to initialize database schema: {}", e);
            eprintln!("FATAL: Failed to initialize database schema: {}", e);
            std::process::exit(1);
        }
        tracing::info!("Database schema initialized successfully");

        tracing::info!("Seeding test data...");
        if let Err(e) = db.seed_test_data() {
            tracing::warn!("Failed to seed test data (non-fatal): {}", e);
            tracing::info!("Continuing without test data - database may already be populated");
        } else {
            tracing::info!("Test data seeded successfully");
        }

        tracing::info!("Database initialized successfully");
        db
    };

    let stores = match Stores::from_env(db.pool.clone()) {
        Ok(stores) => stores,
        Err(e) => {
            tracing::error!("Failed to initialize stores: {}", e);
            eprintln!("FATAL: Failed to initialize stores: {}", e);
            std::process::exit(1);
        }
    };

    // Create application state
    let state = AppState::new_with_stores(db, stores);

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

    // Configure CORS using environment-aware configuration
    let cors_config = security::CorsConfig::new(
        security_config.environment,
        security_config.allowed_origins.clone(),
    );
    tracing::info!(
        "CORS configured for {} environment with origins: {:?}",
        security_config.environment,
        cors_config.allowed_origins()
    );
    let cors = cors_config.to_cors_layer();

    // Create global rate limiter: 100 requests per minute per user
    let rate_limiter = RateLimiter::new(100, 60);

    // Build router - API routes take precedence over static files
    let app = Router::new()
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
        // Serve static files from web directory - only for unmatched routes
        .fallback_service(ServeDir::new("../web"));

    // Start server
    let addr_str = format!("{}:{}", settings.server.host, settings.server.port);
    let addr: SocketAddr = match addr_str.parse() {
        Ok(addr) => addr,
        Err(e) => {
            tracing::error!("Failed to parse server address '{}': {}", addr_str, e);
            eprintln!(
                "FATAL: Failed to parse server address '{}': {}",
                addr_str, e
            );
            std::process::exit(1);
        }
    };

    tracing::info!("Binding to address: {}", addr);

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => {
            tracing::info!("Successfully bound to address: {}", addr);
            listener
        }
        Err(e) => {
            tracing::error!("Failed to bind to address {}: {}", addr, e);
            eprintln!("FATAL: Failed to bind to address {}: {}", addr, e);
            std::process::exit(1);
        }
    };

    tracing::info!("Server starting successfully on {}", addr);

    if let Err(e) = axum::serve(listener, app).await {
        tracing::error!("Server error: {}", e);
        eprintln!("FATAL: Server error: {}", e);
        std::process::exit(1);
    }
}

async fn health_check() -> &'static str {
    "OK"
}

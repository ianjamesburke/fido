use clap::Parser;
use fido_server::{
    config, create_router_with_security_config,
    db::{self, repositories::Repositories},
    security, services,
    state::AppState,
};
use std::net::SocketAddr;
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

    tracing::info!("Starting Fido server v{}...", env!("CARGO_PKG_VERSION"));

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

    // Fail fast if the token encryption key is missing/invalid in production.
    // OAuth token storage is unusable without it, so abort startup rather than
    // failing lazily on first use.
    if security_config.environment.is_production() {
        if let Err(e) = services::github::validate_token_key() {
            tracing::error!("FIDO_TOKEN_KEY validation failed: {}", e);
            eprintln!("FATAL: FIDO_TOKEN_KEY validation failed: {}", e);
            std::process::exit(1);
        }

        // GITHUB_CLIENT_SECRET is required to revoke GitHub grants on logout.
        // Without it, logout deletes only the local ciphertext and leaves the
        // upstream grant live indefinitely, so fail fast rather than silently
        // skipping revocation in production.
        if std::env::var("GITHUB_CLIENT_SECRET")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .is_none()
        {
            tracing::error!("GITHUB_CLIENT_SECRET is required in production");
            eprintln!("FATAL: GITHUB_CLIENT_SECRET is required in production");
            std::process::exit(1);
        }
    }

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

    let db = {
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
                tracing::error!("Failed to create database: {:#}", e);
                eprintln!("FATAL: Failed to create database: {:#}", e);
                std::process::exit(1);
            }
        };

        tracing::info!("Initializing database schema...");
        if let Err(e) = db.initialize() {
            tracing::error!("Failed to initialize database schema: {:#}", e);
            eprintln!("FATAL: Failed to initialize database schema: {:#}", e);
            std::process::exit(1);
        }
        tracing::info!("Database schema initialized successfully");

        if security_config.environment.is_production() {
            tracing::info!("Production environment: skipping test data seeding");
        } else {
            tracing::info!("Seeding test data...");
            if let Err(e) = db.seed_test_data() {
                tracing::warn!("Failed to seed test data (non-fatal): {}", e);
                tracing::info!("Continuing without test data - database may already be populated");
            } else {
                tracing::info!("Test data seeded successfully");
            }
        }

        tracing::info!("Database initialized successfully");
        db
    };

    let repos = Repositories::new(db.pool.clone());

    // Create application state
    let state = match AppState::new_with_repos(db, repos) {
        Ok(state) => state,
        Err(e) => {
            tracing::error!("Failed to build application state: {}", e);
            eprintln!("FATAL: Failed to build application state: {}", e);
            std::process::exit(1);
        }
    };

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

    tracing::info!(
        "CORS configured for {} environment with origins: {:?}",
        security_config.environment,
        security_config.allowed_origins
    );

    // API routes are shared with tests; production only adds the static web fallback.
    let app = create_router_with_security_config(state.clone(), &security_config)
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

    // On ctrl-c, signal the realtime gateway first so WebSocket connections
    // close with a going-away frame, then let axum drain gracefully.
    let gateway = state.realtime.clone();
    let shutdown = async move {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!("Failed to listen for shutdown signal: {}", e);
        }
        tracing::info!("Shutdown signal received, closing WebSocket connections");
        gateway.shutdown();
    };

    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
    {
        tracing::error!("Server error: {}", e);
        eprintln!("FATAL: Server error: {}", e);
        std::process::exit(1);
    }
}

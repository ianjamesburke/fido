mod api;
mod app;
mod auth;
mod config;
mod emoji;
mod event_loop;
#[macro_use]
mod logging;
mod repo_context;
mod session;
mod terminal;
mod text_wrapper;
mod ui;

use anyhow::Result;
use app::App;
use clap::Parser;

fn env_flag(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}
use std::process::Command;

/// Current version from Cargo.toml
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Fido - A blazing-fast, keyboard-driven social platform for developers
#[derive(Parser)]
#[command(name = "fido")]
#[command(about = "A terminal-based social network for developers")]
#[command(version)]
struct Cli {
    /// Server URL to connect to
    #[arg(long, short, env = "FIDO_SERVER_URL")]
    server: Option<String>,

    /// Enable verbose logging
    #[arg(long, short)]
    verbose: bool,

    /// Update fido to the latest version from crates.io
    #[arg(long)]
    update: bool,
}

// Load environment variables from .env file
// This allows FIDO_SERVER_URL and other config to be set without command-line args
fn load_env() {
    // Do not auto-load .env in release binaries (cargo install users).
    // This avoids accidental localhost overrides when running from a repo directory.
    // Allow explicit opt-in for release builds via FIDO_LOAD_DOTENV=true.
    let should_load = cfg!(debug_assertions) || env_flag("FIDO_LOAD_DOTENV");
    if should_load {
        let _ = dotenv::dotenv();
    }
}

/// Check crates.io for the latest version of fido
async fn fetch_latest_version() -> Result<String> {
    #[derive(serde::Deserialize)]
    struct CrateResponse {
        #[serde(rename = "crate")]
        krate: CrateInfo,
    }

    #[derive(serde::Deserialize)]
    struct CrateInfo {
        max_stable_version: String,
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()?;

    let resp = client
        .get("https://crates.io/api/v1/crates/fido")
        .header("User-Agent", format!("fido/{}", CURRENT_VERSION))
        .send()
        .await?;

    let data: CrateResponse = resp.json().await?;
    Ok(data.krate.max_stable_version)
}

/// Check crates.io for the latest version of fido
async fn check_for_updates() -> Option<String> {
    let latest = fetch_latest_version().await.ok()?;

    // Simple version comparison (works for semver)
    if latest != CURRENT_VERSION && is_newer_version(&latest, CURRENT_VERSION) {
        Some(latest)
    } else {
        None
    }
}

/// Simple semver comparison - returns true if `latest` is newer than `current`
fn is_newer_version(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> { v.split('.').filter_map(|s| s.parse().ok()).collect() };

    let latest_parts = parse(latest);
    let current_parts = parse(current);

    for (l, c) in latest_parts.iter().zip(current_parts.iter()) {
        if l > c {
            return true;
        }
        if l < c {
            return false;
        }
    }

    latest_parts.len() > current_parts.len()
}

// Performance optimization notes:
// - Lazy rendering: Only visible posts/messages are rendered (not all 1000+)
// - Virtual scrolling: Empty lines represent off-screen content
// - Viewport caching: Terminal size changes trigger viewport recalculation
// - Smooth scrolling: Scroll margin keeps selected item in middle third
// - Minimal redraws: Only changed portions trigger re-render
//
// Performance testing recommendations:
// 1. Test with 1000+ posts: Create test data with large post count
// 2. Monitor frame rate: Should maintain 60fps even with large datasets
// 3. Memory usage: Should remain constant regardless of post count
// 4. Scroll responsiveness: j/k navigation should feel instant

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line arguments
    let cli = Cli::parse();

    // Handle --update flag
    if cli.update {
        println!("Checking for updates...");

        let latest_version = match fetch_latest_version().await {
            Ok(version) => version,
            Err(e) => {
                eprintln!("✗ Failed to check for updates: {}", e);
                eprintln!("  Try again later, or run `cargo install fido --force` manually.");
                std::process::exit(1);
            }
        };

        if !is_newer_version(&latest_version, CURRENT_VERSION) {
            println!(
                "✓ You are already on the latest version (v{}).",
                CURRENT_VERSION
            );
            return Ok(());
        }

        println!(
            "Updating fido from v{} to v{}...",
            CURRENT_VERSION, latest_version
        );
        let status = Command::new("cargo")
            .args([
                "install",
                "fido",
                "--version",
                latest_version.as_str(),
                "--force",
            ])
            .status();

        match status {
            Ok(exit_status) if exit_status.success() => {
                println!("✓ fido updated successfully!");
                return Ok(());
            }
            Ok(exit_status) => {
                eprintln!("✗ Update failed with exit code: {}", exit_status);
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("✗ Failed to run cargo install: {}", e);
                eprintln!("  Make sure cargo is installed and in your PATH");
                std::process::exit(1);
            }
        }
    }

    // Load environment variables from .env file
    load_env();

    // Initialize logging system
    // Only enable logging if --verbose flag is passed (production default: disabled)
    let log_config = if cli.verbose {
        logging::LogConfig::verbose()
    } else {
        logging::LogConfig::disabled()
    };
    logging::init_logging(&log_config)?;

    // Initialize terminal
    let mut tui = terminal::init()?;

    // Check runtime mode flags
    let is_web_mode = env_flag("FIDO_WEB_MODE");

    // Create app based on mode
    let mut app = if let Some(server_url) = cli.server {
        // Custom server URL
        App::with_server_url(server_url)
    } else {
        // Default mode
        App::new()
    };

    app.log_config = log_config;

    // The launch directory decides the community: inside a GitHub repo the
    // board opens on that repo's community, elsewhere the Home list shows.
    if let Ok(cwd) = std::env::current_dir() {
        app.launch_repo = repo_context::detect_repo_context(&cwd);
    }

    // Check for updates (quick, non-blocking with 3s timeout, skip in web mode)
    if !is_web_mode {
        if let Some(latest_version) = check_for_updates().await {
            app.update_available = Some(latest_version);
        }
    }

    // Check for existing session on startup
    let mut auth_flow = auth::AuthFlow::new(app.api_client.clone())?;
    if let Ok(Some(user)) = auth_flow.check_existing_session().await {
        log::info!("Restored session for user: {}", user.username);
        app.auth_state.current_user = Some(user);
        app.current_screen = app::Screen::Main;

        // Update API client with session token
        app.api_client = auth_flow.api_client().clone();

        // Load initial data
        let _ = app.load_settings().await;
        app.load_filter_preference();
        let _ = app.init_community_context().await;
        let _ = app.load_posts().await;
    } else {
        log::info!("No valid session found, showing authentication screen");
    }

    // Main event loop
    let mut event_loop = event_loop::EventLoop::new();

    event_loop.run(&mut app, &mut auth_flow, &mut tui).await?;

    // Restore terminal
    terminal::restore()?;

    Ok(())
}

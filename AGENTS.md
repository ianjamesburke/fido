# AGENTS.md

This file provides guidance to AI coding assistants when working with code in this repository.

## Task Tracking Workflow

- Use `TASKS.md` at the repository root as the canonical checklist for Firestore migration and Firebase deployment work.
- Before starting substantial implementation, review `TASKS.md` and select the next unchecked items.
- As each item is completed, update `TASKS.md` by changing `- [ ]` to `- [x]`.
- Keep `TASKS.md` current in the same change set as related code updates so status always reflects reality.

## Project Overview

Fido is a Rust-based terminal social platform for developers, built as a workspace with four crates:

- **fido-types**: Shared data structures and models
- **fido-server**: Backend API server (Axum + SQLite) 
- **fido-tui**: Terminal UI client (Ratatui) - main binary
- **fido-migrate**: Database migration utilities

## Development Commands

### Quick Start with Just

The project uses a `justfile` for common development tasks. Environment variables are automatically loaded from `.env` if present.

```bash
# Run the server
just server

# Run the TUI client
just tui

# Run TUI connected to local server
just tui-local
```

### Building and Running (Direct Cargo)

```bash
# Build entire workspace
cargo build

# Run the TUI client (main application)
cargo run --bin fido

# Run the server locally
cargo run --bin fido-server

# Run client against local server
cargo run --bin fido -- --server http://localhost:3000

# Build for release
cargo build --release
```

### Testing

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p fido-server
cargo test -p fido-tui
cargo test -p fido-types

# Run integration tests
cargo test --test integration_test
```

### Code Quality

```bash
# Format code
cargo fmt

# Check for linting issues
cargo clippy

# Check without building
cargo check
```

## Architecture

### Workspace Structure

The project follows a modular architecture with enum-based backend abstraction:

```
fido/
├── fido-types/      # Shared models (User, Post, Vote, etc.)
├── fido-server/     # REST API server with SQLite
│   ├── api/         # Endpoint handlers (auth, posts, dms, friends, hashtags, profile, config)
│   └── db/          # Repository pattern data access layer
├── fido-tui/        # Terminal UI client
│   ├── api/         # Backend enum abstraction (ApiClient + MockBackend)
│   ├── app/         # Application state and event handlers
│   └── ui/          # Rendering with Ratatui (modals, tabs, theme)
└── fido-migrate/    # Database migrations
```

### Key Design Patterns

**Backend Enum Pattern**: The `Backend` enum in `fido-tui/src/api/backend.rs` provides a unified interface that can be either a real `ApiClient` or a `MockBackend` for demo mode. This enables:
- Seamless switching between live API and mock data
- Demo mode without server dependency
- Consistent API surface for all operations

**Repository Pattern**: Each entity (User, Post, DM, etc.) has a dedicated repository in `fido-server/src/db/repositories/` for data access.

**Separation of Concerns**: 
- State management in `app/state.rs` and `app/mod.rs`
- Event handlers organized by feature in `app/handlers/`
- Pure rendering functions in `ui/` module (modals, tabs, theme)
- Isolated API communication in `api/` module

### API Module Structure (fido-tui)

```
fido-tui/src/api/
├── backend.rs       # Backend enum (Api | Mock) - main abstraction
├── client.rs        # ApiClient - real HTTP client implementation
├── mock_backend.rs  # MockBackend - demo mode with sample data
├── sample_data.rs   # Sample data generators for mock mode
├── error.rs         # ApiError and ApiResult types
└── mod.rs           # Module exports
```

### Server API Endpoints (fido-server)

```
fido-server/src/api/
├── auth.rs          # Login, logout, session validation, GitHub OAuth
├── posts.rs         # CRUD for posts, replies, voting
├── dms.rs           # Direct messages and conversations
├── friends.rs       # Following, followers, mutual friends, user search
├── hashtags.rs      # Hashtag following and search
├── profile.rs       # User profiles and bio updates
├── config.rs        # User configuration/settings
└── error.rs         # API error handling
```

## Configuration

### Server Configuration
- Environment variables and command-line arguments
- Settings file: `fido-server/settings.toml`
- Uses `config` crate for TOML-based configuration

### Client Configuration  
- Local `.fido/` directory with JSON configuration files
- Session management with local session files
- Environment variable: `FIDO_SERVER_URL`

## Database

### Current: SQLite
- Single-file database: `fido.db`
- Connection pooling with r2d2
- Schema defined in `fido-server/src/db/schema.rs`
- All queries use parameterized statements (SQL injection protection)

### Future Migration Path
The repository-based database abstraction in `fido-server/src/db/` supports future PostgreSQL migration with minimal code changes.

## Key Files for Development

- `fido-tui/src/main.rs` - TUI application entry point
- `fido-tui/src/event_loop.rs` - Main event loop
- `fido-tui/src/app/state.rs` - Core application state definitions
- `fido-tui/src/app/mod.rs` - App implementation and methods
- `fido-tui/src/app/handlers/` - Event handlers by feature (posts, dms, modals, etc.)
- `fido-tui/src/ui.rs` - Main rendering logic
- `fido-tui/src/ui/modals/` - Modal dialogs (composer, filters, help, social)
- `fido-server/src/main.rs` - Server entry point and Axum setup
- `fido-server/src/api/` - All API endpoint implementations
- `fido-types/src/models.rs` - Core domain models shared between client/server

## Authentication

- GitHub OAuth integration via `oauth2` crate (Device Flow)
- Session-based authentication with server-side session store
- Sessions stored in `~/.fido/session` on client
- Test user login available for development

## Logging

The TUI uses a configurable logging system with feature-specific macros:
- `log_modal_state!` - Modal state changes
- `log_key_event!` - Keyboard events
- `log_rendering!` - UI rendering
- `log_api_call!` - API requests
- `log_settings!` - Settings changes
- `log_debug!` - General debug

Configuration via `LogConfig` in `fido-tui/src/logging.rs`. See `LOGGING.md` for details.

## Debugging & Logging

### Cohesive Logging System
Fido uses a unified logging system built on Rust's `log` and `simplelog` crates with configurable features.

**Key Features:**
- **Master enable/disable switch**: Turn all logging on/off with a single flag
- **Feature-specific logging**: Enable/disable specific categories (modal_state, key_events, rendering, api_calls, settings, general)
- **Configurable log levels**: Control verbosity (Trace, Debug, Info, Warn, Error, Off)
- **File-based output**: All logs written to files (default: `fido_debug.log`) to avoid interfering with TUI
- **Clear on startup**: Optional log file clearing to prevent excessive growth

**Quick Configuration:**
```rust
// Disable all logging
let log_config = logging::LogConfig::disabled();

// Minimal logging (errors/warnings only)
let log_config = logging::LogConfig::minimal();

// Verbose logging (all features)
let log_config = logging::LogConfig::verbose();

// Default configuration
let log_config = logging::LogConfig::default();
```

**Usage Macros:**
- `log_modal_state!(app.log_config, ...)` - Modal state changes
- `log_key_event!(app.log_config, ...)` - Keyboard events
- `log_rendering!(app.log_config, ...)` - UI rendering operations
- `log_api_call!(app.log_config, ...)` - API/network requests
- `log_settings!(app.log_config, ...)` - Settings changes
- `log_debug!(app.log_config, ...)` - General debug messages

**Documentation**: See `fido/LOGGING.md` for complete usage guide and examples

**Best Practices:**
- Use feature-specific macros for better control
- Disable logging in production builds
- Keep `clear_on_startup: true` to avoid massive log files
- Include relevant context in log messages (IDs, states, etc.)

### Production Server Logs (Fly.io)
To get logs from the deployed Fly server, use:
```bash
fly logs -a fido-social --tail 50
```

**Important**: Always use `--tail` with a specific number (e.g., 50) to limit output. Without it, the command runs forever and blocks execution. Use higher numbers for more history or lower for recent logs only.

## Deployment

- Deployed on Fly.io (`fly.toml`)
- Docker-based deployment (`Dockerfile`)
- Logs: `fly logs -a fido-social --tail 50` (always use --tail to avoid blocking)

### Troubleshooting: Firebase Hosted ttyd Preview Blank

If `/ttyd/` works directly but the embedded terminal on `https://<project>.web.app` is blank:

- Use the Cloud Run ttyd URL directly in `web/index.html` iframe `src` (for this project: `https://fido-web-934696923362.us-central1.run.app/ttyd/`) instead of `/ttyd/`.
- Ensure ttyd response headers allow embedding from Firebase Hosting domains via CSP `frame-ancestors`.
  - Update `nginx.conf` and `start.sh` `/ttyd/` location to include:
    - `https://fido-prod-ijb.web.app`
    - `https://fido-prod-ijb.firebaseapp.com`
- Do not rely on `X-Frame-Options` for multi-origin embedding; use CSP `frame-ancestors`.

Verification checklist:

- Confirm iframe source in deployed homepage:
  - `curl -sS https://fido-prod-ijb.web.app/ | rg "iframe|ttyd|run.app"`
- Confirm ttyd endpoint allows embedding:
  - `curl -I -sS https://fido-web-934696923362.us-central1.run.app/ttyd/`
  - Expect `content-security-policy` containing allowed Firebase domains.
- Confirm websocket activity in Cloud Run logs:
  - `gcloud logging read 'resource.type="cloud_run_revision" AND resource.labels.service_name="fido-web" AND textPayload:"/ttyd/ws"' --limit=50 --format='value(timestamp,textPayload)'`
  - Expect websocket connect lines (`WS /ttyd/ws`) and HTTP `101` upgrades.

Important runtime pitfall:

- If ttyd opens but terminal stays blank/disconnects with reconnect prompts, check for glibc mismatch in logs:
  - `/usr/local/bin/fido: ... GLIBC_X.Y not found`
- Fix by matching builder/runtime OS families in Dockerfile (for this repo: `FROM rust:1.91-bookworm` builder with `debian:bookworm-slim` runtime).

## Web Terminal Demo vs Local TUI

Fido has two distinct modes of operation:

### Local TUI (Production Mode)
- **Client**: Native `fido-tui` binary running locally on user's machine
- **Server**: Connects to production API server (https://fido-social.fly.dev or local server)
- **Database**: Uses persistent SQLite database (`fido.db`) on the server
- **Authentication**: GitHub OAuth with session stored in `~/.fido/session`
- **Data**: All posts, DMs, and user data persists across sessions

### Web Terminal Demo (Demo Mode)
- **Client**: `fido-tui` binary running in Docker container, exposed via ttyd (web terminal server)
- **Server**: Uses `MockBackend` instead of `ApiClient` (enabled via `FIDO_DEMO_MODE=true` env var)
- **Database**: Ephemeral in-memory data structures - no database connection
- **Authentication**: Test users only (no GitHub OAuth)
- **Data**: Sample data generated on startup, lost when browser tab closes
- **Architecture**:
  ```
  Browser → WebSocket → ttyd → fido-tui (FIDO_DEMO_MODE=true) → MockBackend (in-memory)
                                                                  ↓
                                                          nginx reverse proxy
  ```

### Key Implementation Details

**Backend Enum Pattern**: The `Backend` enum in `fido-tui/src/api/backend.rs` enables seamless switching:
```rust
pub enum Backend {
    Api(ApiClient),      // Production: HTTP requests to real server
    Mock(MockBackend),   // Demo: In-memory operations, no network
}
```

**Demo Mode Detection**: On startup, `fido-tui` checks `FIDO_DEMO_MODE` environment variable:
- If `true`: Creates `Backend::Mock(MockBackend::new())` with sample data
- If `false` or unset: Creates `Backend::Api(ApiClient::new())` for real API calls

**Web Terminal Stack** (see `start.sh`):
- **nginx** (port 8080): Reverse proxy for static files and routing
- **ttyd** (port 7681): Terminal-over-WebSocket server
- **fido-server** (port 3000): Production API server (not used in demo mode)
- **fido-tui**: TUI binary with `FIDO_DEMO_MODE=true`

**Sample Data**: `fido-tui/src/api/sample_data.rs` generates realistic test data:
- Pre-populated users, posts, DMs, hashtags
- Simulated voting and following relationships
- Reset on each new terminal session

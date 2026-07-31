# AGENTS.md

## DOX Framework

`AGENTS.md` files are binding work contracts for their subtrees. Before editing, read this root file and every `AGENTS.md` on the route from the repository root to each target. The nearest file controls local details; parent files still apply. A child may add local rules but may not weaken this contract.

### Read Before Editing

1. Identify every file or folder the change will touch.
2. Walk from the repository root to each target and read every `AGENTS.md` on that route.
3. Use the nearest file for local rules and its parents for repository-wide rules.

### Update After Editing

For every meaningful change, update the closest owning `AGENTS.md` when it changes purpose, structure, contracts, workflow, constraints, or verification. Update parent files when a child index changes. Remove stale or contradictory guidance in the same pass.

This file provides guidance to AI coding assistants when working with code in this repository.

## Project Overview

Fido is a blazing-fast, keyboard-driven terminal social platform for developers, built as a Rust workspace with three crates:

- **fido-types**: Shared data structures and models (User, Post, Vote, DirectMessage, etc.)
- **fido-server**: Backend API server (Axum + SQLite with connection pooling)
- **fido-tui**: Terminal UI client (Ratatui) - main binary

### Core Principles
- **Speed First**: Lightning-fast, terminal-native UI optimized for developer workflows
- **Keyboard-Driven**: Every action accessible via keyboard shortcuts, no mouse required
- **Privacy-Focused**: No algorithms, no ads, no tracking - user control over their experience
- **Text-Only**: Markdown support for posts, no images or videos to maintain focus
- **Developer-Centric**: Built by developers, for developers

## Development Commands

### Quick Start with Just

The project uses a `justfile` for common development tasks. Environment variables are automatically loaded from `.env` if present.

```bash
# Run the server (writes logs/fido-server.log by default)
just server

# Run the server with fresh database (also writes logs/fido-server.log)
just server-reset

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
cargo run --bin fido -- --server http://127.0.0.1:4747

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

The project follows a modular architecture:

```
fido/
├── fido-types/      # Shared models (User, Post, Vote, etc.)
├── fido-server/     # REST API server with SQLite
│   ├── api/         # Endpoint handlers (auth, posts, dms, friends, hashtags, profile, config)
│   └── db/          # Repository pattern data access layer
├── fido-tui/        # Terminal UI client
│   ├── api/         # HTTP client (ApiClient)
│   ├── app/         # Application state and event handlers
│   └── ui/          # Rendering with Ratatui (modals, tabs, theme)
```

### Key Design Patterns

**Repository Pattern**: Each entity (User, Post, DM, etc.) has a dedicated repository in `fido-server/src/db/repositories/` for data access:
```rust
pub struct UserRepository {
    db: Arc<Mutex<Connection>>,
}

impl UserRepository {
    pub fn get_by_username(&self, username: &str) -> Result<Option<User>> {
        // Database query logic
    }
}
```

Benefits:
- Encapsulates database logic
- Easy to test with mock repositories
- Supports future database migration (SQLite → PostgreSQL)

**Separation of Concerns**: 
- State management in `app/state.rs` and `app/mod.rs`
- Event handlers organized by feature in `app/handlers/`
- Pure rendering functions in `ui/` module (modals, tabs, theme)
- Isolated API communication in `api/` module

### API Module Structure (fido-tui)

```
fido-tui/src/api/
├── client.rs        # ApiClient - HTTP client implementation
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

### Database Schema

**Core Tables**:
- **users**: id (uuid), github_id, username, bio, created_at
- **posts**: id (uuid), author_id, content (text), parent_id (for replies), created_at, upvotes, downvotes
- **votes**: user_id, post_id, direction ('up'|'down') - PRIMARY KEY (user_id, post_id)
- **dms**: id (uuid), from_id, to_id, content, created_at, read_at
- **friends**: user_id, friend_id, created_at - PRIMARY KEY (user_id, friend_id)
- **hashtag_follows**: user_id, hashtag, created_at - PRIMARY KEY (user_id, hashtag)
- **user_config**: user_id (PRIMARY KEY), color_scheme, default_sort

### Future Migration Path
The repository-based database abstraction in `fido-server/src/db/` supports future PostgreSQL migration with minimal code changes:
1. Implement `DatabaseConnection` trait for PostgreSQL
2. Update repositories to use async/await
3. Create migration scripts
4. Swap database implementation in `main.rs` - no changes needed to API handlers

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
- Session-based authentication with server-side session store (in-memory HashMap)
- Sessions stored in `~/.fido/session` on client with unique instance IDs
- Session tokens are UUIDs (cryptographically random)
- Test user login available for development

## Security Considerations

### Input Validation
- Server validates all inputs (character limits, format checks)
- Client provides user feedback but doesn't rely on client-side validation

### SQL Injection Prevention
- All queries use parameterized statements
- No string concatenation for SQL queries

### Session Security
- Session tokens are UUIDs (cryptographically random)
- Sessions stored in memory (cleared on server restart)
- Client stores sessions locally (file permissions protect)

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

### Local Server Logs

`just server` and `just server-reset` write startup and runtime output to `logs/fido-server.log` by default while still echoing to the terminal. The recipes cap the active log at 10 MiB by default and rotate one previous file to `logs/fido-server.log.1`.

```bash
just server
just server-reset
```

Tail the current local server log with:

```bash
just server-log
# or
tail -f logs/fido-server.log
```

Override log path or size only when needed:

```bash
FIDO_SERVER_LOG=/tmp/fido-server.log just server-reset
FIDO_SERVER_LOG_MAX_BYTES=20971520 just server
```

If a server is already running from an older direct command and `lsof -p <pid>` shows stdout/stderr attached to `/dev/tty*`, past startup logs are only in that terminal's scrollback. Restart with `just server` or `just server-reset` before debugging startup behavior.

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

### Production Server Logs (Railway)
```bash
railway logs --deployment
```

## Deployment

- Deployed on Railway (Docker-based)
- Auto-deploys from `main` branch
- Volume at `/data` for SQLite persistence
- Logs: `railway logs`

### Troubleshooting: ttyd Preview Blank

If `/ttyd/` works directly but the embedded terminal is blank:

- The iframe in `web/index.html` must use a relative `/ttyd/` path (not an absolute URL).
- Ensure ttyd response headers allow embedding via CSP `frame-ancestors` in `nginx.conf`.

Important runtime pitfall:

- If ttyd opens but terminal stays blank/disconnects with reconnect prompts, check for glibc mismatch in logs:
  - `/usr/local/bin/fido: ... GLIBC_X.Y not found`
- Fix by matching builder/runtime OS families in Dockerfile (for this repo: `FROM rust:1.91-bookworm` builder with `debian:bookworm-slim` runtime).

## Web Terminal vs Local TUI

### Local TUI
- **Client**: Native `fido-tui` binary running locally on user's machine
- **Server**: Connects to production API server (https://fido-web-production.up.railway.app or local server)
- **Database**: Uses persistent SQLite database (`fido.db`) on the server
- **Authentication**: GitHub OAuth with session stored in `~/.fido/session`
- **Data**: All posts, DMs, and user data persists across sessions

### Web Terminal Stack (see `start.sh`)
- **nginx** (port 8080): Reverse proxy for static files and routing
- **ttyd** (port 7681): Terminal-over-WebSocket server, runs `fido-tui` against the real API server
- **fido-server** (port 4747): API server

## Testing Strategy

### Unit Testing

**Repositories** (Server):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_user() {
        let db = Database::in_memory().unwrap();
        db.initialize().unwrap();
        
        let repo = UserRepository::new(db.connection());
        // Test repository operations
    }
}
```

### Integration Testing
- **Server**: Test API endpoints with in-memory SQLite database (`cargo test -p fido-server --features sqlite-tests`); `fido-server/tests/e2e_community_rewrite.rs` spins a real server plus a GitHub API fixture server via `GITHUB_API_BASE`.
- **TUI End-to-End (required before every merge)**: `just e2e-tui` (`scripts/e2e_tui.sh`) builds the real binaries, starts `fido-server` with a temp DB and a stubbed GitHub API (`scripts/github_stub.py`), then drives the TUI inside a detached tmux session with `send-keys`, asserting on `capture-pane` output, the SQLite database, and log files. It covers repo launch and legacy reply-log cleanup, posting, chat, settings, approvals, Home, profiles, DMs, requests, and GitHub activity posts. On failure it dumps the pane, server log tail, and keeps artifacts in the temp workdir. Every change must pass this harness against the real local server in a real tmux session; unit and integration tests alone are insufficient.

### Directory-Scoped Communities
The launch directory decides the community (see `docs/superpowers/specs/2026-07-02-directory-scoped-communities-design.md`):
- Inside a git repo with a GitHub `origin`: the TUI joins that repo's community (lazily created server-side) and opens its board. Detection in `fido-tui/src/repo_context.rs`.
- Anywhere else: Home mode — the Posts tab lists joined communities (Enter opens, Esc returns).
- `i` on a board opens the community settings modal (role, member count, claim admin via GitHub permission check).

## Performance Characteristics

### Server
- **Throughput**: ~1000 requests/second (single-threaded SQLite)
- **Latency**: <10ms for most operations
- **Scalability**: Limited by SQLite (single writer)

### Client
- **Frame Rate**: 60 FPS (even with 1000+ posts)
- **Memory**: Constant (lazy rendering with virtual scrolling)
- **Startup Time**: <100ms

### Performance Optimizations
- Lazy rendering (only visible items)
- Virtual scrolling for large lists
- Viewport caching
- Smooth scrolling with buffers

## Future Enhancements

### 1. WebSocket Integration
**Current**: REST API with polling  
**Future**: WebSocket for real-time updates

Implementation plan:
1. Create `RealtimeApiClient` trait extending `ApiClientTrait`
2. Implement WebSocket client in `fido-tui/src/api/websocket.rs`
3. Add WebSocket server endpoint in `fido-server`
4. Update UI to handle real-time events (minimal changes to app logic)

### 2. PostgreSQL Migration
**Current**: SQLite with rusqlite  
**Future**: PostgreSQL with tokio-postgres

Implementation plan:
1. Implement `DatabaseConnection` trait for PostgreSQL
2. Update repositories to use async/await
3. Create migration scripts
4. Update connection pooling (use `deadpool-postgres`)
5. No changes needed to API handlers

### 3. External Editor Integration
**Current**: Built-in text input  
**Future**: Launch `$EDITOR` for long-form content

Implementation plan:
1. Create `editor.rs` module in TUI
2. Add keyboard shortcut to launch editor
3. Save content to temp file, open editor, read result
4. Integrate with post creation and bio editing

### 4. Caching Layer
**Current**: Direct API calls  
**Future**: Local cache with TTL

Implementation plan:
1. Create `CachedApiClient` implementing `ApiClientTrait`
2. Wrap existing `ApiClient`
3. Add cache invalidation logic
4. Store cache in `.fido/cache/`

## Child DOX Index

- [`fido-server/AGENTS.md`](fido-server/AGENTS.md): API server crate, tests, and server-specific contracts.
- [`fido-tui/AGENTS.md`](fido-tui/AGENTS.md): terminal client crate, startup, and interaction contracts.
- [`fido-types/AGENTS.md`](fido-types/AGENTS.md): shared public models, events, and enums.

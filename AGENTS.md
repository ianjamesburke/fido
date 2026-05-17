# AGENTS.md

This file provides guidance to AI coding assistants when working with code in this repository.

## Task Tracking Workflow

- Use `TASKS.md` at the repository root as the canonical checklist for Firestore migration and Firebase deployment work.
- Before starting substantial implementation, review `TASKS.md` and select the next unchecked items.
- As each item is completed, update `TASKS.md` by changing `- [ ]` to `- [x]`.
- Keep `TASKS.md` current in the same change set as related code updates so status always reflects reality.

## Project Overview

Fido is a blazing-fast, keyboard-driven terminal social platform for developers, built as a Rust workspace with four crates:

- **fido-types**: Shared data structures and models (User, Post, Vote, DirectMessage, etc.)
- **fido-server**: Backend API server (Axum + SQLite with connection pooling)
- **fido-tui**: Terminal UI client (Ratatui) - main binary
- **fido-migrate**: Database migration utilities

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

**Trait-Based Abstractions**: Core functionality exposed through traits for easy mocking and testing:
```rust
#[async_trait]
pub trait ApiClientTrait: Send + Sync {
    async fn get_posts(&self, limit: Option<i32>, sort: Option<String>) -> ApiResult<Vec<Post>>;
    async fn create_post(&self, content: String) -> ApiResult<Post>;
}
```

This enables:
- Easy mock implementations for testing
- Future WebSocket client implementation
- Offline mode or caching layer additions

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

## Web Terminal Demo vs Local TUI

Fido has two distinct modes of operation:

### Local TUI (Production Mode)
- **Client**: Native `fido-tui` binary running locally on user's machine
- **Server**: Connects to production API server (https://fido-web-production.up.railway.app or local server)
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

**API Client** (TUI):
```rust
// Create mock implementation
struct MockApiClient {
    posts: Vec<Post>,
}

#[async_trait]
impl ApiClientTrait for MockApiClient {
    async fn get_posts(&self, _: Option<i32>, _: Option<String>) -> ApiResult<Vec<Post>> {
        Ok(self.posts.clone())
    }
}

#[tokio::test]
async fn test_app_with_mock_api() {
    let mock_client = MockApiClient { posts: vec![/* test data */] };
    // Test app logic with mock
}
```

### Integration Testing
- **Server**: Test API endpoints with in-memory SQLite database
- **Client**: Test UI flows with mock API client
- **End-to-End**: Test full stack with Docker containers

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

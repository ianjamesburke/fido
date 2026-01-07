# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Fido is a Rust-based terminal social platform for developers, built as a workspace with four crates:

- **fido-types**: Shared data structures and models
- **fido-server**: Backend API server (Axum + SQLite) 
- **fido-tui**: Terminal UI client (Ratatui) - main binary
- **fido-migrate**: Database migration utilities

## Development Commands

### Building and Running

```bash
# Build entire workspace
cargo build

# Run the TUI client (main application)
cargo run --bin fido

# Run the server locally
cargo run --bin fido-server

# Run client against local server
fido --server http://localhost:3000

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

The project follows a modular, trait-based architecture:

```
fido/
├── fido-types/      # Shared models (User, Post, Vote, etc.)
├── fido-server/     # REST API server with SQLite
│   ├── api/         # Endpoint handlers (auth, posts, dms, etc.)
│   └── db/          # Repository pattern data access layer
├── fido-tui/        # Terminal UI client
│   ├── api/         # HTTP client implementation
│   ├── app/         # Application state and logic
│   └── ui/          # Rendering with Ratatui
└── fido-migrate/    # Database migrations
```

### Key Design Patterns

**Repository Pattern**: Each entity (User, Post, etc.) has a dedicated repository in `fido-server/src/db/repositories/` for data access.

**Trait-Based APIs**: Core functionality exposed through traits for easy mocking and testing. See `fido-tui/src/api/traits.rs` for the `ApiClientTrait`.

**Separation of Concerns**: 
- State management in `app.rs` (no rendering logic)
- Pure rendering functions in `ui.rs` (no business logic)  
- Isolated API communication in `api/` module

### Performance Optimizations

The TUI implements several performance optimizations for handling large datasets:
- Lazy rendering (only visible items)
- Virtual scrolling for long lists
- Viewport caching
- Smooth scrolling with buffers

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
The trait-based database abstraction in `fido-server/src/db/connection.rs` supports future PostgreSQL migration with minimal code changes.

## Key Files for Development

- `fido-tui/src/main.rs` - TUI application entry point and event loop
- `fido-tui/src/app/state.rs` - Core application state management  
- `fido-tui/src/ui.rs` - Main rendering logic
- `fido-server/src/main.rs` - Server entry point and Axum setup
- `fido-server/src/api/` - All API endpoint implementations
- `fido-types/src/models.rs` - Core domain models shared between client/server

## Authentication

- GitHub OAuth integration via `oauth2` crate
- Session-based authentication with in-memory session store
- Sessions stored as UUIDs in `~/.fido/session` on client

## Planned Enhancements

The architecture is designed to support:
1. **WebSocket Integration** - Real-time updates via `RealtimeApiClient` trait extension
2. **PostgreSQL Migration** - Async repositories with minimal API handler changes
3. **Caching Layer** - `CachedApiClient` wrapper implementing `ApiClientTrait`
4. **External Editor Integration** - Launch `$EDITOR` for long-form content
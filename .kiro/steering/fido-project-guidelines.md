# Fido Project Development Guidelines

do not need to create summary documents after every code change.

## Kiro Development Best Practices

### Command Execution Guidelines
- **CRITICAL**: When using `fly logs`, ALWAYS include `--tail` with a specific number (e.g., `--tail 50`)
- **Why**: Without `--tail`, the command streams logs indefinitely and blocks agentic execution
- **Examples**:
  - ✅ `fly logs -a fido-social --tail 50` (gets last 50 lines)
  - ✅ `fly logs -a fido-social --tail 100` (gets last 100 lines for more history)
  - ❌ `fly logs -a fido-social` (streams forever, blocks execution)
- **Rule**: Use higher numbers for more history, lower for recent logs only 

## Project Overview

Fido is a blazing-fast, keyboard-driven social platform for developers, featuring a beautiful terminal interface with no algorithmic feeds, no ads, just control and efficiency. This is an MVP focused on core functionality with a privacy-first, developer-centric approach.

## Core Principles

- **Speed First**: Lightning-fast, terminal-native UI optimized for developer workflows
- **Keyboard-Driven**: Every action accessible via keyboard shortcuts, no mouse required
- **Privacy-Focused**: No algorithms, no ads, no tracking - user control over their experience
- **Text-Only**: Markdown support for posts, no images or videos to maintain focus
- **Developer-Centric**: Built by developers, for developers, with developer workflows in mind

## Technology Stack

### Frontend
- **Rust with Ratatui**: Modern, robust TUI framework
- **tui-markdown**: Markdown parsing and rendering in terminal
- **tui-textarea**: Advanced multiline text input handling
- **clap**: CLI argument parsing for direct commands (e.g., `fido dm @user "msg"`)

### Backend
- **Axum**: High-performance, async REST API framework in Rust
- **SQLite with rusqlite**: Simple, file-based database for MVP
- **serde/serde_json**: JSON serialization/deserialization
- **oauth2**: GitHub OAuth authentication integration

### Testing
- **SQLite in-memory**: For unit and integration tests
- **Mock data**: For rapid UI development and iteration

## Architecture Guidelines

### Project Structure
- Use **Rust workspace** layout with separate crates:
  - Backend/API crate
  - Frontend/TUI crate  
  - Shared types crate (e.g., `fido-types`)

### Code Organization
- **Trait-based APIs**: Enable easy mocking and testing
- **Modular design**: Separate concerns for text-entry, Markdown rendering, networking, config
- **Clear documentation**: Document all API endpoints, data models, keyboard shortcuts

## API Design Standards

### Core Routes
- `POST /login/github` - GitHub OAuth authentication
- `GET /posts` - List global board posts (with sort options)
- `POST /posts` - Create new post (markdown body)
- `GET /dms` - List direct messages for user
- `POST /dms` - Send direct message
- `POST /vote` - Vote on post (single endpoint with direction param)
- `GET/PUT /config` - User configuration management

### Voting Implementation
Use single `/vote` endpoint with direction parameter:
```json
{ "post_id": "abc123", "direction": "up" }  // or "down"
```

## Database Schema

### Core Tables
- **users**: id (uuid), github_id, username
- **posts**: id (uuid), author_id, content (text), created_at, upvotes, downvotes
- **votes**: user_id, post_id, direction ('up'|'down') - PRIMARY KEY (user_id, post_id)
- **dms**: id (uuid), from_id, to_id, content, created_at

## Development Workflow

### Sequential Implementation Order
1. **App Skeleton & Mock Data** - Project structure, core types, UI with mock data
2. **Text Input & Markdown** - Integrate tui-textarea and tui-markdown
3. **API/Backend** - Axum REST endpoints, SQLite setup
4. **Config & Settings** - `.fido/` config file, API endpoints
5. **Authentication** - GitHub OAuth, local credential storage
6. **Frontend <-> Backend Integration** - HTTP client (reqwest), data parsing
7. **Testing & Polishing** - Comprehensive testing, documentation
8. **Future Preparation** - WebSocket planning, modular upgrades

### Configuration Management
- Store user preferences in `.fido/` directory
- Support color schemes and sorting preferences
- Local credential storage for GitHub authentication

## MVP Constraints

### What's Included
- GitHub login with local credential storage
- Global message board (single feed)
- Full Markdown support for posts
- Complete keyboard navigation
- Upvote/downvote system
- Direct messaging (dashboard + CLI)
- Configurable sorting and color schemes

### What's Excluded (Post-MVP)
- WebSocket real-time updates (unless trivial)
- External editor integration (`$EDITOR`)
- Advanced database scaling (Postgres migration planned)
- Image/video support
- Algorithmic feeds

## Testing Strategy

- **Mock-first development**: Use mock data for rapid UI iteration
- **SQLite in-memory**: For unit and integration tests
- **Comprehensive TUI testing**: Test all keyboard shortcuts and flows
- **CLI command testing**: Verify direct DM commands work correctly

## Code Quality Standards

- Follow Rust best practices and idioms
- Use `cargo clippy` for linting
- Maintain comprehensive documentation
- Write tests for core functionality
- Keep dependencies minimal and well-justified

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

## Future Considerations

- Plan for WebSocket integration (real-time updates)
- Design for easy database migration (SQLite → Postgres)
- Maintain modular architecture for feature additions
- Consider external editor integration for power users

#[[file:docs/fido_overview.md]]
#[[file:docs/fido_tech_specs.md]]
# Fido

A Rust based terminal social platform for developers reminicent of the BBS days

Built for the Kiroween 2025.

![Fido](assets/Screenshot%202025-12-07%20at%209.43.19 PM.png)

![Fido](assets/Screenshot%202025-12-07%20at%209.53.56 PM.png)

## What is it?

Fido is a social network that lives in your terminal. Think Twitter, but keyboard-driven and without the noise. Post updates, chat with other developers, upvote good content, downvote lame content.

## Live Demo Here
https://fido-social.fly.dev/


## Installation

### Option 1: Install from crates.io

First, make sure you have [Rust](https://rustup.rs/) installed

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Fido
cargo install fido
```

### Option 2: Build from source

```bash
git clone https://github.com/ianburke/fido.git
cd fido
cargo build --release
```

## Quick Start

Launch it:
```bash
fido
```

You'll see an auth screen. Login with GitHub (your browser will open) or pick a test user to try it out.

That's it. Press `?` for help, `Tab` to switch tabs, `n` to post, `q` to quit.

Your session saves to `~/.fido/session`. Press `Shift+L` to logout.

See [QUICKSTART.md](QUICKSTART.md) for more details.

## Features

- **Keyboard-driven** - `j/k` to navigate, `u/d` to vote, `n` to post
- **Direct messages** - Private conversations with other users
- **GitHub auth** - Login with your GitHub account
- **Customizable** - Themes, sorting, display preferences
- **Fast** - Terminal-native, no web bloat
- **Secure** - Comprehensive security hardening with input validation, audit logging, and session management

## Key Controls

- `Tab` - Switch tabs
- `j/k` or arrows - Navigate
- `u/d` - Upvote/Downvote
- `n` - New post
- `?` - Help
- `q` - Quit

## Development Scripts

### Web Terminal Demo

Launch the full web-based terminal demo (includes nginx, ttyd, and server):

```bash
./start.sh
```

This script:
- Builds the Rust binaries if needed
- Starts the Fido API server on port 3000
- Runs ttyd (terminal-over-web) on port 7681  
- Configures nginx as a reverse proxy on port 8080
- Provides a web interface at http://localhost:8080

The script works in both local development and Docker environments.

### Publishing to Crates.io

Publish new versions to crates.io with version checking:

```bash
./publish.sh
```

This script:
- Validates version consistency across workspace crates
- Checks if versions are already published
- Runs dry-run tests before publishing
- Publishes `fido-types` first, then `fido` (respects dependency order)
- Handles the 20-second wait for crates.io indexing

## Tech Stack

Built with Rust using a workspace architecture:

### Core Crates
- **fido-types** - Shared models and data structures
- **fido-tui** - Terminal UI client (main binary, uses Ratatui)
- **fido-server** - REST API server (Axum + SQLite)
- **fido-migrate** - Database migration utilities

### Technologies
- **Ratatui** - Terminal UI framework
- **Axum** - Fast, ergonomic web framework
- **SQLite** - Lightweight database with r2d2 connection pooling
- **tokio** - Async runtime
- **oauth2** - GitHub authentication
- **Security**: Input validation, audit logging, rate limiting, security headers
- Deployed on Fly.io with HTTPS enforcement

## Documentation

- [QUICKSTART.md](QUICKSTART.md) - Detailed getting started guide
- [ARCHITECTURE.md](ARCHITECTURE.md) - System design and technical details
- [CLAUDE.md](CLAUDE.md) - Development guidance for AI assistants

## Troubleshooting

**Session expired?** Press `Shift+L` to logout and login again.

**UI look weird?** Use a modern terminal with UTF-8 support (iTerm2, Alacritty, Ghostty).


## Development

### Local Development Setup

To run Fido locally for development:

```bash
# Clone the repository
git clone https://github.com/ianburke/fido.git
cd fido

# Set required environment variables
export GITHUB_CLIENT_ID=your_github_client_id_here

# Build the workspace
cargo build

# Start the server
cargo run --bin fido-server

# In another terminal, connect to it
cargo run --bin fido -- --server http://localhost:3000
```

### Required Environment Variables

The following environment variables must be set before starting the server:

#### Required Variables

- **GITHUB_CLIENT_ID** (required): GitHub OAuth application client ID
  - Register an OAuth app at: https://github.com/settings/developers
  - Note: Device Flow doesn't require a callback URL or client secret
  - The server will fail to start in production if this is not set

#### Optional Environment Variables

- **ENVIRONMENT** or **RUST_ENV**: Application environment (`development` or `production`, default: `development`)
  - Controls security features like CORS origins, HTTPS enforcement, and configuration validation
  - Production mode enforces stricter security requirements
  
- **HOST**: Server bind address (default: `0.0.0.0`)
- **PORT**: Server port (default: `3000`)
- **DATABASE_PATH**: SQLite database file path (default: `fido.db`)
- **FIDO_SERVER_URL**: Server URL for TUI client (default: `http://localhost:3000`)

#### Security Configuration Variables

- **MAX_REQUEST_SIZE**: Maximum request body size in bytes (default: `1048576` = 1MB)
  - Protects against denial-of-service attacks via large payloads
  
- **SESSION_MAX_AGE_DAYS**: Maximum session lifetime in days (default: `7`)
  - Sessions expire after this duration regardless of activity
  
- **SESSION_IDLE_TIMEOUT_HOURS**: Session idle timeout in hours (default: `24`)
  - Sessions expire after this period of inactivity

### Testing

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p fido-server
cargo test -p fido-tui
cargo test -p fido-types

# Format code
cargo fmt

# Check for linting issues
cargo clippy
```

### Web Demo

For the full web terminal experience (see Development Scripts above):

```bash
./start.sh
```

## Security Features

Fido implements comprehensive security hardening to protect user data and prevent common web vulnerabilities:

### Authentication & Session Management

- **GitHub OAuth Integration**: Secure authentication using GitHub's Device Flow
  - No client secrets required
  - User-friendly device code flow for terminal applications
  
- **Session Security**:
  - 7-day maximum session lifetime (configurable)
  - 24-hour idle timeout (configurable)
  - Automatic session invalidation on new login
  - Session tokens stored securely in `~/.fido/session`
  - All sessions invalidated when user logs out

### Input Validation & Sanitization

All user input is validated before processing to prevent injection attacks:

- **Username Validation**:
  - 3-32 characters
  - Alphanumeric, underscores, and hyphens only
  - Must start with a letter or number
  
- **Post Content Validation**:
  - Maximum 280 characters
  - XSS pattern detection and prevention
  - HTML entity escaping
  
- **Bio Validation**:
  - Maximum 160 characters
  - XSS pattern detection
  
- **Hashtag Validation**:
  - 1-50 characters
  - Alphanumeric and underscores only

### Request Protection

- **Body Size Limits**: 1MB maximum request size (configurable)
  - Prevents denial-of-service attacks via large payloads
  
- **Rate Limiting**: Per-user request rate limiting
  - Protects against abuse and brute-force attacks
  
- **SQL Injection Prevention**:
  - All database queries use parameterized statements
  - Enum-based sort order validation
  - No dynamic SQL construction from user input

### Security Headers

All HTTP responses include security headers for defense-in-depth:

- `X-Content-Type-Options: nosniff` - Prevents MIME type sniffing
- `X-Frame-Options: DENY` - Prevents clickjacking attacks
- `X-XSS-Protection: 1; mode=block` - Enables browser XSS protection
- `Content-Security-Policy` - Restricts resource loading
- `Referrer-Policy: strict-origin-when-cross-origin` - Controls referrer information
- `Strict-Transport-Security` (production only) - Enforces HTTPS connections

### CORS Configuration

Environment-aware Cross-Origin Resource Sharing:

- **Production**: Only `https://fido-social.fly.dev` allowed
- **Development**: Localhost origins allowed for local testing
- **CLI/TUI Clients**: Requests without Origin header always allowed

### Error Handling

- **Sanitized Error Messages**: Internal errors return generic messages to clients
  - Database errors don't expose schema details
  - Stack traces never included in API responses
  - Detailed errors logged server-side for debugging
  
- **Validation Errors**: Specific validation feedback without internal details

### Audit Logging

Comprehensive security event logging for monitoring and investigation:

- Login attempts (success and failure)
- Session creation and revocation
- Admin actions
- Rate limit violations
- Validation failures
- Suspicious activity detection

Audit logs include:
- Event type and timestamp
- User ID (when applicable)
- Client IP address
- User-Agent header
- Event-specific details

### Admin Authorization

- **Admin-Only Endpoints**: Protected by middleware requiring admin privileges
- **Admin Flag**: Database-backed `is_admin` flag on user accounts
- **Audit Trail**: All admin actions logged for accountability

### HTTPS & Cookie Security

- **HTTPS Enforcement**: Automatic HTTPS redirect in production (via Fly.io)
- **Secure Cookies**: Session cookies marked as `Secure` when HTTPS detected
- **Cookie Attributes**: HttpOnly, SameSite=Lax for CSRF protection

### Configuration Validation

- **Startup Validation**: Server validates security configuration on startup
- **Production Requirements**: Strict validation in production mode
  - GITHUB_CLIENT_ID must be set
  - Valid session timeouts required
  - Allowed origins must be configured
  
- **Admin Endpoint**: `/admin/config/validate` returns configuration status
  - Requires admin authentication
  - Shows security settings without exposing secrets

## Admin User Setup

To grant admin privileges to a user:

1. **Using SQLite CLI**:
   ```bash
   sqlite3 fido.db
   UPDATE users SET is_admin = 1 WHERE username = 'your_username';
   .quit
   ```

2. **Using Test Data** (development only):
   ```bash
   # The test user 'alice' is automatically set as admin
   cargo run --bin fido-server
   # Login as alice in the TUI
   ```

3. **Verify Admin Status**:
   ```bash
   sqlite3 fido.db "SELECT username, is_admin FROM users WHERE is_admin = 1;"
   ```

Admin users can:
- Access `/admin/cleanup` endpoint to clean up expired sessions
- Access `/admin/config/validate` endpoint to view security configuration
- Perform other administrative operations (as features are added)

## Security Best Practices

When deploying Fido in production:

1. **Environment Variables**:
   - Set `ENVIRONMENT=production` or `RUST_ENV=production`
   - Always set `GITHUB_CLIENT_ID` from a secure source
   - Never commit secrets to version control
   
2. **HTTPS**:
   - Always use HTTPS in production (enforced on Fly.io)
   - Ensure `Strict-Transport-Security` header is enabled
   
3. **Database**:
   - Regularly backup your SQLite database
   - Set appropriate file permissions on `fido.db` (600 or 640)
   - Consider migrating to PostgreSQL for larger deployments
   
4. **Monitoring**:
   - Review audit logs regularly for suspicious activity
   - Monitor rate limit violations
   - Track failed login attempts
   
5. **Session Management**:
   - Use default session timeouts (7 days max, 24 hours idle)
   - Shorter timeouts for high-security environments
   - Educate users to logout when done
   
6. **Admin Accounts**:
   - Limit the number of admin users
   - Use strong GitHub accounts for admin users
   - Audit admin actions regularly
   
7. **Updates**:
   - Keep Rust and dependencies up to date
   - Monitor security advisories for dependencies
   - Test updates in development before production deployment

## License

MIT


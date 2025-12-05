# Production Deployment Design Document

## Overview

This design document outlines the architecture and implementation strategy for transforming Fido from a local development prototype into a production-ready, publicly accessible social platform. The system will enable developers to install the TUI client via Cargo and immediately connect to a live server with GitHub OAuth authentication and persistent sessions.

## Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Developer's Machine                       │
│                                                               │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Fido TUI Client (installed via cargo install fido)  │  │
│  │  - Hardcoded server URL: https://fido-social.fly.dev │  │
│  │  - Session token stored in ~/.fido/session           │  │
│  │  - Opens browser for GitHub OAuth                    │  │
│  └────────────────┬─────────────────────────────────────┘  │
│                   │                                          │
└───────────────────┼──────────────────────────────────────────┘
                    │ HTTPS
                    │
┌───────────────────▼──────────────────────────────────────────┐
│                    Fly.io Cloud                               │
│                                                               │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Fido API Server (fido-server)                       │  │
│  │  - GitHub OAuth endpoints                            │  │
│  │  - Session management                                │  │
│  │  - REST API for posts, DMs, profiles                │  │
│  │  - Environment: GITHUB_CLIENT_ID, GITHUB_CLIENT_SECRET │
│  └────────────────┬─────────────────────────────────────┘  │
│                   │                                          │
│  ┌────────────────▼─────────────────────────────────────┐  │
│  │  SQLite Database (/data/fido.db)                     │  │
│  │  - Persistent volume                                 │  │
│  │  - Users, posts, DMs, sessions                       │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                               │
└───────────────────────────────────────────────────────────────┘
                    │
                    │ OAuth redirect
                    ▼
┌───────────────────────────────────────────────────────────────┐
│                    GitHub OAuth                               │
│  - User authorizes Fido app                                   │
│  - Returns authorization code                                 │
└───────────────────────────────────────────────────────────────┘
```

### Component Interaction Flow

1. **Installation**: User runs `cargo install fido`
2. **First Launch**: TUI detects no session, prompts for GitHub auth
3. **OAuth Flow**: TUI opens browser → GitHub → callback to server
4. **Session Creation**: Server creates session token, returns to TUI
5. **Session Storage**: TUI stores token in `~/.fido/session`
6. **Subsequent Launches**: TUI loads token, validates with server
7. **API Requests**: All requests include session token in `X-Session-Token` header

## Components and Interfaces

### 1. GitHub OAuth Integration

#### Server-Side OAuth Handler

**New Module**: `fido-server/src/oauth.rs`

```rust
pub struct GitHubOAuthConfig {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

pub struct GitHubUser {
    pub id: i64,
    pub login: String,
    pub name: Option<String>,
    pub email: Option<String>,
}

impl GitHubOAuthConfig {
    pub fn from_env() -> Result<Self>;
    pub async fn exchange_code(&self, code: String) -> Result<String>; // Returns access token
    pub async fn get_user(&self, access_token: String) -> Result<GitHubUser>;
}
```

**New API Endpoints**:
- `GET /auth/github/login` - Initiates OAuth flow, returns authorization URL
- `GET /auth/github/callback?code=xxx` - Handles GitHub callback, creates session
- `GET /auth/validate` - Validates session token
- `POST /auth/logout` - Invalidates session token

#### Database Schema Changes

**New Table**: `sessions`
```sql
CREATE TABLE sessions (
    token TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_expires_at ON sessions(expires_at);
```

**Update `users` Table**:
```sql
ALTER TABLE users ADD COLUMN github_id INTEGER UNIQUE;
ALTER TABLE users ADD COLUMN github_login TEXT;
```

### 2. Session Management

#### Server-Side Session Store

**New Module**: `fido-server/src/session.rs`

```rust
pub struct SessionManager {
    db: Arc<Database>,
}

impl SessionManager {
    pub fn create_session(&self, user_id: Uuid) -> Result<String>; // Returns session token
    pub fn validate_session(&self, token: &str) -> Result<Uuid>; // Returns user_id
    pub fn delete_session(&self, token: &str) -> Result<()>;
    pub fn cleanup_expired_sessions(&self) -> Result<usize>; // Returns count deleted
}
```

**Session Token Format**: UUID v4 (cryptographically secure)
**Session Expiry**: 30 days from creation

#### Client-Side Session Store

**New Module**: `fido-tui/src/session.rs`

```rust
pub struct SessionStore {
    file_path: PathBuf, // ~/.fido/session
}

impl SessionStore {
    pub fn new() -> Result<Self>;
    pub fn load(&self) -> Result<Option<String>>; // Returns session token
    pub fn save(&self, token: &str) -> Result<()>;
    pub fn delete(&self) -> Result<()>;
}
```

**File Format**: Plain text, single line containing session token
**File Permissions**: 0600 (read/write for owner only)

### 3. TUI Client Configuration

#### Server URL Configuration

**Update**: `fido-tui/src/api/client.rs`

```rust
impl ApiClient {
    pub fn new_with_config() -> Self {
        let base_url = Self::determine_server_url();
        Self::new(base_url)
    }
    
    fn determine_server_url() -> String {
        // Priority: CLI arg > env var > default
        std::env::var("FIDO_SERVER_URL")
            .unwrap_or_else(|_| "https://fido-social.fly.dev".to_string())
    }
}
```

**CLI Arguments**: Add `--server <URL>` flag using clap

#### OAuth Flow in TUI

**New Module**: `fido-tui/src/auth.rs`

```rust
pub struct AuthFlow {
    api_client: ApiClient,
    session_store: SessionStore,
}

impl AuthFlow {
    pub async fn authenticate(&mut self) -> Result<User>;
    pub async fn check_existing_session(&self) -> Result<Option<User>>;
    pub async fn initiate_github_oauth(&self) -> Result<String>; // Returns auth URL
    pub async fn poll_for_session(&self, state: &str) -> Result<String>; // Returns session token
    pub fn open_browser(&self, url: &str) -> Result<()>;
}
```

**OAuth Flow Steps**:
1. TUI calls `GET /auth/github/login` to get authorization URL with state parameter
2. TUI opens browser to authorization URL
3. User authorizes on GitHub
4. GitHub redirects to server callback
5. Server creates session, stores state → token mapping temporarily
6. TUI polls `GET /auth/session?state=xxx` until token is available
7. TUI saves token to `~/.fido/session`

### 4. Cargo Crate Publishing

#### Package Metadata Updates

**Update**: `fido/Cargo.toml` (workspace)
```toml
[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["Fido Contributors"]
license = "MIT"
repository = "https://github.com/yourusername/fido"
homepage = "https://github.com/yourusername/fido"
```

**Update**: `fido-tui/Cargo.toml`
```toml
[package]
name = "fido"  # Changed from fido-tui
description = "A blazing-fast, keyboard-driven social platform for developers"
keywords = ["tui", "social", "terminal", "ratatui"]
categories = ["command-line-utilities"]

[dependencies]
fido-types = { version = "0.1.0", path = "../fido-types" }  # For local dev
# fido-types = "0.1.0"  # For published version
```

**Update**: `fido-types/Cargo.toml`
```toml
[package]
name = "fido-types"
description = "Shared types for the Fido social platform"
```

#### Publishing Order
1. `cargo publish -p fido-types`
2. Update `fido-tui/Cargo.toml` to use published `fido-types`
3. `cargo publish -p fido` (the TUI)

### 5. Fly.io Deployment

#### Dockerfile

**Create**: `fido/Dockerfile`
```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin fido-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/fido-server /usr/local/bin/
EXPOSE 3000
CMD ["fido-server"]
```

#### Fly.io Configuration

**Update**: `fly.toml`
```toml
app = 'fido-social'
primary_region = 'ord'

[build]
  dockerfile = "Dockerfile"

[env]
  DATABASE_PATH = '/data/fido.db'
  RUST_LOG = 'info'
  HOST = '0.0.0.0'
  PORT = '3000'

[[mounts]]
  source = 'fido_data'
  destination = '/data'

[http_service]
  internal_port = 3000
  force_https = true
  auto_stop_machines = true
  auto_start_machines = true
  min_machines_running = 0

[[services]]
  protocol = 'tcp'
  internal_port = 3000
  
  [[services.ports]]
    port = 80
    handlers = ['http']
  
  [[services.ports]]
    port = 443
    handlers = ['tls', 'http']
```

**Secrets to Set**:
```bash
flyctl secrets set GITHUB_CLIENT_ID=xxx
flyctl secrets set GITHUB_CLIENT_SECRET=yyy
```

### 6. CI/CD Pipeline

#### GitHub Actions Workflow

**Create**: `.github/workflows/deploy.yml`
```yaml
name: Deploy

on:
  push:
    branches: [main]
  release:
    types: [published]

jobs:
  deploy-server:
    runs-on: ubuntu-latest
    if: github.event_name == 'push'
    steps:
      - uses: actions/checkout@v3
      - uses: superfly/flyctl-actions/setup-flyctl@master
      - run: flyctl deploy --remote-only
        env:
          FLY_API_TOKEN: ${{ secrets.FLY_API_TOKEN }}
  
  publish-crate:
    runs-on: ubuntu-latest
    if: github.event_name == 'release'
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Publish fido-types
        run: cargo publish -p fido-types --token ${{ secrets.CARGO_TOKEN }}
      - name: Wait for fido-types to be available
        run: sleep 30
      - name: Update fido dependency
        run: |
          cd fido-tui
          sed -i 's|path = "../fido-types"|version = "0.1.0"|' Cargo.toml
      - name: Publish fido
        run: cargo publish -p fido --token ${{ secrets.CARGO_TOKEN }}
```

## Data Models

### Session Model

```rust
#[derive(Debug, Clone)]
pub struct Session {
    pub token: String,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
```

### GitHub User Model

```rust
#[derive(Debug, Deserialize)]
pub struct GitHubUser {
    pub id: i64,
    pub login: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
}
```

### OAuth State Model

```rust
// Temporary storage for OAuth state during flow
pub struct OAuthState {
    pub state: String,
    pub session_token: Option<String>,
    pub created_at: DateTime<Utc>,
}
```

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system-essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property 1: Session token uniqueness
*For any* two sessions created by the system, their session tokens must be unique (no collisions)
**Validates: Requirements 4.4**

### Property 2: Session validation correctness
*For any* valid session token stored in the database, validating that token must return the correct user_id
**Validates: Requirements 4.5**

### Property 3: Session expiry enforcement
*For any* session token with an expiry time in the past, validation must fail and return an authentication error
**Validates: Requirements 3.3**

### Property 4: OAuth code exchange idempotency
*For any* GitHub authorization code, exchanging it multiple times must either succeed once and fail subsequently, or fail consistently (no duplicate sessions)
**Validates: Requirements 4.1, 4.2**

### Property 5: Session storage file permissions
*For any* session file created by the TUI, the file permissions must be 0600 (read/write for owner only)
**Validates: Requirements 3.4**

### Property 6: Server URL configuration precedence
*For any* TUI launch, if both CLI argument and environment variable are set, the CLI argument must take precedence over the environment variable
**Validates: Requirements 8.1, 8.2**

### Property 7: GitHub user creation idempotency
*For any* GitHub user authenticating multiple times, only one user record must exist in the database (no duplicate users)
**Validates: Requirements 4.3**

### Property 8: Session cleanup correctness
*For any* expired session in the database, running cleanup must remove it and leave non-expired sessions intact
**Validates: Requirements 3.3**

### Property 9: OAuth state parameter validation
*For any* OAuth callback, the state parameter must match a recently generated state value (within 10 minutes)
**Validates: Requirements 7.1**

### Property 10: API authentication requirement
*For any* protected API endpoint, requests without a valid session token must return 401 Unauthorized
**Validates: Requirements 4.5**

## Error Handling

### OAuth Errors

**GitHub API Errors**:
- Network timeout → Retry with exponential backoff (3 attempts)
- Invalid code → Display "Authentication failed, please try again"
- Rate limit → Display "GitHub rate limit exceeded, please wait"

**Browser Opening Errors**:
- Cannot open browser → Display manual URL with instructions
- User closes browser → Timeout after 5 minutes, allow retry

### Session Errors

**Invalid Session**:
- Expired token → Clear local session, prompt for re-authentication
- Token not found → Clear local session, prompt for re-authentication
- Malformed token → Clear local session, prompt for re-authentication

**File System Errors**:
- Cannot create `~/.fido/` directory → Display error with permissions help
- Cannot write session file → Display error, continue without persistence
- Cannot read session file → Treat as no session, prompt for authentication

### Server Connection Errors

**Network Errors**:
- Server unreachable → Display "Cannot connect to server, check your internet connection"
- Timeout → Retry with exponential backoff (3 attempts)
- SSL/TLS errors → Display "Secure connection failed, check your system time"

**Server Errors**:
- 500 Internal Server Error → Display "Server error, please try again later"
- 503 Service Unavailable → Display "Server is temporarily unavailable"

## Testing Strategy

### Unit Tests

**OAuth Module Tests**:
- Test GitHub API response parsing
- Test token exchange with mock responses
- Test user profile fetching with mock responses
- Test error handling for various GitHub API errors

**Session Management Tests**:
- Test session creation generates unique tokens
- Test session validation with valid/invalid tokens
- Test session expiry logic
- Test session cleanup removes only expired sessions

**Session Store Tests**:
- Test file creation with correct permissions
- Test token save and load round-trip
- Test handling of missing/corrupted files
- Test directory creation when `~/.fido/` doesn't exist

**Configuration Tests**:
- Test server URL precedence (CLI > env > default)
- Test environment variable parsing
- Test invalid URL handling

### Integration Tests

**OAuth Flow Tests**:
- Test complete OAuth flow with mock GitHub server
- Test state parameter generation and validation
- Test duplicate code exchange handling
- Test session creation after successful OAuth

**API Authentication Tests**:
- Test protected endpoints require valid session
- Test invalid session returns 401
- Test expired session returns 401
- Test session validation endpoint

**End-to-End Tests**:
- Test fresh install → OAuth → session storage → API call
- Test session persistence across TUI restarts
- Test session expiry and re-authentication
- Test logout clears session

### Property-Based Tests

**Property 1: Session Token Uniqueness**
- Generate N sessions, verify all tokens are unique
- **Validates: Property 1**

**Property 2: Session Validation Round-Trip**
- Create session for random user, validate token returns same user
- **Validates: Property 2**

**Property 3: Expired Session Rejection**
- Create session with past expiry, validation must fail
- **Validates: Property 3**

**Property 4: File Permissions**
- Create session file, verify permissions are 0600
- **Validates: Property 5**

**Property 5: URL Configuration Precedence**
- Set random URLs in CLI/env/default, verify correct precedence
- **Validates: Property 6**

**Property 6: Session Cleanup Correctness**
- Create mix of expired/valid sessions, cleanup removes only expired
- **Validates: Property 8**

### Manual Testing Checklist

- [ ] Install from crates.io: `cargo install fido`
- [ ] First launch prompts for GitHub auth
- [ ] Browser opens to GitHub OAuth page
- [ ] After authorization, TUI shows logged-in state
- [ ] Session persists after TUI restart
- [ ] Can create posts, send DMs, etc.
- [ ] Logout clears session
- [ ] Re-authentication works after logout
- [ ] Server URL override works: `fido --server http://localhost:3000`
- [ ] Environment variable override works: `FIDO_SERVER_URL=http://localhost:3000 fido`

## Deployment Checklist

### Pre-Deployment

- [ ] Register GitHub OAuth app, get client ID/secret
- [ ] Create Fly.io app: `flyctl launch`
- [ ] Create persistent volume: `flyctl volumes create fido_data --size 1`
- [ ] Set Fly secrets: `flyctl secrets set GITHUB_CLIENT_ID=xxx GITHUB_CLIENT_SECRET=yyy`
- [ ] Update OAuth callback URL in GitHub app settings to `https://fido-social.fly.dev/auth/github/callback`

### Deployment

- [ ] Deploy server: `flyctl deploy`
- [ ] Verify server health: `curl https://fido-social.fly.dev/health`
- [ ] Test OAuth flow manually
- [ ] Publish fido-types: `cargo publish -p fido-types`
- [ ] Update fido-tui dependency to use published fido-types
- [ ] Publish fido: `cargo publish -p fido`
- [ ] Test installation: `cargo install fido`
- [ ] Test end-to-end flow

### Post-Deployment

- [ ] Set up GitHub Actions for auto-deployment
- [ ] Add FLY_API_TOKEN to GitHub secrets
- [ ] Add CARGO_TOKEN to GitHub secrets
- [ ] Test CI/CD pipeline with a test commit
- [ ] Monitor Fly.io logs for errors
- [ ] Update README with installation instructions

## Repository Migration Checklist

### Preparation

- [ ] Create new GitHub repository named `fido`
- [ ] Clone current repository to temporary location
- [ ] Identify markdown files to keep vs. remove

### Files to Keep

- [ ] README.md
- [ ] QUICKSTART.md
- [ ] ARCHITECTURE.md
- [ ] DEPLOYMENT.md
- [ ] LOGGING.md
- [ ] TESTING_GUIDE.md
- [ ] LICENSE
- [ ] .gitignore

### Files to Remove

- [ ] All FIXES_*.md files
- [ ] BUGFIXES.md
- [ ] ISSUES_FIXED.md
- [ ] Implementation notes (HASHTAG_*.md, DM_*.md, etc.)
- [ ] Test SQL files
- [ ] Debug logs
- [ ] Old migration guides

### Migration Steps

1. [ ] Copy `fido/` directory contents to new repo root
2. [ ] Copy `.kiro/` directory to new repo root
3. [ ] Copy essential markdown files to new repo root
4. [ ] Update all relative paths in documentation
5. [ ] Update Cargo.toml repository URLs
6. [ ] Create initial commit: "Initial commit - Fido v0.1.0"
7. [ ] Push to GitHub
8. [ ] Verify all files are present
9. [ ] Test local build: `cargo build --release`
10. [ ] Test local run: `cargo run --bin fido-server`

## Security Considerations

### Session Token Security

- Use UUID v4 for cryptographically secure random tokens
- Store tokens with 0600 permissions on client
- Use HTTPS for all API communication
- Implement session expiry (30 days)
- Clear sessions on logout

### OAuth Security

- Validate state parameter to prevent CSRF
- Use short-lived authorization codes
- Store client secret only on server (never in TUI)
- Validate redirect URI matches registered callback

### API Security

- Require authentication for all protected endpoints
- Validate session token on every request
- Rate limit authentication endpoints
- Log authentication failures for monitoring

## Performance Considerations

### Session Validation

- Cache session validation results for 5 minutes
- Use database index on session token for fast lookups
- Implement session cleanup as background task (runs hourly)

### OAuth Flow

- Implement timeout for OAuth polling (5 minutes)
- Use exponential backoff for GitHub API retries
- Cache GitHub user profile for session duration

### Database

- Use connection pooling (already implemented with r2d2)
- Create indexes on frequently queried columns
- Implement database migrations for schema changes

## Monitoring and Logging

### Server Logs

- Log all authentication attempts (success/failure)
- Log session creation/validation/deletion
- Log OAuth errors with sanitized details (no secrets)
- Log database errors

### Client Logs

- Log authentication flow steps (optional, user-controlled)
- Log API errors with request context
- Log session file operations (optional)

### Metrics to Track

- Authentication success/failure rate
- Session creation rate
- Active sessions count
- API request rate by endpoint
- Error rate by type

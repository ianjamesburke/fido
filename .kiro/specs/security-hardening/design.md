# Design Document: Security Hardening

## Overview

This design implements comprehensive security hardening for the Fido social platform based on the security audit findings. The implementation focuses on six key areas:

1. **Device Code Persistence** - Moving OAuth device codes from in-memory storage to SQLite database
2. **Session Management** - Reducing session expiry, implementing refresh tokens, session binding, and concurrent limits
3. **Request Protection** - Adding body size limits to prevent DoS attacks
4. **Security Headers** - Adding defense-in-depth HTTP headers
5. **Error Sanitization** - Preventing information leakage through error messages
6. **Audit Logging** - Recording security events for monitoring and investigation

The design maintains backward compatibility with the existing TUI client while significantly improving the security posture of the platform.

## Architecture

### Security Component Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Client Layer                              │
│              (TUI Client, CLI Client, Web Terminal)              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Fly.io Edge (HTTPS)                           │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Middleware Stack                               │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  1. Request Body Limit (1MB)                              │  │
│  │  2. Security Headers Middleware                           │  │
│  │  3. Rate Limiting Middleware (existing)                   │  │
│  │  4. CORS Middleware (existing)                            │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Route Handlers                               │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  Auth Endpoints:                                          │  │
│  │  - POST /auth/github/device (device code generation)      │  │
│  │  - POST /auth/github/device/poll (device code polling)    │  │
│  │  - POST /auth/refresh (token refresh)                     │  │
│  │  - POST /auth/logout                                      │  │
│  │  - GET /auth/validate                                     │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Security Services                             │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │ DeviceCodeStore │  │ SessionManager  │  │  AuditLogger    │  │
│  │ (DB-backed)     │  │ (Enhanced)      │  │                 │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    SQLite Database                               │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │  device_codes   │  │    sessions     │  │   audit_logs    │  │
│  │  (new table)    │  │  (enhanced)     │  │  (new table)    │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### Token Flow Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                    Authentication Flow                            │
│                                                                   │
│  1. Device Flow Login:                                           │
│     Client ──► POST /auth/github/device                          │
│            ◄── {device_code, user_code, verification_uri}        │
│                                                                   │
│  2. Poll for Authorization:                                      │
│     Client ──► POST /auth/github/device/poll {device_code}       │
│            ◄── {access_token, refresh_token, user}               │
│                                                                   │
│  3. API Requests (with access_token):                            │
│     Client ──► GET /posts (X-Session-Token: access_token)        │
│            ◄── {posts: [...]}                                    │
│                                                                   │
│  4. Token Refresh (when access_token expires):                   │
│     Client ──► POST /auth/refresh {refresh_token}                │
│            ◄── {access_token, refresh_token} (rotated)           │
│                                                                   │
└──────────────────────────────────────────────────────────────────┘
```

## Components and Interfaces

### 1. DeviceCodeStore

**Purpose:** Persist OAuth device codes in SQLite instead of in-memory HashMap.

**Location:** `fido-server/src/security/device_codes.rs`

**Interface:**
```rust
pub struct DeviceCodeStore {
    db: Database,
}

impl DeviceCodeStore {
    pub fn new(db: Database) -> Self;
    
    /// Store a device code with 15-minute TTL
    pub fn store(&self, device_code: &str, user_code: &str, 
                 verification_uri: &str, expires_in: i64, 
                 interval: i64) -> Result<()>;
    
    /// Retrieve a device code if it exists and hasn't expired
    pub fn get(&self, device_code: &str) -> Result<Option<StoredDeviceCode>>;
    
    /// Delete a device code (after successful use or expiry)
    pub fn delete(&self, device_code: &str) -> Result<()>;
    
    /// Clean up all expired device codes, returns count deleted
    pub fn cleanup_expired(&self) -> Result<usize>;
}

pub struct StoredDeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_at: DateTime<Utc>,
    pub interval: i64,
}
```

### 2. Enhanced SessionManager

**Purpose:** Manage sessions with refresh tokens, binding, and concurrent limits.

**Location:** `fido-server/src/session.rs` (enhanced)

**Interface:**
```rust
pub struct SessionManager {
    db: Database,
}

/// Session with access and refresh tokens
pub struct SessionTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub access_expires_at: DateTime<Utc>,
    pub refresh_expires_at: DateTime<Utc>,
}

impl SessionManager {
    /// Create a new session with access and refresh tokens
    /// Enforces max 5 concurrent sessions per user
    pub fn create_session(&self, user_id: Uuid, ip_address: Option<&str>, 
                          user_agent: Option<&str>) -> Result<SessionTokens>;
    
    /// Validate an access token, returns user_id if valid
    /// Logs warning if IP/User-Agent differs from stored values
    pub fn validate_access_token(&self, token: &str, 
                                  current_ip: Option<&str>,
                                  current_user_agent: Option<&str>) -> Result<Uuid>;
    
    /// Refresh tokens using a valid refresh token
    /// Rotates the refresh token (old one invalidated)
    pub fn refresh_tokens(&self, refresh_token: &str) -> Result<SessionTokens>;
    
    /// Delete a session (logout)
    pub fn delete_session(&self, access_token: &str) -> Result<()>;
    
    /// Delete all sessions for a user
    pub fn delete_user_sessions(&self, user_id: Uuid) -> Result<usize>;
    
    /// Clean up expired sessions
    pub fn cleanup_expired_sessions(&self) -> Result<usize>;
    
    /// Count active sessions for a user
    fn count_user_sessions(&self, user_id: Uuid) -> Result<usize>;
    
    /// Invalidate oldest session if user has >= 5 sessions
    fn enforce_session_limit(&self, user_id: Uuid) -> Result<()>;
}
```

### 3. AuditLogger

**Purpose:** Record security-relevant events for monitoring.

**Location:** `fido-server/src/security/audit.rs`

**Interface:**
```rust
pub struct AuditLogger {
    db: Database,
}

#[derive(Debug, Clone)]
pub enum AuditEventType {
    LoginSuccess,
    LoginFailure,
    SessionCreated,
    SessionRevoked,
    SessionRefreshed,
    SuspiciousActivity,
    DeviceCodeGenerated,
    DeviceCodeUsed,
}

pub struct AuditEvent {
    pub event_type: AuditEventType,
    pub user_id: Option<Uuid>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub details: Option<String>,
}

impl AuditLogger {
    pub fn new(db: Database) -> Self;
    
    /// Log a security event
    pub fn log(&self, event: AuditEvent) -> Result<()>;
    
    /// Get recent audit logs (for admin viewing)
    pub fn get_recent(&self, limit: usize) -> Result<Vec<AuditLogEntry>>;
}

pub struct AuditLogEntry {
    pub id: Uuid,
    pub event_type: String,
    pub user_id: Option<Uuid>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub details: Option<String>,
    pub timestamp: DateTime<Utc>,
}
```

### 4. SecurityHeadersMiddleware

**Purpose:** Add security headers to all HTTP responses.

**Location:** `fido-server/src/security/headers.rs`

**Interface:**
```rust
/// Middleware that adds security headers to all responses
pub async fn security_headers_middleware(
    State(config): State<SecurityConfig>,
    request: Request,
    next: Next,
) -> Response;
```

**Headers Added:**
- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- `X-XSS-Protection: 1; mode=block`
- `Strict-Transport-Security: max-age=31536000; includeSubDomains` (production only)

### 5. SecureErrorHandler

**Purpose:** Sanitize error messages before returning to clients.

**Location:** `fido-server/src/api/error.rs` (enhanced)

**Interface:**
```rust
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message, details) = match self {
            // ... existing cases ...
            ApiError::InternalError(msg) => {
                // Log the actual error server-side
                tracing::error!("Internal error: {}", msg);
                // Return generic message to client
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error",
                    Some("An unexpected error occurred".to_string()),
                )
            }
        };
        // ...
    }
}
```

## Data Models

### Device Codes Table (New)

```sql
CREATE TABLE device_codes (
    device_code TEXT PRIMARY KEY,
    user_code TEXT NOT NULL,
    verification_uri TEXT NOT NULL,
    interval INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL
);

CREATE INDEX idx_device_codes_expires_at ON device_codes(expires_at);
```

### Sessions Table (Enhanced)

```sql
-- Add new columns to existing sessions table
ALTER TABLE sessions ADD COLUMN refresh_token TEXT;
ALTER TABLE sessions ADD COLUMN access_expires_at TEXT;
ALTER TABLE sessions ADD COLUMN ip_address TEXT;
ALTER TABLE sessions ADD COLUMN user_agent TEXT;

-- Create index for refresh token lookups
CREATE INDEX idx_sessions_refresh_token ON sessions(refresh_token);
CREATE INDEX idx_sessions_user_id ON sessions(user_id);
```

**Updated Sessions Schema:**
```sql
CREATE TABLE sessions (
    token TEXT PRIMARY KEY,           -- access token
    refresh_token TEXT UNIQUE,        -- refresh token (new)
    user_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,         -- refresh token expiry (7 days)
    access_expires_at TEXT NOT NULL,  -- access token expiry (15 min) (new)
    ip_address TEXT,                  -- client IP (new)
    user_agent TEXT,                  -- client User-Agent (new)
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
```

### Audit Logs Table (New)

```sql
CREATE TABLE audit_logs (
    id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    user_id TEXT,
    ip_address TEXT,
    user_agent TEXT,
    details TEXT,
    timestamp TEXT NOT NULL
);

CREATE INDEX idx_audit_logs_timestamp ON audit_logs(timestamp DESC);
CREATE INDEX idx_audit_logs_user_id ON audit_logs(user_id);
CREATE INDEX idx_audit_logs_event_type ON audit_logs(event_type);
```

### Data Types

```rust
/// Stored device code from database
pub struct StoredDeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval: i64,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Session tokens returned to client
pub struct SessionTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub access_expires_at: DateTime<Utc>,
    pub refresh_expires_at: DateTime<Utc>,
}

/// Session stored in database
pub struct StoredSession {
    pub token: String,              // access token
    pub refresh_token: String,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,  // refresh token expiry
    pub access_expires_at: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

/// Audit log entry
pub struct AuditLogEntry {
    pub id: Uuid,
    pub event_type: String,
    pub user_id: Option<Uuid>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub details: Option<String>,
    pub timestamp: DateTime<Utc>,
}
```


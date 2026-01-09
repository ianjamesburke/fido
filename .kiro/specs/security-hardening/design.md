# Design Document: Security Hardening

## Overview

This design implements comprehensive security hardening for the Fido social platform, addressing critical vulnerabilities identified in the security audit. The implementation focuses on production-ready security controls while maintaining compatibility with CLI, TUI, and web terminal clients.

The design follows a defense-in-depth approach with multiple layers of security:
- Network layer: CORS configuration and HTTPS enforcement
- Application layer: Input validation, authentication, and authorization
- Data layer: SQL injection prevention and secure error handling
- Operational layer: Audit logging and security monitoring

## Architecture

### Security Layers

```
┌─────────────────────────────────────────────────────────┐
│                    Client Layer                          │
│  (TUI Client, CLI Client, Web Terminal)                 │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│              Fly.io Edge (HTTPS Termination)            │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│                 Security Middleware Stack                │
│  ┌────────────────────────────────────────────────────┐ │
│  │  1. Security Headers Middleware                    │ │
│  │  2. CORS Middleware                                │ │
│  │  3. Rate Limiting Middleware                       │ │
│  │  4. Request Size Limit Middleware                  │ │
│  └────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│                  Route Handlers                          │
│  ┌────────────────────────────────────────────────────┐ │
│  │  - Authentication Middleware (per-route)           │ │
│  │  - Admin Authorization Middleware (admin routes)   │ │
│  │  - Input Validation (per-endpoint)                 │ │
│  └────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│                  Business Logic                          │
│  (Repositories, Services)                                │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│              Database (SQLite with Audit Log)            │
└─────────────────────────────────────────────────────────┘
```

### Configuration Management

The system uses environment-based configuration with validation:

```rust
pub struct SecurityConfig {
    pub environment: Environment,  // Development | Production
    pub allowed_origins: Vec<String>,
    pub github_client_id: String,
    pub enable_https_redirect: bool,
    pub max_request_size: usize,
    pub session_max_age_days: i64,
    pub session_idle_timeout_hours: i64,
}

pub enum Environment {
    Development,
    Production,
}
```

## Components and Interfaces

### 1. CORS Configuration Module

**Purpose:** Configure Cross-Origin Resource Sharing to protect web terminal while allowing CLI/TUI clients.

**Interface:**
```rust
pub struct CorsConfig {
    allowed_origins: Vec<String>,
    allowed_methods: Vec<Method>,
    allowed_headers: Vec<HeaderName>,
}

impl CorsConfig {
    pub fn for_environment(env: Environment) -> Self;
    pub fn to_cors_layer(&self) -> CorsLayer;
}
```

**Behavior:**
- In production: Allow only `https://fido-social.fly.dev` origin
- In development: Allow `http://localhost:*` origins
- Always allow requests without Origin header (CLI/TUI clients)
- Restrict methods to GET, POST, PUT, DELETE
- Restrict headers to Content-Type, X-Session-Token

### 2. Input Validation Module

**Purpose:** Validate and sanitize all user inputs to prevent injection attacks and enforce business rules.

**Interface:**
```rust
pub struct InputValidator;

impl InputValidator {
    pub fn validate_username(username: &str) -> Result<(), ValidationError>;
    pub fn validate_bio(bio: &str) -> Result<(), ValidationError>;
    pub fn validate_post_content(content: &str) -> Result<(), ValidationError>;
    pub fn validate_hashtag(hashtag: &str) -> Result<(), ValidationError>;
    pub fn sanitize_text(text: &str) -> String;
}

pub enum ValidationError {
    TooLong { max: usize, actual: usize },
    TooShort { min: usize, actual: usize },
    InvalidCharacters { allowed: String },
    Empty,
}
```

**Validation Rules:**
- Username: 3-30 chars, alphanumeric + underscore + hyphen
- Bio: 0-500 chars
- Post content: 1-5000 chars
- Hashtag: 1-50 chars, alphanumeric + underscore
- All text: HTML entity encoding for XSS prevention

### 3. Admin Authorization Middleware

**Purpose:** Protect admin endpoints with role-based access control.

**Interface:**
```rust
pub async fn require_admin(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode>;
```

**Database Schema Addition:**
```sql
ALTER TABLE users ADD COLUMN is_admin INTEGER DEFAULT 0;
```

**Behavior:**
- Extract session token from X-Session-Token header
- Validate session and get user_id
- Check if user has is_admin = 1
- Return 403 Forbidden if not admin
- Log all admin access attempts to audit log

### 4. Security Headers Middleware

**Purpose:** Add security headers to all responses for defense-in-depth.

**Interface:**
```rust
pub async fn security_headers_middleware(
    request: Request,
    next: Next,
) -> Response;
```

**Headers Applied:**
- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- `X-XSS-Protection: 1; mode=block`
- `Content-Security-Policy: default-src 'self'`
- `Referrer-Policy: strict-origin-when-cross-origin`
- `Strict-Transport-Security: max-age=31536000; includeSubDomains` (production only)

### 5. Secure Error Handler

**Purpose:** Handle errors without leaking sensitive information.

**Interface:**
```rust
pub struct SecureError {
    pub code: ErrorCode,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

pub enum ErrorCode {
    ValidationError,
    AuthenticationError,
    AuthorizationError,
    NotFound,
    InternalError,
    RateLimitExceeded,
}

impl SecureError {
    pub fn to_response(&self) -> Response;
    pub fn log_internal_details(&self, internal_error: &dyn Error);
}
```

**Behavior:**
- Log full error details (including stack traces) internally
- Return only safe error messages to clients
- Use error codes for client-side error handling
- Never expose database schema or SQL errors

### 6. Audit Logging Module

**Purpose:** Record security-relevant events for monitoring and investigation.

**Database Schema:**
```sql
CREATE TABLE audit_logs (
    id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    user_id TEXT,
    ip_address TEXT,
    endpoint TEXT,
    details TEXT,
    timestamp TEXT NOT NULL,
    success INTEGER NOT NULL
);

CREATE INDEX idx_audit_logs_timestamp ON audit_logs(timestamp);
CREATE INDEX idx_audit_logs_user_id ON audit_logs(user_id);
CREATE INDEX idx_audit_logs_event_type ON audit_logs(event_type);
```

**Interface:**
```rust
pub struct AuditLogger {
    db: Database,
}

pub enum AuditEvent {
    Login { user_id: Uuid, ip: String },
    Logout { user_id: Uuid },
    AdminAccess { user_id: Uuid, endpoint: String },
    AuthenticationFailure { username: String, ip: String },
    RateLimitExceeded { user_id: Option<Uuid>, ip: String },
    ValidationFailure { endpoint: String, error: String },
}

impl AuditLogger {
    pub async fn log(&self, event: AuditEvent, success: bool) -> Result<()>;
    pub async fn get_recent_events(&self, limit: usize) -> Result<Vec<AuditLog>>;
}
```

### 7. Session Security Enhancement

**Purpose:** Improve session token security with expiry and idle timeout.

**Modified Session Schema:**
```sql
ALTER TABLE sessions ADD COLUMN last_activity TEXT;
UPDATE sessions SET last_activity = created_at WHERE last_activity IS NULL;
```

**Interface:**
```rust
impl SessionManager {
    pub fn create_session(&self, user_id: Uuid) -> Result<String>;
    pub fn validate_session(&self, token: &str) -> Result<Uuid>;
    pub fn update_activity(&self, token: &str) -> Result<()>;
    pub fn invalidate_user_sessions(&self, user_id: Uuid) -> Result<()>;
}
```

**Behavior:**
- Sessions expire after 7 days (reduced from 90)
- Sessions invalidated after 24 hours of inactivity
- Update last_activity on each validated request
- Invalidate all previous sessions on new login

### 8. Request Size Limiting

**Purpose:** Prevent denial-of-service attacks via large payloads.

**Interface:**
```rust
pub fn request_size_limit_layer(max_size: usize) -> RequestBodyLimitLayer;
```

**Configuration:**
- Maximum request body size: 1MB
- Applied globally to all routes
- Returns 413 Payload Too Large on violation

## Data Models

### SecurityConfig
```rust
pub struct SecurityConfig {
    pub environment: Environment,
    pub allowed_origins: Vec<String>,
    pub github_client_id: String,
    pub enable_https_redirect: bool,
    pub max_request_size: usize,
    pub session_max_age_days: i64,
    pub session_idle_timeout_hours: i64,
}
```

### AuditLog
```rust
pub struct AuditLog {
    pub id: Uuid,
    pub event_type: String,
    pub user_id: Option<Uuid>,
    pub ip_address: Option<String>,
    pub endpoint: Option<String>,
    pub details: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
    pub success: bool,
}
```

### ValidationError
```rust
pub enum ValidationError {
    TooLong { field: String, max: usize, actual: usize },
    TooShort { field: String, min: usize, actual: usize },
    InvalidCharacters { field: String, allowed: String },
    Empty { field: String },
}
```

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system—essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*


### Input Validation Properties

Property 1: Bio length validation
*For any* bio string, if its length exceeds 500 characters, then validation should fail with a TooLong error
**Validates: Requirements 5.1**

Property 2: Post content length validation
*For any* post content string, if its length exceeds 5000 characters, then validation should fail with a TooLong error
**Validates: Requirements 5.2**

Property 3: Username character validation
*For any* username string, if it contains characters other than alphanumeric, underscore, or hyphen, then validation should fail with an InvalidCharacters error
**Validates: Requirements 5.3**

Property 4: Username length validation
*For any* username string, if its length is less than 3 or greater than 30 characters, then validation should fail with a length error
**Validates: Requirements 5.4**

Property 5: Hashtag character validation
*For any* hashtag string, if it contains characters other than alphanumeric or underscore, then validation should fail with an InvalidCharacters error
**Validates: Requirements 5.5**

Property 6: Hashtag length validation
*For any* hashtag string, if its length exceeds 50 characters, then validation should fail with a TooLong error
**Validates: Requirements 5.6**

Property 7: XSS sanitization
*For any* text input containing HTML or script tags, the sanitized output should not contain executable code
**Validates: Requirements 5.7**

### Authorization Properties

Property 8: Admin endpoint protection
*For any* non-admin user and any admin endpoint, requests should be rejected with a 403 Forbidden error
**Validates: Requirements 3.1, 3.2**

Property 9: Admin access audit logging
*For any* admin endpoint access attempt, an audit log entry should exist with user ID, endpoint, and result
**Validates: Requirements 3.5, 11.3**

### CORS Properties

Property 10: Non-origin requests allowed
*For any* HTTP request without an Origin header, the CORS middleware should allow the request to proceed
**Validates: Requirements 1.2, 1.7**

Property 11: Untrusted origin rejection
*For any* HTTP request with an Origin header not in the allowed list, the CORS middleware should reject the request
**Validates: Requirements 1.3**

### SQL Injection Prevention Properties

Property 12: Sort order whitelist validation
*For any* sort order value not in the SortOrder enum, the system should reject it before query construction
**Validates: Requirements 6.3, 6.5**

### Error Handling Properties

Property 13: Database error sanitization
*For any* database error, the client response should contain a generic error message without database-specific details
**Validates: Requirements 7.2, 7.4**

Property 14: Stack trace concealment
*For any* internal error, the API response should not contain stack trace information
**Validates: Requirements 7.3**

Property 15: Error logging completeness
*For any* database error, an internal log entry should exist containing the full error details
**Validates: Requirements 7.1**

Property 16: Validation error structure
*For any* validation failure, the response should contain specific validation details in a structured format without internal system details
**Validates: Requirements 7.5, 7.6**

### Security Headers Properties

Property 17: Security headers presence
*For any* HTTP response, it should include all required security headers (X-Content-Type-Options, X-Frame-Options, X-XSS-Protection, Content-Security-Policy, Referrer-Policy)
**Validates: Requirements 9.1, 9.2, 9.3, 9.4, 9.6**

### Session Security Properties

Property 18: Session expiry limit
*For any* created session, the expiry timestamp should be no more than 7 days from the creation timestamp
**Validates: Requirements 10.1**

Property 19: Idle timeout enforcement
*For any* session with last_activity more than 24 hours ago, validation should fail and the session should be invalidated
**Validates: Requirements 10.2, 10.3**

Property 20: Session invalidation on login
*For any* user login, all previous sessions for that user should be invalidated
**Validates: Requirements 10.4**

Property 21: Session token length
*For any* generated session token, its length should be at least 32 characters
**Validates: Requirements 10.5**

### Audit Logging Properties

Property 22: Login audit logging
*For any* successful login, an audit log entry should exist containing user ID, timestamp, and IP address
**Validates: Requirements 11.1**

Property 23: Logout audit logging
*For any* logout, an audit log entry should exist containing user ID and timestamp
**Validates: Requirements 11.2**

Property 24: Authentication failure audit logging
*For any* failed authentication attempt, an audit log entry should exist containing attempted username and IP address
**Validates: Requirements 11.4**

Property 25: Rate limit audit logging
*For any* rate limit trigger, an audit log entry should exist containing user ID or IP address
**Validates: Requirements 11.5**

Property 26: Validation failure audit logging
*For any* input validation failure, an audit log entry should exist containing validation error details
**Validates: Requirements 11.6**

### HTTPS Properties

Property 27: Cookie secure flag
*For any* cookie set when HTTPS is detected, the Secure flag should be present
**Validates: Requirements 4.3**

## Error Handling

### Error Response Structure

All errors follow a consistent structure:

```rust
{
    "error": {
        "code": "VALIDATION_ERROR",
        "message": "Invalid input provided",
        "details": {
            "field": "username",
            "reason": "too_short",
            "min": 3,
            "actual": 2
        }
    }
}
```

### Error Categories

1. **Validation Errors (400)**
   - Input validation failures
   - Safe to expose validation details
   - Include field name and constraint violated

2. **Authentication Errors (401)**
   - Invalid or expired session
   - Generic message: "Authentication required"
   - No details about why authentication failed

3. **Authorization Errors (403)**
   - Insufficient permissions
   - Generic message: "Access denied"
   - No details about required permissions

4. **Not Found Errors (404)**
   - Resource not found
   - Generic message with resource type
   - No details about database structure

5. **Rate Limit Errors (429)**
   - Too many requests
   - Include Retry-After header
   - Generic message about rate limiting

6. **Internal Errors (500)**
   - Database errors
   - Unexpected errors
   - Generic message: "Internal server error"
   - Full details logged internally only

### Error Logging

All errors are logged with different levels:
- Validation errors: DEBUG level
- Authentication/Authorization errors: INFO level
- Rate limit errors: WARN level
- Internal errors: ERROR level with full stack trace

## Testing Strategy

### Unit Testing

Unit tests will verify specific examples and edge cases:

1. **Input Validation Tests**
   - Test exact boundary values (3 chars, 30 chars for username)
   - Test empty strings
   - Test strings with special characters
   - Test XSS payloads (e.g., `<script>alert('xss')</script>`)

2. **Configuration Tests**
   - Test environment-specific configuration loading
   - Test missing required configuration
   - Test CORS configuration for each environment

3. **Authorization Tests**
   - Test admin endpoint with admin user (should succeed)
   - Test admin endpoint with non-admin user (should fail)
   - Test admin endpoint without authentication (should fail)

4. **Error Handling Tests**
   - Test database error response sanitization
   - Test error response structure
   - Test error logging

5. **Session Security Tests**
   - Test session creation with correct expiry
   - Test expired session validation
   - Test idle session validation
   - Test session invalidation on login

6. **Audit Logging Tests**
   - Test audit log creation for each event type
   - Test audit log contains required fields

### Property-Based Testing

Property-based tests will verify universal properties across many generated inputs. Each test should run a minimum of 100 iterations.

**Testing Framework:** Use `proptest` crate for Rust property-based testing.

**Test Organization:**
- Create `fido-server/tests/security_properties.rs` for property tests
- Each property test should reference its design document property number
- Tag format: `// Feature: security-hardening, Property N: [property text]`

**Property Test Examples:**

1. **Input Validation Properties (1-7)**
   - Generate random strings of various lengths and character sets
   - Verify validation behaves correctly for all inputs
   - Test sanitization removes all dangerous content

2. **Authorization Properties (8-9)**
   - Generate random user objects with varying admin flags
   - Generate random admin endpoint requests
   - Verify authorization logic and audit logging

3. **CORS Properties (10-11)**
   - Generate random HTTP requests with and without Origin headers
   - Generate random origin values
   - Verify CORS middleware behavior

4. **Error Handling Properties (13-16)**
   - Generate random error conditions
   - Verify error responses never contain sensitive information
   - Verify error logging captures full details

5. **Security Headers Property (17)**
   - Generate random HTTP responses
   - Verify all required headers are present

6. **Session Security Properties (18-21)**
   - Generate random session creation times
   - Generate random last_activity timestamps
   - Verify session validation logic

7. **Audit Logging Properties (22-26)**
   - Generate random security events
   - Verify audit logs are created with correct fields

### Integration Testing

Integration tests will verify end-to-end security flows:

1. **Authentication Flow**
   - Test complete login flow with audit logging
   - Test session validation across requests
   - Test logout with session cleanup

2. **Admin Endpoint Flow**
   - Test admin endpoint access with admin user
   - Test admin endpoint rejection for non-admin
   - Verify audit logs for both cases

3. **Error Handling Flow**
   - Trigger database errors and verify response sanitization
   - Verify internal logging captures full details

4. **CORS Flow**
   - Test web terminal requests with proper origin
   - Test TUI client requests without origin
   - Test browser requests with untrusted origin

### Security Testing

Additional security-focused testing:

1. **SQL Injection Testing**
   - Attempt SQL injection in all user inputs
   - Verify parameterized queries prevent injection

2. **XSS Testing**
   - Submit various XSS payloads
   - Verify sanitization prevents execution

3. **CSRF Testing**
   - Attempt cross-origin requests from untrusted origins
   - Verify CORS protection works

4. **Session Security Testing**
   - Test session hijacking scenarios
   - Test session fixation scenarios
   - Verify session invalidation works correctly

### Test Coverage Goals

- Unit test coverage: >80% for security-critical code
- Property test coverage: All 27 correctness properties
- Integration test coverage: All major security flows
- Security test coverage: All OWASP Top 10 relevant vulnerabilities

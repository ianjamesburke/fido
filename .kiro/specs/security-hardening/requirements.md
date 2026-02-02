# Requirements Document

## Introduction

This specification addresses critical security vulnerabilities identified in the security audit for the Fido social platform. The system currently has several security issues that must be resolved for production readiness, including in-memory device code storage, overly long session expiry, missing request body limits, missing security headers, and insufficient audit logging.

## Glossary

- **Device_Code**: Temporary code used during GitHub OAuth Device Flow authentication
- **Session_Token**: UUID v4 token used to authenticate user sessions
- **Access_Token**: Short-lived token (~15 minutes) for API authentication
- **Refresh_Token**: Long-lived token (~7 days) used to obtain new access tokens
- **Session_Manager**: Component that manages user session lifecycle
- **Rate_Limiter**: Component that limits request frequency per user
- **Security_Headers**: HTTP headers that enhance application security
- **Audit_Logger**: Component that records security-relevant events
- **TTL**: Time-To-Live, the duration before an item expires

## Requirements

### Requirement 1: Device Code Database Storage

**User Story:** As a security engineer, I want device codes stored in the database with TTL, so that authentication state survives server restarts and expired codes are automatically cleaned up.

#### Acceptance Criteria

1. WHEN a device code is generated THEN the System SHALL store it in the SQLite database with creation timestamp and expiry time
2. WHEN a device code is stored THEN the System SHALL set a TTL of 15 minutes from creation
3. WHEN polling for device authorization THEN the System SHALL retrieve the device code from the database
4. WHEN a device code expires THEN the System SHALL reject authorization attempts with an appropriate error
5. WHEN the server starts THEN the System SHALL run cleanup of expired device codes
6. THE System SHALL run periodic cleanup of expired device codes every 5 minutes
7. WHEN a device code is successfully used THEN the System SHALL delete it from the database
8. THE System SHALL NOT use in-memory storage (lazy_static HashMap) for device codes

### Requirement 2: Session Expiry Reduction

**User Story:** As a security engineer, I want session expiry reduced from 90 days to 7 days, so that compromised sessions have limited validity windows.

#### Acceptance Criteria

1. WHEN a session is created THEN the System SHALL set expiry to 7 days from creation
2. WHEN validating a session THEN the System SHALL reject sessions older than 7 days
3. THE System SHALL update the session creation logic to use 7-day expiry instead of 90-day expiry

### Requirement 3: Refresh Token Mechanism

**User Story:** As a security engineer, I want a refresh token mechanism with short-lived access tokens, so that token theft has limited impact and sessions can be extended securely.

#### Acceptance Criteria

1. WHEN a user logs in THEN the System SHALL issue both an access token (15 minute expiry) and a refresh token (7 day expiry)
2. WHEN an access token expires THEN the System SHALL allow the client to obtain a new access token using the refresh token
3. WHEN a refresh token is used THEN the System SHALL rotate it by issuing a new refresh token and invalidating the old one
4. WHEN a refresh token expires THEN the System SHALL require the user to re-authenticate
5. THE System SHALL store refresh tokens in the database with user_id, token, created_at, and expires_at
6. WHEN a refresh token is used THEN the System SHALL verify it exists in the database and has not expired

### Requirement 4: Session Binding

**User Story:** As a security engineer, I want sessions bound to client context, so that stolen tokens cannot be used from different clients.

#### Acceptance Criteria

1. WHEN a session is created THEN the System SHALL store the client IP address
2. WHEN a session is created THEN the System SHALL store the User-Agent header
3. WHEN validating a session THEN the System SHALL log a warning if IP address differs from stored value
4. WHEN validating a session THEN the System SHALL log a warning if User-Agent differs from stored value
5. THE System SHALL NOT reject sessions based on IP/User-Agent changes (to support mobile users)

### Requirement 5: Concurrent Session Limits

**User Story:** As a security engineer, I want concurrent session limits per user, so that compromised accounts have limited attack surface.

#### Acceptance Criteria

1. WHEN a user has 5 or more active sessions THEN the System SHALL invalidate the oldest session when creating a new one
2. WHEN counting active sessions THEN the System SHALL only count non-expired sessions
3. THE System SHALL enforce a maximum of 5 concurrent sessions per user

### Requirement 6: Request Body Size Limits

**User Story:** As a security engineer, I want request body size limits enforced, so that the server is protected from denial-of-service attacks via large payloads.

#### Acceptance Criteria

1. THE System SHALL enforce a maximum request body size of 1MB
2. WHEN a request body exceeds 1MB THEN the System SHALL reject it with HTTP 413 Payload Too Large
3. THE System SHALL apply the body size limit globally to all routes
4. THE System SHALL use tower-http RequestBodyLimitLayer for enforcement

### Requirement 7: Security Headers

**User Story:** As a security engineer, I want security headers added to all responses, so that the application has defense-in-depth protection against common web attacks.

#### Acceptance Criteria

1. WHEN any response is sent THEN the System SHALL include X-Content-Type-Options: nosniff header
2. WHEN any response is sent THEN the System SHALL include X-Frame-Options: DENY header
3. WHEN any response is sent THEN the System SHALL include X-XSS-Protection: 1; mode=block header
4. WHERE the application runs in production THEN the System SHALL include Strict-Transport-Security: max-age=31536000; includeSubDomains header
5. THE System SHALL implement security headers as middleware applied to all routes

### Requirement 8: Error Message Sanitization

**User Story:** As a security engineer, I want internal errors sanitized before returning to clients, so that implementation details are not leaked to potential attackers.

#### Acceptance Criteria

1. WHEN a database error occurs THEN the System SHALL return a generic "Internal server error" message to the client
2. WHEN a database error occurs THEN the System SHALL log the actual error details server-side
3. WHEN an internal error occurs THEN the System SHALL NOT expose stack traces in API responses
4. WHEN an internal error occurs THEN the System SHALL NOT expose database schema details in API responses
5. WHEN validation fails THEN the System SHALL return specific validation errors without internal details

### Requirement 9: Security Audit Logging

**User Story:** As a security engineer, I want security events logged, so that authentication issues and suspicious activity can be monitored and investigated.

#### Acceptance Criteria

1. WHEN a user successfully logs in THEN the System SHALL log the event with user_id, timestamp, and IP address
2. WHEN a user fails to authenticate THEN the System SHALL log the event with attempted identifier and IP address
3. WHEN a session is created THEN the System SHALL log the event with user_id and session_id
4. WHEN a session is revoked THEN the System SHALL log the event with user_id and session_id
5. WHEN suspicious activity is detected (e.g., IP change) THEN the System SHALL log the event with relevant details
6. THE System SHALL store audit logs in a dedicated audit_logs table
7. THE System SHALL include event_type, user_id, ip_address, user_agent, details, and timestamp in audit logs

### Requirement 10: Device Code Cleanup

**User Story:** As a security engineer, I want expired device codes automatically cleaned up, so that the database does not accumulate stale authentication data.

#### Acceptance Criteria

1. WHEN the server starts THEN the System SHALL delete all expired device codes from the database
2. THE System SHALL run periodic cleanup of expired device codes every 5 minutes
3. WHEN cleanup runs THEN the System SHALL log the number of expired codes removed
4. THE System SHALL use a background task for periodic cleanup


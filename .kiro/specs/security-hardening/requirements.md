# Requirements Document

## Introduction

This specification addresses critical security vulnerabilities and implements production-ready security hardening for the Fido social platform. The system currently has several high-risk security issues that must be resolved before production deployment, including CORS misconfigurations, exposed credentials, unprotected admin endpoints, and missing input validation.

## Glossary

- **CORS**: Cross-Origin Resource Sharing - HTTP security mechanism controlling which origins can access resources
- **CSRF**: Cross-Site Request Forgery - Attack forcing authenticated users to execute unwanted actions
- **OAuth_App**: GitHub OAuth application credentials used for authentication
- **Session_Token**: UUID v4 token used to authenticate user sessions
- **Rate_Limiter**: Component that limits request frequency per user
- **Admin_Endpoint**: API endpoint that performs administrative operations
- **Input_Validation**: Process of verifying user input meets security and format requirements
- **Security_Headers**: HTTP headers that enhance application security
- **HTTPS**: Secure HTTP protocol using TLS encryption
- **SQL_Injection**: Attack inserting malicious SQL code through user inputs

## Requirements

### Requirement 1: CORS Security Configuration

**User Story:** As a security engineer, I want properly configured CORS policies, so that the web terminal interface is protected from CSRF attacks while allowing CLI and TUI clients to function.

#### Acceptance Criteria

1. WHEN the server starts THEN the System SHALL configure CORS to allow the web terminal origin (https://fido-social.fly.dev)
2. WHEN a request arrives from the TUI client THEN the System SHALL allow the request (TUI clients do not send Origin headers)
3. WHEN a request arrives from a browser with an untrusted origin THEN the System SHALL reject the request with appropriate CORS headers
4. WHEN configuring CORS THEN the System SHALL restrict allowed HTTP methods to only those required (GET, POST, PUT, DELETE)
5. WHEN configuring CORS THEN the System SHALL restrict allowed headers to only necessary headers (Content-Type, X-Session-Token)
6. WHERE the application runs in development mode THEN the System SHALL allow localhost origins for local web terminal testing
7. THE System SHALL allow requests without Origin headers to support CLI and TUI clients

### Requirement 2: Secrets Management

**User Story:** As a security engineer, I want OAuth credentials removed from version control, so that sensitive authentication secrets are not exposed publicly.

#### Acceptance Criteria

1. WHEN the application starts THEN the System SHALL load GitHub OAuth credentials from environment variables only
2. THE System SHALL NOT include any OAuth credentials in the .env file committed to Git
3. THE System SHALL provide clear documentation for setting required environment variables
4. WHEN OAuth credentials are missing THEN the System SHALL fail to start with a descriptive error message
5. THE System SHALL include .env in .gitignore to prevent accidental commits

### Requirement 3: Admin Endpoint Authentication

**User Story:** As a security engineer, I want admin endpoints protected by authentication, so that only authorized administrators can perform administrative operations.

#### Acceptance Criteria

1. WHEN a request is made to an admin endpoint THEN the System SHALL verify the user has admin privileges
2. WHEN a non-admin user attempts to access an admin endpoint THEN the System SHALL return a 403 Forbidden error
3. THE System SHALL implement an admin role flag in the user database schema
4. WHEN cleaning up sessions THEN the System SHALL require admin authentication
5. THE System SHALL log all admin endpoint access attempts for audit purposes

### Requirement 4: HTTPS Enforcement

**User Story:** As a security engineer, I want HTTPS enforced for all connections, so that session tokens and user data are protected from interception.

#### Acceptance Criteria

1. WHERE the application runs on Fly.io THEN the System SHALL rely on Fly.io's automatic HTTPS termination
2. WHEN a session token is transmitted THEN the System SHALL ensure the connection uses HTTPS in production
3. THE System SHALL set Secure flag on any cookies when HTTPS is detected
4. THE System SHALL include Strict-Transport-Security header in production responses
5. WHERE the application runs in development mode THEN the System SHALL allow HTTP for local testing
6. THE System SHALL configure fly.toml to force HTTPS connections at the edge

### Requirement 5: Input Validation and Sanitization

**User Story:** As a security engineer, I want comprehensive input validation, so that the application is protected from injection attacks and malformed data.

#### Acceptance Criteria

1. WHEN a user updates their bio THEN the System SHALL enforce a maximum length of 500 characters
2. WHEN a user creates a post THEN the System SHALL enforce a maximum length of 5000 characters
3. WHEN a username is provided THEN the System SHALL validate it contains only alphanumeric characters, underscores, and hyphens
4. WHEN a username is provided THEN the System SHALL enforce a length between 3 and 30 characters
5. WHEN a hashtag is created THEN the System SHALL validate it contains only alphanumeric characters and underscores
6. WHEN a hashtag is created THEN the System SHALL enforce a maximum length of 50 characters
7. WHEN any text input is received THEN the System SHALL sanitize it to prevent XSS attacks
8. WHEN request body size exceeds 1MB THEN the System SHALL reject the request with a 413 Payload Too Large error

### Requirement 6: SQL Injection Prevention

**User Story:** As a security engineer, I want all SQL queries to use parameterized statements, so that the application is protected from SQL injection attacks.

#### Acceptance Criteria

1. THE System SHALL use parameterized queries for all database operations
2. THE System SHALL NOT construct SQL queries using string concatenation with user input
3. WHEN building dynamic ORDER BY clauses THEN the System SHALL use whitelisted values only
4. WHEN building dynamic WHERE clauses THEN the System SHALL use parameterized placeholders
5. THE System SHALL validate all enum-based inputs against allowed values before query construction

### Requirement 7: Error Handling and Information Disclosure

**User Story:** As a security engineer, I want secure error handling, so that sensitive system information is not leaked to API clients.

#### Acceptance Criteria

1. WHEN a database error occurs THEN the System SHALL log the full error internally
2. WHEN a database error occurs THEN the System SHALL return a generic error message to the client
3. WHEN an internal error occurs THEN the System SHALL NOT expose stack traces to API responses
4. WHEN an internal error occurs THEN the System SHALL NOT expose database schema details to API responses
5. WHEN validation fails THEN the System SHALL return specific validation errors without exposing internal details
6. THE System SHALL implement structured error responses with error codes and safe messages

### Requirement 8: Rate Limiting Enhancement (DEFERRED)

**User Story:** As a security engineer, I want persistent rate limiting, so that rate limits survive server restarts and prevent abuse effectively.

**Note:** This requirement is deferred for post-MVP implementation. The current in-memory rate limiting is acceptable for MVP with the understanding that limits reset on server restart.

#### Acceptance Criteria (Future Implementation)

1. WHEN rate limiting is applied THEN the System SHALL persist rate limit state to the database
2. WHEN the server restarts THEN the System SHALL restore rate limit state from the database
3. WHEN rate limit is exceeded THEN the System SHALL include Retry-After header in the response
4. THE System SHALL apply rate limiting to both authenticated and unauthenticated requests
5. WHEN an unauthenticated request arrives THEN the System SHALL rate limit by IP address
6. THE System SHALL implement different rate limits for different endpoint categories (auth: 10/min, posts: 30/min, reads: 100/min)

### Requirement 9: Security Headers Middleware

**User Story:** As a security engineer, I want security headers added to all responses, so that the application has defense-in-depth protection against common web attacks.

#### Acceptance Criteria

1. WHEN any response is sent THEN the System SHALL include X-Content-Type-Options: nosniff header
2. WHEN any response is sent THEN the System SHALL include X-Frame-Options: DENY header
3. WHEN any response is sent THEN the System SHALL include X-XSS-Protection: 1; mode=block header
4. WHEN any response is sent THEN the System SHALL include Content-Security-Policy header with restrictive policy
5. WHERE the application runs in production THEN the System SHALL include Strict-Transport-Security header
6. WHEN any response is sent THEN the System SHALL include Referrer-Policy: strict-origin-when-cross-origin header

### Requirement 10: Session Security Enhancement

**User Story:** As a security engineer, I want enhanced session security, so that session tokens are protected from theft and misuse.

#### Acceptance Criteria

1. WHEN a session is created THEN the System SHALL set session expiry to 7 days maximum
2. WHEN a session is validated THEN the System SHALL check if the session has been idle for more than 24 hours
3. WHEN a session has been idle for more than 24 hours THEN the System SHALL invalidate the session
4. WHEN a user logs in THEN the System SHALL invalidate all previous sessions for that user
5. WHEN a session token is transmitted THEN the System SHALL ensure it is at least 32 characters long
6. THE System SHALL implement session token rotation on sensitive operations

### Requirement 11: Audit Logging

**User Story:** As a security engineer, I want comprehensive audit logging, so that security events can be monitored and investigated.

#### Acceptance Criteria

1. WHEN a user logs in THEN the System SHALL log the event with user ID, timestamp, and IP address
2. WHEN a user logs out THEN the System SHALL log the event with user ID and timestamp
3. WHEN an admin endpoint is accessed THEN the System SHALL log the event with user ID, endpoint, and result
4. WHEN authentication fails THEN the System SHALL log the event with attempted username and IP address
5. WHEN rate limiting triggers THEN the System SHALL log the event with user ID or IP address
6. WHEN input validation fails THEN the System SHALL log the event with validation error details
7. THE System SHALL store audit logs in a separate audit_logs table with retention policy

### Requirement 12: Configuration Management

**User Story:** As a security engineer, I want environment-specific security configurations, so that development and production environments have appropriate security settings.

#### Acceptance Criteria

1. THE System SHALL support environment-specific configuration via environment variables
2. WHEN the environment is "production" THEN the System SHALL enforce all security features
3. WHEN the environment is "development" THEN the System SHALL allow relaxed CORS and HTTP for local testing
4. THE System SHALL validate all required security configuration on startup
5. WHEN required security configuration is missing in production THEN the System SHALL fail to start
6. THE System SHALL provide a configuration validation endpoint for deployment verification

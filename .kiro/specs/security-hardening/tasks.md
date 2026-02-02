# Implementation Plan: Security Hardening

## Overview

This implementation plan addresses critical security vulnerabilities in the Fido social platform through a systematic, incremental approach. Tasks are organized to fix the most critical issues first (CORS, secrets, admin auth) followed by comprehensive hardening (input validation, error handling, audit logging, session security).

## Tasks

- [x] 1. Remove secrets from Git and update configuration
  - Remove GITHUB_CLIENT_ID from .env file
  - Update .env.example with placeholder and documentation
  - Update config.rs to require GITHUB_CLIENT_ID from environment
  - Add startup validation that fails if GITHUB_CLIENT_ID is missing
  - Document required environment variables in README.md
  - _Requirements: 2.1, 2.4_

- [x] 2. Implement environment-based security configuration
  - [x] 2.1 Create SecurityConfig struct and Environment enum
    - Add fido-server/src/security/mod.rs with SecurityConfig
    - Implement Environment enum (Development, Production)
    - Add configuration loading from environment variables
    - Add validation method for required configuration
    - _Requirements: 12.1, 12.4_

  - [x] 2.2 Update main.rs to load and validate security configuration
    - Load SecurityConfig on startup
    - Fail fast if required configuration is missing in production
    - Log security configuration (without secrets)
    - _Requirements: 12.2, 12.5_

- [x] 3. Fix CORS configuration
  - [x] 3.1 Implement environment-aware CORS configuration
    - Create fido-server/src/security/cors.rs
    - Implement CorsConfig struct with environment-specific origins
    - Production: allow https://fido-social.fly.dev only
    - Development: allow http://localhost:* origins
    - Always allow requests without Origin header (CLI/TUI clients)
    - Restrict methods to GET, POST, PUT, DELETE
    - Restrict headers to Content-Type, X-Session-Token
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7_

  - [ ]* 3.2 Write property test for CORS non-origin requests
    - **Property 10: Non-origin requests allowed**
    - **Validates: Requirements 1.2, 1.7**

  - [ ]* 3.3 Write property test for CORS untrusted origin rejection
    - **Property 11: Untrusted origin rejection**
    - **Validates: Requirements 1.3**

  - [x] 3.4 Update main.rs to use new CORS configuration
    - Replace current CorsLayer::new().allow_origin(Any) with environment-aware config
    - _Requirements: 1.1_

- [x] 4. Implement input validation module
  - [x] 4.1 Create input validation module with validation functions
    - Create fido-server/src/security/validation.rs
    - Implement InputValidator with validation methods
    - Implement ValidationError enum
    - Add validate_username, validate_bio, validate_post_content, validate_hashtag
    - Add sanitize_text for XSS prevention
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7_

  - [ ]* 4.2 Write property tests for input validation
    - **Property 1: Bio length validation**
    - **Property 2: Post content length validation**
    - **Property 3: Username character validation**
    - **Property 4: Username length validation**
    - **Property 5: Hashtag character validation**
    - **Property 6: Hashtag length validation**
    - **Property 7: XSS sanitization**
    - **Validates: Requirements 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7**

  - [x] 4.3 Integrate validation into API endpoints
    - Update api/profile.rs to validate bio updates
    - Update api/posts.rs to validate post content
    - Update api/auth.rs to validate usernames
    - Update api/hashtags.rs to validate hashtag names
    - Return 400 Bad Request with validation errors
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7_

  - [x] 4.4 Add request size limiting middleware
    - Add tower_http::limit::RequestBodyLimitLayer with 1MB limit
    - Apply globally in main.rs
    - _Requirements: 5.8_

- [x] 5. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 6. Implement admin authorization
  - [x] 6.1 Add is_admin column to users table
    - Create migration to add is_admin INTEGER DEFAULT 0 to users table
    - Update seed_test_data to set alice as admin (is_admin = 1)
    - _Requirements: 3.3_

  - [x] 6.2 Create admin authorization middleware
    - Create fido-server/src/security/admin.rs
    - Implement require_admin middleware function
    - Extract session token, validate session, check is_admin flag
    - Return 403 Forbidden if not admin
    - _Requirements: 3.1, 3.2_

  - [ ]* 6.3 Write property test for admin authorization
    - **Property 8: Admin endpoint protection**
    - **Validates: Requirements 3.1, 3.2**

  - [x] 6.4 Apply admin middleware to cleanup endpoint
    - Update auth::cleanup_sessions route to use require_admin middleware
    - _Requirements: 3.4_

- [x] 7. Implement secure error handling
  - [x] 7.1 Create secure error handling module
    - Create fido-server/src/security/errors.rs
    - Implement SecureError struct and ErrorCode enum
    - Implement to_response() method that returns safe error messages
    - Implement log_internal_details() for internal logging
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6_

  - [ ]* 7.2 Write property tests for error handling
    - **Property 13: Database error sanitization**
    - **Property 14: Stack trace concealment**
    - **Property 15: Error logging completeness**
    - **Property 16: Validation error structure**
    - **Validates: Requirements 7.1, 7.2, 7.3, 7.4, 7.5, 7.6**

  - [x] 7.3 Update API error handling to use SecureError
    - Update api/error.rs to use SecureError
    - Update all API handlers to return SecureError
    - Ensure database errors return generic messages
    - _Requirements: 7.2, 7.3, 7.4_

- [x] 8. Implement security headers middleware
  - [x] 8.1 Create security headers middleware
    - Create fido-server/src/security/headers.rs
    - Implement security_headers_middleware function
    - Add X-Content-Type-Options, X-Frame-Options, X-XSS-Protection
    - Add Content-Security-Policy, Referrer-Policy
    - Add Strict-Transport-Security in production only
    - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5, 9.6_

  - [ ]* 8.2 Write property test for security headers
    - **Property 17: Security headers presence**
    - **Validates: Requirements 9.1, 9.2, 9.3, 9.4, 9.6**

  - [x] 8.3 Apply security headers middleware in main.rs
    - Add security_headers_middleware to middleware stack
    - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5, 9.6_

- [x] 9. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 10. Implement audit logging
  - [x] 10.1 Create audit_logs table and module
    - Create migration for audit_logs table with indexes
    - Create fido-server/src/security/audit.rs
    - Implement AuditLogger struct and AuditEvent enum
    - Implement log() method to write audit logs
    - _Requirements: 11.7_

  - [ ]* 10.2 Write property tests for audit logging
    - **Property 9: Admin access audit logging**
    - **Property 22: Login audit logging**
    - **Property 23: Logout audit logging**
    - **Property 24: Authentication failure audit logging**
    - **Property 25: Rate limit audit logging**
    - **Property 26: Validation failure audit logging**
    - **Validates: Requirements 3.5, 11.1, 11.2, 11.3, 11.4, 11.5, 11.6**

  - [x] 10.3 Integrate audit logging into authentication endpoints
    - Add audit logging to login endpoint (success and failure)
    - Add audit logging to logout endpoint
    - Add audit logging to admin endpoints
    - _Requirements: 11.1, 11.2, 11.3, 11.4_

  - [x] 10.4 Integrate audit logging into rate limiter
    - Update rate_limit.rs to log rate limit events
    - _Requirements: 11.5_

  - [x] 10.5 Integrate audit logging into validation
    - Update validation module to log validation failures
    - _Requirements: 11.6_

- [x] 11. Enhance session security
  - [x] 11.1 Update session schema and logic
    - Create migration to add last_activity column to sessions table
    - Update sessions table to set default last_activity = created_at
    - Reduce session expiry from 90 days to 7 days
    - _Requirements: 10.1_

  - [x] 11.2 Implement idle timeout and activity tracking
    - Update SessionManager::validate_session to check idle timeout
    - Add SessionManager::update_activity method
    - Invalidate sessions idle > 24 hours
    - Update last_activity on each validated request
    - _Requirements: 10.2, 10.3_

  - [ ]* 11.3 Write property tests for session security
    - **Property 18: Session expiry limit**
    - **Property 19: Idle timeout enforcement**
    - **Property 20: Session invalidation on login**
    - **Property 21: Session token length**
    - **Validates: Requirements 10.1, 10.2, 10.3, 10.4, 10.5**

  - [x] 11.4 Implement session invalidation on login
    - Add SessionManager::invalidate_user_sessions method
    - Call invalidate_user_sessions in login and github_device_poll
    - _Requirements: 10.4_

  - [x] 11.5 Update authentication middleware to track activity
    - Update middleware to call update_activity on each request
    - _Requirements: 10.2_

- [x] 12. Implement SQL injection prevention
  - [x] 12.1 Audit and fix dynamic SQL queries
    - Review PostRepository::get_posts for SQL injection risks
    - Ensure ORDER BY uses whitelisted SortOrder enum values only
    - Review all repositories for string concatenation in queries
    - Ensure all user inputs use parameterized queries
    - _Requirements: 6.1, 6.2, 6.3, 6.4_

  - [ ]* 12.2 Write property test for sort order validation
    - **Property 12: Sort order whitelist validation**
    - **Validates: Requirements 6.3, 6.5**

  - [x] 12.3 Add enum validation before query construction
    - Add validation for SortOrder enum in API handlers
    - Reject invalid enum values before passing to repositories
    - _Requirements: 6.5_

- [x] 13. Configure HTTPS enforcement
  - [x] 13.1 Update fly.toml for HTTPS enforcement
    - Add force_https = true to [[services.ports]] configuration
    - Ensure auto_stop_machines and auto_start_machines are configured
    - _Requirements: 4.6_

  - [x] 13.2 Implement HTTPS detection and cookie security
    - Create helper function to detect HTTPS from headers
    - Update cookie setting logic to add Secure flag when HTTPS detected
    - _Requirements: 4.2, 4.3_

  - [ ]* 13.3 Write property test for cookie secure flag
    - **Property 27: Cookie secure flag**
    - **Validates: Requirements 4.3**

- [x] 14. Add configuration validation endpoint
  - [x] 14.1 Create configuration validation endpoint
    - Add GET /admin/config/validate endpoint
    - Return security configuration status (without secrets)
    - Require admin authentication
    - _Requirements: 12.6_

  - [x] 14.2 Write unit test for configuration validation endpoint
    - Test endpoint returns configuration status
    - Test endpoint requires admin authentication
    - _Requirements: 12.6_

- [x] 15. Final checkpoint - Comprehensive testing
  - Run all unit tests
  - Run all property tests (minimum 100 iterations each)
  - Run integration tests
  - Verify all 27 correctness properties pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 16. Update documentation
  - Update README.md with security features
  - Document required environment variables
  - Document admin user setup
  - Add security best practices section
  - _Requirements: 2.3_

## Notes

- Tasks marked with `*` are optional property-based tests and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties using proptest crate
- Unit tests validate specific examples and edge cases
- Critical vulnerabilities (CORS, secrets, admin auth) are addressed first
- All database migrations should be tested before deployment

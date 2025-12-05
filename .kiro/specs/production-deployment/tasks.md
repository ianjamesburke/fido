# Implementation Plan

## Current Status

The production deployment is **nearly complete**. The core authentication system using GitHub Device Flow is fully implemented and working. The server is deployed to Fly.io and the TUI client connects to it successfully.

**What's Working:**
- ✅ GitHub Device Flow authentication (no callback URL needed)
- ✅ Session management with 30-day expiry
- ✅ Session persistence in `~/.fido/session`
- ✅ Server deployed to Fly.io at `https://fido-social.fly.dev`
- ✅ Database migrations for sessions and GitHub fields
- ✅ TUI authentication flow with test users and GitHub option
- ✅ Logout functionality (Shift+L)
- ✅ Environment variable override (`FIDO_SERVER_URL`)
- ✅ Documentation (README, QUICKSTART, DEPLOYMENT)

**What's Remaining:**
- ⏳ CLI argument support for `--server` flag (optional enhancement)
- ⏳ Publishing to crates.io (fido-types and fido packages)
- ⏳ Final end-to-end testing
- ⏳ CI/CD automation (optional)

**Next Steps:**
1. Add CLI argument parsing for `--server` flag (task 7.2)
2. Update repository URL in Cargo.toml (task 14.1)
3. Publish to crates.io (tasks 15.1-15.4)
4. Final testing (task 16)

---

- [x] 1. Set up GitHub OAuth infrastructure
- [x] 1.1 Register GitHub OAuth application and obtain client ID/secret
  - Create OAuth app at https://github.com/settings/developers
  - Set callback URL to production URL (Device Flow doesn't use callback)
  - Save client ID for environment configuration (Device Flow doesn't need secret)
  - _Requirements: 2.2, 4.1_

- [x] 1.2 Create OAuth configuration module in server
  - Create `fido-server/src/oauth.rs` with `GitHubOAuthConfig` struct
  - Implement `from_env()` to load credentials from environment variables
  - Implement GitHub Device Flow: `request_device_code()` and `poll_device_token()`
  - Implement `get_user()` to fetch GitHub user profile
  - Add `reqwest` dependency
  - _Requirements: 4.1, 4.2_

- [ ]* 1.3 Write unit tests for OAuth module
  - Test configuration loading from environment
  - Test GitHub API response parsing with mock data
  - Test error handling for invalid responses
  - _Requirements: 4.1, 4.2_

- [x] 2. Implement database schema changes for authentication
- [x] 2.1 Create migration for sessions table
  - Add `sessions` table via `db::initialize()` migration
  - Add `sessions` table with token, user_id, created_at, expires_at
  - Add indexes on user_id and expires_at
  - _Requirements: 4.4_

- [x] 2.2 Create migration for GitHub user fields
  - Add GitHub fields via `db::initialize()` migration
  - Add `github_id` and `github_login` columns to users table
  - Add unique constraint on github_id
  - _Requirements: 4.3_

- [x] 2.3 Update database initialization to run new migrations
  - Update `db::initialize()` to include sessions table and GitHub fields
  - Test migrations run successfully on fresh database
  - _Requirements: 4.3, 4.4_

- [x] 3. Implement server-side session management
- [x] 3.1 Create session management module
  - Create `fido-server/src/session.rs` with `SessionManager` struct
  - Implement `create_session()` to generate UUID v4 tokens
  - Implement `validate_session()` to check token validity and expiry
  - Implement `delete_session()` to remove session from database
  - Implement `cleanup_expired_sessions()` for background cleanup
  - _Requirements: 3.1, 3.2, 3.3, 4.4, 4.5_

- [ ]* 3.2 Write property test for session token uniqueness
  - **Property 1: Session token uniqueness**
  - **Validates: Requirements 4.4**
  - Generate multiple sessions, verify all tokens are unique
  - _Requirements: 4.4_

- [ ]* 3.3 Write property test for session validation
  - **Property 2: Session validation correctness**
  - **Validates: Requirements 4.5**
  - Create session, validate token returns correct user_id
  - _Requirements: 4.5_

- [ ]* 3.4 Write property test for session expiry
  - **Property 3: Session expiry enforcement**
  - **Validates: Requirements 3.3**
  - Create expired session, verify validation fails
  - _Requirements: 3.3_

- [x] 3.5 Implement automatic session cleanup
  - Add background task to periodically clean up expired sessions from database
  - Run cleanup on server startup to remove stale sessions
  - Implement cleanup endpoint for manual triggering (admin use)
  - Add metrics/logging for session cleanup operations
  - Ensure cleanup doesn't impact active sessions
  - _Requirements: 3.3, 4.4_

- [x] 4. Implement OAuth API endpoints
- [x] 4.1 Create authentication API module
  - Create `fido-server/src/api/auth.rs` (or update existing)
  - Implement `GET /auth/github/login` endpoint
  - Generate OAuth state parameter, return GitHub authorization URL
  - Store state temporarily for validation
  - _Requirements: 2.2, 4.1_

- [x] 4.2 Implement OAuth callback endpoint
  - Implement `GET /auth/github/callback` endpoint
  - Validate state parameter matches stored value
  - Exchange authorization code for access token
  - Fetch GitHub user profile
  - Create or update user record in database
  - Create session and return token
  - _Requirements: 2.3, 4.2, 4.3, 4.4_

- [x] 4.3 Implement session validation endpoint
  - Implement `GET /auth/validate` endpoint
  - Validate session token from header
  - Return user information if valid
  - Return 401 if invalid or expired
  - _Requirements: 3.2, 4.5_

- [x] 4.4 Update existing auth middleware
  - Update session token extraction to use new session manager
  - Ensure all protected endpoints validate sessions
  - Return 401 for missing or invalid tokens
  - _Requirements: 4.5_

- [ ]* 4.5 Write integration tests for OAuth flow
  - Test complete OAuth flow with mock GitHub server
  - Test state parameter validation
  - Test duplicate code exchange handling
  - Test session creation after successful OAuth
  - _Requirements: 2.2, 4.1, 4.2, 4.3_

- [x] 5. Implement client-side session storage
- [x] 5.1 Create session store module in TUI
  - Create `fido-tui/src/session.rs` with `SessionStore` struct
  - Implement `new()` to initialize with `~/.fido/session` path
  - Implement `load()` to read session token from file
  - Implement `save()` to write session token with 0600 permissions
  - Implement `delete()` to remove session file
  - Add `dirs` crate for cross-platform home directory detection
  - _Requirements: 3.1, 3.4, 3.5_

- [ ]* 5.2 Write property test for file permissions
  - **Property 5: Session storage file permissions**
  - **Validates: Requirements 3.4**
  - Create session file, verify permissions are 0600
  - _Requirements: 3.4_

- [ ]* 5.3 Write unit tests for session store
  - Test file creation and directory creation
  - Test token save and load round-trip
  - Test handling of missing/corrupted files
  - Test delete removes file
  - _Requirements: 3.1, 3.4, 3.5_

- [x] 5.4 Implement session file cleanup and validation
  - Ensure only ONE session file exists per user (prevent multiple session files)
  - On session save, check for and remove any old/stale session files
  - Validate session file format on load (handle corrupted files gracefully)
  - Add logging to track session file operations for debugging
  - Implement atomic file writes to prevent partial writes
  - _Requirements: 3.1, 3.4_

- [x] 6. Implement OAuth flow in TUI
- [x] 6.1 Create authentication flow module
  - Create `fido-tui/src/auth.rs` with `AuthFlow` struct
  - Implement `check_existing_session()` to load and validate stored token
  - Implement `initiate_github_oauth()` to call server and get auth URL
  - Implement `open_browser()` to open system browser
  - Implement `poll_for_session()` to wait for OAuth completion
  - Add `webbrowser` crate for cross-platform browser opening
  - _Requirements: 2.1, 2.2, 2.3, 3.1, 3.2_

- [x] 6.2 Update TUI main loop to handle authentication
  - Check for existing session on startup
  - If no session, show authentication screen with test users and GitHub option
  - Show only 3 test users (alice, bob, charlie) for auto-login selection
  - Add "Login with GitHub" button/option on authentication screen
  - Display GitHub OAuth instructions when GitHub option selected
  - Open browser when user initiates GitHub auth
  - Poll server for session completion
  - Save session token on success
  - _Requirements: 2.1, 2.2, 2.3, 3.1_

- [x] 6.3 Add logout functionality
  - Verify Shift+L logout works for both test users AND GitHub users
  - Call server logout endpoint to invalidate session
  - Delete local session file (`~/.fido/session`)
  - Return to authentication screen (showing test users + GitHub option)
  - Ensure logout clears all session state properly
  - _Requirements: 3.5_

- [x] 6.4 Implement error handling for auth failures
  - Handle browser opening failures (show manual URL)
  - Handle OAuth timeout (5 minutes)
  - Handle server connection errors
  - Handle invalid/expired sessions
  - Display user-friendly error messages
  - _Requirements: 7.1, 7.2, 7.3, 7.4_

- [ ]* 6.5 Write integration tests for TUI auth flow
  - Test session loading on startup
  - Test OAuth initiation and browser opening
  - Test session saving after successful auth
  - Test logout clears session
  - _Requirements: 2.1, 2.2, 2.3, 3.1, 3.5_

- [x] 7. Configure server URL in TUI
- [x] 7.1 Update API client to use production server URL
  - Update `ApiClient::default()` to use `https://fido-social.fly.dev`
  - Support `FIDO_SERVER_URL` environment variable override
  - _Requirements: 1.2, 1.4, 8.1_


- [x] 8. Prepare Cargo crates for publishing
- [x] 8.1 Update workspace Cargo.toml metadata
  - Add repository URL
  - Add homepage URL
  - Ensure version, authors, license are set
  - _Requirements: 10.1, 10.2_

- [x] 8.2 Update fido-types Cargo.toml
  - Add description: "Shared types for the Fido social platform"
  - Add keywords and categories
  - Ensure all metadata is complete
  - _Requirements: 10.1_

- [x] 8.3 Rename and update fido-tui Cargo.toml
  - Change package name from `fido-tui` to `fido`
  - Add description: "A blazing-fast, keyboard-driven social platform for developers"
  - Add keywords: ["tui", "social", "terminal", "ratatui"]
  - Add categories: ["command-line-utilities"]
  - Keep directory name as `fido-tui` (doesn't affect package name)
  - _Requirements: 1.1, 10.2_

- [x] 8.4 Update fido-tui dependencies for publishing
  - Change fido-types dependency from path to version
  - Ensure all dependencies use published versions
  - Test build with published dependencies
  - _Requirements: 10.2, 10.5_

- [x] 9. Set up Fly.io deployment
- [x] 9.1 Create Dockerfile for server
  - Create multi-stage Dockerfile
  - Build fido-server in builder stage
  - Use slim Debian image for runtime
  - Copy binary and set up entrypoint
  - Expose port 3000
  - _Requirements: 5.1_

- [x] 9.2 Update fly.toml configuration
  - Set correct app name
  - Configure build to use Dockerfile
  - Set environment variables (DATABASE_PATH, HOST, PORT)
  - Configure persistent volume mount
  - Configure HTTP service and ports
  - _Requirements: 5.1, 5.2, 5.3_

- [x] 9.3 Update server to bind to 0.0.0.0
  - Change default host from 127.0.0.1 to 0.0.0.0 for production
  - Use HOST environment variable if set
  - _Requirements: 5.1_

- [x] 9.4 Test local Docker build
  - Build Docker image locally
  - Run container with volume mount
  - Test API endpoints work
  - Test database persistence
  - _Requirements: 5.1, 5.2_

- [ ] 10. Deploy to Fly.io
- [x] 10.1 Initialize Fly.io application
  - Run `flyctl launch --no-deploy`
  - Verify fly.toml configuration
  - _Requirements: 5.1_

- [x] 10.2 Create persistent volume
  - Run `flyctl volumes create fido_data --size 1`
  - Verify volume is created in correct region
  - _Requirements: 5.2_

- [x] 10.3 Set GitHub OAuth secrets
  - Run `flyctl secrets set GITHUB_CLIENT_ID=xxx`
  - Run `flyctl secrets set GITHUB_CLIENT_SECRET=yyy`
  - Update GitHub OAuth app callback URL to production URL
  - _Requirements: 5.4_

- [x] 10.4 Deploy server to Fly.io
  - Run `flyctl deploy`
  - Monitor deployment logs
  - Verify server starts successfully
  - _Requirements: 5.1, 5.3_

- [x] 10.5 Test production deployment
  - Test health endpoint: `curl https://fido-social.fly.dev/health`
  - Test OAuth flow with production server
  - Test API endpoints work
  - Test database persistence after restart
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5_



- [ ] 11. Clean up current repository
- [x] 11.1 Remove temporary documentation files
  - Remove all FIXES_*, BUGFIXES, DM_FIXES, etc. markdown files
  - Remove test verification markdown files
  - Remove implementation summary files
  - Keep only essential docs (README, QUICKSTART, DEPLOYMENT, ARCHITECTURE)
  - _Requirements: 11.3, 11.5_

- [x] 11.2 Remove test and debug files
  - Remove test SQL files (test_*.sql, check_*.sql, add_*.sql)
  - Remove debug logs (debug.log, deploy.log)
  - Remove test scripts (test_*.sh, test_*.ps1, test_*.md)
  - Keep only production scripts (start.sh)
  - _Requirements: 11.3, 11.5_

- [x] 11.3 Clean up configuration files
  - Review and clean .gitignore
  - Remove unnecessary Docker files if any
  - Verify fly.toml is production-ready
  - _Requirements: 11.3_

- [ ] 12. Update documentation
- [x] 12.1 Update README with production instructions
  - Add installation section: `cargo install fido`
  - Add first-run instructions with GitHub OAuth
  - Add production server URL
  - Add troubleshooting section
  - _Requirements: 6.1, 6.2, 6.3_

- [x] 12.2 Update QUICKSTART guide
  - Update for production installation
  - Add GitHub OAuth setup instructions
  - Add session management explanation
  - _Requirements: 6.1, 6.2_

- [x] 12.3 Update DEPLOYMENT guide
  - Add Fly.io deployment instructions
  - Add GitHub OAuth app setup
  - Add CI/CD setup instructions
  - Add monitoring and troubleshooting
  - _Requirements: 6.4_

- [x] 12.4 Create CONTRIBUTING guide
  - Add development setup instructions
  - Add local testing with `--server` flag
  - Add contribution guidelines
  - _Requirements: 8.4_

- [ ] 13. Verify build and tests
- [x] 13.1 Run full build
  - Run `cargo build --release` in workspace
  - Verify all crates build successfully
  - Check for any warnings or issues
  - _Requirements: 11.3, 11.4_

- [x] 13.2 Run all tests
  - Run `cargo test --workspace`
  - Verify all tests pass
  - Address any test failures
  - _Requirements: 11.4_

- [x] 13.3 Test local server
  - Run `cargo run --bin fido-server`
  - Test API endpoints work
  - Test database operations
  - _Requirements: 11.4_

- [x] 13.4 Test TUI client
  - Run `cargo run --bin fido`
  - Test authentication flow
  - Test core features
  - _Requirements: 11.4_

- [ ] 14. Prepare for crates.io publishing


- [ ] 14.2 Verify package metadata completeness
  - Check all required fields in fido-types/Cargo.toml
  - Check all required fields in fido-tui/Cargo.toml (published as `fido`)
  - Ensure descriptions, keywords, and categories are set
  - _Requirements: 10.1, 10.2_

- [x] 14.3 Test local package build
  - Run `cargo package -p fido-types` to verify package builds
  - Run `cargo package -p fido` to verify TUI package builds
  - Check for any warnings or missing files
  - _Requirements: 10.5_

- [ ] 15. Publish crates to crates.io
- [ ] 15.1 Obtain crates.io API token
  - Login to crates.io with GitHub account
  - Generate API token from account settings
  - Store token securely for publishing
  - _Requirements: 10.1, 10.2_

- [ ] 15.2 Publish fido-types
  - Run `cargo publish -p fido-types`
  - Verify package appears on crates.io
  - Wait for package to be available (usually ~1 minute)
  - _Requirements: 10.1_

- [ ] 15.3 Update fido dependency and publish
  - Update fido-tui/Cargo.toml to use published fido-types version
  - Change `fido-types = { path = "../fido-types" }` to `fido-types = "0.1.0"`
  - Test build with published dependency: `cargo build -p fido`
  - Run `cargo publish -p fido`
  - Verify package appears on crates.io
  - _Requirements: 10.2, 10.5_

- [ ] 15.4 Test installation from crates.io
  - Wait 5-10 minutes for crates.io to fully index the package
  - Run `cargo install fido` on a clean machine or in a fresh directory
  - Verify binary is installed correctly: `which fido` or `where fido`
  - Test running `fido` command
  - Test GitHub Device Flow authentication works
  - Test connecting to production server
  - _Requirements: 1.1, 1.2, 2.1, 2.2, 10.5_

- [ ] 16. Final testing and polish
- [ ] 16.1 End-to-end testing
  - Test fresh install: `cargo install fido`
  - Test first launch and GitHub Device Flow authentication
  - Test session persistence across restarts
  - Test all core features (posts, DMs, profiles, hashtags, friends)
  - Test logout (Shift+L) and re-authentication
  - _Requirements: 1.1, 2.1, 2.2, 2.3, 3.1, 3.2_

- [ ] 16.2 Test server URL overrides
  - Test with environment variable: `FIDO_SERVER_URL=http://localhost:3000 fido`
  - Test default production URL works: `fido` (should connect to https://fido-social.fly.dev)
  - Verify Settings tab shows correct server URL
  - _Requirements: 8.1, 8.3_

- [ ] 16.3 Test error scenarios
  - Test with server offline (connection error message)
  - Test with expired session (automatic re-authentication prompt)
  - Test with invalid session token (automatic re-authentication prompt)
  - Test Device Flow timeout (15 minute timeout with clear error)
  - Test browser opening failure (manual URL displayed)
  - Test canceling Device Flow with Esc key
  - _Requirements: 7.1, 7.2, 7.3, 7.4_

- [ ] 16.4 Performance testing
  - Test with large datasets (scroll through 100+ posts)
  - Test session validation performance (should be instant)
  - Test Device Flow polling (should poll every 5 seconds)
  - Monitor server resource usage on Fly.io
  - _Requirements: 5.1_

- [ ] 16.5 Security review
  - Verify session tokens are cryptographically secure (UUID v4)
  - Verify session file permissions are 0600: `ls -la ~/.fido/session`
  - Verify HTTPS is enforced (check Fly.io configuration)
  - Verify Device Flow state validation works
  - Verify no secrets in client code (only client ID, no secret)
  - Verify session cleanup runs on server startup
  - _Requirements: 3.4, 4.4, 5.1_

- [ ] 16.6 Documentation review
  - Verify README has correct installation instructions
  - Verify QUICKSTART has Device Flow instructions
  - Verify DEPLOYMENT has Fly.io setup steps
  - Verify all documentation references production URL
  - Check for any outdated OAuth callback references
  - _Requirements: 6.1, 6.2, 6.3, 6.4_

- [ ] 17. Optional: Set up CI/CD automation
- [ ] 17.1 Create GitHub Actions workflow for server deployment
  - Create `.github/workflows/deploy-server.yml`
  - Configure automatic deployment to Fly.io on push to main
  - Add FLY_API_TOKEN to GitHub secrets
  - Test workflow with a test commit
  - _Requirements: 9.1, 9.2, 9.3_

- [ ] 17.2 Create GitHub Actions workflow for crate publishing
  - Create `.github/workflows/publish-crates.yml`
  - Configure automatic publishing on git tag (v*.*.*)
  - Add CARGO_TOKEN to GitHub secrets
  - Document release process in CONTRIBUTING.md
  - _Requirements: 9.2, 9.4_

- [ ] 18. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

# Implementation Plan

- [ ] 0. Open Source Contributor Setup: Clone repository and create feature branch
  - Clone the Fido repository from GitHub: `git clone https://github.com/ianjamesburke/fido.git`
  - Navigate to project directory: `cd fido`
  - Create and checkout new feature branch: `git checkout -b feature/web-terminal-interface`
  - Verify you're on the correct branch: `git branch --show-current`
  - Set up any required environment variables from `.env.example`

- [ ] 1. Set up local development environment and baseline
  - Review existing `web/` directory (already has index.html, style.css, script.js)
  - Install and configure nginx locally for development
  - Install ttyd for terminal-to-web functionality
  - Create `start.sh` script to coordinate all services (nginx, ttyd, API server)
  - Verify existing Fido TUI and API server work in native mode
  - Document local development setup and port assignments
  - _Requirements: 5.1, 5.2, 5.3_

- [ ] 1.1 Verification: Local development environment works
  - Run `cargo build` to ensure Fido project compiles successfully
  - Start API server: `cd fido-server && cargo run` - verify it runs on port 3000
  - Start TUI: `cargo run` - verify native Fido TUI launches and works
  - Test API endpoints: `curl http://localhost:3000/posts` returns valid response
  - Install nginx and ttyd, verify they can be started locally
  - Create and test initial `./start.sh` script that coordinates all services

- [ ] 2. Configure GitHub OAuth for local development
  - Create new OAuth app at https://github.com/settings/developers
  - Set Authorization callback URL to `http://localhost:3000/auth/github/callback`
  - Check "Enable Device Flow" box for web terminal authentication
  - Generate Client ID and Client Secret
  - Set environment variables: `GITHUB_CLIENT_ID` and `GITHUB_CLIENT_SECRET`
  - Configure `FIDO_SERVER_URL=http://localhost:3000` for local development
  - Update `.env` file with OAuth configuration
  - _Requirements: 2.1, 2.2_

- [ ] 2.1 Verification: GitHub OAuth configuration works
  - Verify environment variables are set: `echo $GITHUB_CLIENT_ID`
  - Start API server and confirm OAuth endpoints are available
  - Test OAuth flow: `curl http://localhost:3000/auth/github` returns redirect
  - Verify callback URL is configured correctly in GitHub app settings
  - Test device flow authentication works for web terminal mode

- [ ] 3. Set up web mode detection and configuration
  - Create mode detection system using `FIDO_WEB_MODE` environment variable
  - Implement configuration switching between file and browser storage
  - Add web mode flags to application startup
  - _Requirements: 4.1, 4.5, 2.5_

- [ ]* 3.1 Write property test for mode detection
  - **Property 7: Mode Detection Accuracy**
  - **Validates: Requirements 4.1**

- [ ] 3.2 Verification: Test mode detection works
  - Run `cargo build` to ensure project compiles successfully
  - Run `FIDO_WEB_MODE=true cargo run` and verify web mode is detected in logs
  - Run `cargo run` (without env var) and verify native mode is detected in logs
  - Confirm different storage paths are used for each mode

- [ ] 4. Implement storage adapter system
  - Create storage adapter trait for credential management
  - Implement file-based storage adapter for native mode
  - Implement browser storage adapter for web mode using JavaScript bridge
  - Add storage adapter factory based on mode detection
  - _Requirements: 2.1, 2.5, 4.2_

- [ ]* 4.1 Write property test for authentication storage mode selection
  - **Property 2: Authentication Storage Mode Selection**
  - **Validates: Requirements 2.1, 2.5**

- [ ]* 4.2 Write property test for session cleanup on logout
  - **Property 4: Session Cleanup on Logout**
  - **Validates: Requirements 2.3, 2.4**

- [ ] 4.3 Verification: Test storage adapters work correctly
  - Run `cargo build` to ensure project compiles successfully
  - Test file storage: Run app in native mode, verify credentials save to `.fido/` directory
  - Test browser storage: Run app in web mode, verify no local files created
  - Verify storage adapter factory selects correct adapter based on mode

- [ ] 5. Create test user isolation system
  - Implement user context system with test/real user types
  - Create database adapter with isolation support
  - Add test user data reset mechanism
  - Implement data filtering to prevent test user content in production feeds
  - _Requirements: 3.1, 3.2, 3.3, 3.4_

- [ ]* 5.1 Write property test for test user data isolation
  - **Property 5: Test User Data Isolation**
  - **Validates: Requirements 3.1, 3.3, 3.4**

- [ ]* 5.2 Write property test for test user data reset
  - **Property 6: Test User Data Reset on Load**
  - **Validates: Requirements 3.2**

- [ ] 5.3 Verification: Test user isolation works correctly
  - Run `cargo build` to ensure project compiles successfully
  - Create test user posts and verify they don't appear in production feeds
  - Restart application and verify test user data is reset to clean state
  - Verify real user data remains untouched during test user operations

- [ ] 6. Update API server for web mode support
  - Add web session management endpoints
  - Implement test user data isolation in API routes
  - Add user context detection in request handlers
  - Ensure API routes maintain no "/api" prefix
  - _Requirements: 2.2, 3.1, 5.4_

- [ ]* 6.1 Write property test for authenticated user data access
  - **Property 3: Authenticated User Data Access**
  - **Validates: Requirements 2.2**

- [ ]* 6.2 Write property test for API route prefix absence
  - **Property 10: API Route Prefix Absence**
  - **Validates: Requirements 5.4**

- [ ] 6.3 Verification: API server supports web mode correctly
  - Run `cargo build` to ensure project compiles successfully
  - Start API server on port 3000: `cd fido-server && cargo run`
  - Test API routes work without "/api" prefix: `curl http://localhost:3000/posts`
  - Verify web session endpoints respond correctly
  - Confirm user context detection works for both test and real users

- [ ] 7. Create nginx configuration
  - Configure nginx to serve static files on port 8080
  - Set up API proxy to forward requests to port 3000 without path modification
  - Configure terminal proxy to route `/terminal/` to ttyd on port 7681
  - Ensure proper CORS headers for web terminal
  - _Requirements: 5.1, 5.2, 5.3, 5.5_

- [ ]* 7.1 Write property test for nginx path preservation
  - **Property 11: Nginx Path Preservation**
  - **Validates: Requirements 5.5**

- [ ] 7.2 Verification: nginx configuration works correctly
  - Install and configure nginx with the new configuration
  - Start nginx on port 8080 and verify static files are served
  - Test API proxy: `curl http://localhost:8080/posts` should reach API server
  - Test terminal proxy: verify `/terminal/` routes to ttyd service
  - Confirm CORS headers are present for web terminal requests

- [ ] 8. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 9. Implement web interface with terminal integration
  - Update HTML to include terminal iframe pointing to ttyd
  - Add dark theme CSS for terminal container
  - Implement JavaScript for terminal initialization and communication
  - Add authentication UI for GitHub login in web mode
  - _Requirements: 1.1, 7.1, 7.2_

- [ ]* 9.1 Write property test for keyboard shortcut consistency
  - **Property 1: Keyboard Shortcut Consistency**
  - **Validates: Requirements 1.2**

- [ ]* 9.2 Write property test for ANSI color code support
  - **Property 12: ANSI Color Code Support**
  - **Validates: Requirements 7.5**

- [ ] 9.3 Verification: Web interface displays and functions correctly
  - Open browser and navigate to `http://localhost:8080`
  - Verify dark theme CSS is applied and looks modern
  - Confirm terminal iframe loads and displays Fido TUI with proper colors and fonts
  - Test GitHub authentication UI appears and functions
  - **Critical: Click inside terminal iframe and verify keyboard input is captured**
  - **Critical: Test specific keyboard shortcuts (Tab, Enter, Ctrl+C, arrow keys) work in iframe**
  - Verify terminal text is readable and properly formatted
  - Confirm terminal responds to mouse clicks for focus

- [ ] 10. Update TUI for web mode compatibility
  - Modify TUI initialization to detect web mode
  - Implement browser storage integration for web sessions
  - Add web-specific authentication flow
  - Ensure keyboard shortcuts work consistently across modes
  - _Requirements: 1.2, 2.1, 4.1, 4.2_

- [ ]* 10.1 Write property test for cross-mode functional consistency
  - **Property 8: Cross-Mode Functional Consistency**
  - **Validates: Requirements 4.2, 4.3**

- [ ]* 10.2 Write property test for mode-specific configuration handling
  - **Property 9: Mode-Specific Configuration Handling**
  - **Validates: Requirements 4.5**

- [ ] 10.3 Verification: TUI works identically in both modes
  - Test native mode: Run `cargo run` and verify all features work
  - Test web mode: Run with `FIDO_WEB_MODE=true` and verify same functionality
  - Compare keyboard shortcuts between modes - they should be identical
  - Verify authentication flows work correctly in both modes
  - Confirm configuration settings apply correctly per mode

- [ ] 11. Configure ttyd for web terminal
  - Set up ttyd to spawn Fido TUI with `FIDO_WEB_MODE=true`
  - Configure dark theme and monospace font settings
  - Set ttyd to run on port 7681 with proper WebSocket configuration
  - Add terminal styling for modern aesthetics
  - _Requirements: 5.3, 7.1, 7.2, 7.5_

- [ ] 11.1 Verification: ttyd terminal service works correctly
  - Install ttyd and start it on port 7681
  - Verify ttyd spawns Fido TUI with `FIDO_WEB_MODE=true` environment variable
  - Open browser to `http://localhost:7681` and confirm terminal loads
  - Test dark theme and monospace font are applied correctly
  - **Critical: Verify terminal appearance matches native TUI (colors, layout, text)**
  - **Critical: Test keyboard input works directly in ttyd (before iframe integration)**
  - **Critical: Test all Fido keyboard shortcuts work in ttyd web terminal**
  - Verify WebSocket connection is stable and responsive
  - Confirm terminal handles special characters and Unicode properly

- [ ] 12. Update startup script and deployment
  - Modify start.sh to coordinate nginx, ttyd, and API server startup
  - Ensure proper port configuration (API: 3000, nginx: 8080, ttyd: 7681)
  - Add health checks for all services
  - Configure proper service shutdown and cleanup
  - _Requirements: 5.1, 5.2, 5.3_

- [ ] 12.1 Verification: All services start and coordinate correctly
  - Run `./start.sh` and verify all three services start successfully
  - Check API server is running on port 3000: `curl http://localhost:3000/posts`
  - Check nginx is running on port 8080: `curl http://localhost:8080`
  - Check ttyd is running on port 7681: open browser to `http://localhost:7681`
  - Verify health checks pass for all services
  - Test graceful shutdown when stopping start.sh

- [ ] 13. Add user documentation and warnings
  - Create clear documentation distinguishing test and real user modes
  - Add prominent warnings about test user data persistence
  - Implement user notifications for test data resets
  - Add guidance for creating real accounts
  - _Requirements: 6.1, 6.2, 6.3, 6.4_

- [ ] 13.1 Verification: User documentation is clear and accessible
  - Review documentation for clarity and completeness
  - Verify warnings about test user data are prominent and clear
  - Test that user notifications appear when test data is reset
  - Confirm guidance for creating real accounts is easy to follow
  - Have a non-technical person review the documentation for clarity

- [ ] 14. Terminal Functionality Verification: Comprehensive keyboard and display testing
  - **Visual Verification**: Confirm web terminal looks identical to native TUI
  - **Keyboard Input**: Test all keyboard shortcuts work in web terminal iframe
  - **Navigation**: Verify Tab, Shift+Tab, arrow keys navigate correctly
  - **Text Input**: Test typing in compose mode, search, and other text fields
  - **Special Keys**: Verify Ctrl+C, Ctrl+D, Escape, Enter work as expected
  - **Focus Management**: Confirm clicking in iframe captures keyboard focus
  - **Performance**: Verify keyboard input has no noticeable lag compared to native
  - **Browser Compatibility**: Test in Chrome, Firefox, Safari (if available)

- [ ] 15. Implement server configuration management
  - Add server URL configuration system to distinguish local vs production
  - Create configuration file or environment variable for `FIDO_SERVER_URL`
  - Set local development default to `http://localhost:3000`
  - Set production/crates.io default to `https://fido-social.fly.dev`
  - Add clear indication in TUI of which server is being used
  - Make server configuration easily discoverable in documentation
  - **Note**: Production server changes are outside scope - contributor cannot modify deployed server

- [ ] 15.1 Verification: Server configuration works correctly
  - Test local development: Verify TUI connects to `http://localhost:3000` by default
  - Test production mode: Set production URL to `https://fido-social.fly.dev` and verify TUI connects correctly
  - Verify TUI clearly shows which server it's connected to
  - Test configuration discovery: Ensure users can easily find and modify server settings
  - Test local terminal connecting to local server: Run TUI and verify it communicates with local API
  - **Note**: Only test production URL configuration - actual production server testing requires deployment access

- [ ] 16. Final checkpoint - Ensure all tests pass
  - Run all automated tests: `cargo test`
  - Ensure all unit tests, integration tests, and property-based tests pass
  - Fix any failing tests before proceeding to final verification

- [ ] 17. End-to-End Verification: Complete web terminal interface works
  - Run `./start.sh` to start all services (API, nginx, ttyd)
  - Open browser to `http://localhost:8080` and verify web interface loads
  - **Critical: Verify terminal iframe captures keyboard input immediately on page load**
  - **Critical: Test complete user workflow using only keyboard in web terminal**
  - Test as test user: Use demo account, create posts, verify they don't pollute production
  - Test as real user: Authenticate with GitHub, verify access to real data
  - Test data isolation: Restart services, confirm test data resets but real data persists
  - Test cross-mode consistency: Compare web terminal behavior with native TUI
  - Verify all port configurations are correct (API:3000, nginx:8080, ttyd:7681)
  - Confirm documentation and warnings are clear and helpful

- [ ] 18. Comprehensive User Experience Testing
  - **Digital Testing**: Verify all automated tests pass (`cargo test`)
  - **Web App Navigation**: Navigate to local page (`http://localhost:8080`) and use web app as test user
  - **GitHub Authentication**: Log in to web app using GitHub OAuth and verify functionality
  - **Local Terminal Integration**: Run local terminal and verify it connects to local server
  - **Cross-Platform Testing**: Test web terminal in multiple browsers (Chrome, Firefox, Safari)
  - **Performance Testing**: Verify keyboard input responsiveness matches native TUI
  - **Feature Parity**: Confirm all TUI features work identically in web terminal

- [ ] 19. Pre-Pull Request Validation
  - Run final test suite: `cargo test --all`
  - Verify no compiler warnings: `cargo clippy`
  - Check code formatting: `cargo fmt --check`
  - Test complete user workflows in both native and web modes
  - Verify all configuration documentation is accurate and discoverable
  - Confirm all new files are properly committed to feature branch
  - Review changes for any sensitive information (API keys, passwords)

- [ ] 20. Submit Pull Request
  - Commit all changes to feature branch: `git add . && git commit -m "Add web terminal interface"`
  - Push feature branch to GitHub: `git push origin feature/web-terminal-interface`
  - Create pull request on GitHub with detailed description of changes
  - Include testing instructions and verification steps in PR description
  - Add screenshots or demo links if applicable
  - Request review from project maintainers
  - Respond to any feedback and make requested changes
  - Ensure CI/CD pipeline passes all checks before requesting merge
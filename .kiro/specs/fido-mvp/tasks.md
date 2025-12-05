# Implementation Plan

- [x] 1. Set up Rust workspa/e structure and shared types



  - Create workspace Cargo.toml with fido-types, fido-server, fido-tui, and fido-cli crates
  - Define shared data structures in fido-types (User, Post, Vote, DirectMessage, UserProfile, UserConfig)
  - Implement serialization/deserialization with serde for all types
  - Define enums (VoteDirection, ColorScheme, SortOrder) with proper derives
  - _Requirements: 1.1, 2.1, 3.1, 4.1, 5.1, 6.1, 8.1_

- [x] 2. Implement SQLite database layer and migrations




  - [x] 2.1 Create database schema with all tables (users, posts, hashtags, votes, direct_messages, user_configs)

    - Write SQL migration scripts for table creation with proper constraints
    - Implement foreign key relationships and indexes
    - Add CHECK constraints for content length and vote direction
    - Add indexes on created_at for post sorting and user_id for votes
    - _Requirements: 2.4, 3.4, 4.2, 5.3, 6.5, 8.4_
  - [x] 2.2 Implement database connection pooling and initialization

    - Create database connection manager with rusqlite
    - Implement connection pooling for concurrent access
    - Add database initialization logic with test data seeding
    - _Requirements: 1.1, 1.5_

  - [x] 2.3 Create repository layer for data access

    - Implement UserRepository with CRUD operations
    - Implement PostRepository with hashtag extraction and vote counting
    - Implement HashtagRepository to store extracted hashtags in hashtags table
    - Implement VoteRepository with upsert logic for vote updates
    - Implement DirectMessageRepository with conversation queries
    - Implement ConfigRepository with default configuration handling
    - _Requirements: 2.4, 3.4, 4.2, 4.3, 5.3, 6.4, 6.5_

- [x] 3. Build Axum REST API server




  - [x] 3.1 Set up Axum server with routing and middleware


    - Create main server application with Axum
    - Configure CORS and JSON middleware
    - Implement error handling middleware with proper error responses
    - Set up shared application state with database pool
    - _Requirements: 2.4, 3.4, 4.5_
  - [x] 3.2 Implement authentication endpoints








    - Create GET /users/test endpoint to list test users
    - Create POST /auth/login endpoint for test user selection
    - Create POST /auth/logout endpoint
    - Implement simple session management with local storage
    - _Requirements: 1.1, 1.2, 1.3, 1.4_
  - [x] 3.3 Implement post management endpoints



    - Create GET /posts endpoint with limit and sort query parameters
    - Create POST /posts endpoint with 280 character validation (validate authenticated user)
    - Implement hashtag extraction logic using regex (match #word pattern)
    - Store extracted hashtags in hashtags table via HashtagRepository on every post create/update
    - Create POST /posts/{id}/vote endpoint with vote upsert logic (validate authenticated user)
    - _Requirements: 2.4, 2.5, 3.2, 3.3, 3.4, 3.5, 4.2, 4.3, 4.4_
  - [x] 3.4 Implement profile management endpoints



    - Create GET /users/{id}/profile endpoint with karma calculation
    - Create PUT /users/{id}/profile endpoint for bio updates with authorization check (users can only edit their own bio)
    - Create GET /users/{id}/hashtags endpoint querying hashtags table for recent usage
    - Implement karma calculation by summing upvotes across all user posts (consider caching for performance)
    - Update profile stats dynamically when votes or posts change
    - _Requirements: 8.1, 8.2, 8.3, 8.4_
  - [x] 3.5 Implement direct messaging endpoints



    - Create GET /dms/conversations endpoint to list conversations (only for authenticated user)
    - Create GET /dms/conversations/{user_id} endpoint for message history (validate user is participant)
    - Create POST /dms endpoint to send messages (validate sender authentication)
    - Implement unread message tracking logic
    - _Requirements: 5.1, 5.2, 5.3, 5.5_
  - [x] 3.6 Implement configuration endpoints


    - Create GET /config endpoint to retrieve user configuration
    - Create PUT /config endpoint to update preferences
    - Implement default configuration with 25 post limit
    - Add validation for config values (max_posts > 0, valid color scheme, valid sort order)
    - _Requirements: 6.1, 6.2, 6.4, 6.5_

- [x] 4. Create Ratatui TUI foundation




  - [x] 4.1 Set up Ratatui application structure


    - Create main TUI application with event loop
    - Implement terminal initialization and cleanup
    - Set up keyboard event handling with crossterm
    - Create application state management structure
    - _Requirements: 2.3, 7.1, 7.4_
  - [x] 4.2 Implement HTTP client for API communication


    - Create API client wrapper using reqwest with UTF-8 emoji support
    - Implement error handling for network requests
    - Add retry logic for failed requests
    - Create response parsing utilities
    - Add placeholder/hook for future WebSocket integration (comment where it would connect)
    - _Requirements: 2.5, 4.5_
  - [x] 4.3 Build authentication screen


    - Create test user selection list UI
    - Implement arrow key navigation for user selection
    - Add login action on Enter key press
    - Store authenticated user locally
    - _Requirements: 1.1, 1.2, 1.3_

- [-] 5. Implement main tabbed interface


  - [x] 5.1 Create tab navigation system


    - Build tab header with Posts, DMs, Profile, Settings tabs
    - Implement Tab key navigation between tabs
    - Add visual highlighting for active tab
    - Create keyboard shortcuts footer display
    - _Requirements: 7.1, 7.2, 7.3, 7.5_
  - [x] 5.2 Build Posts tab with global feed



    - Create post list rendering with username, timestamp, vote counts
    - Implement j/k arrow key navigation through posts
    - Add visual highlighting for selected post
    - Display hashtags with special formatting
    - Show post limit indicator at bottom
    - _Requirements: 2.1, 2.2, 2.3, 3.3, 6.2, 6.3, 10.1, 10.2, 10.3, 10.4, 10.5_
  - [x] 5.3 Implement voting functionality in Posts tab



    - Add u/d keyboard shortcuts for upvote/downvote
    - Send vote requests to API server
    - Update vote counts in UI after successful vote
    - Trigger profile karma recalculation when viewing Profile tab after voting
    - Display error messages on vote failure
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_
  - [x] 5.4 Create new post modal



    - Build centered modal overlay with text input
    - Implement character counter with 280 limit
    - Add real-time hashtag highlighting as user types
    - Handle Ctrl+Enter to submit and Esc to cancel
    - Clear input and refresh feed after successful post
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_
  - [x] 5.5 Build Profile tab



    - Display profile stats (username, bio, karma, post count, join date, recent hashtags)
    - Show list of user's own posts with vote counts
    - Implement e key to edit bio for own profile
    - Add j/k navigation through personal posts
    - Refresh profile stats dynamically when switching to Profile tab
    - _Requirements: 8.1, 8.2, 8.3, 8.5_
  - [x] 5.6 Build DMs tab



    - Create conversation list view
    - Implement conversation selection with arrow keys
    - Display message history for selected conversation
    - Add message input with Ctrl+Enter to send
    - Show unread message indicators
    - _Requirements: 5.1, 5.2, 5.3, 5.5_
  - [x] 5.7 Build Settings tab



    - Create settings form with color scheme options (Default, Dark, Light, Solarized)
    - Add sort order preference selection (Newest, Popular, Controversial)
    - Implement max posts display count input with validation (must be > 0)
    - Display validation errors for invalid inputs
    - Save settings to API on change
    - Load saved preferences on startup
    - _Requirements: 6.1, 6.2, 6.4, 6.5_

- [ ] 6. Implement emoji support
  - [x] 6.1 Add emoji rendering in posts



    - Integrate emoji rendering library for terminal
    - Implement emoji shortcode parsing (e.g., :smile:)
    - Count emojis correctly toward 280 character limit
    - Ensure consistent spacing and alignment
    - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5_

- [ ] 7. Build CLI interface for direct commands
  - [x] 7.1 Create fido-cli crate with clap



    - Set up CLI argument parsing with clap
    - Implement `fido dm @user "message"` command
    - Add emoji support in CLI messages (parse shortcodes and render emojis)
    - Ensure reqwest serialization handles emoji UTF-8 correctly
    - Add authentication handling for CLI commands
    - Send DM via API and display confirmation
    - _Requirements: 5.4, 9.1, 9.4_

- [ ] 8. Implement configuration persistence
  - [x] 8.1 Create .fido directory management



    - Create .fido directory in user home on first run
    - Implement configuration file read/write utilities
    - Store user session data locally with unique session IDs
    - Support multiple concurrent instances with different users
    - Handle configuration migration on updates
    - _Requirements: 1.3, 1.5, 6.5_

- [ ] 9. Add keyboard shortcuts and help system
  - [x] 9.1 Implement context-aware help modal



    - Create help modal triggered by ? key
    - Display shortcuts relevant to current tab/context
    - Implement modal close on Esc or ? key
    - Prioritize context-specific shortcuts over global ones
    - _Requirements: 7.1, 7.2, 7.5_
  - [ ]* 9.2 Create comprehensive documentation
    - Document all API endpoints with request/response examples
    - Document data structures and database schema
    - Create keyboard shortcuts reference guide
    - Add inline code comments for complex logic
    - _Requirements: 7.2_

- [ ] 10. Implement logout and account switching
  - [x] 10.1 Add logout functionality



    - Create logout keyboard shortcut or menu option
    - Clear local session data on logout
    - Return to authentication screen
    - Support multiple instances with different users
    - _Requirements: 1.4, 1.5_

- [ ] 11. Polish and error handling
  - [x] 11.1 Implement comprehensive error handling



    - Add network error retry UI
    - Display validation errors inline
    - Handle authentication errors gracefully
    - Show loading indicators for API requests
    - _Requirements: 4.5_
  - [x] 11.2 Optimize TUI performance





    - Implement lazy rendering for visible posts only
    - Add efficient scrolling for large post lists (handle thousands of posts gracefully)
    - Minimize redraws to changed UI portions only
    - Test performance with large datasets (1000+ posts)
    - _Requirements: 10.5_
  - [x] 11.3 Ensure modular architecture for future enhancements



    - Use trait-based APIs for easy mocking and testing
    - Separate concerns (text-entry, rendering, networking, config)
    - Design API client to support future WebSocket integration
    - Structure code for easy database migration (SQLite → Postgres)
    - _Requirements: All_

- [ ] 12. Integration and end-to-end testing
  - [x] 12.1 Create test data and seed database



    - Generate test users (alice, bob, charlie)
    - Create sample posts with hashtags and votes
    - Add sample direct messages between users
    - Set up test configurations
    - _Requirements: 1.1, 2.1, 5.1_
  - [x] 12.2 Test complete user workflows



    - Test login → post creation → voting → logout flow
    - Test direct messaging between users
    - Test profile viewing and bio editing
    - Test configuration changes and persistence
    - Verify keyboard navigation across all tabs
    - _Requirements: 1.1-1.5, 2.1-2.5, 3.1-3.5, 4.1-4.5, 5.1-5.5, 6.1-6.5, 7.1-7.5, 8.1-8.5_
  - [ ]* 12.3 Test edge cases and concurrency scenarios
    - Test posts with exactly 280 characters including emojis
    - Test emoji + hashtags at character limit boundary
    - Test max posts config values beyond UI limits
    - Test simultaneous voting conflicts (up/down from different users)
    - Test concurrent DM sending between multiple users
    - Test config persistence across multiple app instances
    - Test hashtag extraction with special characters and edge cases
    - _Requirements: 3.2, 4.2, 5.3, 6.5, 9.2_
  - [ ]* 12.4 Test security and authorization
    - Test users cannot edit other users' bios
    - Test users can only view their own DM conversations
    - Test authentication is required for all protected endpoints
    - Test session isolation between multiple app instances
    - _Requirements: 1.2, 1.3, 5.1, 8.3_

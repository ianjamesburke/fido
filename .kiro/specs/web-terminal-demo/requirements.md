# Requirements Document

## Introduction

This document specifies the requirements for a web-based terminal demo of Fido, allowing visitors to experience the terminal UI directly in their browser. The demo operates in complete isolation from the production system—using ephemeral in-memory data that resets on page refresh. Visitors can log in as a test user and interact with pre-populated sample posts and other test users, providing an authentic feel for the real application without affecting production data.

## Glossary

- **Demo Mode**: A runtime configuration where the TUI operates with mock data instead of connecting to the real API server
- **Mock Backend**: An in-memory data store that simulates API responses without network calls
- **Test User**: Pre-defined user accounts available for demo login (e.g., demo_user, alice, bob)
- **Ephemeral Data**: Data that exists only in browser memory and is lost on page refresh
- **xterm.js**: A terminal emulator library for web browsers
- **WASM**: WebAssembly, allowing Rust code to run in the browser
- **ttyd**: A tool that shares terminal sessions over HTTP (alternative approach)

## Requirements

### Requirement 1

**User Story:** As a website visitor, I want to try Fido in my browser, so that I can experience the terminal interface without installing anything.

#### Acceptance Criteria

1. WHEN a visitor loads the demo page THEN the Web_Terminal_Demo SHALL display a terminal interface within 5 seconds
2. WHEN the terminal initializes THEN the Web_Terminal_Demo SHALL display a welcome message explaining demo mode limitations
3. WHEN the demo loads THEN the Web_Terminal_Demo SHALL present a login screen with available test users
4. WHEN a visitor selects a test user THEN the Web_Terminal_Demo SHALL authenticate the user and display the main feed

### Requirement 2

**User Story:** As a demo user, I want to see sample content in the feed, so that I can understand how the platform works.

#### Acceptance Criteria

1. WHEN a demo user views the posts feed THEN the Mock_Backend SHALL return pre-populated sample posts from multiple test users
2. WHEN sample posts are displayed THEN the Mock_Backend SHALL include posts with hashtags, varying vote counts, and replies
3. WHEN a demo user scrolls the feed THEN the Web_Terminal_Demo SHALL display posts in the configured sort order
4. WHEN a demo user filters by hashtag THEN the Mock_Backend SHALL return only posts containing that hashtag

### Requirement 3

**User Story:** As a demo user, I want to create posts, so that I can experience the posting workflow.

#### Acceptance Criteria

1. WHEN a demo user creates a post THEN the Mock_Backend SHALL store the post in memory and display it in the feed
2. WHEN a demo user creates a post with hashtags THEN the Mock_Backend SHALL extract and index the hashtags
3. WHEN a demo user votes on a post THEN the Mock_Backend SHALL update the vote count in memory
4. WHEN a demo user replies to a post THEN the Mock_Backend SHALL create a reply linked to the parent post

### Requirement 4

**User Story:** As a demo user, I want to send direct messages to test users, so that I can experience the DM functionality.

#### Acceptance Criteria

1. WHEN a demo user opens the DMs tab THEN the Mock_Backend SHALL display pre-populated conversations with test users
2. WHEN a demo user sends a message THEN the Mock_Backend SHALL store the message in memory and display it in the conversation
3. WHEN a demo user views a conversation THEN the Web_Terminal_Demo SHALL display messages in chronological order
4. WHEN a demo user starts a new conversation THEN the Mock_Backend SHALL create a conversation with the selected test user

### Requirement 5

**User Story:** As a demo user, I want all my actions to be temporary, so that I understand this is a sandbox environment.

#### Acceptance Criteria

1. WHEN the browser page is refreshed THEN the Mock_Backend SHALL reset all data to the initial demo state
2. WHEN the demo initializes THEN the Web_Terminal_Demo SHALL display a banner indicating data is ephemeral
3. WHEN a demo user creates content THEN the Mock_Backend SHALL store data only in browser memory
4. WHEN the browser tab is closed THEN the Mock_Backend SHALL discard all session data

### Requirement 6

**User Story:** As a demo user, I want the terminal to respond to keyboard input, so that I can navigate using the same shortcuts as the real application.

#### Acceptance Criteria

1. WHEN a demo user presses navigation keys THEN the Web_Terminal_Demo SHALL respond identically to the native TUI
2. WHEN a demo user uses keyboard shortcuts THEN the Web_Terminal_Demo SHALL execute the corresponding action
3. WHEN a demo user types in input fields THEN the Web_Terminal_Demo SHALL capture and display the input correctly
4. WHEN a demo user presses Tab THEN the Web_Terminal_Demo SHALL switch between tabs as in the native application

### Requirement 7

**User Story:** As a developer, I want the demo to reuse existing TUI code, so that the demo accurately represents the real application.

#### Acceptance Criteria

1. WHEN the demo is built THEN the Build_System SHALL compile the existing fido-tui crate for the web target
2. WHEN the demo runs THEN the Web_Terminal_Demo SHALL use the same UI rendering code as the native application
3. WHEN the demo needs data THEN the Web_Terminal_Demo SHALL use a mock API client implementing the same interface
4. WHEN the demo is updated THEN the Build_System SHALL require minimal changes to the core TUI code

### Requirement 8

**User Story:** As a website maintainer, I want the demo to be self-contained, so that it does not affect the production system.

#### Acceptance Criteria

1. WHEN the demo runs THEN the Web_Terminal_Demo SHALL make zero network requests to the production API
2. WHEN the demo is deployed THEN the Build_System SHALL produce static assets servable from any CDN
3. WHEN multiple visitors use the demo THEN the Mock_Backend SHALL maintain separate isolated sessions
4. WHEN the demo encounters an error THEN the Web_Terminal_Demo SHALL display a user-friendly message without exposing system details

# Requirements Document

## Introduction

Fido is a blazing-fast, keyboard-driven social platform for developers, featuring a beautiful terminal interface with no algorithmic feeds, no ads, just control and efficiency. The MVP focuses on Twitter-like short-form posting with hashtag support, user profiles, voting, direct messaging, and configuration management, all delivered through a lightning-fast terminal user interface optimized for developer workflows.

## Glossary

- **Fido_System**: The complete Fido terminal social platform application
- **TUI**: Terminal User Interface built with Ratatui
- **API_Server**: The Axum-based REST API backend server
- **Test_Auth**: Simple test user authentication system for development and testing
- **Global_Feed**: The main message board showing all user posts
- **DM_System**: Direct messaging functionality between users
- **Vote_System**: Upvote/downvote functionality for posts
- **Profile_System**: User profile management with stats and bio information
- **Hashtag_System**: Hashtag parsing and organization functionality
- **Config_System**: User configuration management for preferences including emoji display
- **CLI_Interface**: Command-line interface for direct actions
- **Quick_Post**: Simple inline text input for creating short posts

## Requirements

### Requirement 1

**User Story:** As a developer, I want to select from predefined test users to authenticate, so that I can quickly test the platform with different user accounts for rapid iteration.

#### Acceptance Criteria

1. WHEN the application starts, THE Fido_System SHALL display a list of available test users
2. WHEN a user selects a test user, THE Test_Auth SHALL authenticate them without external OAuth
3. WHEN authentication is successful, THE Fido_System SHALL store the selected user identity locally
4. WHEN the user wants to switch accounts, THE TUI SHALL provide an option to logout and select a different test user
5. WHEN multiple app instances run, THE Fido_System SHALL allow different test users to be selected in each instance

### Requirement 2

**User Story:** As a developer, I want to view a global message board of all posts, so that I can see what the community is discussing.

#### Acceptance Criteria

1. WHEN the user accesses the main interface, THE TUI SHALL display the Global_Feed with all posts
2. WHEN posts are displayed, THE Fido_System SHALL render plain text content with highlighted hashtags
3. WHEN the user navigates the feed, THE TUI SHALL support keyboard-only navigation
4. WHEN posts are loaded, THE API_Server SHALL return posts sorted by the user's preferred sort order
5. WHEN the feed is refreshed, THE Fido_System SHALL fetch the latest posts from the API_Server

### Requirement 3

**User Story:** As a developer, I want to create short posts with hashtag support, so that I can quickly share thoughts and categorize content for easy discovery.

#### Acceptance Criteria

1. WHEN the user initiates post creation, THE TUI SHALL display a Quick_Post input field with character counter
2. WHEN the user types content, THE Fido_System SHALL enforce a 280 character maximum limit
3. WHEN the user includes hashtags, THE Hashtag_System SHALL automatically parse and highlight hashtags in the text
4. WHEN the user submits a post, THE API_Server SHALL validate content length and store the post with extracted hashtags
5. WHEN a post is created successfully, THE Fido_System SHALL update the Global_Feed and clear the input field

### Requirement 4

**User Story:** As a developer, I want to upvote and downvote posts, so that I can express my opinion on content quality and relevance.

#### Acceptance Criteria

1. WHEN the user selects a post, THE TUI SHALL display voting options via keyboard shortcuts
2. WHEN the user votes on a post, THE API_Server SHALL record the vote in the Vote_System
3. WHEN a user has already voted on a post, THE Vote_System SHALL update the existing vote rather than create a duplicate
4. WHEN vote counts change, THE TUI SHALL update the display to reflect current vote totals
5. WHEN voting fails, THE Fido_System SHALL display an error message and maintain the previous state

### Requirement 5

**User Story:** As a developer, I want to send and receive direct messages, so that I can have private conversations with other users.

#### Acceptance Criteria

1. WHEN the user accesses the DM interface, THE TUI SHALL display a list of conversations
2. WHEN the user selects a conversation, THE TUI SHALL show the message history with that user
3. WHEN the user sends a message, THE API_Server SHALL store it in the DM_System
4. WHEN the user uses the CLI command, THE CLI_Interface SHALL send direct messages via `fido dm @user "message"`
5. WHEN new messages arrive, THE TUI SHALL indicate unread messages in the interface

### Requirement 6

**User Story:** As a developer, I want to configure my color scheme, sorting preferences, and maximum post display count, so that I can customize the interface to match my workflow and control my browsing habits.

#### Acceptance Criteria

1. WHEN the user accesses settings, THE TUI SHALL display configurable options for color schemes, sorting, and maximum posts to display
2. WHEN the user changes the maximum post count, THE Config_System SHALL limit the Global_Feed to show only that many posts
3. WHEN the user reaches the bottom of the post limit, THE TUI SHALL display a message indicating the limit has been reached
4. WHEN the application starts, THE Fido_System SHALL load saved preferences with a default maximum of 25 posts
5. WHEN settings are changed, THE Config_System SHALL store all preferences in the .fido directory

### Requirement 7

**User Story:** As a developer, I want complete keyboard navigation throughout the application, so that I can use the platform efficiently without touching the mouse.

#### Acceptance Criteria

1. WHEN the user is in any interface, THE TUI SHALL provide keyboard shortcuts for all available actions
2. WHEN the user presses help keys, THE TUI SHALL display available keyboard shortcuts for the current context
3. WHEN navigating between interface sections, THE TUI SHALL support tab-based or arrow-key navigation
4. WHEN performing actions, THE TUI SHALL provide immediate visual feedback for keyboard inputs
5. WHEN shortcuts conflict, THE TUI SHALL prioritize context-specific shortcuts over global ones
### Requirement 8

**User Story:** As a developer, I want to view and manage user profiles, so that I can see user statistics and learn about other community members.

#### Acceptance Criteria

1. WHEN the user views a profile, THE Profile_System SHALL display username, bio, karma score, post count, and join date
2. WHEN viewing a profile, THE TUI SHALL show the user's most recently used hashtags
3. WHEN the user edits their own profile, THE TUI SHALL allow updating bio information
4. WHEN calculating karma, THE Profile_System SHALL sum total upvotes received across all user posts
5. WHEN displaying profiles, THE TUI SHALL format information in a clean, readable layout

### Requirement 9

**User Story:** As a developer, I want basic emoji support in posts, so that I can add minimal expression to my content while keeping the interface clean.

#### Acceptance Criteria

1. WHEN posting content, THE Fido_System SHALL allow basic emoji input and render them in posts
2. WHEN emojis are used, THE TUI SHALL count them appropriately toward the 280 character limit
3. WHEN displaying posts, THE TUI SHALL render emojis consistently without breaking layout
4. WHEN emojis are entered, THE Fido_System SHALL support common emoji shortcodes (e.g., :smile:)
5. WHEN the interface displays emojis, THE TUI SHALL maintain consistent spacing and alignment

### Requirement 10

**User Story:** As a developer, I want posts to be displayed in a Twitter-like format, so that I can quickly scan and interact with short-form content.

#### Acceptance Criteria

1. WHEN viewing the Global_Feed, THE TUI SHALL display posts in a compact list with username, timestamp, and vote count
2. WHEN posts contain hashtags, THE TUI SHALL highlight hashtags for easy identification
3. WHEN posts are displayed, THE TUI SHALL show character-limited content without truncation (280 char max)
4. WHEN the user selects a post, THE TUI SHALL highlight the selected post for clear visual feedback
5. WHEN navigating posts, THE TUI SHALL maintain fast scrolling performance for quick browsing
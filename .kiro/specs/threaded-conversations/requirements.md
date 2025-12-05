# Requirements Document

## Introduction

This document defines the requirements for implementing threaded conversations and post interactions in Fido. The feature enables users to engage in discussions through replies, manage their own content through editing and deletion, and view detailed post information. This transforms posts from static content into interactive conversations while maintaining Fido's keyboard-driven, terminal-native experience.

## Glossary

- **Fido_TUI**: The terminal user interface application for the Fido platform
- **Post_Detail_View**: A dedicated screen displaying a single post with full content and replies
- **Reply**: A response to a post, creating a parent-child relationship between posts
- **Parent_Post**: The original post that a reply responds to
- **Feed_View**: The main screen displaying a scrollable list of posts
- **Action_Button**: A keyboard-triggered command displayed in the interface
- **Reply_Count**: The number of replies associated with a post
- **Post_Owner**: The user who created a specific post
- **Fido_API**: The backend REST API service for the Fido platform

## Requirements

### Requirement 1: Post Detail Navigation

**User Story:** As a Fido user, I want to view detailed information about any post, so that I can see the full content and engage with it.

#### Acceptance Criteria

1. WHEN the user presses Enter on a post in Feed_View, THEN Fido_TUI SHALL transition to Post_Detail_View for that post
2. WHILE displaying Post_Detail_View, Fido_TUI SHALL render the complete post content without truncation
3. WHILE displaying Post_Detail_View, Fido_TUI SHALL display the author username, timestamp, and current vote counts
4. WHEN the user presses ESC in Post_Detail_View, THEN Fido_TUI SHALL return to Feed_View at the previous scroll position
5. WHILE transitioning between views, Fido_TUI SHALL maintain state without visual flickering

### Requirement 2: Context-Sensitive Post Actions

**User Story:** As a Fido user, I want to see different action options based on post ownership, so that I can perform appropriate operations on posts.

#### Acceptance Criteria

1. WHEN Post_Owner views their own post in Post_Detail_View, THEN Fido_TUI SHALL display edit, delete, and reply action buttons
2. WHEN a user views another user's post in Post_Detail_View, THEN Fido_TUI SHALL display reply, upvote, and downvote action buttons
3. WHILE displaying action buttons, Fido_TUI SHALL show the corresponding keyboard shortcuts
4. WHEN the user presses a keyboard shortcut, THEN Fido_TUI SHALL execute the corresponding action
5. WHILE determining Post_Owner, Fido_TUI SHALL compare the post author identifier with the authenticated user identifier

### Requirement 3: Reply Creation

**User Story:** As a Fido user, I want to reply to posts, so that I can participate in conversations.

#### Acceptance Criteria

1. WHEN the user presses 'r' in Post_Detail_View, THEN Fido_TUI SHALL open the reply composer interface
2. WHILE composing a reply, Fido_TUI SHALL display the parent post author and truncated content as context
3. WHILE composing a reply, Fido_TUI SHALL support Markdown formatting in the text input
4. WHEN the user presses Ctrl+Enter in the reply composer, THEN Fido_TUI SHALL submit the reply to Fido_API
5. WHEN Fido_API successfully creates a reply, THEN Fido_API SHALL increment the Reply_Count on the Parent_Post
6. WHEN the user presses ESC in the reply composer, THEN Fido_TUI SHALL cancel reply creation and return to Post_Detail_View

### Requirement 4: Reply Display and Navigation

**User Story:** As a Fido user, I want to view all replies to a post, so that I can follow the conversation.

#### Acceptance Criteria

1. WHEN Post_Detail_View loads, THEN Fido_TUI SHALL fetch and display all replies for the post from Fido_API
2. WHILE displaying replies, Fido_TUI SHALL show each reply with author, timestamp, and vote counts
3. WHILE displaying replies, Fido_TUI SHALL render reply content with Markdown formatting
4. WHEN the user presses up or down arrow keys in Post_Detail_View, THEN Fido_TUI SHALL navigate between replies
5. WHILE a reply is selected, Fido_TUI SHALL highlight that reply visually
6. WHEN the user presses 'u' or 'd' on a selected reply, THEN Fido_TUI SHALL submit an upvote or downvote to Fido_API

### Requirement 5: Post Editing

**User Story:** As a Fido user, I want to edit my own posts, so that I can correct mistakes or update content.

#### Acceptance Criteria

1. WHEN Post_Owner presses 'e' on their post in Post_Detail_View, THEN Fido_TUI SHALL open the post editor
2. WHILE the post editor loads, Fido_TUI SHALL pre-populate the editor with the current post content
3. WHILE editing a post, Fido_TUI SHALL support Markdown formatting in the text input
4. WHEN the user presses Ctrl+Enter in the post editor, THEN Fido_TUI SHALL submit the updated content to Fido_API
5. WHEN Fido_API receives an edit request, THEN Fido_API SHALL verify the requesting user is Post_Owner before allowing the update
6. IF the requesting user is not Post_Owner, THEN Fido_API SHALL return a 403 Forbidden response
7. WHEN the user presses ESC in the post editor, THEN Fido_TUI SHALL cancel editing and return to Post_Detail_View

### Requirement 6: Post Deletion

**User Story:** As a Fido user, I want to delete my own posts, so that I can remove content I no longer want published.

#### Acceptance Criteria

1. WHEN Post_Owner presses 'x' on their post in Post_Detail_View, THEN Fido_TUI SHALL display a confirmation prompt
2. IF the post has one or more replies, THEN Fido_TUI SHALL display a warning indicating the Reply_Count in the confirmation prompt
3. WHEN the user confirms deletion, THEN Fido_TUI SHALL submit a delete request to Fido_API
4. WHEN Fido_API receives a delete request, THEN Fido_API SHALL verify the requesting user is Post_Owner before allowing deletion
5. IF the requesting user is not Post_Owner, THEN Fido_API SHALL return a 403 Forbidden response
6. WHEN Fido_API successfully deletes a post, THEN Fido_TUI SHALL return to Feed_View and display a confirmation message
7. WHEN the user cancels deletion, THEN Fido_TUI SHALL return to Post_Detail_View without changes

### Requirement 7: Reply Count Display in Feed

**User Story:** As a Fido user, I want to see how many replies a post has from the feed, so that I can identify active conversations.

#### Acceptance Criteria

1. WHILE displaying posts in Feed_View, Fido_TUI SHALL show the Reply_Count for each post
2. WHILE displaying Reply_Count, Fido_TUI SHALL position it alongside vote counts
3. WHEN a new reply is created, THEN Fido_TUI SHALL update the Reply_Count display in Feed_View
4. WHILE displaying Reply_Count, Fido_TUI SHALL use a visual indicator such as a speech bubble icon

### Requirement 8: Database Schema for Replies

**User Story:** As a system administrator, I want posts to support parent-child relationships, so that replies can be stored and retrieved efficiently.

#### Acceptance Criteria

1. THE Fido_API SHALL store a parent_post_id field for each post in the database
2. WHEN a post is a top-level post, THEN Fido_API SHALL set parent_post_id to NULL
3. WHEN a post is a reply, THEN Fido_API SHALL set parent_post_id to the identifier of the Parent_Post
4. THE Fido_API SHALL store a reply_count field for each post in the database
5. THE Fido_API SHALL create a database index on the parent_post_id field for query performance

### Requirement 9: Reply API Endpoints

**User Story:** As a frontend developer, I want REST API endpoints for reply operations, so that the TUI can create and fetch replies.

#### Acceptance Criteria

1. THE Fido_API SHALL provide a GET endpoint at /posts/{post_id}/replies that returns all replies for a post
2. THE Fido_API SHALL provide a POST endpoint at /posts/{post_id}/reply that creates a new reply
3. WHEN Fido_API returns replies, THEN Fido_API SHALL include author, timestamp, content, and vote counts for each reply
4. WHEN Fido_API creates a reply, THEN Fido_API SHALL return the created reply with all metadata
5. IF a post_id does not exist, THEN Fido_API SHALL return a 404 Not Found response

### Requirement 10: Error Handling and Network Resilience

**User Story:** As a Fido user, I want graceful error handling when network issues occur, so that I can understand what went wrong and retry if needed.

#### Acceptance Criteria

1. IF Fido_API returns an error response, THEN Fido_TUI SHALL display an error message to the user
2. IF a network request fails, THEN Fido_TUI SHALL display a network error message with retry option
3. IF a user attempts to view a deleted post, THEN Fido_TUI SHALL display a "Post deleted" placeholder message
4. WHILE an API request is in progress, Fido_TUI SHALL display a loading indicator
5. IF Fido_API returns a 403 Forbidden response, THEN Fido_TUI SHALL display an authorization error message

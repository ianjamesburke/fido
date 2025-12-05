# Design Document: Threaded Conversations & Post Interactions

## Overview

This design document outlines the architecture for implementing threaded conversations and post interactions in Fido. The feature enables users to reply to posts, edit and delete their own content, and navigate detailed post views. The design maintains Fido's keyboard-driven, terminal-native experience while adding social interaction capabilities.

The implementation follows a phased approach:
1. Post Detail View foundation
2. Database schema modifications for parent-child relationships
3. Reply system with single-level threading
4. Post editing and deletion with authorization
5. Feed integration showing reply counts

## Architecture

### High-Level Component Interaction

```
┌─────────────┐         ┌──────────────┐         ┌─────────────┐
│   Fido TUI  │ ◄─────► │  Fido API    │ ◄─────► │  Database   │
│             │  HTTP   │  (Axum)      │  SQL    │  (SQLite)   │
└─────────────┘         └──────────────┘         └─────────────┘
      │
      │ State Management
      ▼
┌─────────────────────────────────────────────────┐
│  App State                                      │
│  - Screen: Feed | PostDetail                    │
│  - PostDetailState: post_id, replies, selected  │
│  - InputMode: Navigation | Typing               │
└─────────────────────────────────────────────────┘
```

### State Management Pattern

The application follows the existing state management pattern seen in `app.rs`:
- Centralized `App` struct holds all application state
- Screen enum extended with `PostDetail` variant
- New `PostDetailState` struct manages post detail view state
- Input modes control keyboard shortcut availability

## Components and Interfaces

### 1. TUI Components (fido-tui)

#### 1.1 App State Extensions

```rust
// Add to Screen enum
#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Auth,
    Main,
}

// Add new state for post detail view
pub struct PostDetailState {
    pub post: Option<Post>,
    pub replies: Vec<Post>,
    pub selected_reply_index: Option<usize>,
    pub loading: bool,
    pub error: Option<String>,
    pub show_reply_composer: bool,
    pub reply_content: String,
    pub show_edit_modal: bool,
    pub edit_content: String,
    pub show_delete_confirmation: bool,
    pub previous_feed_position: Option<usize>,
}

// Add to App struct
pub struct App {
    // ... existing fields ...
    pub post_detail_state: Option<PostDetailState>,
    pub viewing_post_detail: bool,
}
```

#### 1.2 Navigation Flow

```
Feed View (Posts Tab)
    │
    ├─ Press Enter on post
    │     │
    │     ▼
    │  Post Detail View
    │     │
    │     ├─ Press 'r' → Reply Composer
    │     ├─ Press 'e' (own post) → Edit Modal
    │     ├─ Press 'x' (own post) → Delete Confirmation
    │     ├─ Press 'u'/'d' → Vote on post/reply
    │     └─ Press ESC → Return to Feed
    │
    └─ Continue browsing feed
```

#### 1.3 UI Rendering Functions

New rendering functions to add to `ui.rs`:

```rust
// Render post detail view with replies
fn render_post_detail_view(frame: &mut Frame, app: &mut App, area: Rect)

// Render reply composer modal
fn render_reply_composer_modal(frame: &mut Frame, app: &mut App, area: Rect)

// Render post edit modal
fn render_edit_post_modal(frame: &mut Frame, app: &mut App, area: Rect)

// Render delete confirmation modal
fn render_delete_confirmation_modal(frame: &mut Frame, app: &mut App, area: Rect)

// Format reply content with indentation
fn format_reply_content(reply: &Post, is_selected: bool, theme: &ThemeColors) -> Vec<Line>
```

### 2. API Components (fido-api)

#### 2.1 New API Endpoints

```rust
// Get all replies for a post
GET /posts/{post_id}/replies
Response: Vec<Post>

// Create a reply to a post
POST /posts/{post_id}/reply
Request: { "content": String }
Response: Post

// Update a post (edit)
PUT /posts/{post_id}
Request: { "content": String }
Response: Post

// Delete a post
DELETE /posts/{post_id}
Response: { "success": bool }
```

#### 2.2 Authorization Middleware

```rust
// Verify post ownership before edit/delete operations
async fn verify_post_ownership(
    user_id: Uuid,
    post_id: Uuid,
    db: &Database
) -> Result<bool, ApiError>
```

### 3. Database Schema

#### 3.1 Posts Table Modifications

```sql
-- Add parent_post_id column for reply relationships
ALTER TABLE posts ADD COLUMN parent_post_id TEXT NULL;

-- Add reply_count column for efficient counting
ALTER TABLE posts ADD COLUMN reply_count INTEGER DEFAULT 0;

-- Create index for efficient reply queries
CREATE INDEX idx_posts_parent ON posts(parent_post_id);

-- Create index for efficient author queries (if not exists)
CREATE INDEX idx_posts_author ON posts(author_id);
```

#### 3.2 Data Model

```
Post {
    id: UUID (primary key)
    author_id: UUID (foreign key to users)
    author_username: String (denormalized for performance)
    content: String
    created_at: DateTime
    upvotes: i32
    downvotes: i32
    parent_post_id: UUID? (NULL for top-level posts)
    reply_count: i32
    user_vote: String? (denormalized, set by API)
}
```

## Data Models

### Post Detail View Model

```rust
pub struct PostDetailViewModel {
    pub post: Post,
    pub replies: Vec<Post>,
    pub can_edit: bool,
    pub can_delete: bool,
}
```

### Reply Creation Request

```rust
pub struct CreateReplyRequest {
    pub content: String,
}
```

### Post Update Request

```rust
pub struct UpdatePostRequest {
    pub content: String,
}
```

## Error Handling

### TUI Error Display

Following the existing error handling pattern in the codebase:

```rust
// Categorize errors for user-friendly messages
fn categorize_post_detail_error(error: &str) -> String {
    if error.contains("404") {
        "Post not found - it may have been deleted".to_string()
    } else if error.contains("403") {
        "Authorization Error: You don't have permission for this action".to_string()
    } else if error.contains("connection") || error.contains("timeout") {
        "Network Error: Connection failed - check your network and try again".to_string()
    } else {
        format!("Error: {}", error)
    }
}
```

### API Error Responses

```rust
pub enum PostDetailError {
    PostNotFound,
    Unauthorized,
    InvalidContent,
    DatabaseError(String),
    NetworkError(String),
}

impl IntoResponse for PostDetailError {
    fn into_response(self) -> Response {
        match self {
            PostDetailError::PostNotFound => 
                (StatusCode::NOT_FOUND, "Post not found").into_response(),
            PostDetailError::Unauthorized => 
                (StatusCode::FORBIDDEN, "Unauthorized").into_response(),
            PostDetailError::InvalidContent => 
                (StatusCode::BAD_REQUEST, "Invalid content").into_response(),
            // ... other cases
        }
    }
}
```

### Graceful Degradation

- If replies fail to load, show post detail without replies
- If post is deleted while viewing, show placeholder message
- Network failures display retry option
- Optimistic updates with rollback on failure (following existing vote pattern)

## Testing Strategy

### Unit Tests

#### Database Layer
- Test parent_post_id relationships
- Test reply_count increment/decrement
- Test cascade behavior on post deletion
- Test index performance with large datasets

#### API Layer
- Test reply creation with valid/invalid data
- Test post edit authorization (owner vs non-owner)
- Test post deletion authorization
- Test reply fetching with pagination
- Test error responses for all endpoints

#### TUI Layer
- Test state transitions (Feed → Detail → Feed)
- Test reply composer input handling
- Test edit modal pre-population
- Test delete confirmation flow
- Test keyboard navigation in reply list

### Integration Tests

- Test full reply creation flow (TUI → API → DB → TUI)
- Test post editing flow with authorization
- Test post deletion with replies
- Test navigation state preservation
- Test error handling across layers

### Manual Testing Scenarios

1. **Reply Flow**
   - Create reply to own post
   - Create reply to another user's post
   - View post with multiple replies
   - Navigate between replies with arrow keys

2. **Edit Flow**
   - Edit own post with no replies
   - Edit own post with replies
   - Attempt to edit another user's post (should fail)
   - Cancel edit without saving

3. **Delete Flow**
   - Delete own post with no replies
   - Delete own post with replies (warning shown)
   - Attempt to delete another user's post (should fail)
   - Cancel deletion

4. **Navigation**
   - Enter post detail from feed
   - Return to feed (scroll position preserved)
   - Navigate between multiple posts
   - Handle deleted post gracefully

5. **Error Scenarios**
   - Network failure during reply creation
   - Network failure during post edit
   - Viewing deleted post
   - Authorization failures

## Implementation Phases

### Phase 1: Post Detail View Foundation
- Add `PostDetailState` to app state
- Implement navigation (Enter to open, ESC to close)
- Render full post content with action buttons
- Preserve feed scroll position

### Phase 2: Database Schema
- Add `parent_post_id` and `reply_count` columns
- Create indexes
- Write migration script
- Test with existing data

### Phase 3: Reply System
- Implement reply API endpoints
- Add reply composer UI
- Implement reply display with navigation
- Add reply count to feed view

### Phase 4: Post Editing
- Implement edit API endpoint with authorization
- Add edit modal UI
- Pre-populate editor with current content
- Handle edit success/failure

### Phase 5: Post Deletion
- Implement delete API endpoint with authorization
- Add delete confirmation modal
- Handle cascade behavior for replies
- Update feed after deletion

### Phase 6: Polish & Testing
- Comprehensive error handling
- Performance optimization (lazy loading, caching)
- Edge case handling
- User feedback and refinement

## Performance Considerations

### Lazy Loading
- Fetch replies only when post detail view is opened
- Cache replies to avoid redundant API calls
- Clear cache when returning to feed

### Optimistic Updates
- Apply reply creation immediately in UI
- Rollback on API failure
- Follow existing vote pattern for consistency

### Database Optimization
- Use indexes for parent_post_id queries
- Denormalize reply_count for efficient display
- Consider pagination for posts with many replies (future enhancement)

### UI Rendering
- Reuse existing text wrapping and formatting functions
- Minimize re-renders during navigation
- Use Ratatui's efficient diffing for updates

## Security Considerations

### Authorization
- Verify post ownership before edit/delete operations
- Use session tokens for authentication
- Return 403 Forbidden for unauthorized actions

### Input Validation
- Sanitize reply content (same as post content)
- Enforce character limits (280 characters)
- Prevent empty replies
- Validate post_id format (UUID)

### Data Integrity
- Use transactions for reply creation + count increment
- Handle concurrent edits gracefully
- Prevent orphaned replies on post deletion

## Future Enhancements (Post-MVP)

### Nested Replies
- Allow replies to replies (2-3 levels deep)
- Implement tree structure for reply display
- Add collapse/expand functionality

### Reply Sorting
- Sort by newest, oldest, most upvoted
- Store preference in user settings
- Add UI controls for sorting

### Notifications
- Notify users when their post receives a reply
- Notify users when mentioned in reply
- Add notification badge to UI

### Edit History
- Track post edit timestamps
- Show "edited" indicator on posts
- Allow viewing edit history (admin feature)

### Reply Pagination
- Paginate replies for posts with many responses
- Implement "Load more" functionality
- Optimize for large reply threads

## Dependencies

### New Dependencies
- None required - uses existing dependencies:
  - `ratatui` for TUI rendering
  - `axum` for API endpoints
  - `rusqlite` for database operations
  - `uuid` for post identifiers
  - `chrono` for timestamps

### Existing Patterns to Follow
- State management pattern from `app.rs`
- Error categorization from existing error handling
- Modal rendering from new post/edit bio modals
- API client pattern from `api.rs`
- Theme system from `ui.rs`

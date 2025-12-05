# Design Document

## Overview

Fido MVP is a Twitter-like terminal social platform built with Rust, featuring short-form posts (280 chars), hashtag support, user profiles, and configurable browsing limits. The design emphasizes simplicity, speed, and keyboard-driven interaction while maintaining a clean, developer-friendly interface.

## Architecture

### High-Level Architecture

```
┌─────────────────┐    HTTP/REST    ┌─────────────────┐
│   Ratatui TUI   │ ◄──────────────► │   Axum Server   │
│   (Frontend)    │                  │   (Backend)     │
└─────────────────┘                  └─────────────────┘
         │                                    │
         │                                    │
         ▼                                    ▼
┌─────────────────┐                  ┌─────────────────┐
│ Local Config    │                  │ SQLite Database │
│ (.fido/)        │                  │                 │
└─────────────────┘                  └─────────────────┘
```

### Project Structure (Rust Workspace)

```
fido/
├── Cargo.toml (workspace)
├── fido-types/          # Shared data types and schemas
├── fido-server/         # Axum REST API backend
├── fido-tui/           # Ratatui terminal interface
└── fido-cli/           # CLI commands (dm, etc.)
```

## Components and Interfaces

### Frontend Components (Ratatui TUI)

#### 1. Authentication Screen
- **Purpose**: Test user selection for development
- **Interface**: Simple list selection with arrow keys
- **State**: Selected user stored locally

#### 2. Main Interface with Tabs
- **Purpose**: Tabbed interface for different views
- **Layout**: Header with tab navigation, content area, keyboard shortcuts footer
- **Tabs**:
  - **Posts Tab**: Global feed with post list
  - **DMs Tab**: Direct message conversations
  - **Profile Tab**: User profile with personal posts
  - **Settings Tab**: Configuration options

#### 3. New Post Modal
- **Purpose**: Overlay modal for creating posts
- **Layout**: Centered modal with text input and character counter
- **Features**:
  - 280 character limit with live counter
  - Hashtag highlighting as you type
  - Ctrl+Enter to submit, Esc to cancel

#### 4. Profile Tab View
- **Purpose**: User profile display with personal post history
- **Layout**: Profile stats at top, personal posts below
- **Features**:
  - Profile stats (karma, post count, join date, recent hashtags)
  - Bio editing for own profile (press 'e')
  - List of user's own posts with voting stats
  - Navigation through personal post history

#### 4. Direct Messages View
- **Purpose**: Private messaging interface
- **Layout**: Conversation list + message history
- **Features**:
  - Conversation selection
  - Message input and display
  - Unread message indicators

#### 5. Settings View
- **Purpose**: Configuration management
- **Features**:
  - Color scheme selection
  - Sort order preferences
  - Maximum post count setting
  - Configuration persistence

### Backend Components (Axum Server)

#### 1. Authentication Service
- **Purpose**: Test user management
- **Interface**: Simple user selection without OAuth
- **Storage**: User sessions in memory/local storage

#### 2. Post Service
- **Purpose**: Post CRUD operations with hashtag parsing
- **Features**:
  - 280 character validation
  - Hashtag extraction and storage
  - Vote counting and management
  - Feed generation with sorting

#### 3. Profile Service
- **Purpose**: User profile management
- **Features**:
  - Bio updates
  - Karma calculation (sum of upvotes)
  - Recent hashtag tracking
  - Profile statistics

#### 4. Direct Message Service
- **Purpose**: Private messaging
- **Features**:
  - Message storage and retrieval
  - Conversation management
  - Unread status tracking

#### 5. Configuration Service
- **Purpose**: User preference management
- **Features**:
  - Settings storage and retrieval
  - Default configuration handling

## Data Models

### Core Entities

#### User
```rust
struct User {
    id: Uuid,
    username: String,
    bio: Option<String>,
    join_date: DateTime<Utc>,
    is_test_user: bool,
}
```

#### Post
```rust
struct Post {
    id: Uuid,
    author_id: Uuid,
    content: String,        // Max 280 characters
    created_at: DateTime<Utc>,
    upvotes: i32,
    downvotes: i32,
    hashtags: Vec<String>,  // Extracted from content
}
```

#### Vote
```rust
struct Vote {
    user_id: Uuid,
    post_id: Uuid,
    direction: VoteDirection, // Up or Down
    created_at: DateTime<Utc>,
}

enum VoteDirection {
    Up,
    Down,
}
```

#### DirectMessage
```rust
struct DirectMessage {
    id: Uuid,
    from_user_id: Uuid,
    to_user_id: Uuid,
    content: String,
    created_at: DateTime<Utc>,
    is_read: bool,
}
```

#### UserProfile
```rust
struct UserProfile {
    user_id: Uuid,
    karma: i32,              // Sum of upvotes received
    post_count: i32,
    recent_hashtags: Vec<String>, // Last 10 unique hashtags used
}
```

#### UserConfig
```rust
struct UserConfig {
    user_id: Uuid,
    color_scheme: ColorScheme,
    sort_order: SortOrder,
    max_posts_display: i32,  // Default: 25
    emoji_enabled: bool,
}

enum ColorScheme {
    Default,
    Dark,
    Light,
    Solarized,
}

enum SortOrder {
    Newest,
    Popular,
    Controversial,
}
```

### Database Schema

#### Tables
```sql
-- Users table
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    bio TEXT,
    join_date DATETIME NOT NULL,
    is_test_user BOOLEAN DEFAULT FALSE
);

-- Posts table
CREATE TABLE posts (
    id TEXT PRIMARY KEY,
    author_id TEXT NOT NULL,
    content TEXT NOT NULL CHECK(length(content) <= 280),
    created_at DATETIME NOT NULL,
    upvotes INTEGER DEFAULT 0,
    downvotes INTEGER DEFAULT 0,
    FOREIGN KEY (author_id) REFERENCES users(id)
);

-- Hashtags table (for future hashtag filtering)
CREATE TABLE hashtags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    post_id TEXT NOT NULL,
    hashtag TEXT NOT NULL,
    FOREIGN KEY (post_id) REFERENCES posts(id)
);

-- Votes table
CREATE TABLE votes (
    user_id TEXT NOT NULL,
    post_id TEXT NOT NULL,
    direction TEXT NOT NULL CHECK(direction IN ('up', 'down')),
    created_at DATETIME NOT NULL,
    PRIMARY KEY (user_id, post_id),
    FOREIGN KEY (user_id) REFERENCES users(id),
    FOREIGN KEY (post_id) REFERENCES posts(id)
);

-- Direct messages table
CREATE TABLE direct_messages (
    id TEXT PRIMARY KEY,
    from_user_id TEXT NOT NULL,
    to_user_id TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at DATETIME NOT NULL,
    is_read BOOLEAN DEFAULT FALSE,
    FOREIGN KEY (from_user_id) REFERENCES users(id),
    FOREIGN KEY (to_user_id) REFERENCES users(id)
);

-- User configurations table
CREATE TABLE user_configs (
    user_id TEXT PRIMARY KEY,
    color_scheme TEXT DEFAULT 'Default',
    sort_order TEXT DEFAULT 'Newest',
    max_posts_display INTEGER DEFAULT 25,
    emoji_enabled BOOLEAN DEFAULT TRUE,
    FOREIGN KEY (user_id) REFERENCES users(id)
);
```

## API Design

### REST Endpoints

#### Authentication
- `GET /users/test` - List available test users
- `POST /auth/login` - Login with selected test user
- `POST /auth/logout` - Logout current user

#### Posts
- `GET /posts?limit={max_posts}&sort={order}` - Get posts with limit
- `POST /posts` - Create new post (with hashtag extraction)
- `POST /posts/{id}/vote` - Vote on post

#### Profiles
- `GET /users/{id}/profile` - Get user profile with stats
- `PUT /users/{id}/profile` - Update user bio
- `GET /users/{id}/hashtags` - Get recent hashtags for user

#### Direct Messages
- `GET /dms/conversations` - List conversations for current user
- `GET /dms/conversations/{user_id}` - Get messages with specific user
- `POST /dms` - Send direct message

#### Configuration
- `GET /config` - Get user configuration
- `PUT /config` - Update user configuration

### Request/Response Examples

#### Create Post
```json
POST /posts
{
  "content": "Just shipped a new feature! 🚀 #rust #coding #productivity"
}

Response:
{
  "id": "uuid",
  "author_id": "uuid", 
  "content": "Just shipped a new feature! 🚀 #rust #coding #productivity",
  "created_at": "2024-01-01T12:00:00Z",
  "upvotes": 0,
  "downvotes": 0,
  "hashtags": ["rust", "coding", "productivity"]
}
```

#### Vote on Post
```json
POST /posts/{id}/vote
{
  "direction": "up"
}
```

## User Interface Design

### Main Interface Layout
```
┌─────────────────────────────────────────────────────────────┐
│ Fido - Terminal Social Platform                             │
├─────────────────────────────────────────────────────────────┤
│ [Posts] [DMs] [Profile] [Settings]                          │
├─────────────────────────────────────────────────────────────┤
│ ▶ @alice (karma: 42) • 2h ago                          ↑ 5 │
│   Just shipped a new #rust feature! 🚀                 ↓ 0 │
├─────────────────────────────────────────────────────────────┤
│   @bob (karma: 23) • 4h ago                           ↑ 12 │
│   Working on #terminal #ui design. Any tips?           ↓ 1 │
├─────────────────────────────────────────────────────────────┤
│   @charlie (karma: 67) • 6h ago                        ↑ 8 │
│   Love the simplicity of #sqlite for MVPs              ↓ 0 │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│ [Showing 25 posts]                                          │
├─────────────────────────────────────────────────────────────┤
│ j/k:nav u/d:vote n:new post r:reply p:profile tab:switch q:quit │
└─────────────────────────────────────────────────────────────┘
```

### New Post Modal
```
┌─────────────────────────────────────────────────────────────┐
│ Fido - Terminal Social Platform                             │
├─────────────────────────────────────────────────────────────┤
│ [Posts] [DMs] [Profile] [Settings]                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│    ┌─────────────── New Post ───────────────┐               │
│    │                                        │               │
│    │ What's happening?                      │               │
│    │ ┌────────────────────────────────────┐ │               │
│    │ │Just shipped a new #rust feature! 🚀│ │               │
│    │ │                                    │ │               │
│    │ └────────────────────────────────────┘ │               │
│    │                                        │               │
│    │ Characters: 33/280                     │               │
│    │                                        │               │
│    │ [Ctrl+Enter: Post] [Esc: Cancel]       │               │
│    └────────────────────────────────────────┘               │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│ j/k:nav u/d:vote n:new post r:reply p:profile tab:switch q:quit │
└─────────────────────────────────────────────────────────────┘
```

### Profile Tab Layout
```
┌─────────────────────────────────────────────────────────────┐
│ Fido - Terminal Social Platform                             │
├─────────────────────────────────────────────────────────────┤
│ [Posts] [DMs] [Profile] [Settings]                          │
├─────────────────────────────────────────────────────────────┤
│ @alice                                    Karma: 42         │
│ Bio: Rust enthusiast and terminal lover   Posts: 15         │
│ Joined: Jan 2024                         Recent: #rust #tui │
├─────────────────────────────────────────────────────────────┤
│ Your Recent Posts:                                          │
├─────────────────────────────────────────────────────────────┤
│ ▶ @alice (you) • 2h ago                               ↑ 5 │
│   Just shipped a new #rust feature! 🚀                ↓ 0 │
├─────────────────────────────────────────────────────────────┤
│   @alice (you) • 1d ago                               ↑ 8 │
│   Working on terminal UI improvements #tui             ↓ 1 │
├─────────────────────────────────────────────────────────────┤
│ e:edit bio j/k:nav u/d:vote n:new post tab:switch q:quit    │
└─────────────────────────────────────────────────────────────┘
```

### Keyboard Shortcuts

#### Global Navigation
- `Tab` - Switch between tabs (Posts, DMs, Profile, Settings)
- `j/↓` - Next item/post
- `k/↑` - Previous item/post  
- `q` - Quit application
- `?` - Help modal

#### Post Actions
- `u` - Upvote selected post
- `d` - Downvote selected post
- `n` - New post modal
- `r` - Reply/DM to post author
- `Enter` - View post details/profile

#### Modal Controls
- `Ctrl+Enter` - Submit (post, message, etc.)
- `Esc` - Cancel/close modal
- `Tab` - Navigate within modal

#### Profile Tab
- `e` - Edit bio (when viewing own profile)
- `j/k` - Navigate through own posts

## Error Handling

### Client-Side Error Handling
- **Network Errors**: Display retry options with clear error messages
- **Validation Errors**: Show inline validation for character limits
- **Authentication Errors**: Redirect to user selection screen

### Server-Side Error Handling
- **Database Errors**: Log errors, return generic error messages to client
- **Validation Errors**: Return specific validation messages
- **Rate Limiting**: Implement basic rate limiting for post creation

### Error Response Format
```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Post content exceeds 280 character limit",
    "details": {
      "field": "content",
      "current_length": 295,
      "max_length": 280
    }
  }
}
```

## Testing Strategy

### Unit Testing
- **Backend Services**: Test all CRUD operations with in-memory SQLite
- **Hashtag Parsing**: Test hashtag extraction from post content
- **Karma Calculation**: Test upvote aggregation logic
- **Character Limits**: Test 280 character validation

### Integration Testing
- **API Endpoints**: Test all REST endpoints with test database
- **Multi-User Scenarios**: Test voting, DMs between different test users
- **Configuration Persistence**: Test settings save/load functionality

### TUI Testing
- **Keyboard Navigation**: Test all keyboard shortcuts and navigation
- **Display Formatting**: Test post display with various content types
- **Error Display**: Test error message presentation

### Test Data Setup
```rust
// Test users for development
let test_users = vec![
    User { username: "alice".to_string(), bio: Some("Rust enthusiast".to_string()), .. },
    User { username: "bob".to_string(), bio: Some("Terminal UI lover".to_string()), .. },
    User { username: "charlie".to_string(), bio: Some("SQLite advocate".to_string()), .. },
];
```

## Performance Considerations

### Database Optimization
- **Indexing**: Index on `created_at` for post sorting, `user_id` for votes
- **Query Limits**: Always use LIMIT clauses for post queries
- **Connection Pooling**: Use SQLite connection pooling for concurrent access

### TUI Performance
- **Lazy Loading**: Only render visible posts in the terminal
- **Efficient Scrolling**: Use virtual scrolling for large post lists
- **Minimal Redraws**: Only redraw changed portions of the interface

### Memory Management
- **Post Caching**: Cache recent posts in memory with LRU eviction
- **Configuration Caching**: Keep user config in memory during session
- **Hashtag Indexing**: Efficient hashtag lookup for future filtering features

## Security Considerations

### Input Validation
- **Content Sanitization**: Sanitize post content to prevent injection
- **Character Limits**: Enforce 280 character limit server-side
- **Hashtag Validation**: Validate hashtag format and length

### Data Protection
- **Local Storage**: Secure local configuration storage
- **Session Management**: Simple session handling for test users
- **SQL Injection**: Use parameterized queries for all database operations

## Future Enhancements

### Post-MVP Features
1. **Hashtag Filtering**: Filter posts by hashtag
2. **Doom Scroll Protection**: Configurable scroll limits with confirmation prompts
3. **Real-time Updates**: WebSocket integration for live feed updates
4. **Advanced Emoji**: Full emoji picker and configuration
5. **GitHub OAuth**: Replace test users with real GitHub authentication
6. **Export/Import**: Configuration and data export functionality
7. **Themes**: Additional color schemes and customization options
8. **Search**: Full-text search across posts and users

### Scalability Considerations
- **Database Migration**: Plan for SQLite → PostgreSQL migration
- **Caching Layer**: Redis integration for high-traffic scenarios  
- **API Versioning**: Version API endpoints for backward compatibility
- **Horizontal Scaling**: Design for multiple server instances
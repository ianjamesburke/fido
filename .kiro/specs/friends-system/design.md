# Design Document

## Overview

This design implements a friends system for Fido, enabling users to maintain a list of friends for easier direct messaging and content discovery. The solution includes database schema for friendship storage, API endpoints for friend management, and UI components for adding and managing friends. The design prioritizes simplicity with direct username entry rather than complex search algorithms.

## Architecture

### System Components

1. **Database Layer**: SQLite table for friendship storage
2. **API Layer**: REST endpoints for friend operations (add, remove, list)
3. **Validation Layer**: Username existence checking
4. **TUI Layer**: Friends modal, add friend interface, DM integration
5. **State Management**: Friends list state, modal state

### Data Flow

```
User Input → Username Validation → Database Operation → State Update → UI Refresh
DM Creation → Friends Check → Prioritized List → User Selection
```

## Components and Interfaces

### 1. Database Schema

#### friendships Table

```sql
CREATE TABLE friendships (
    user_id TEXT NOT NULL,
    friend_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (user_id, friend_id),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (friend_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_friendships_user ON friendships(user_id);
CREATE INDEX idx_friendships_friend ON friendships(friend_id);
```

**Design Notes:**
- Composite primary key prevents duplicate friendships
- Unidirectional relationships (A→B doesn't imply B→A)
- Cascade delete removes friendships when users are deleted
- Indexes on both columns for efficient queries


### 2. API Endpoints

#### GET /friends

Get user's friends list.

**Response:**
```json
{
  "friends": [
    {
      "id": "user-uuid",
      "username": "alice",
      "display_name": "Alice Smith",
      "friend_count": 42,
      "added_at": 1700000000
    }
  ]
}
```

**Implementation:**
```rust
// server/src/routes/friends.rs
pub async fn get_friends(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<FriendsResponse>, AppError> {
    let friends = sqlx::query_as::<_, FriendInfo>(
        "SELECT u.id, u.username, u.display_name, u.friend_count, f.created_at as added_at
         FROM friendships f
         JOIN users u ON f.friend_id = u.id
         WHERE f.user_id = ?
         ORDER BY f.created_at DESC"
    )
    .bind(&user.id)
    .fetch_all(&state.db)
    .await?;
    
    Ok(Json(FriendsResponse { friends }))
}
```

#### POST /friends/:username

Add a friend by username.

**Path Parameters:**
- `username`: Username of the user to add as friend

**Response:**
- 200 OK: Friend added successfully
- 404 Not Found: User does not exist
- 400 Bad Request: Cannot add self or duplicate

**Implementation:**
```rust
pub async fn add_friend(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(username): Path<String>,
) -> Result<StatusCode, AppError> {
    let username = username.trim();
    
    // Validate username is not empty
    if username.is_empty() {
        return Err(AppError::BadRequest("Username cannot be empty".to_string()));
    }
    
    // Look up user by username (case-insensitive)
    let friend = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE LOWER(username) = LOWER(?)"
    )
    .bind(username)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("User '{}' not found", username)))?;
    
    // Prevent adding self as friend
    if friend.id == user.id {
        return Err(AppError::BadRequest("Cannot add yourself as a friend".to_string()));
    }
    
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    
    // Insert friendship (ignore if already exists)
    sqlx::query(
        "INSERT OR IGNORE INTO friendships (user_id, friend_id, created_at)
         VALUES (?, ?, ?)"
    )
    .bind(&user.id)
    .bind(&friend.id)
    .bind(now)
    .execute(&state.db)
    .await?;
    
    Ok(StatusCode::OK)
}
```


#### DELETE /friends/:id

Remove a friend by user ID.

**Path Parameters:**
- `id`: User ID of the friend to remove

**Response:**
- 200 OK: Friend removed successfully
- 404 Not Found: Friendship does not exist

**Implementation:**
```rust
pub async fn remove_friend(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(friend_id): Path<String>,
) -> Result<StatusCode, AppError> {
    let result = sqlx::query(
        "DELETE FROM friendships WHERE user_id = ? AND friend_id = ?"
    )
    .bind(&user.id)
    .bind(&friend_id)
    .execute(&state.db)
    .await?;
    
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Friendship not found".to_string()));
    }
    
    Ok(StatusCode::OK)
}
```

### 3. TUI Implementation

#### State Management

```rust
// tui/src/app.rs
pub struct FriendsState {
    pub friends: Vec<FriendInfo>,
    pub show_friends_modal: bool,
    pub show_add_friend_input: bool,
    pub add_friend_username: String,
    pub selected_friend: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FriendInfo {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub friend_count: usize,
    pub added_at: i64,
}

impl App {
    pub async fn load_friends(&mut self) -> Result<()> {
        self.friends_state.friends = self.api_client.get_friends().await?;
        Ok(())
    }
    
    pub async fn add_friend(&mut self, username: &str) -> Result<()> {
        match self.api_client.add_friend(username).await {
            Ok(_) => {
                self.load_friends().await?;
                self.friends_state.add_friend_username.clear();
                self.friends_state.show_add_friend_input = false;
                self.friends_state.error = None;
                Ok(())
            }
            Err(e) => {
                self.friends_state.error = Some(format!("User '@{}' not found", username));
                Err(e)
            }
        }
    }
    
    pub async fn remove_friend(&mut self, friend_id: &str) -> Result<()> {
        self.api_client.remove_friend(friend_id).await?;
        self.load_friends().await?;
        Ok(())
    }
}
```


#### Friends Modal Rendering

```rust
// tui/src/ui.rs
fn render_friends_modal(frame: &mut Frame, app: &App, area: Rect) {
    let modal_area = centered_rect(60, 70, area);
    
    // Clear background
    frame.render_widget(Clear, modal_area);
    
    let block = Block::default()
        .title("Friends")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    
    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);
    
    if app.friends_state.show_add_friend_input {
        render_add_friend_input(frame, app, inner);
    } else {
        render_friends_list(frame, app, inner);
    }
}

fn render_friends_list(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),     // Friends list
            Constraint::Length(3),  // Add friend button
            Constraint::Length(2),  // Footer
        ])
        .split(area);
    
    // Render friends list
    let mut items: Vec<ListItem> = app.friends_state.friends
        .iter()
        .map(|f| {
            let content = format!(
                "@{} ({} friends) - added {}",
                f.username,
                f.friend_count,
                format_timestamp(f.added_at)
            );
            ListItem::new(content)
        })
        .collect();
    
    // Add "Add Friend" option at bottom
    items.push(ListItem::new(Line::from(vec![
        Span::styled("+ ", Style::default().fg(Color::Green)),
        Span::styled("Add Friend", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
    ])));
    
    let list = List::new(items)
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");
    
    let mut list_state = ListState::default();
    list_state.select(Some(app.friends_state.selected_friend));
    
    frame.render_stateful_widget(list, chunks[0], &mut list_state);
    
    // Render footer
    let footer = Paragraph::new("↑/↓: Navigate | Enter: Select | X: Remove | Esc: Close")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));
    frame.render_widget(footer, chunks[2]);
}

fn render_add_friend_input(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Length(3),  // Input
            Constraint::Length(2),  // Error (if any)
            Constraint::Min(0),     // Spacer
            Constraint::Length(2),  // Footer
        ])
        .split(area);
    
    // Title
    let title = Paragraph::new("Enter username to add as friend:")
        .alignment(Alignment::Center)
        .style(Style::default().add_modifier(Modifier::BOLD));
    frame.render_widget(title, chunks[0]);
    
    // Input field
    let input = Paragraph::new(format!("@{}", app.friends_state.add_friend_username))
        .block(Block::default().borders(Borders::ALL).title("Username"))
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(input, chunks[1]);
    
    // Error message (if any)
    if let Some(error) = &app.friends_state.error {
        let error_widget = Paragraph::new(error.as_str())
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Red));
        frame.render_widget(error_widget, chunks[2]);
    }
    
    // Footer
    let footer = Paragraph::new("Enter: Add Friend | Esc: Cancel")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));
    frame.render_widget(footer, chunks[4]);
}
```


#### Key Handling

```rust
// tui/src/app.rs
pub fn handle_friends_modal_keys(&mut self, key: KeyEvent) -> Result<()> {
    if self.friends_state.show_add_friend_input {
        return self.handle_add_friend_input_keys(key);
    }
    
    match key.code {
        KeyCode::Esc => {
            self.friends_state.show_friends_modal = false;
            return Ok(());
        }
        KeyCode::Up => {
            if self.friends_state.selected_friend > 0 {
                self.friends_state.selected_friend -= 1;
            }
        }
        KeyCode::Down => {
            let max_index = self.friends_state.friends.len(); // +1 for "Add Friend" option
            if self.friends_state.selected_friend < max_index {
                self.friends_state.selected_friend += 1;
            }
        }
        KeyCode::Enter => {
            if self.friends_state.selected_friend == self.friends_state.friends.len() {
                // Selected "Add Friend" option
                self.friends_state.show_add_friend_input = true;
                self.friends_state.add_friend_username.clear();
                self.friends_state.error = None;
            }
        }
        KeyCode::Char('x') | KeyCode::Char('X') => {
            if self.friends_state.selected_friend < self.friends_state.friends.len() {
                let friend = &self.friends_state.friends[self.friends_state.selected_friend];
                // Show confirmation modal
                self.show_remove_friend_confirmation(&friend.username, &friend.id);
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_add_friend_input_keys(&mut self, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            self.friends_state.show_add_friend_input = false;
            self.friends_state.add_friend_username.clear();
            self.friends_state.error = None;
        }
        KeyCode::Enter => {
            let username = self.friends_state.add_friend_username.trim().to_string();
            if !username.is_empty() {
                // Trigger async add friend operation
                tokio::spawn(async move {
                    self.add_friend(&username).await
                });
            }
        }
        KeyCode::Char(c) => {
            self.friends_state.add_friend_username.push(c);
        }
        KeyCode::Backspace => {
            self.friends_state.add_friend_username.pop();
        }
        _ => {}
    }
    Ok(())
}

// In main key handler
KeyCode::Char('f') | KeyCode::Char('F') => {
    if self.current_tab == Tab::Profile {
        self.friends_state.show_friends_modal = true;
        self.friends_state.selected_friend = 0;
        tokio::spawn(async move {
            self.load_friends().await
        });
    }
}
```


### 4. DM Integration

#### Friends-First Conversation List

```rust
// tui/src/app.rs
pub async fn load_dm_users(&mut self) -> Result<()> {
    // Load friends
    let friends = self.api_client.get_friends().await?;
    
    // Load all users (or recent DM contacts)
    let all_users = self.api_client.get_users().await?;
    
    // Separate friends from non-friends
    let friend_ids: HashSet<String> = friends.iter().map(|f| f.id.clone()).collect();
    
    let mut friend_users: Vec<UserInfo> = all_users
        .iter()
        .filter(|u| friend_ids.contains(&u.id))
        .cloned()
        .collect();
    
    let mut other_users: Vec<UserInfo> = all_users
        .iter()
        .filter(|u| !friend_ids.contains(&u.id))
        .cloned()
        .collect();
    
    // Sort friends by most recent friendship
    friend_users.sort_by(|a, b| {
        let a_time = friends.iter().find(|f| f.id == a.id).map(|f| f.added_at).unwrap_or(0);
        let b_time = friends.iter().find(|f| f.id == b.id).map(|f| f.added_at).unwrap_or(0);
        b_time.cmp(&a_time)
    });
    
    // Combine: friends first, then others
    self.dms_state.available_users = friend_users;
    self.dms_state.available_users.extend(other_users);
    self.dms_state.friends_count = friend_ids.len();
    
    Ok(())
}
```

#### DM User List Rendering

```rust
// tui/src/ui.rs
fn render_dm_user_list(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app.dms_state.available_users
        .iter()
        .enumerate()
        .map(|(i, user)| {
            let mut content = format!("@{}", user.username);
            
            // Add separator after friends
            if i == app.dms_state.friends_count && app.dms_state.friends_count > 0 {
                // This will be rendered as a separator line
                content = format!("─── Other Users ───");
            }
            
            ListItem::new(content)
        })
        .collect();
    
    let list = List::new(items)
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");
    
    let mut list_state = ListState::default();
    list_state.select(Some(app.dms_state.selected_user));
    
    frame.render_stateful_widget(list, area, &mut list_state);
}
```


### 5. DM Error Handling

#### Error Modal for Non-Existent Users

```rust
// tui/src/app.rs
pub struct DMErrorState {
    pub show_error_modal: bool,
    pub error_message: String,
    pub failed_username: Option<String>,
}

pub async fn send_dm_to_username(&mut self, username: &str) -> Result<()> {
    match self.api_client.send_dm(username, &self.dms_state.message_input).await {
        Ok(_) => {
            self.dms_state.message_input.clear();
            self.load_conversations().await?;
            Ok(())
        }
        Err(e) if e.to_string().contains("404") || e.to_string().contains("not found") => {
            // User not found - show error modal with friend suggestion
            self.dms_state.dm_error.show_error_modal = true;
            self.dms_state.dm_error.error_message = format!(
                "User '@{}' not found. Add them as a friend first?",
                username
            );
            self.dms_state.dm_error.failed_username = Some(username.to_string());
            Err(e)
        }
        Err(e) => {
            self.dms_state.error = Some(format!("Failed to send DM: {}", e));
            Err(e)
        }
    }
}
```

#### Error Modal Rendering

```rust
// tui/src/ui.rs
fn render_dm_error_modal(frame: &mut Frame, app: &App, area: Rect) {
    let modal_area = centered_rect(50, 30, area);
    
    frame.render_widget(Clear, modal_area);
    
    let block = Block::default()
        .title("Error")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));
    
    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),     // Error message
            Constraint::Length(2),  // Footer
        ])
        .split(inner);
    
    let message = Paragraph::new(app.dms_state.dm_error.error_message.as_str())
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    frame.render_widget(message, chunks[0]);
    
    let footer = Paragraph::new("Enter: Add Friend | Esc: Cancel")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));
    frame.render_widget(footer, chunks[1]);
}

// Key handling for error modal
fn handle_dm_error_modal_keys(&mut self, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Enter => {
            // Open add friend interface with pre-filled username
            if let Some(username) = &self.dms_state.dm_error.failed_username {
                self.friends_state.show_friends_modal = true;
                self.friends_state.show_add_friend_input = true;
                self.friends_state.add_friend_username = username.clone();
            }
            self.dms_state.dm_error.show_error_modal = false;
        }
        KeyCode::Esc => {
            self.dms_state.dm_error.show_error_modal = false;
            self.dms_state.dm_error.failed_username = None;
        }
        _ => {}
    }
    Ok(())
}
```

## Data Models

```rust
// Shared types (fido-types crate)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendInfo {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub friend_count: usize,
    pub added_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FriendsResponse {
    pub friends: Vec<FriendInfo>,
}
```

## Error Handling

- User not found: Return 404 with message "User '{username}' not found"
- Cannot add self: Return 400 with message "Cannot add yourself as a friend"
- Duplicate friendship: Silently ignore (INSERT OR IGNORE)
- Empty username: Return 400 with message "Username cannot be empty"
- Friendship not found on removal: Return 404 with message "Friendship not found"

## Testing Strategy

### Unit Tests

- Test username validation (empty, whitespace, case-insensitive)
- Test duplicate friendship prevention
- Test self-friendship prevention
- Test friends list sorting

### Integration Tests

- Test add friend flow (valid and invalid usernames)
- Test remove friend flow
- Test friends list retrieval
- Test DM user list prioritization
- Test DM error handling with friend suggestion

### Manual Testing

- Add friends by username
- Remove friends from modal
- Verify friends appear first in DM user list
- Test error modal when DMing non-existent user
- Verify add friend from error modal


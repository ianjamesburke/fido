# Design Document

## Overview

This design implements a comprehensive hashtag system for Fido, enabling content organization, discovery, and filtering. The solution includes database schema for hashtag storage, API endpoints for hashtag operations, automatic hashtag extraction from posts, and a simplified filter modal UI. The design prioritizes simplicity and performance while providing powerful content filtering capabilities.

## Architecture

### System Components

1. **Database Layer**: SQLite tables for hashtags, post-hashtag relationships, follows, and activity tracking
2. **API Layer**: REST endpoints for hashtag operations (follow, unfollow, search, filter)
3. **Parsing Layer**: Regex-based hashtag extraction from post content
4. **TUI Layer**: Filter modal, filter display, keyboard navigation
5. **State Management**: Filter state, followed hashtags, activity metrics

### Data Flow

```
Post Creation → Hashtag Extraction → Database Storage → Post-Hashtag Linking
User Action → Filter Selection → API Query → Filtered Posts → UI Rendering
User Interaction → Activity Tracking → Metrics Update → Profile Display
```

## Components and Interfaces

### 1. Database Schema

#### Tables

**hashtags**
```sql
CREATE TABLE hashtags (
    id TEXT PRIMARY KEY,
    name TEXT UNIQUE NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE INDEX idx_hashtags_name ON hashtags(name);
```

**post_hashtags**
```sql
CREATE TABLE post_hashtags (
    post_id TEXT NOT NULL,
    hashtag_id TEXT NOT NULL,
    PRIMARY KEY (post_id, hashtag_id),
    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE,
    FOREIGN KEY (hashtag_id) REFERENCES hashtags(id) ON DELETE CASCADE
);

CREATE INDEX idx_post_hashtags_post ON post_hashtags(post_id);
CREATE INDEX idx_post_hashtags_hashtag ON post_hashtags(hashtag_id);
```


**user_hashtag_follows**
```sql
CREATE TABLE user_hashtag_follows (
    user_id TEXT NOT NULL,
    hashtag_id TEXT NOT NULL,
    followed_at INTEGER NOT NULL,
    PRIMARY KEY (user_id, hashtag_id),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (hashtag_id) REFERENCES hashtags(id) ON DELETE CASCADE
);

CREATE INDEX idx_user_hashtag_follows_user ON user_hashtag_follows(user_id);
```

**user_hashtag_activity**
```sql
CREATE TABLE user_hashtag_activity (
    user_id TEXT NOT NULL,
    hashtag_id TEXT NOT NULL,
    interaction_count INTEGER DEFAULT 0,
    last_interaction INTEGER,
    PRIMARY KEY (user_id, hashtag_id),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (hashtag_id) REFERENCES hashtags(id) ON DELETE CASCADE
);

CREATE INDEX idx_user_hashtag_activity_user ON user_hashtag_activity(user_id);
```

### 2. Hashtag Parsing and Extraction

#### Regex Pattern

```rust
// Matches hashtags: letters, numbers, underscores, minimum 2 chars
const HASHTAG_REGEX: &str = r"#(\w{2,})";
```

#### Extraction Implementation

```rust
// server/src/hashtag.rs
use regex::Regex;
use once_cell::sync::Lazy;

static HASHTAG_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"#(\w{2,})").unwrap()
});

pub fn extract_hashtags(content: &str) -> Vec<String> {
    HASHTAG_REGEX
        .captures_iter(content)
        .map(|cap| cap[1].to_lowercase())  // Normalize to lowercase
        .collect::<std::collections::HashSet<_>>()  // Remove duplicates
        .into_iter()
        .collect()
}
```


#### Post Creation with Hashtag Extraction

```rust
// server/src/routes/posts.rs
pub async fn create_post(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<CreatePostRequest>,
) -> Result<Json<Post>, AppError> {
    let post_id = Uuid::new_v4().to_string();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    
    // Extract hashtags from content
    let hashtags = extract_hashtags(&payload.content);
    
    // Insert post
    sqlx::query(
        "INSERT INTO posts (id, author_id, content, created_at, upvotes, downvotes)
         VALUES (?, ?, ?, ?, 0, 0)"
    )
    .bind(&post_id)
    .bind(&user.id)
    .bind(&payload.content)
    .bind(now)
    .execute(&state.db)
    .await?;
    
    // Process hashtags
    for hashtag_name in hashtags {
        // Get or create hashtag
        let hashtag_id = get_or_create_hashtag(&state.db, &hashtag_name).await?;
        
        // Link post to hashtag
        sqlx::query(
            "INSERT INTO post_hashtags (post_id, hashtag_id) VALUES (?, ?)"
        )
        .bind(&post_id)
        .bind(&hashtag_id)
        .execute(&state.db)
        .await?;
        
        // Update user activity
        increment_hashtag_activity(&state.db, &user.id, &hashtag_id).await?;
    }
    
    // Return created post
    Ok(Json(get_post_by_id(&state.db, &post_id).await?))
}

async fn get_or_create_hashtag(db: &SqlitePool, name: &str) -> Result<String, AppError> {
    // Try to get existing hashtag
    if let Some(row) = sqlx::query("SELECT id FROM hashtags WHERE name = ?")
        .bind(name)
        .fetch_optional(db)
        .await?
    {
        return Ok(row.get("id"));
    }
    
    // Create new hashtag
    let id = Uuid::new_v4().to_string();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    
    sqlx::query("INSERT INTO hashtags (id, name, created_at) VALUES (?, ?, ?)")
        .bind(&id)
        .bind(name)
        .bind(now)
        .execute(db)
        .await?;
    
    Ok(id)
}
```


### 3. API Endpoints

#### GET /hashtags/followed

Get user's followed hashtags.

**Response:**
```json
{
  "hashtags": [
    {
      "id": "uuid",
      "name": "rust",
      "followed_at": 1700000000,
      "post_count": 42
    }
  ]
}
```

**Implementation:**
```rust
pub async fn get_followed_hashtags(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<FollowedHashtagsResponse>, AppError> {
    let hashtags = sqlx::query_as::<_, HashtagWithCount>(
        "SELECT h.id, h.name, uhf.followed_at,
                COUNT(DISTINCT ph.post_id) as post_count
         FROM hashtags h
         JOIN user_hashtag_follows uhf ON h.id = uhf.hashtag_id
         LEFT JOIN post_hashtags ph ON h.id = ph.hashtag_id
         WHERE uhf.user_id = ?
         GROUP BY h.id
         ORDER BY uhf.followed_at DESC"
    )
    .bind(&user.id)
    .fetch_all(&state.db)
    .await?;
    
    Ok(Json(FollowedHashtagsResponse { hashtags }))
}
```

#### POST /hashtags/follow

Follow a hashtag.

**Request:**
```json
{
  "hashtag_name": "rust"
}
```

**Implementation:**
```rust
pub async fn follow_hashtag(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<FollowHashtagRequest>,
) -> Result<StatusCode, AppError> {
    // Get or create hashtag
    let hashtag_id = get_or_create_hashtag(&state.db, &payload.hashtag_name).await?;
    
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    
    // Insert follow relationship (ignore if already exists)
    sqlx::query(
        "INSERT OR IGNORE INTO user_hashtag_follows (user_id, hashtag_id, followed_at)
         VALUES (?, ?, ?)"
    )
    .bind(&user.id)
    .bind(&hashtag_id)
    .bind(now)
    .execute(&state.db)
    .await?;
    
    Ok(StatusCode::OK)
}
```


#### DELETE /hashtags/follow/:id

Unfollow a hashtag.

**Implementation:**
```rust
pub async fn unfollow_hashtag(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(hashtag_id): Path<String>,
) -> Result<StatusCode, AppError> {
    sqlx::query(
        "DELETE FROM user_hashtag_follows
         WHERE user_id = ? AND hashtag_id = ?"
    )
    .bind(&user.id)
    .bind(&hashtag_id)
    .execute(&state.db)
    .await?;
    
    Ok(StatusCode::OK)
}
```

#### GET /posts?hashtag=:name

Filter posts by hashtag.

**Query Parameters:**
- `hashtag`: Hashtag name (without #)
- `sort`: Sort order (newest, top, hot)

**Implementation:**
```rust
pub async fn get_posts_by_hashtag(
    State(state): State<AppState>,
    Query(params): Query<PostFilterParams>,
) -> Result<Json<Vec<Post>>, AppError> {
    let sort_clause = match params.sort.as_deref() {
        Some("top") => "ORDER BY (p.upvotes - p.downvotes) DESC",
        Some("hot") => "ORDER BY (p.upvotes - p.downvotes) / (julianday('now') - julianday(p.created_at, 'unixepoch')) DESC",
        _ => "ORDER BY p.created_at DESC",  // newest (default)
    };
    
    let query = format!(
        "SELECT p.* FROM posts p
         JOIN post_hashtags ph ON p.id = ph.post_id
         JOIN hashtags h ON ph.hashtag_id = h.id
         WHERE LOWER(h.name) = LOWER(?)
         {}
         LIMIT 100",
        sort_clause
    );
    
    let posts = sqlx::query_as::<_, Post>(&query)
        .bind(&params.hashtag)
        .fetch_all(&state.db)
        .await?;
    
    Ok(Json(posts))
}
```

#### GET /hashtags/search?q=:query

Search hashtags.

**Implementation:**
```rust
pub async fn search_hashtags(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<Vec<HashtagWithCount>>, AppError> {
    let hashtags = sqlx::query_as::<_, HashtagWithCount>(
        "SELECT h.id, h.name, COUNT(DISTINCT ph.post_id) as post_count
         FROM hashtags h
         LEFT JOIN post_hashtags ph ON h.id = ph.hashtag_id
         WHERE LOWER(h.name) LIKE LOWER(?)
         GROUP BY h.id
         ORDER BY post_count DESC
         LIMIT 20"
    )
    .bind(format!("%{}%", params.q))
    .fetch_all(&state.db)
    .await?;
    
    Ok(Json(hashtags))
}
```


### 4. Activity Tracking

#### Increment Activity

```rust
async fn increment_hashtag_activity(
    db: &SqlitePool,
    user_id: &str,
    hashtag_id: &str,
) -> Result<(), AppError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    
    sqlx::query(
        "INSERT INTO user_hashtag_activity (user_id, hashtag_id, interaction_count, last_interaction)
         VALUES (?, ?, 1, ?)
         ON CONFLICT(user_id, hashtag_id) DO UPDATE SET
            interaction_count = interaction_count + 1,
            last_interaction = ?"
    )
    .bind(user_id)
    .bind(hashtag_id)
    .bind(now)
    .bind(now)
    .execute(db)
    .await?;
    
    Ok(())
}
```

#### Get Most Active Hashtags

```rust
pub async fn get_most_active_hashtags(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<HashtagActivity>>, AppError> {
    let hashtags = sqlx::query_as::<_, HashtagActivity>(
        "SELECT h.name, uha.interaction_count, uha.last_interaction
         FROM user_hashtag_activity uha
         JOIN hashtags h ON uha.hashtag_id = h.id
         WHERE uha.user_id = ?
         ORDER BY uha.interaction_count DESC
         LIMIT 5"
    )
    .bind(&user.id)
    .fetch_all(&state.db)
    .await?;
    
    Ok(Json(hashtags))
}
```

### 5. TUI Implementation

#### State Management

```rust
// tui/src/app.rs
pub enum FilterType {
    All,
    Hashtag(String),
    User(String),
}

pub struct FilterState {
    pub current_filter: FilterType,
    pub show_filter_modal: bool,
    pub modal_tab: FilterModalTab,  // Hashtags, Users, All
    pub followed_hashtags: Vec<HashtagInfo>,
    pub bookmarked_users: Vec<UserInfo>,
    pub selected_item: usize,
}

pub enum FilterModalTab {
    Hashtags,
    Users,
    All,
}

pub struct HashtagInfo {
    pub id: String,
    pub name: String,
    pub post_count: usize,
}
```


#### Filter Modal Rendering

```rust
// tui/src/ui.rs
fn render_filter_modal(frame: &mut Frame, app: &App, area: Rect) {
    let modal_area = centered_rect(60, 70, area);
    
    // Clear background
    frame.render_widget(Clear, modal_area);
    
    let block = Block::default()
        .title("Filter Posts")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    
    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);
    
    // Split into tabs and content
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Tab bar
            Constraint::Min(0),     // Content
            Constraint::Length(2),  // Footer
        ])
        .split(inner);
    
    // Render tabs
    render_filter_tabs(frame, app, chunks[0]);
    
    // Render content based on active tab
    match app.filter_state.modal_tab {
        FilterModalTab::Hashtags => render_hashtags_tab(frame, app, chunks[1]),
        FilterModalTab::Users => render_users_tab(frame, app, chunks[1]),
        FilterModalTab::All => render_all_tab(frame, app, chunks[1]),
    }
    
    // Render footer
    let footer = Paragraph::new("Tab: Switch | ↑/↓: Navigate | Enter: Select | X: Remove | Esc: Cancel")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));
    frame.render_widget(footer, chunks[2]);
}

fn render_filter_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let tabs = vec!["Hashtags", "Users", "All"];
    let selected = match app.filter_state.modal_tab {
        FilterModalTab::Hashtags => 0,
        FilterModalTab::Users => 1,
        FilterModalTab::All => 2,
    };
    
    let tabs_widget = Tabs::new(tabs)
        .select(selected)
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .divider("|");
    
    frame.render_widget(tabs_widget, area);
}

fn render_hashtags_tab(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app.filter_state.followed_hashtags
        .iter()
        .map(|h| {
            let content = format!("#{} ({} posts)", h.name, h.post_count);
            ListItem::new(content)
        })
        .collect();
    
    let list = List::new(items)
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");
    
    let mut list_state = ListState::default();
    list_state.select(Some(app.filter_state.selected_item));
    
    frame.render_stateful_widget(list, area, &mut list_state);
}
```


#### Filter Display

```rust
// tui/src/ui.rs
fn render_posts_header(frame: &mut Frame, app: &App, area: Rect) {
    let filter_text = match &app.filter_state.current_filter {
        FilterType::All => "[All Posts]".to_string(),
        FilterType::Hashtag(name) => format!("[#{}]", name),
        FilterType::User(username) => format!("[@{}]", username),
    };
    
    let sort_text = match app.posts_state.sort_order {
        SortOrder::Newest => "Newest",
        SortOrder::Top => "Top",
        SortOrder::Hot => "Hot",
    };
    
    let header_text = format!("{} (Sorted by: {})", filter_text, sort_text);
    
    let header = Paragraph::new(header_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    
    frame.render_widget(header, area);
}
```

#### Key Handling

```rust
// tui/src/app.rs
pub fn handle_filter_modal_keys(&mut self, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            self.filter_state.show_filter_modal = false;
            return Ok(());
        }
        KeyCode::Tab => {
            // Cycle through tabs
            self.filter_state.modal_tab = match self.filter_state.modal_tab {
                FilterModalTab::Hashtags => FilterModalTab::Users,
                FilterModalTab::Users => FilterModalTab::All,
                FilterModalTab::All => FilterModalTab::Hashtags,
            };
            self.filter_state.selected_item = 0;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if self.filter_state.selected_item > 0 {
                self.filter_state.selected_item -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let max_items = match self.filter_state.modal_tab {
                FilterModalTab::Hashtags => self.filter_state.followed_hashtags.len(),
                FilterModalTab::Users => self.filter_state.bookmarked_users.len(),
                FilterModalTab::All => 1,
            };
            if self.filter_state.selected_item < max_items.saturating_sub(1) {
                self.filter_state.selected_item += 1;
            }
        }
        KeyCode::Enter => {
            self.apply_selected_filter().await?;
            self.filter_state.show_filter_modal = false;
        }
        KeyCode::Char('x') | KeyCode::Char('X') => {
            self.remove_selected_filter_item().await?;
        }
        _ => {}
    }
    Ok(())
}

async fn apply_selected_filter(&mut self) -> Result<()> {
    match self.filter_state.modal_tab {
        FilterModalTab::Hashtags => {
            if let Some(hashtag) = self.filter_state.followed_hashtags.get(self.filter_state.selected_item) {
                self.filter_state.current_filter = FilterType::Hashtag(hashtag.name.clone());
                self.load_filtered_posts().await?;
            }
        }
        FilterModalTab::Users => {
            if let Some(user) = self.filter_state.bookmarked_users.get(self.filter_state.selected_item) {
                self.filter_state.current_filter = FilterType::User(user.username.clone());
                self.load_filtered_posts().await?;
            }
        }
        FilterModalTab::All => {
            self.filter_state.current_filter = FilterType::All;
            self.load_posts().await?;
        }
    }
    Ok(())
}
```


### 6. Sort Order Management

#### State

```rust
pub enum SortOrder {
    Newest,
    Top,
    Hot,
}

pub struct PostsState {
    // ... existing fields ...
    pub sort_order: SortOrder,
}
```

#### Sort Cycling

```rust
pub fn cycle_sort_order(&mut self) {
    self.posts_state.sort_order = match self.posts_state.sort_order {
        SortOrder::Newest => SortOrder::Top,
        SortOrder::Top => SortOrder::Hot,
        SortOrder::Hot => SortOrder::Newest,
    };
    
    // Reload posts with new sort order
    tokio::spawn(async move {
        self.load_filtered_posts().await
    });
}

// In handle_key_event
KeyCode::Char('s') | KeyCode::Char('S') => {
    if self.current_tab == Tab::Posts {
        self.cycle_sort_order();
    }
}
```

#### Persistence

```rust
// config.rs
#[derive(Serialize, Deserialize)]
pub struct SortPreferences {
    pub global: SortOrder,
    pub hashtag_filters: HashMap<String, SortOrder>,
    pub user_filters: HashMap<String, SortOrder>,
}

impl App {
    pub fn save_sort_preference(&self) -> Result<()> {
        let prefs = SortPreferences {
            global: if matches!(self.filter_state.current_filter, FilterType::All) {
                self.posts_state.sort_order
            } else {
                self.config.sort_prefs.global
            },
            hashtag_filters: self.config.sort_prefs.hashtag_filters.clone(),
            user_filters: self.config.sort_prefs.user_filters.clone(),
        };
        
        // Update based on current filter
        match &self.filter_state.current_filter {
            FilterType::Hashtag(name) => {
                prefs.hashtag_filters.insert(name.clone(), self.posts_state.sort_order);
            }
            FilterType::User(username) => {
                prefs.user_filters.insert(username.clone(), self.posts_state.sort_order);
            }
            _ => {}
        }
        
        let config_path = dirs::home_dir()
            .ok_or_else(|| anyhow!("Could not find home directory"))?
            .join(".fido")
            .join("sort_preferences.json");
        
        let json = serde_json::to_string_pretty(&prefs)?;
        std::fs::write(config_path, json)?;
        
        Ok(())
    }
}
```

## Data Models

```rust
// Shared types (fido-types crate)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hashtag {
    pub id: String,
    pub name: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashtagWithCount {
    pub id: String,
    pub name: String,
    pub post_count: usize,
    pub followed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashtagActivity {
    pub name: String,
    pub interaction_count: i32,
    pub last_interaction: i64,
}
```

## Error Handling

- Invalid hashtag names: Return 400 Bad Request
- Hashtag not found: Return 404 Not Found
- Duplicate follows: Silently ignore (INSERT OR IGNORE)
- Database errors: Return 500 Internal Server Error
- Empty filter results: Return empty array (not an error)

## Testing Strategy

### Unit Tests

- Test hashtag extraction regex with various inputs
- Test get_or_create_hashtag with existing and new hashtags
- Test activity increment logic
- Test sort order cycling

### Integration Tests

- Test post creation with hashtag extraction
- Test follow/unfollow flow
- Test filtered post retrieval
- Test search functionality
- Test activity tracking across multiple interactions

### Manual Testing

- Create posts with various hashtag formats
- Follow and unfollow hashtags
- Filter posts by hashtag
- Cycle through sort orders
- Verify activity metrics on profile page


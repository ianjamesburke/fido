# Design Document

## Overview

This design addresses critical UI/UX issues in the Fido TUI application. The solution focuses on refactoring the bio editor, standardizing navigation UI, resolving keyboard shortcut conflicts, stabilizing feed sorting behavior, implementing pull-to-refresh, adding text wrapping, improving edge case handling, and ensuring comment counts update immediately after user actions. The design maintains the existing Ratatui architecture while improving consistency, reliability, and user experience through optimistic updates and intelligent state management.

## Architecture

### Current Architecture Analysis

The Fido TUI follows a clean separation of concerns:

- **app.rs**: Application state management, event handling, and business logic
- **ui.rs**: Rendering logic using Ratatui widgets
- **api/client.rs**: HTTP client for backend communication
- **main.rs**: Event loop and async runtime coordination

### Key Components Affected

1. **Bio Editor Modal** (ui.rs, app.rs)
2. **Navigation Bars** (ui.rs - footer rendering)
3. **Event Handler** (app.rs - handle_key_event)
4. **Posts State** (app.rs - PostsState, vote_on_selected_post)
5. **Text Rendering** (ui.rs - format_post_content, render functions)

## Components and Interfaces

### 1. Bio Editor Refactor

#### Current Issues
- Enter key adds newline instead of saving
- Escape exits entire application instead of closing modal
- Cursor positioning is incorrect
- Authorization errors are unclear
- Modal doesn't resemble Settings page style

#### Design Solution

**State Management (app.rs)**
```rust
pub struct ProfileState {
    // ... existing fields ...
    pub show_edit_bio_modal: bool,
    pub edit_bio_content: String,
    pub edit_bio_cursor_position: usize,  // NEW: Track cursor position
}
```

**Modal Rendering (ui.rs)**
- Use `tui-textarea` widget for multi-line input with proper cursor handling
- Match Settings page modal style: centered, bordered, with clear instructions
- Display character count (160 max) with color coding (green < 140, yellow < 160, red >= 160)
- Show clear error messages with specific authorization details

**Key Handling (app.rs)**
```rust
fn handle_edit_bio_modal_keys(&mut self, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Enter => {
            // Save bio and close modal
            // Trigger async save in main loop
        }
        KeyCode::Esc => {
            // Close modal without saving
            self.close_edit_bio_modal();
        }
        KeyCode::Char(c) => {
            // Add character with proper cursor handling
        }
        KeyCode::Backspace => {
            // Remove character at cursor position
        }
        _ => {}
    }
}
```

**API Error Handling**
- Parse 401/403 responses to show "You can only edit your own profile"
- Parse 400 responses to show validation errors
- Parse network errors to show "Connection failed - check your network"

### 2. Navigation Bar Consistency

#### Current Issues
- Post count and navigation are left-aligned instead of centered
- Different formatting across tabs (Settings vs Posts vs DMs)
- Page-specific shortcuts mixed with global shortcuts

#### Design Solution

**Two-Tier Navigation System**

**Tier 1: Page-Specific Actions** (centered box above global footer)
```
┌─────────────────────────────────────────────────┐
│  Showing 25 posts | ↑/↓: Navigate | u/d: Vote  │
│  Ctrl+R: Refresh | Type above to post          │
└─────────────────────────────────────────────────┘
```

**Tier 2: Global Navigation** (bottom bar)
```
┌─────────────────────────────────────────────────┐
│ Tab: Next | Shift+Tab: Previous | Shift+L:     │
│ Logout | Ctrl+H: Help | q/Esc: Quit            │
└─────────────────────────────────────────────────┘
```

**Implementation**
```rust
// ui.rs
fn render_main_screen(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),      // Tab header
            Constraint::Min(0),         // Content
            Constraint::Length(3),      // Page-specific actions (NEW)
            Constraint::Length(3),      // Global footer
        ])
        .split(area);
    
    render_tab_header(frame, app, chunks[0]);
    render_tab_content(frame, app, chunks[1]);
    render_page_actions(frame, app, chunks[2]);  // NEW
    render_global_footer(frame, chunks[3]);
}

fn render_page_actions(frame: &mut Frame, app: &mut App, area: Rect) {
    let text = match app.current_tab {
        Tab::Posts => format!(
            "Showing {} posts | ↑/↓: Navigate | u/d: Vote | Ctrl+R: Refresh",
            app.posts_state.posts.len()
        ),
        Tab::DMs => "↑/↓: Navigate | n: New Conversation | Enter: Send".to_string(),
        Tab::Profile => "↑/↓: Navigate | e: Edit Bio".to_string(),
        Tab::Settings => "↑/↓: Navigate | ←/→: Change | s: Save".to_string(),
    };
    
    let widget = Paragraph::new(text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(widget, area);
}
```

### 3. Escape Key Behavior Resolution

#### Current Issues
- Escape closes application even when modals are open
- Inconsistent behavior across different screens
- No clear priority for Escape key handling

#### Design Solution

**Event Processing Priority**
1. Help modal (highest priority)
2. Save confirmation modal
3. Input modals (Bio, New Post, New Conversation)
4. Application exit (lowest priority)

**Implementation**
```rust
// app.rs
pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
    if key.kind != KeyEventKind::Press {
        return Ok(());
    }

    // Priority 1: Help modal
    if self.show_help {
        if matches!(key.code, KeyCode::Esc) {
            self.toggle_help();
            return Ok(());
        }
    }

    // Priority 2: Save confirmation modal
    if self.settings_state.show_save_confirmation {
        if matches!(key.code, KeyCode::Esc) {
            self.cancel_tab_switch();
            return Ok(());
        }
        // Handle Y/N for save confirmation
        return self.handle_save_confirmation_keys(key);
    }

    // Priority 3: Input modals
    if self.posts_state.show_new_post_modal {
        if matches!(key.code, KeyCode::Esc) {
            self.close_new_post_modal();
            return Ok(());
        }
        return self.handle_new_post_modal_keys(key);
    }

    if self.profile_state.show_edit_bio_modal {
        if matches!(key.code, KeyCode::Esc) {
            self.close_edit_bio_modal();
            return Ok(());
        }
        return self.handle_edit_bio_modal_keys(key);
    }

    if self.dms_state.show_new_conversation_modal {
        if matches!(key.code, KeyCode::Esc) {
            self.close_new_conversation_modal();
            return Ok(());
        }
        return self.handle_new_conversation_modal_keys(key);
    }

    // Priority 4: Global keys (including application exit)
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            self.running = false;
            return Ok(());
        }
        KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            self.toggle_help();
            return Ok(());
        }
        _ => {}
    }

    // Screen-specific handling
    match self.current_screen {
        Screen::Auth => self.handle_auth_keys(key),
        Screen::Main => self.handle_main_keys(key),
    }
}
```

### 4. Feed Sorting Stability

#### Current Issues
- Voting triggers immediate feed reload and re-sort
- Post order changes while browsing (disorienting)
- Selected post index becomes invalid after reload

#### Design Solution

**Local Vote Updates**
```rust
// app.rs
pub async fn vote_on_selected_post(&mut self, direction: &str) -> Result<()> {
    if let Some(selected_index) = self.posts_state.list_state.selected() {
        self.posts_state.error = None;
        
        let selected_post = &mut self.posts_state.posts[selected_index];
        let post_id = selected_post.id;
        
        // Optimistic update: modify local state immediately
        match direction {
            "up" => selected_post.upvotes += 1,
            "down" => selected_post.downvotes += 1,
            _ => {}
        }
        
        // Send vote to server (don't reload feed)
        match self.api_client.vote_on_post(post_id, direction.to_string()).await {
            Ok(_) => {
                // Success - local state already updated
            }
            Err(e) => {
                // Revert optimistic update on error
                match direction {
                    "up" => selected_post.upvotes -= 1,
                    "down" => selected_post.downvotes -= 1,
                    _ => {}
                }
                self.posts_state.error = Some(format!("Vote failed: {}", e));
            }
        }
        
        // Preserve selection - no reload, no re-sort
    }
    Ok(())
}
```

**Explicit Refresh**
- Only reload and re-sort when user presses Ctrl+R
- Preserve scroll position after refresh when possible
- Show loading indicator during refresh

### 5. Pull-to-Refresh Behavior

#### Current Issues
- Pressing up at top wraps to bottom
- No social media-style "pull to refresh" pattern
- No visual feedback for refresh action

#### Design Solution

**State Management**
```rust
pub struct PostsState {
    // ... existing fields ...
    pub show_refresh_prompt: bool,  // NEW: Show "Pull to Refresh" prompt
}
```

**Navigation Logic**
```rust
// app.rs
pub fn previous_post(&mut self) {
    if self.posts_state.posts.is_empty() {
        return;
    }
    
    let current = self.posts_state.list_state.selected();
    
    match current {
        Some(0) => {
            // At first post - show refresh prompt
            if self.posts_state.show_refresh_prompt {
                // Already showing prompt - trigger refresh
                // Will be handled async in main loop
            } else {
                // Show refresh prompt
                self.posts_state.show_refresh_prompt = true;
            }
        }
        Some(i) => {
            // Normal navigation
            self.posts_state.list_state.select(Some(i - 1));
            self.posts_state.show_refresh_prompt = false;
        }
        None => {
            self.posts_state.list_state.select(Some(0));
        }
    }
}

pub fn next_post(&mut self) {
    // Hide refresh prompt when moving down
    self.posts_state.show_refresh_prompt = false;
    
    // ... existing navigation logic ...
}
```

**UI Rendering**
```rust
// ui.rs
fn render_posts_tab_with_data(frame: &mut Frame, app: &mut App, area: Rect) {
    // ... existing code ...
    
    let mut items: Vec<ListItem> = Vec::new();
    
    // Add refresh prompt at top if showing
    if app.posts_state.show_refresh_prompt {
        let refresh_prompt = vec![
            Line::from(""),
            Line::from(Span::styled(
                "↑ Pull to Refresh ↑",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            )),
            Line::from(Span::styled(
                "Press ↑ again to refresh feed",
                Style::default().fg(Color::Gray)
            )),
            Line::from(""),
            Line::from("─".repeat(50)),
            Line::from(""),
        ];
        items.push(ListItem::new(refresh_prompt));
    }
    
    // Add posts
    for (i, post) in app.posts_state.posts.iter().enumerate() {
        // ... existing post rendering ...
    }
    
    // ... rest of rendering ...
}
```

### 6. Text Wrapping for Posts

#### Current Issues
- Long lines overflow terminal width
- No word wrapping in posts, compose box, bio editor, or DMs
- Text becomes unreadable on narrow terminals

#### Design Solution

**Use textwrap crate** (already in dependencies)

**Post Content Wrapping**
```rust
// ui.rs
fn format_post_content(content: &str, is_selected: bool, theme: &ThemeColors, max_width: usize) -> Vec<Line<'static>> {
    let mut lines = vec![];
    
    for line in content.lines() {
        // Wrap each line to terminal width
        let wrapped = textwrap::wrap(line, max_width - 4); // -4 for indent and borders
        
        for wrapped_line in wrapped {
            let mut spans = vec![Span::raw("  ")]; // Indent
            
            // Split by words and highlight hashtags/mentions
            for word in wrapped_line.split_whitespace() {
                if word.starts_with('#') {
                    let hashtag_style = if is_selected {
                        Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.secondary)
                    };
                    spans.push(Span::styled(word.to_string(), hashtag_style));
                } else if word.starts_with('@') {
                    let mention_style = if is_selected {
                        Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.primary)
                    };
                    spans.push(Span::styled(word.to_string(), mention_style));
                } else {
                    let text_style = if is_selected {
                        Style::default().fg(theme.text).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.text)
                    };
                    spans.push(Span::styled(word.to_string(), text_style));
                }
                spans.push(Span::raw(" "));
            }
            
            lines.push(Line::from(spans));
        }
    }
    
    lines
}
```

**Compose Box Wrapping**
```rust
// ui.rs
fn render_compose_box(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);
    let input_width = area.width.saturating_sub(4) as usize; // Account for borders and padding
    
    let content_lines = if app.posts_state.new_post_content.is_empty() {
        vec![Line::from(Span::styled(
            "Commit your thoughts here...",
            Style::default().fg(theme.text_dim),
        ))]
    } else {
        // Wrap text as user types
        let wrapped = textwrap::wrap(&app.posts_state.new_post_content, input_width);
        wrapped.iter().map(|line| Line::from(line.to_string())).collect()
    };
    
    // ... rest of rendering with proper cursor positioning ...
}
```

**Bio Editor Wrapping**
- Apply same wrapping logic to bio editor modal
- Ensure cursor position accounts for wrapped lines

**DM Message Wrapping**
- Wrap message content in conversation view
- Wrap input text in message compose area

**Terminal Resize Handling**
- Ratatui automatically triggers re-render on terminal resize
- Wrapping will recalculate based on new dimensions
- No additional resize handling needed

### 7. Edge Case Analysis and Bug Prevention

#### Design Solutions

**Network Error Handling**
```rust
// app.rs
pub async fn vote_on_selected_post(&mut self, direction: &str) -> Result<()> {
    // ... existing code ...
    
    match self.api_client.vote_on_post(post_id, direction.to_string()).await {
        Ok(_) => { /* success */ }
        Err(e) => {
            // Categorize errors
            let error_msg = if e.to_string().contains("connection") {
                "Network error: Check your connection and try again"
            } else if e.to_string().contains("401") || e.to_string().contains("403") {
                "Authorization error: Please log in again"
            } else if e.to_string().contains("timeout") {
                "Request timeout: Server is slow or unreachable"
            } else {
                "Vote failed: Please try again"
            };
            
            self.posts_state.error = Some(error_msg.to_string());
            
            // Revert optimistic update
            // ... existing revert logic ...
        }
    }
    Ok(())
}
```

**Minimum Terminal Size**
```rust
// ui.rs
pub fn render(app: &mut App, frame: &mut Frame) {
    let area = frame.size();
    
    // Check minimum dimensions
    const MIN_WIDTH: u16 = 60;
    const MIN_HEIGHT: u16 = 20;
    
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        let warning = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "Terminal Too Small",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            )),
            Line::from(""),
            Line::from(format!("Minimum size: {}x{}", MIN_WIDTH, MIN_HEIGHT)),
            Line::from(format!("Current size: {}x{}", area.width, area.height)),
            Line::from(""),
            Line::from("Please resize your terminal"),
        ])
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
        
        frame.render_widget(warning, area);
        return;
    }
    
    // Normal rendering
    match app.current_screen {
        Screen::Auth => render_auth_screen(frame, app),
        Screen::Main => render_main_screen(frame, app),
    }
}
```

**Rapid Key Press Handling**
- Ratatui's event loop already handles events sequentially
- No additional debouncing needed
- Async operations (API calls) are queued naturally by Tokio runtime

**Empty Input Prevention**
```rust
// app.rs
pub async fn submit_new_post(&mut self) -> Result<()> {
    let trimmed = self.posts_state.new_post_content.trim();
    
    if trimmed.is_empty() {
        self.posts_state.error = Some("Cannot post empty content".to_string());
        return Ok(());
    }
    
    if trimmed.len() > 280 {
        self.posts_state.error = Some("Post exceeds 280 characters".to_string());
        return Ok(());
    }
    
    // ... proceed with post creation ...
}

pub async fn send_dm(&mut self) -> Result<()> {
    let trimmed = self.dms_state.message_input.trim();
    
    if trimmed.is_empty() {
        self.dms_state.error = Some("Cannot send empty message".to_string());
        return Ok(());
    }
    
    // ... proceed with message sending ...
}
```

**Unsaved Changes Consistency**
```rust
// app.rs
fn try_switch_tab(&mut self, new_tab: Tab) {
    // Always check for unsaved changes when leaving Settings
    if self.current_tab == Tab::Settings && self.settings_state.has_unsaved_changes {
        self.settings_state.show_save_confirmation = true;
        self.settings_state.pending_tab = Some(new_tab);
    } else {
        self.current_tab = new_tab;
    }
}

// Also check on logout
pub fn logout(&mut self) -> Result<()> {
    if self.current_tab == Tab::Settings && self.settings_state.has_unsaved_changes {
        self.settings_state.show_save_confirmation = true;
        self.settings_state.pending_tab = None; // Special case: logout after save
        return Ok(());
    }
    
    // ... proceed with logout ...
}
```

**End of Feed Handling**
- Already implemented: end-of-feed message displays after last post
- No wrapping to top when scrolling past last post
- Clear visual indicator that user has reached the end

### 8. DM Unread Indicator Management

#### Current Issues
- Unread message count doesn't clear when viewing conversations
- Badge remains on DMs tab even after reading messages
- No mechanism to mark messages as read

#### Design Solution

**State Management**
```rust
// app.rs
pub struct DMsState {
    // ... existing fields ...
    pub unread_counts: HashMap<String, usize>,  // NEW: user_id -> unread count
    pub current_conversation_user: Option<String>,  // Track which conversation is open
}

pub struct DirectMessage {
    pub id: String,
    pub from_id: String,
    pub to_id: String,
    pub content: String,
    pub created_at: String,
    pub read: bool,  // NEW: Track read status
}
```

**Mark Messages as Read**
```rust
// app.rs
pub async fn open_conversation(&mut self, user_id: String) -> Result<()> {
    self.dms_state.current_conversation_user = Some(user_id.clone());
    
    // Load messages for this conversation
    self.load_conversation_messages(&user_id).await?;
    
    // Mark all messages in this conversation as read
    self.mark_conversation_as_read(&user_id).await?;
    
    // Clear unread count for this user
    self.dms_state.unread_counts.insert(user_id, 0);
    
    Ok(())
}

pub async fn mark_conversation_as_read(&mut self, user_id: &str) -> Result<()> {
    // Call API to mark messages as read
    self.api_client.mark_messages_read(user_id).await?;
    
    // Update local state
    if let Some(messages) = self.dms_state.conversations.get_mut(user_id) {
        for msg in messages.iter_mut() {
            msg.read = true;
        }
    }
    
    Ok(())
}
```

**API Endpoint**
```rust
// Add to api/client.rs
pub async fn mark_messages_read(&self, user_id: &str) -> Result<()> {
    let url = format!("{}/dms/mark-read/{}", self.base_url, user_id);
    
    self.client
        .post(&url)
        .header("Authorization", format!("Bearer {}", self.token))
        .send()
        .await?
        .error_for_status()?;
    
    Ok(())
}
```

**Unread Badge Display**
```rust
// ui.rs
fn render_tab_header(frame: &mut Frame, app: &App, area: Rect) {
    // Calculate total unread count
    let total_unread: usize = app.dms_state.unread_counts.values().sum();
    
    let dm_label = if total_unread > 0 {
        format!("DMs ({})", total_unread)
    } else {
        "DMs".to_string()
    };
    
    // ... render tabs with badge ...
}
```

### 9. Keyboard Shortcut Conflict Resolution

#### Current Issues
- 'N' key consumed when typing DM messages
- 'u' and 'd' keys consumed when typing posts
- No distinction between typing mode and navigation mode

#### Design Solution

**Input Mode Tracking**
```rust
// app.rs
pub enum InputMode {
    Navigation,  // Browsing content, shortcuts active
    Typing,      // In text input, shortcuts disabled
}

pub struct App {
    // ... existing fields ...
    pub input_mode: InputMode,  // NEW: Track current input mode
}
```

**Mode-Aware Key Handling**
```rust
// app.rs
pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
    if key.kind != KeyEventKind::Press {
        return Ok(());
    }

    // Priority 1-3: Modals (same as before)
    // ...

    // Check input mode before processing shortcuts
    match self.input_mode {
        InputMode::Typing => {
            // In typing mode, only process special keys
            match key.code {
                KeyCode::Esc => {
                    // Exit typing mode
                    self.input_mode = InputMode::Navigation;
                    self.hide_active_input();
                    return Ok(());
                }
                KeyCode::Enter => {
                    // Submit input
                    self.submit_active_input().await?;
                    self.input_mode = InputMode::Navigation;
                    return Ok(());
                }
                _ => {
                    // Pass all other keys to active input handler
                    return self.handle_typing_input(key);
                }
            }
        }
        InputMode::Navigation => {
            // In navigation mode, process all shortcuts
            match key.code {
                KeyCode::Char('n') => {
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        // Ctrl+N: New DM conversation
                        if self.current_tab == Tab::DMs {
                            self.show_new_conversation_modal();
                            self.input_mode = InputMode::Typing;
                        }
                    } else {
                        // 'n': New post (show compose box)
                        if self.current_tab == Tab::Posts {
                            self.show_compose_box();
                            self.input_mode = InputMode::Typing;
                        }
                    }
                    return Ok(());
                }
                KeyCode::Char('u') => {
                    // Upvote (only in navigation mode)
                    if self.current_tab == Tab::Posts {
                        self.vote_on_selected_post("up").await?;
                    }
                    return Ok(());
                }
                KeyCode::Char('d') => {
                    // Downvote (only in navigation mode)
                    if self.current_tab == Tab::Posts {
                        self.vote_on_selected_post("down").await?;
                    }
                    return Ok(());
                }
                _ => {}
            }
            
            // Continue with other navigation shortcuts
            // ...
        }
    }
    
    Ok(())
}
```

**DM Input Handling**
```rust
// app.rs
fn handle_typing_input(&mut self, key: KeyEvent) -> Result<()> {
    match self.current_tab {
        Tab::Posts if self.posts_state.show_compose_box => {
            // Handle post compose input
            match key.code {
                KeyCode::Char(c) => {
                    self.posts_state.new_post_content.push(c);
                }
                KeyCode::Backspace => {
                    self.posts_state.new_post_content.pop();
                }
                _ => {}
            }
        }
        Tab::DMs if self.dms_state.show_message_input => {
            // Handle DM message input
            match key.code {
                KeyCode::Char(c) => {
                    self.dms_state.message_input.push(c);
                }
                KeyCode::Backspace => {
                    self.dms_state.message_input.pop();
                }
                _ => {}
            }
        }
        _ => {}
    }
    Ok(())
}
```

### 10. Compose Box Visibility Toggle

#### Current Issues
- Compose box always visible at top of Posts tab
- Creates keyboard shortcut conflicts
- Clutters interface when not composing

#### Design Solution

**State Management**
```rust
// app.rs
pub struct PostsState {
    // ... existing fields ...
    pub show_compose_box: bool,  // NEW: Toggle compose box visibility
}
```

**Show/Hide Logic**
```rust
// app.rs
pub fn show_compose_box(&mut self) {
    self.posts_state.show_compose_box = true;
    self.posts_state.new_post_content.clear();
    self.input_mode = InputMode::Typing;
}

pub fn hide_compose_box(&mut self) {
    self.posts_state.show_compose_box = false;
    self.posts_state.new_post_content.clear();
    self.input_mode = InputMode::Navigation;
}

pub async fn submit_new_post(&mut self) -> Result<()> {
    let trimmed = self.posts_state.new_post_content.trim();
    
    if trimmed.is_empty() {
        self.posts_state.error = Some("Cannot post empty content".to_string());
        return Ok(());
    }
    
    // Submit post
    match self.api_client.create_post(trimmed.to_string()).await {
        Ok(_) => {
            self.hide_compose_box();  // Hide on success
            self.load_posts().await?;
        }
        Err(e) => {
            self.posts_state.error = Some(format!("Failed to post: {}", e));
        }
    }
    
    Ok(())
}
```

**Conditional Rendering**
```rust
// ui.rs
fn render_posts_tab_with_data(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = if app.posts_state.show_compose_box {
        // Show compose box at top
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),  // Compose box
                Constraint::Min(0),     // Posts list
            ])
            .split(area)
    } else {
        // No compose box, full height for posts
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),  // Posts list
            ])
            .split(area)
    };
    
    if app.posts_state.show_compose_box {
        render_compose_box(frame, app, chunks[0]);
        render_posts_list(frame, app, chunks[1]);
    } else {
        render_posts_list(frame, app, chunks[0]);
    }
}
```

**Navigation Bar Update**
```rust
// ui.rs
fn render_page_actions(frame: &mut Frame, app: &mut App, area: Rect) {
    let text = match app.current_tab {
        Tab::Posts => {
            if app.posts_state.show_compose_box {
                "Type your post | Enter: Submit | Esc: Cancel".to_string()
            } else {
                format!(
                    "Showing {} posts | ↑/↓: Navigate | u/d: Vote | n: New Post | Ctrl+R: Refresh",
                    app.posts_state.posts.len()
                )
            }
        }
        Tab::DMs => {
            if app.input_mode == InputMode::Typing {
                "Type your message | Enter: Send | Esc: Cancel".to_string()
            } else {
                "↑/↓: Navigate | Ctrl+N: New Conversation | Enter: Open".to_string()
            }
        }
        // ... other tabs ...
    };
    
    // ... render with centered alignment ...
}
```

### 11. Cross-Platform Keyboard Input Consistency

#### Current Issues
- Ctrl+H doesn't work on macOS (help modal doesn't open)
- Ctrl+R and other Control-based shortcuts inconsistent on macOS
- Ctrl+D works for DM deletion but other Ctrl shortcuts fail
- UI displays "Ctrl" on all platforms instead of "Cmd" on macOS

#### Design Solution

**Platform Detection**
```rust
// Add to app.rs or ui.rs
#[cfg(target_os = "macos")]
const MODIFIER_KEY_DISPLAY: &str = "Cmd";

#[cfg(not(target_os = "macos"))]
const MODIFIER_KEY_DISPLAY: &str = "Ctrl";

pub fn get_modifier_key_name() -> &'static str {
    MODIFIER_KEY_DISPLAY
}
```

**Event Handler Investigation**
```rust
// app.rs - Debug Control key detection
pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
    if key.kind != KeyEventKind::Press {
        return Ok(());
    }

    // Log Control key events for debugging
    #[cfg(debug_assertions)]
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        eprintln!("Control key detected: {:?}", key);
    }

    // Ensure Control modifier is checked consistently
    match key.code {
        KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            self.toggle_help();
            return Ok(());
        }
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if self.current_tab == Tab::Posts {
                self.refresh_posts().await?;
            }
            return Ok(());
        }
        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if self.current_tab == Tab::DMs {
                self.show_new_conversation_modal();
                self.input_mode = InputMode::Typing;
            }
            return Ok(());
        }
        _ => {}
    }
    
    // ... rest of event handling ...
}
```

**UI Display Updates**
```rust
// ui.rs - Update all shortcut displays
fn render_global_footer(frame: &mut Frame, area: Rect) {
    let modifier = get_modifier_key_name();
    
    let shortcuts = format!(
        "Tab: Next | Shift+Tab: Previous | Shift+L: Logout | {}+H: Help | q/Esc: Quit",
        modifier
    );
    
    let widget = Paragraph::new(shortcuts)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    
    frame.render_widget(widget, area);
}

fn render_page_actions(frame: &mut Frame, app: &mut App, area: Rect) {
    let modifier = get_modifier_key_name();
    
    let text = match app.current_tab {
        Tab::Posts => format!(
            "Showing {} posts | ↑/↓: Navigate | u/d: Vote | n: New Post | {}+R: Refresh",
            app.posts_state.posts.len(),
            modifier
        ),
        Tab::DMs => format!(
            "↑/↓: Navigate | {}+N: New Conversation | Enter: Open",
            modifier
        ),
        _ => String::new(),
    };
    
    // ... render widget ...
}
```

**Testing Approach**
- Test Ctrl+H, Ctrl+R, Ctrl+N on macOS specifically
- Verify Control modifier detection in crossterm events
- Compare working Ctrl+D implementation with non-working shortcuts
- Ensure UI displays "Cmd" on macOS, "Ctrl" elsewhere

### 12. Reply Modal Submission Consistency

#### Current Issues
- Reply modal requires Ctrl+Enter to submit
- Other composer modals (new post, DM) use plain Enter
- Inconsistent user experience across similar interfaces
- Help text shows "Ctrl+Enter to submit" in reply modal

#### Design Solution

**Key Handler Update**
```rust
// app.rs
fn handle_reply_modal_keys(&mut self, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Enter => {
            // Submit reply on plain Enter (not Ctrl+Enter)
            self.submit_reply().await?;
            self.close_reply_modal();
            return Ok(());
        }
        KeyCode::Esc => {
            self.close_reply_modal();
            return Ok(());
        }
        KeyCode::Char(c) => {
            self.reply_content.push(c);
        }
        KeyCode::Backspace => {
            self.reply_content.pop();
        }
        _ => {}
    }
    Ok(())
}
```

**UI Footer Update**
```rust
// ui.rs
fn render_reply_modal(frame: &mut Frame, app: &App, area: Rect) {
    // ... existing modal rendering ...
    
    let footer_text = "Enter: Submit | Esc: Cancel";
    
    let footer = Paragraph::new(footer_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));
    
    frame.render_widget(footer, footer_area);
}
```

**Consistency Check**
- Verify new post composer uses Enter (not Ctrl+Enter)
- Verify DM message input uses Enter (not Ctrl+Enter)
- Update reply modal to match this behavior
- Ensure all composer modals have consistent help text

### 13. DMs Page UX Improvements

#### Current Issues
- Cursor starts at None, requiring two down-presses to reach first conversation
- "New Conversations" header has no top padding (cramped appearance)
- Message input box is too small (1-2 lines instead of 4-5)

#### Design Solution

**Cursor Initialization**
```rust
// app.rs
pub struct DMsState {
    pub conversations: Vec<Conversation>,
    pub selected_conversation: Option<usize>,  // Initialize to Some(0)
    pub messages: Vec<DirectMessage>,
    pub message_input: String,
    // ... other fields ...
}

impl DMsState {
    pub fn new() -> Self {
        Self {
            conversations: Vec::new(),
            selected_conversation: Some(0),  // Start at first conversation
            messages: Vec::new(),
            message_input: String::new(),
            // ... other fields ...
        }
    }
}

// When loading conversations, ensure cursor is valid
pub async fn load_conversations(&mut self) -> Result<()> {
    self.dms_state.conversations = self.api_client.get_conversations().await?;
    
    // Initialize cursor to first conversation if we have any
    if !self.dms_state.conversations.is_empty() {
        self.dms_state.selected_conversation = Some(0);
    } else {
        self.dms_state.selected_conversation = None;
    }
    
    Ok(())
}
```

**Layout Improvements**
```rust
// ui.rs
fn render_dms_tab(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),      // Top padding (NEW)
            Constraint::Length(3),      // Header
            Constraint::Min(0),         // Conversation list
            Constraint::Length(6),      // Message input (increased from 3)
        ])
        .split(area);
    
    // Render empty space for top padding
    frame.render_widget(Block::default(), chunks[0]);
    
    // Render header with proper spacing
    let header = Paragraph::new("Conversations")
        .style(Style::default().add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    frame.render_widget(header, chunks[1]);
    
    // Render conversation list
    render_conversation_list(frame, app, chunks[2]);
    
    // Render message input with increased height
    render_message_input(frame, app, chunks[3]);
}

fn render_message_input(frame: &mut Frame, app: &mut App, area: Rect) {
    let input_block = Block::default()
        .borders(Borders::ALL)
        .title("Message")
        .border_style(Style::default().fg(Color::Cyan));
    
    let inner_area = input_block.inner(area);
    
    // Wrap text to fit within input area (4-5 lines visible)
    let wrapped = textwrap::wrap(&app.dms_state.message_input, inner_area.width as usize);
    let lines: Vec<Line> = wrapped.iter().map(|s| Line::from(s.to_string())).collect();
    
    let input_widget = Paragraph::new(lines)
        .block(input_block);
    
    frame.render_widget(input_widget, area);
}
```

### 14. Scroll Behavior Boundaries

#### Current Issues
- Posts page wraps to top when scrolling past bottom (disorienting)
- Trackpad scrolling triggers pull-to-refresh too aggressively
- No distinction between intentional pull-to-refresh and fast scrolling

#### Design Solution

**Scroll Bounds Implementation**
```rust
// app.rs
pub fn next_post(&mut self) {
    if self.posts_state.posts.is_empty() {
        return;
    }
    
    let current = self.posts_state.list_state.selected();
    let max_index = self.posts_state.posts.len() - 1;
    
    match current {
        Some(i) if i < max_index => {
            // Normal navigation
            self.posts_state.list_state.select(Some(i + 1));
            self.posts_state.show_refresh_prompt = false;
        }
        Some(i) if i >= max_index => {
            // At last post - stay here, don't wrap
            // Optionally show "End of Feed" message
            self.posts_state.at_end_of_feed = true;
        }
        None => {
            self.posts_state.list_state.select(Some(0));
        }
    }
}

pub fn previous_post(&mut self) {
    if self.posts_state.posts.is_empty() {
        return;
    }
    
    let current = self.posts_state.list_state.selected();
    
    match current {
        Some(0) => {
            // At first post - handle pull-to-refresh
            if self.posts_state.show_refresh_prompt {
                // Check scroll velocity before triggering refresh
                if self.should_trigger_refresh() {
                    self.posts_state.trigger_refresh = true;
                }
            } else {
                self.posts_state.show_refresh_prompt = true;
            }
        }
        Some(i) => {
            // Normal navigation
            self.posts_state.list_state.select(Some(i - 1));
            self.posts_state.show_refresh_prompt = false;
            self.posts_state.at_end_of_feed = false;
        }
        None => {
            self.posts_state.list_state.select(Some(0));
        }
    }
}

### 15. Comment Count Refresh After Reply

#### Current Issues
- Comment count doesn't update when user returns from threaded conversation view
- User must manually refresh (Ctrl+R removed) to see updated comment count
- Creates perception that reply wasn't successful
- Inconsistent with optimistic update pattern used for voting

#### Design Solution

**State Management**
```rust
// app.rs
pub struct ConversationState {
    pub viewing_post_id: Option<String>,  // NEW: Track which post's conversation is open
    pub reply_submitted: bool,  // NEW: Flag when reply is submitted
}

pub struct App {
    // ... existing fields ...
    pub conversation_state: ConversationState,  // NEW: Track conversation context
}
```

**Track Conversation Context**
```rust
// app.rs
pub fn open_conversation(&mut self, post_id: String) {
    // Store which post we're viewing
    self.conversation_state.viewing_post_id = Some(post_id.clone());
    self.conversation_state.reply_submitted = false;
    
    // Navigate to conversation view
    self.current_view = View::Conversation;
    
    // Load conversation messages
    self.load_conversation(post_id).await?;
}

pub async fn submit_reply(&mut self) -> Result<()> {
    let reply_content = self.reply_state.reply_content.trim();
    
    if reply_content.is_empty() {
        self.reply_state.error = Some("Cannot submit empty reply".to_string());
        return Ok(());
    }
    
    // Submit reply to API
    match self.api_client.create_reply(
        &self.reply_state.replying_to_post_id.unwrap(),
        reply_content.to_string()
    ).await {
        Ok(_) => {
            // Mark that a reply was submitted
            self.conversation_state.reply_submitted = true;
            
            // Clear reply content
            self.reply_state.reply_content.clear();
            self.reply_state.error = None;
        }
        Err(e) => {
            self.reply_state.error = Some(format!("Failed to submit reply: {}", e));
        }
    }
    
    Ok(())
}
```

**Update Comment Count on Return**
```rust
// app.rs
pub fn return_to_posts_tab(&mut self) {
    // Check if we submitted a reply
    if self.conversation_state.reply_submitted {
        if let Some(post_id) = &self.conversation_state.viewing_post_id {
            // Find the post in the cached list and increment comment count
            if let Some(post) = self.posts_state.posts.iter_mut()
                .find(|p| &p.id == post_id) {
                post.comment_count += 1;
            }
        }
    }
    
    // Clear conversation state
    self.conversation_state.viewing_post_id = None;
    self.conversation_state.reply_submitted = false;
    
    // Return to posts view
    self.current_view = View::Posts;
}
```

**Alternative: Increment on Each Reply**
```rust
// app.rs - More granular approach
pub async fn submit_reply(&mut self) -> Result<()> {
    let reply_content = self.reply_state.reply_content.trim();
    
    if reply_content.is_empty() {
        self.reply_state.error = Some("Cannot submit empty reply".to_string());
        return Ok(());
    }
    
    let post_id = self.reply_state.replying_to_post_id.clone().unwrap();
    
    // Submit reply to API
    match self.api_client.create_reply(&post_id, reply_content.to_string()).await {
        Ok(_) => {
            // Immediately update comment count in cached post
            if let Some(post) = self.posts_state.posts.iter_mut()
                .find(|p| p.id == post_id) {
                post.comment_count += 1;
            }
            
            // Clear reply content
            self.reply_state.reply_content.clear();
            self.reply_state.error = None;
        }
        Err(e) => {
            self.reply_state.error = Some(format!("Failed to submit reply: {}", e));
        }
    }
    
    Ok(())
}
```

**Scroll Velocity Detection**
```rust
// app.rs
use std::time::{Duration, Instant};

pub struct PostsState {
    // ... existing fields ...
    pub last_scroll_time: Option<Instant>,
    pub scroll_velocity_threshold: Duration,  // e.g., 100ms
}

impl App {
    fn should_trigger_refresh(&mut self) -> bool {
        let now = Instant::now();
        
        if let Some(last_time) = self.posts_state.last_scroll_time {
            let elapsed = now.duration_since(last_time);
            
            // If scrolling too fast (< threshold), ignore pull-to-refresh
            if elapsed < self.posts_state.scroll_velocity_threshold {
                self.posts_state.last_scroll_time = Some(now);
                return false;
            }
        }
        
        self.posts_state.last_scroll_time = Some(now);
        true
    }
}
```

**Configuration**
```rust
// config.rs or app.rs
pub struct ScrollConfig {
    pub velocity_threshold_ms: u64,  // Default: 100ms
    pub enable_pull_to_refresh: bool,  // Default: true
}

impl Default for ScrollConfig {
    fn default() -> Self {
        Self {
            velocity_threshold_ms: 100,
            enable_pull_to_refresh: true,
        }
    }
}
```

**End of Feed Indicator**
```rust
// ui.rs
fn render_posts_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let mut items: Vec<ListItem> = Vec::new();
    
    // Add refresh prompt at top if showing
    if app.posts_state.show_refresh_prompt {
        // ... existing refresh prompt code ...
    }
    
    // Add posts
    for post in &app.posts_state.posts {
        // ... existing post rendering ...
    }
    
    // Add end-of-feed indicator if at bottom
    if app.posts_state.at_end_of_feed {
        let end_message = vec![
            Line::from(""),
            Line::from("─".repeat(50)),
            Line::from(""),
            Line::from(Span::styled(
                "End of Feed",
                Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC)
            )),
            Line::from(""),
        ];
        items.push(ListItem::new(end_message));
    }
    
    // ... render list ...
}
```

## Data Models

Modifications to existing state structures:

```rust
// app.rs
pub enum InputMode {
    Navigation,  // Browsing content, shortcuts active
    Typing,      // In text input, shortcuts disabled
}

pub struct App {
    // ... existing fields ...
    pub input_mode: InputMode,  // NEW: Track current input mode
    pub conversation_state: ConversationState,  // NEW: Track conversation context
}

pub struct PostsState {
    pub posts: Vec<Post>,
    pub list_state: ListState,
    pub loading: bool,
    pub error: Option<String>,
    pub show_compose_box: bool,  // MODIFIED: was show_new_post_modal
    pub new_post_content: String,
    pub show_refresh_prompt: bool,  // NEW
    pub at_end_of_feed: bool,  // NEW: Track if at bottom of feed
    pub last_scroll_time: Option<Instant>,  // NEW: For velocity detection
    pub scroll_velocity_threshold: Duration,  // NEW: Configurable threshold
    pub trigger_refresh: bool,  // NEW: Flag to trigger refresh in main loop
}

pub struct ConversationState {
    pub viewing_post_id: Option<String>,  // NEW: Track which post's conversation is open
    pub reply_submitted: bool,  // NEW: Flag when reply is submitted
}

pub struct ProfileState {
    pub profile: Option<UserProfile>,
    pub user_posts: Vec<Post>,
    pub list_state: ListState,
    pub loading: bool,
    pub error: Option<String>,
    pub show_edit_bio_modal: bool,
    pub edit_bio_content: String,
    pub edit_bio_cursor_position: usize,  // NEW
}

pub struct DMsState {
    pub conversations: Vec<Conversation>,
    pub selected_conversation: Option<usize>,  // MODIFIED: Initialize to Some(0)
    pub messages: Vec<DirectMessage>,
    pub message_input: String,
    pub unread_counts: HashMap<String, usize>,  // NEW: user_id -> unread count
    pub current_conversation_user: Option<String>,  // NEW: Track open conversation
    pub show_message_input: bool,  // NEW: Track if message input is active
}

pub struct ReplyState {
    pub show_reply_modal: bool,
    pub reply_content: String,
    pub replying_to_post_id: Option<String>,
}

pub struct DirectMessage {
    pub id: String,
    pub from_id: String,
    pub to_id: String,
    pub content: String,
    pub created_at: String,
    pub read: bool,  // NEW: Track read status
}

pub struct Post {
    pub id: String,
    pub author_id: String,
    pub author_username: String,
    pub content: String,
    pub created_at: String,
    pub upvotes: i32,
    pub downvotes: i32,
    pub comment_count: i32,  // Track number of comments/replies
}
```

## Error Handling

### Error Categories

1. **Network Errors**: Connection failures, timeouts
2. **Authorization Errors**: 401/403 responses
3. **Validation Errors**: Empty input, character limits
4. **API Errors**: Unexpected server responses

### Error Display Strategy

- Display errors in context (modal errors in modal, tab errors in tab)
- Use color coding: Red for errors, Yellow for warnings, Green for success
- Provide actionable guidance: "Press 'r' to retry", "Check your connection"
- Clear errors on successful action or when user dismisses

### Error Recovery

- Optimistic updates with rollback on failure
- Preserve user input on error (don't clear forms)
- Maintain UI state (don't close modals on network error)
- Provide retry mechanisms (Ctrl+R for refresh, 's' for save)

## Testing Strategy

### Unit Testing

**State Management Tests**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_escape_key_priority() {
        let mut app = App::new();
        app.show_help = true;
        
        // Escape should close help, not exit app
        app.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty())).unwrap();
        
        assert!(!app.show_help);
        assert!(app.running); // App still running
    }
    
    #[test]
    fn test_vote_preserves_selection() {
        let mut app = App::new();
        app.posts_state.posts = vec![/* mock posts */];
        app.posts_state.list_state.select(Some(2));
        
        // Vote should preserve selection
        app.vote_on_selected_post("up").await.unwrap();
        
        assert_eq!(app.posts_state.list_state.selected(), Some(2));
    }
    
    #[test]
    fn test_pull_to_refresh_flow() {
        let mut app = App::new();
        app.posts_state.posts = vec![/* mock posts */];
        app.posts_state.list_state.select(Some(0));
        
        // First up at top shows prompt
        app.previous_post();
        assert!(app.posts_state.show_refresh_prompt);
        
        // Second up triggers refresh (checked in main loop)
        app.previous_post();
        // Refresh flag would be set here
    }
    
    #[test]
    fn test_text_wrapping() {
        let long_text = "a".repeat(100);
        let wrapped = textwrap::wrap(&long_text, 50);
        
        assert_eq!(wrapped.len(), 2); // Should wrap to 2 lines
    }
    
    #[test]
    fn test_empty_input_prevention() {
        let mut app = App::new();
        app.posts_state.new_post_content = "   ".to_string();
        
        app.submit_new_post().await.unwrap();
        
        assert!(app.posts_state.error.is_some());
        assert!(app.posts_state.error.as_ref().unwrap().contains("empty"));
    }
}
```

### Integration Testing

**Modal Flow Tests**
- Open bio editor → type content → press Enter → verify save called
- Open bio editor → press Escape → verify modal closed without save
- Vote on post → verify local update → verify no reload

**Navigation Tests**
- Navigate to top post → press up → verify refresh prompt shown
- Press up again → verify refresh triggered
- Navigate down → verify refresh prompt hidden

**Error Handling Tests**
- Simulate network error during vote → verify error displayed
- Simulate auth error during bio save → verify specific error message
- Submit empty post → verify validation error

### Manual Testing Checklist

- [ ] Bio editor: Enter saves, Escape cancels, cursor aligns correctly
- [ ] Navigation bars: Centered on all tabs, consistent styling
- [ ] Escape key: Closes modals first, then exits app
- [ ] Voting: Post order stable, counts update locally
- [ ] Pull-to-refresh: Prompt shows at top, second up refreshes
- [ ] Text wrapping: Long posts wrap correctly, no overflow
- [ ] Terminal resize: UI adapts correctly, text re-wraps
- [ ] Minimum size: Warning shows when terminal too small
- [ ] Empty input: Cannot submit empty posts or DMs
- [ ] Network errors: Clear messages, state preserved

## Implementation Notes

### Dependencies

All required dependencies already in Cargo.toml:
- `ratatui` - TUI framework
- `textwrap` - Text wrapping
- `crossterm` - Terminal event handling
- `tokio` - Async runtime

### Performance Considerations

- Text wrapping adds minimal overhead (O(n) where n = text length)
- Local vote updates eliminate network round-trip for UI update
- Refresh prompt adds one extra list item (negligible)
- Modal priority checking is O(1) constant time

### Backward Compatibility

- No breaking changes to API contracts
- No database schema changes
- Existing keyboard shortcuts preserved
- New shortcuts added without conflicts

### Future Enhancements

- Consider `tui-textarea` crate for advanced text editing (multi-line with proper cursor)
- Add undo/redo for text inputs
- Implement text selection and copy/paste
- Add search functionality in posts feed
- Implement infinite scroll with pagination

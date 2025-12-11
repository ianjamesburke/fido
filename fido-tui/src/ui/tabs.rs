use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::{log_modal_state, log_rendering};
use super::theme::{ThemeColors, get_theme_colors};
use super::formatting::*;
use super::modals::*;

pub fn render_auth_screen(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let theme = get_theme_colors(app);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    // Header
    let header = Paragraph::new("Fido - Terminal Social Platform")
        .style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    // Main content - ASCII logo
    let mut lines = vec![Line::from("")];
    
    // Fido ASCII art logo with theme colors
    const LOGO_LINES: &[&str] = &[
        "  _____ _     _       ",
        " |  ___(_) __| | ___  ",
        " | |_  | |/ _` |/ _ \\ ",
        " |  _| | | (_| | (_) |",
        " |_|   |_|\\__,_|\\___/ ",
    ];
    
    for logo_line in LOGO_LINES {
        lines.push(Line::from(Span::styled(
            *logo_line,
            Style::default().fg(theme.primary).add_modifier(Modifier::BOLD),
        )));
    }
    
    lines.push(Line::from(""));
    lines.push(Line::from(""));

    if app.auth_state.github_auth_in_progress {
        // Show GitHub Device Flow in progress
        lines.push(Line::from(Span::styled(
            "GitHub Device Authorization",
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
        
        if let Some(user_code) = &app.auth_state.github_user_code {
            lines.push(Line::from(Span::styled(
                "Enter this code on GitHub:",
                Style::default().fg(Color::White),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                user_code.clone(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::UNDERLINED),
            )));
            lines.push(Line::from(""));
        }
        
        if let Some(uri) = &app.auth_state.github_verification_uri {
            lines.push(Line::from(Span::styled(
                "If the browser didn't open, visit:",
                Style::default().fg(Color::White),
            )));
            lines.push(Line::from(Span::styled(
                uri.clone(),
                Style::default().fg(Color::White),
            )));
            lines.push(Line::from(""));
        }
        
        lines.push(Line::from(Span::styled(
            "Waiting for authorization...",
            Style::default().fg(Color::White),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Press Esc to cancel",
            Style::default().fg(Color::White),
        )));
    } else if app.auth_state.loading {
        lines.push(Line::from(Span::styled(
            "Loading...",
            Style::default().fg(Color::White),
        )));
    } else if let Some(error) = &app.auth_state.error {
        lines.push(Line::from(Span::styled(
            error.clone(),
            Style::default().fg(theme.error),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Press any key to continue",
            Style::default().fg(Color::White),
        )));
    } else if app.auth_state.test_users.is_empty() {
        lines.push(Line::from(Span::styled(
            "Choose authentication method:",
            Style::default().fg(Color::White),
        )));
        lines.push(Line::from(""));
        
        if app.auth_state.show_github_option {
            lines.push(Line::from(Span::styled(
                "Press 'g' to login with GitHub",
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
        }
        
        lines.push(Line::from(Span::styled(
            "Press 'l' to load test users (development only)",
            Style::default().fg(Color::White),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "Select a test user (development only):",
            Style::default().fg(Color::White),
        )));
        lines.push(Line::from(""));

        // Show only first 3 test users (alice, bob, charlie)
        for (i, user) in app.auth_state.test_users.iter().take(3).enumerate() {
            let style = if i == app.auth_state.selected_index {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let prefix = if i == app.auth_state.selected_index {
                "▶ "
            } else {
                "  "
            };
            let bio = user.bio.as_deref().unwrap_or("No bio");
            lines.push(Line::from(Span::styled(
                format!("{}{} - {}", prefix, user.username, bio),
                style,
            )));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Press Enter to login with test user",
            Style::default().fg(Color::White),
        )));
        
        if app.auth_state.show_github_option {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Press 'g' to login with GitHub instead",
                Style::default().fg(Color::White),
            )));
        }
    }

    let content = Paragraph::new(lines).alignment(Alignment::Center).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Authentication"),
    );
    frame.render_widget(content, chunks[1]);

    // Footer
    let footer_text = if app.auth_state.github_auth_in_progress {
        "Esc: Cancel | q: Quit"
    } else if app.auth_state.test_users.is_empty() {
        if app.auth_state.show_github_option {
            "g: GitHub Login | l: Load test users | q/Esc: Quit"
        } else {
            "l: Load test users | q/Esc: Quit"
        }
    } else {
        if app.auth_state.show_github_option {
            "↑/k: Up | ↓/j: Down | Enter: Login | g: GitHub | q/Esc: Quit"
        } else {
            "↑/k: Up | ↓/j: Down | Enter: Login | q/Esc: Quit"
        }
    };

    let footer = Paragraph::new(footer_text)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, chunks[2]);
}

/// Render the main screen with tabs
pub fn render_main_screen(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Adaptive layout: reduce footer sizes on small terminals
    let (header_height, footer_height) = if area.height < 30 {
        (3u16, 2u16) // Compact mode for small terminals
    } else {
        (3u16, 3u16) // Normal mode
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_height), // Tab header
            Constraint::Min(0),                // Content (flexible)
            Constraint::Length(1),             // Page-specific actions
            Constraint::Length(footer_height), // Global footer
        ])
        .split(area);

    // Render tab header
    render_tab_header(frame, app, chunks[0]);

    // Render tab content
    match app.current_tab {
        crate::app::Tab::Posts => {
            // Always render the feed
            render_posts_tab_with_data(frame, app, chunks[1]);
            // Modal is rendered later at the top level (after all tabs)
        }
        crate::app::Tab::DMs => render_dms_tab(frame, app, chunks[1]),
        crate::app::Tab::Profile => render_profile_tab(frame, app, chunks[1]),
        crate::app::Tab::Settings => render_settings_tab(frame, app, chunks[1]),
    }

    // Render page-specific actions bar (NEW)
    render_page_actions(frame, app, chunks[2]);

    // Render global footer
    render_global_footer(frame, app, chunks[3]);

    // Render modals (in priority order - LAST rendered = TOP of stack)
    
    // ============================================================================
    // MODAL RENDERING PATTERN - CRITICAL FOR CORRECT LAYERING
    // ============================================================================
    // 
    // This section implements the correct modal layering pattern that ensures:
    // 1. Background modals remain visible when foreground modals open on top
    // 2. Keyboard input is only handled by the topmost modal
    // 3. No flickering or visual glitches during modal transitions
    //
    // KEY PRINCIPLES:
    // - Modals are rendered in Z-order (bottom to top)
    // - Each modal renders independently based on its state flag
    // - NO conditional rendering based on other modal states
    // - Dimmed backgrounds are NOT used (they interfere with layering)
    //
    // RENDERING ORDER (bottom to top):
    // 1. Thread/Post detail modal (background)
    // 2. Delete confirmation modal (if active)
    // 3. Composer modal (foreground - new post, reply, edit)
    // 4. Other modals (friends, filters, help, etc.)
    //
    // WHY THIS WORKS:
    // - Thread modal renders when show_full_post_modal=true, regardless of composer state
    // - Composer modal renders AFTER thread modal, so it appears on top
    // - Each modal uses Clear widget to ensure clean rendering
    // - Ratatui's rendering order ensures later renders appear on top
    //
    // PREVIOUS BUG:
    // - Thread modal rendering was conditional on !composer_state.is_open()
    // - This caused thread modal to disappear when composer opened
    // - Dimmed background logic was interfering with modal visibility
    //
    // FIX APPLIED:
    // - Removed conditional rendering based on composer state
    // - Removed dimmed background (it was causing the thread modal to be skipped)
    // - Each modal now renders independently based only on its own state
    //
    // REFERENCE IMPLEMENTATION:
    // - Profile modal (render_user_profile_view) works correctly as a reference
    // - It renders after composer and appears on top without issues
    // ============================================================================

    // Log modal state before rendering (for debugging)
    let composer_mode = if let Some(mode) = &app.composer_state.mode {
        format!("{:?}", mode)
    } else {
        "None".to_string()
    };
    log_modal_state!(app.log_config, 
        "viewing_post_detail={}, show_full_post_modal={}, composer_open={}, composer_mode={}",
        app.viewing_post_detail,
        app.post_detail_state.as_ref().map(|s| s.show_full_post_modal).unwrap_or(false),
        app.composer_state.is_open(),
        composer_mode
    );

    // ============================================================================
    // LAYER 1: Thread/Post Detail Modal (Background)
    // ============================================================================
    // Renders the full post modal for viewing nested reply threads.
    // This modal MUST render regardless of composer state to remain visible
    // in the background when the composer opens on top.
    let show_full_post_modal = app
        .post_detail_state
        .as_ref()
        .map(|s| s.show_full_post_modal)
        .unwrap_or(false);
    
    if show_full_post_modal {
        log_rendering!(app.log_config, "Rendering thread modal (full post modal)");
        render_full_post_modal(frame, app, area);
    }

    // ============================================================================
    // LAYER 2: Delete Confirmation Modal
    // ============================================================================
    // Renders AFTER thread modal so it appears on top when active.
    let show_delete_confirmation = app
        .post_detail_state
        .as_ref()
        .map(|s| s.show_delete_confirmation)
        .unwrap_or(false);
    if show_delete_confirmation {
        render_delete_confirmation_modal(frame, app, area);
    }

    // ============================================================================
    // LAYER 3: Composer Modal (Foreground)
    // ============================================================================
    // Unified composer for new posts, replies, edits, and bio editing.
    // Renders AFTER thread modal to appear on top, allowing users to see
    // the thread context while composing a reply.
    if app.composer_state.is_open() {
        log_rendering!(app.log_config, "Rendering composer modal (mode: {})", composer_mode);
        render_unified_composer_modal(frame, app, area);
    }

    if app.dms_state.show_new_conversation_modal {
        render_new_conversation_modal(frame, app, area);
    }

    // Render save confirmation modal (before help modal)
    if app.settings_state.show_save_confirmation {
        render_save_confirmation_modal(frame, app, area);
    }

    // Render DM error modal
    if app.dms_state.show_dm_error_modal {
        render_dm_error_modal(frame, app, area);
    }

    // Render friends modal
    if app.friends_state.show_friends_modal {
        render_friends_modal(frame, app, area);
    }

    // Render user search modal
    if app.user_search_state.show_modal {
        render_user_search_modal(frame, app, area);
    }

    // Render user profile view
    if app.user_profile_view.is_some() {
        render_user_profile_view(frame, app, area);
    }

    // Render help modal (highest priority - render last)
    if app.show_help {
        render_help_modal(frame, app, area);
    }
}

/// Render tab header
pub fn render_tab_header(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);

    // Calculate total unread count for DMs
    let total_unread: usize = app.dms_state.unread_counts.values().sum();

    let tabs = ["Posts", "DMs", "Profile", "Settings"];
    let current_index = match app.current_tab {
        crate::app::Tab::Posts => 0,
        crate::app::Tab::DMs => 1,
        crate::app::Tab::Profile => 2,
        crate::app::Tab::Settings => 3,
    };

    let mut tab_spans = vec![];
    for (i, tab) in tabs.iter().enumerate() {
        let style = if i == current_index {
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(theme.text_dim)
        };

        // Add unread badge for DMs tab
        let tab_text = if i == 1 && total_unread > 0 {
            format!(" {} ({}) ", tab, total_unread)
        } else {
            format!(" {} ", tab)
        };

        tab_spans.push(Span::styled(tab_text, style));
        if i < tabs.len() - 1 {
            tab_spans.push(Span::raw(" | "));
        }
    }

    let header = Paragraph::new(Line::from(tab_spans))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, area);
}

/// Get context-appropriate action text for the current view
pub fn get_action_bar_text(app: &App) -> &'static str {
    // Don't show page actions when modal is open (modal has its own footer)
    if app.viewing_post_detail {
        if let Some(detail_state) = &app.post_detail_state {
            if detail_state.show_full_post_modal {
                return ""; // Empty - modal has its own footer
            }
        }
    }
    
    match app.current_tab {
        crate::app::Tab::Posts => {
            "u/d: Vote | n: Post | f: Filter | s: Search | Space: View | p: Profile"
        }
        crate::app::Tab::DMs => {
            // Check if user can compose (active conversation or pending draft)
            let has_active_conversation = app.dms_state.selected_conversation_index
                .filter(|&idx| idx != usize::MAX)
                .is_some();
            let has_pending_draft = app.dms_state.pending_conversation_username.is_some();
            let can_compose = has_active_conversation || has_pending_draft;
            
            if app.dms_state.selected_conversation_index == Some(usize::MAX) {
                "Enter: Start New Conversation | ↑/↓/j/k: Navigate | Esc: Back"
            } else if can_compose {
                "↑/↓/j/k: Navigate | Type to compose | Enter: Send | Esc: Clear"
            } else {
                "↑/↓/j/k: Navigate | Enter: Select conversation | n: New Conversation"
            }
        }
        crate::app::Tab::Profile => "e: Edit Bio | f: Friends",
        crate::app::Tab::Settings => "←/→/h/l: Change | s: Save",
    }
}

/// Render page-specific actions bar (centered, with wrapping support)
pub fn render_page_actions(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);

    // Clear the area first to prevent text bleeding from previous renders.
    // This is especially important when terminal is resized or content changes.
    frame.render_widget(Clear, area);
    
    // Fill with background color to ensure complete clearing
    let background = Block::default().style(Style::default().bg(theme.background));
    frame.render_widget(background, area);

    let text = get_action_bar_text(app);
    let widget = Paragraph::new(text)
        .style(Style::default().fg(theme.text).bg(theme.background))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });
    frame.render_widget(widget, area);
}

/// Render global footer with global shortcuts only
pub fn render_global_footer(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);
    
    // Clear the area first to prevent text bleeding
    frame.render_widget(Clear, area);
    
    let footer =
        Paragraph::new("Tab: Next | Shift+Tab: Previous | Shift+L: Logout | ?: Help | q/Esc: Quit | ↑/↓/j/k: Navigate")
            .style(Style::default().fg(theme.text_dim).bg(theme.background))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border)),
            );
    frame.render_widget(footer, area);
}

/// Render Posts tab with global feed
pub fn render_posts_tab_with_data(frame: &mut Frame, app: &mut App, area: Rect) {
    // Log at start of render
    log_rendering!(app.log_config, "render_posts_tab_with_data: START");
    
    let theme = get_theme_colors(app);
    
    // Check if we need to show message, error, or demo warning banners
    let has_message = app.posts_state.message.is_some();
    let has_error = app.posts_state.error.is_some();
    let has_demo_warning = app.should_show_demo_warning();

    // Layout: Demo warning (if present), Message banner (if present), Error banner (if present), posts feed
    let mut constraints = Vec::new();
    
    if has_demo_warning {
        constraints.push(Constraint::Length(3)); // Demo warning banner
    }
    if has_message {
        constraints.push(Constraint::Length(3)); // Message banner
    }
    if has_error {
        constraints.push(Constraint::Length(3)); // Error banner
    }
    constraints.push(Constraint::Min(0)); // Posts feed
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut chunk_idx = 0;

    // Demo mode warning banner (if in web mode)
    if let Some((warning, _)) = &app.demo_mode_warning {
        let warning_text = format!("🚨 {} 🚨", warning);
        let warning_banner = Paragraph::new(warning_text)
            .style(Style::default()
                .fg(Color::Yellow)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD | Modifier::SLOW_BLINK))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(Block::default()
                .borders(Borders::ALL)
                .title("⚠️  IMPORTANT: DEMO MODE ACTIVE  ⚠️")
                .title_style(Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD))
                .border_style(Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD))
                .style(Style::default().bg(Color::Red)));
        frame.render_widget(warning_banner, chunks[chunk_idx]);
        chunk_idx += 1;
    }

    // Message banner (success messages - auto-clear after 3 seconds)
    if let Some((message, _)) = &app.posts_state.message {
        let message_banner = Paragraph::new(message.clone())
            .style(Style::default().fg(theme.success).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Message").border_style(Style::default().fg(theme.border)).style(Style::default().bg(theme.background)));
        frame.render_widget(message_banner, chunks[chunk_idx]);
        chunk_idx += 1;
    }

    // Error banner (if present)
    if let Some(error) = &app.posts_state.error {
        let error_banner = Paragraph::new(error.clone())
            .style(Style::default().fg(theme.error).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Error").border_style(Style::default().fg(theme.border)).style(Style::default().bg(theme.background)));
        frame.render_widget(error_banner, chunks[chunk_idx]);
        chunk_idx += 1;
    }

    // Main posts area (no inline compose box - use 'n' to open modal)
    let posts_area = chunks[chunk_idx];

    // Only show full-page loading on initial load (when there are no posts yet)
    if app.posts_state.loading && app.posts_state.posts.is_empty() {
        let loading = Paragraph::new(create_loading_display("Loading posts...", &theme))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Global Feed"));
        frame.render_widget(loading, posts_area);
        
        // Render filter modal if open (even when loading)
        if app.posts_state.show_filter_modal {
            render_filter_modal(frame, app, area);
        }
        return;
    }

    if app.posts_state.posts.is_empty() && !app.posts_state.loading {
        let theme = get_theme_colors(app);
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "No posts yet",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press 'n' to create the first post!",
                Style::default().fg(theme.text_dim),
            )),
        ])
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Global Feed"));
        frame.render_widget(empty, posts_area);
        
        // Render filter modal if open (even when no posts)
        if app.posts_state.show_filter_modal {
            render_filter_modal(frame, app, area);
        }
        return;
    }

    // Get theme colors
    let theme = get_theme_colors(app);

    let mut items: Vec<ListItem> = Vec::new();

    let available_width = posts_area.width.saturating_sub(BORDER_PADDING) as usize;

    // Add loading spinner at top if refreshing (when posts already exist)
    if app.posts_state.loading && !app.posts_state.posts.is_empty() {
        let style = Style::default()
            .fg(theme.warning)
            .add_modifier(Modifier::BOLD);
        let loading_item = create_centered_indicator("⟳ Loading...", style, available_width);
        items.push(ListItem::new(loading_item));
    }

    // Calculate available width for post content
    let post_width = (posts_area.width as usize).saturating_sub(4);

    // Get the currently selected post index (if any)
    let selected_post_index = app.posts_state.list_state.selected()
        .and_then(|list_idx| app.posts_state.list_index_to_post_index(list_idx));

    // Add posts
    let post_items: Vec<ListItem> = app
        .posts_state
        .posts
        .iter()
        .enumerate()
        .flat_map(|(i, post)| {
            // Check if THIS post is the selected one
            let is_selected = selected_post_index == Some(i);

            let mut post_lines: Vec<Line> = Vec::new();

            // Post header with username and timestamp
            let header_style = if is_selected {
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.primary)
            };

            let prefix = if is_selected { "▶ " } else { "  " };
            let timestamp = format_timestamp(&post.created_at);

            post_lines.push(Line::from(vec![
                Span::styled(prefix, header_style),
                Span::styled(format!("@{}", post.author_username), header_style),
                Span::raw(" • "),
                Span::styled(timestamp, Style::default().fg(theme.text_dim)),
            ]));

            // Post content with hashtag highlighting and wrapping
            let content_lines =
                format_post_content_with_width(&post.content, is_selected, &theme, post_width);
            post_lines.extend(content_lines);

            // Vote counts with highlighting for user's vote
            let user_voted_up = post.user_vote.as_deref() == Some("up");
            let user_voted_down = post.user_vote.as_deref() == Some("down");

            let upvote_style = if user_voted_up {
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text_dim)
            };

            let downvote_style = if user_voted_down {
                Style::default()
                    .fg(theme.error)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text_dim)
            };

            post_lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(format!("↑ {} ", post.upvotes), upvote_style),
                Span::styled(format!("↓ {} ", post.downvotes), downvote_style),
                Span::styled(
                    format!("💬 {}", post.reply_count),
                    Style::default().fg(theme.text_dim),
                ),
            ]));

            // Separator
            if i < app.posts_state.posts.len() - 1 {
                post_lines.push(Line::from(""));
            }

            vec![ListItem::new(post_lines)]
        })
        .collect();

    items.extend(post_items);

    // Add end-of-feed message
    if !app.posts_state.posts.is_empty() {
        let end_of_feed = vec![
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                "─── End of feed ───",
                Style::default()
                    .fg(theme.text_dim)
                    .add_modifier(Modifier::DIM),
            )),
        ];
        items.push(ListItem::new(end_of_feed));
    }

    // Build title with current filter
    let title = match &app.posts_state.current_filter {
        crate::app::PostFilter::All => "Global Feed".to_string(),
        crate::app::PostFilter::Hashtag(tag) => format!("#{}", tag),
        crate::app::PostFilter::User(username) => format!("@{}", username),
        crate::app::PostFilter::Multi { hashtags, users } => {
            let total = hashtags.len() + users.len();
            format!("Filtered ({} items)", total)
        }
    };

    let posts_widget = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(Style::default().bg(theme.highlight_bg));

    frame.render_stateful_widget(posts_widget, posts_area, &mut app.posts_state.list_state);

    // Render filter modal if open
    if app.posts_state.show_filter_modal {
        render_filter_modal(frame, app, area);
    }
}

/// Create a formatted error message display with optional help text
/// 
/// # Arguments
/// * `error_message` - The error message to display
/// * `help_text` - Optional help text (e.g., "Press Esc to go back")
/// * `theme` - The theme colors to use
fn create_error_display(error_message: &str, help_text: Option<&str>, theme: &ThemeColors) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            error_message.to_string(),
            Style::default().fg(theme.error).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    
    if let Some(help) = help_text {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            help.to_string(),
            Style::default().fg(theme.text_dim),
        )));
    }
    
    lines
}

/// Create a formatted loading state display
/// 
/// # Arguments
/// * `message` - The loading message (e.g., "Loading posts...")
/// * `theme` - The theme colors to use
fn create_loading_display(message: &str, theme: &ThemeColors) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("⟳ {}", message),
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Please wait",
            Style::default().fg(theme.text_dim),
        )),
    ]
}

/// Create a centered indicator item for the feed
/// 
/// # Arguments
/// * `text` - The text to display
/// * `style` - The style to apply to the text
/// * `available_width` - The available width for centering
fn create_centered_indicator(text: &str, style: Style, available_width: usize) -> Vec<Line<'static>> {
    let padding = (available_width.saturating_sub(text.len())) / 2;
    vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("{}{}", " ".repeat(padding), text),
            style,
        )),
        Line::from(""),
    ]
}



/// Format timestamp for display
fn format_timestamp(timestamp: &chrono::DateTime<chrono::Utc>) -> String {
    // Format as date and time
    timestamp.format("%Y-%m-%d %H:%M").to_string()
}

/// Render DMs tab
pub fn render_dms_tab(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);

    if app.dms_state.loading {
        let loading = Paragraph::new(create_loading_display("Loading conversations...", &theme))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Direct Messages"),
            );
        frame.render_widget(loading, area);
        return;
    }

    if let Some(error) = &app.dms_state.error {
        let error_lines = create_error_display(
            error,
            Some("Press Esc to go back to conversations"),
            &theme,
        );
        
        let error_msg = Paragraph::new(error_lines)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Direct Messages"),
            );
        frame.render_widget(error_msg, area);
        return;
    }

    // Split into conversations list and messages (no footer - now in page actions bar)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // Conversations list
            Constraint::Percentage(70), // Messages
        ])
        .split(area);

    // Render conversations list
    render_conversations_list(frame, app, chunks[0]);

    // Render messages and input
    render_messages_view(frame, app, chunks[1]);

    // Render new conversation modal if open
    if app.dms_state.show_new_conversation_modal {
        render_new_conversation_modal(frame, app, area);
    }
}

/// Render conversations list
pub fn render_conversations_list(frame: &mut Frame, app: &App, area: Rect) {
    let theme = get_theme_colors(app);
    let mut lines = vec![];

    // Add top padding (2 lines for better spacing)
    lines.push(Line::from(""));
    lines.push(Line::from(""));

    // Add "New Conversation" button at the top
    let new_convo_selected = app.dms_state.selected_conversation_index == Some(usize::MAX);
    let new_convo_style = if new_convo_selected {
        Style::default()
            .fg(theme.success)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.primary)
    };
    let new_convo_prefix = if new_convo_selected { "▶ " } else { "  " };

    lines.push(Line::from(vec![
        Span::styled(new_convo_prefix, new_convo_style),
        Span::styled("+ New Conversation", new_convo_style),
    ]));
    lines.push(Line::from(Span::styled(
        "  Press Enter to start",
        Style::default().fg(theme.text_dim),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "─".repeat(area.width.saturating_sub(4) as usize),
        Style::default().fg(theme.text_dim),
    )));
    lines.push(Line::from(""));

    // Show pending conversation at the top of the list if it exists
    if let Some(pending_username) = &app.dms_state.pending_conversation_username {
        let is_selected = app.dms_state.selected_conversation_index.is_none();
        
        let style = if is_selected {
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text)
        };

        let prefix = if is_selected { "▶ " } else { "  " };

        // Username with draft indicator
        let mut username_spans = vec![Span::styled(prefix, style)];
        username_spans.push(Span::styled(pending_username, style));
        username_spans.push(Span::raw(" "));
        username_spans.push(Span::styled(
            "(Draft)",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::ITALIC),
        ));

        lines.push(Line::from(username_spans));

        // Draft message preview
        lines.push(Line::from(Span::styled(
            "  Type your first message...",
            Style::default().fg(theme.text_dim).add_modifier(Modifier::ITALIC),
        )));

        lines.push(Line::from(""));
    }

    for (i, convo) in app.dms_state.conversations.iter().enumerate() {
        let is_selected = app.dms_state.selected_conversation_index == Some(i);

        let style = if is_selected {
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text)
        };

        let prefix = if is_selected { "▶ " } else { "  " };

        // Username with unread indicator
        let mut username_spans = vec![Span::styled(prefix, style)];
        username_spans.push(Span::styled(&convo.other_username, style));

        if convo.unread_count > 0 {
            username_spans.push(Span::raw(" "));
            username_spans.push(Span::styled(
                format!("({})", convo.unread_count),
                Style::default()
                    .fg(theme.error)
                    .add_modifier(Modifier::BOLD),
            ));
        }

        lines.push(Line::from(username_spans));

        // Last message preview
        let preview = if convo.last_message.chars().count() > 30 {
            let truncated: String = convo.last_message.chars().take(30).collect();
            format!("  {}", truncated)
        } else {
            format!("  {}", convo.last_message)
        };

        lines.push(Line::from(Span::styled(
            preview,
            Style::default().fg(theme.text_dim),
        )));

        lines.push(Line::from(""));
    }

    let conversations = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Conversations"),
    );
    frame.render_widget(conversations, area);
}

/// Render messages view
pub fn render_messages_view(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // Messages
            Constraint::Length(6), // Input (increased from 4 to 6)
        ])
        .split(area);

    // Render messages
    render_messages(frame, app, chunks[0]);

    // Render input
    render_message_input(frame, app, chunks[1]);
}

/// Render messages
pub fn render_messages(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);

    // Check if there's a pending new conversation
    if let Some(username) = &app.dms_state.pending_conversation_username {
        let empty_text = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("New conversation with @{}", username),
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Type your first message below",
                Style::default().fg(theme.text),
            )),
            Line::from(""),
        ];
        let empty = Paragraph::new(empty_text)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Messages"));
        frame.render_widget(empty, area);
        return;
    }

    // Check if a conversation is selected (or if "New Conversation" button is selected)
    if app.dms_state.selected_conversation_index.is_none()
        || app.dms_state.selected_conversation_index == Some(usize::MAX)
    {
        let empty_text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "No conversation selected",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Use ↑/↓ to select a conversation",
                Style::default().fg(theme.text),
            )),
            Line::from(Span::styled(
                "or navigate to 'New Conversation' button",
                Style::default().fg(theme.text),
            )),
            Line::from(""),
        ];
        let empty = Paragraph::new(empty_text)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Messages"));
        frame.render_widget(empty, area);
        return;
    }

    if app.dms_state.messages.is_empty() {
        let empty = Paragraph::new("No messages yet. Start the conversation!")
            .style(Style::default().fg(theme.text_dim))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Messages"));
        frame.render_widget(empty, area);
        return;
    }

    // Optimized rendering: show most recent messages (auto-scroll to bottom)
    // For large conversation histories, only render visible messages
    let viewport_height = (area.height as usize).saturating_sub(2);
    let lines_per_message = 3; // header + content + blank
    let messages_per_screen = viewport_height / lines_per_message;

    // Always show most recent messages (scroll to bottom by default)
    let total_messages = app.dms_state.messages.len();
    let start_index = total_messages.saturating_sub(messages_per_screen);

    let current_user_id = app.auth_state.current_user.as_ref().map(|u| u.id);

    let mut lines = vec![];

    // Calculate available width for message content (account for borders and indent)
    let message_width = (area.width as usize).saturating_sub(6);

    // Render only visible messages (performance optimization for long conversations)
    for msg in app.dms_state.messages.iter().skip(start_index) {
        let is_from_me = Some(msg.from_user_id) == current_user_id;

        let timestamp = msg.created_at.format("%H:%M").to_string();
        // Use actual username from message
        let sender = &msg.from_username;

        let header_style = if is_from_me {
            Style::default().fg(theme.primary)
        } else {
            Style::default().fg(theme.success)
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!("[{}] ", timestamp),
                Style::default().fg(theme.text_dim),
            ),
            Span::styled(sender, header_style.add_modifier(Modifier::BOLD)),
        ]));

        // Message content with wrapping
        for content_line in msg.content.lines() {
            let wrapped = textwrap::wrap(content_line, message_width);
            for wrapped_line in wrapped {
                let prefix = "  ";
                lines.push(Line::from(Span::styled(
                    format!("{}{}", prefix, wrapped_line),
                    Style::default().fg(theme.text),
                )));
            }
        }

        lines.push(Line::from(""));
    }

    let messages =
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Messages"));
    frame.render_widget(messages, area);
}

/// Render message input
pub fn render_message_input(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);

    // Check if conversation is selected and user can type
    let can_type = app.dms_state.pending_conversation_username.is_some()
        || (app.dms_state.selected_conversation_index.is_some()
            && app.dms_state.selected_conversation_index != Some(usize::MAX));

    if !can_type {
        // Show placeholder when no conversation is selected
        let placeholder = if app.dms_state.selected_conversation_index.is_none() {
            "Select a conversation to send messages"
        } else {
            "Press Enter on 'New Conversation' to start"
        };

        let input = Paragraph::new(placeholder)
            .style(Style::default().fg(theme.text_dim))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Message Input"),
            );
        frame.render_widget(input, area);
        return;
    }

    // Set block on textarea before rendering
    let title = if app.dms_state.pending_conversation_username.is_some() {
        "Type your first message (Enter to send)"
    } else {
        "Message Input (Enter to send)"
    };

    // Apply theme styling to textarea - use primary color for text to ensure visibility
    app.dms_state.message_textarea.set_style(
        Style::default()
            .fg(theme.primary)  // Use primary color for better visibility
    );
    app.dms_state.message_textarea.set_cursor_style(
        Style::default()
            .fg(theme.background)
            .bg(theme.primary)  // Visible cursor
    );
    app.dms_state.message_textarea.set_cursor_line_style(
        Style::default()  // No special cursor line styling
    );
    
    // Use default border style (no theme.border) to match other boxes
    app.dms_state.message_textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .title(title),
    );

    // Render TextArea widget
    frame.render_widget(&app.dms_state.message_textarea, area);
}

/// Render Profile tab
pub fn render_profile_tab(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);
    
    if app.profile_state.loading {
        let loading = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "⟳ Loading profile...",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Please wait",
                Style::default().fg(theme.text_dim),
            )),
        ])
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Profile").border_style(Style::default().fg(theme.border)).style(Style::default().bg(theme.background)));
        frame.render_widget(loading, area);
        return;
    }

    if let Some(error) = &app.profile_state.error {
        let error_msg = Paragraph::new(error.clone())
            .style(Style::default().fg(theme.error))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Profile"));
        frame.render_widget(error_msg, area);
        return;
    }

    if let Some(profile) = &app.profile_state.profile {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(10), // Profile stats
                Constraint::Min(0),     // User posts (no footer - now in page actions bar)
            ])
            .split(area);

        // Profile stats
        render_profile_stats(frame, app, profile, chunks[0]);

        // User posts
        render_user_posts(frame, app, chunks[1]);
    } else {
        let empty = Paragraph::new("No profile data")
            .style(Style::default().fg(theme.text_dim))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Profile"));
        frame.render_widget(empty, area);
    }
}

/// Render profile stats
pub fn render_profile_stats(
    frame: &mut Frame,
    app: &App,
    profile: &fido_types::UserProfile,
    area: Rect,
) {
    let theme = get_theme_colors(app);
    let mut lines = vec![];

    lines.push(Line::from(vec![
        Span::styled("Username: ", Style::default().fg(theme.primary)),
        Span::styled(
            &profile.username,
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Bio: ", Style::default().fg(theme.primary)),
        Span::styled(
            profile.bio.as_deref().unwrap_or("No bio set"),
            Style::default().fg(theme.text),
        ),
    ]));

    lines.push(Line::from(""));

    lines.push(Line::from(vec![
        Span::styled("Karma: ", Style::default().fg(theme.success)),
        Span::styled(
            profile.karma.to_string(),
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled("Posts: ", Style::default().fg(theme.secondary)),
        Span::styled(
            profile.post_count.to_string(),
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Joined: ", Style::default().fg(theme.text_dim)),
        Span::styled(
            profile.join_date.format("%Y-%m-%d").to_string(),
            Style::default().fg(theme.text),
        ),
    ]));

    if !profile.recent_hashtags.is_empty() {
        lines.push(Line::from(""));
        let mut hashtag_spans = vec![Span::styled(
            "Recent hashtags: ",
            Style::default().fg(theme.accent),
        )];
        for (i, tag) in profile.recent_hashtags.iter().take(5).enumerate() {
            if i > 0 {
                hashtag_spans.push(Span::raw(", "));
            }
            hashtag_spans.push(Span::styled(
                format!("#{}", tag),
                Style::default().fg(theme.primary),
            ));
        }
        lines.push(Line::from(hashtag_spans));
    }

    let stats = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Profile Stats"),
    );
    frame.render_widget(stats, area);
}

/// Render user posts
pub fn render_user_posts(frame: &mut Frame, app: &mut App, area: Rect) {
    // Get theme colors
    let theme = get_theme_colors(app);
    
    if app.profile_state.user_posts.is_empty() {
        let empty = Paragraph::new("No posts yet")
            .style(Style::default().fg(theme.text_dim))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Your Posts").border_style(Style::default().fg(theme.border)).style(Style::default().bg(theme.background)));
        frame.render_widget(empty, area);
        return;
    }

    // Calculate available width for post content
    let post_width = (area.width as usize).saturating_sub(4);

    let items: Vec<ListItem> = app
        .profile_state
        .user_posts
        .iter()
        .enumerate()
        .flat_map(|(i, post)| {
            let is_selected = app.profile_state.list_state.selected() == Some(i);

            let mut post_lines: Vec<Line> = Vec::new();

            let prefix = if is_selected { "▶ " } else { "  " };
            let timestamp = post.created_at.format("%Y-%m-%d %H:%M").to_string();

            let header_style = if is_selected {
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.primary)
            };

            post_lines.push(Line::from(vec![
                Span::styled(prefix, header_style),
                Span::styled(timestamp, Style::default().fg(theme.text_dim)),
            ]));

            // Post content with wrapping
            let content_lines =
                format_post_content_with_width(&post.content, is_selected, &theme, post_width);
            post_lines.extend(content_lines);

            // Vote counts with highlighting for user's vote
            let user_voted_up = post.user_vote.as_deref() == Some("up");
            let user_voted_down = post.user_vote.as_deref() == Some("down");

            let upvote_style = if user_voted_up {
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text_dim)
            };

            let downvote_style = if user_voted_down {
                Style::default()
                    .fg(theme.error)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text_dim)
            };

            post_lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(format!("↑ {} ", post.upvotes), upvote_style),
                Span::styled(format!("↓ {} ", post.downvotes), downvote_style),
                Span::styled(
                    format!("💬 {}", post.reply_count),
                    Style::default().fg(theme.text_dim),
                ),
            ]));

            if i < app.profile_state.user_posts.len() - 1 {
                post_lines.push(Line::from(""));
            }

            vec![ListItem::new(post_lines)]
        })
        .collect();

    let posts_widget = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Your Posts"))
        .highlight_style(Style::default().bg(theme.highlight_bg));

    frame.render_stateful_widget(posts_widget, area, &mut app.profile_state.list_state);
}

/// Render Settings tab
pub fn render_settings_tab(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);

    if app.settings_state.loading {
        let loading = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "⟳ Loading settings...",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Please wait",
                Style::default().fg(theme.text_dim),
            )),
        ])
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Settings"));
        frame.render_widget(loading, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Error/Success message
            Constraint::Min(0), // Settings form (no instructions footer - now in page actions bar)
        ])
        .split(area);

    // Error/Success message
    if let Some(error) = &app.settings_state.error {
        let is_success = error.contains("successfully");
        let style = if is_success {
            Style::default().fg(theme.success)
        } else {
            Style::default().fg(theme.error)
        };

        let message = Paragraph::new(error.clone())
            .style(style)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(message, chunks[0]);
    }

    // Settings form
    if let Some(config) = &app.settings_state.config {
        let mut lines = vec![];

        lines.push(Line::from(""));

        // Color Scheme
        let color_selected =
            app.settings_state.selected_field == crate::app::SettingsField::ColorScheme;
        let color_style = if color_selected {
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text)
        };

        lines.push(Line::from(vec![
            Span::styled(if color_selected { "▶ " } else { "  " }, color_style),
            Span::styled("Color Scheme: ", Style::default().fg(theme.primary)),
            Span::styled(config.color_scheme.as_str(), color_style),
            Span::raw("  "),
            Span::styled("(←/→ to change)", Style::default().fg(theme.text_dim)),
        ]));

        lines.push(Line::from(""));

        // Sort Order
        let sort_selected =
            app.settings_state.selected_field == crate::app::SettingsField::SortOrder;
        let sort_style = if sort_selected {
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text)
        };

        lines.push(Line::from(vec![
            Span::styled(if sort_selected { "▶ " } else { "  " }, sort_style),
            Span::styled("Sort Order: ", Style::default().fg(theme.primary)),
            Span::styled(config.sort_order.as_str(), sort_style),
            Span::raw("  "),
            Span::styled("(←/→ to change)", Style::default().fg(theme.text_dim)),
        ]));

        lines.push(Line::from(""));

        // Max Posts Display
        let max_posts_selected =
            app.settings_state.selected_field == crate::app::SettingsField::MaxPosts;
        let max_posts_style = if max_posts_selected {
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text)
        };

        let max_posts_value =
            if max_posts_selected && !app.settings_state.max_posts_input.is_empty() {
                &app.settings_state.max_posts_input
            } else {
                &config.max_posts_display.to_string()
            };

        lines.push(Line::from(vec![
            Span::styled(
                if max_posts_selected { "▶ " } else { "  " },
                max_posts_style,
            ),
            Span::styled("Max Posts Display: ", Style::default().fg(theme.primary)),
            Span::styled(max_posts_value, max_posts_style),
            Span::raw("  "),
            Span::styled("(←/→ or type number)", Style::default().fg(theme.text_dim)),
        ]));

        lines.push(Line::from(""));

        // Server URL (read-only display)
        let server_description = app.server_config_manager.get_server_description(&app.current_server_url);
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("Server URL: ", Style::default().fg(theme.primary)),
            Span::styled(&app.current_server_url, Style::default().fg(theme.text)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("Server Type: ", Style::default().fg(theme.primary)),
            Span::styled(server_description, Style::default().fg(theme.text_dim)),
        ]));

        lines.push(Line::from(""));

        // Show unsaved changes indicator
        if app.settings_state.has_unsaved_changes {
            lines.push(Line::from(vec![
                Span::styled(
                    "⚠ ",
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "You have unsaved changes. Press 's' to save.",
                    Style::default().fg(theme.warning),
                ),
            ]));
        }

        let settings_widget =
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Settings"));
        frame.render_widget(settings_widget, chunks[1]);
    } else {
        let empty = Paragraph::new("No settings loaded")
            .style(Style::default().fg(theme.text_dim))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Settings"));
        frame.render_widget(empty, chunks[1]);
    }
}

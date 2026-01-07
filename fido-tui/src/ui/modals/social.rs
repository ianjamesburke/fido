use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use super::super::theme::get_theme_colors;
use super::social_components::*;
use super::utils::centered_rect;
use crate::app::App;

// Implement UserListItem for existing types
impl UserListItem for crate::api::SocialUserInfo {
    fn username(&self) -> &str {
        &self.username
    }

    fn follower_count(&self) -> Option<usize> {
        Some(self.follower_count)
    }

    fn following_count(&self) -> Option<usize> {
        Some(self.following_count)
    }
}

/// Render social connections modal (Following/Followers/Mutual Friends)
pub fn render_friends_modal(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);

    use super::social_components::*;

    let config = SocialModalConfig {
        title: " Social Connections ",
        width_percent: 70,
        height_percent: 80,
    };

    let inner = create_modal_container(frame, area, &config, &theme);

    if app.friends_state.loading {
        let loading = Paragraph::new("Loading...")
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.warning));
        frame.render_widget(loading, inner);
        return;
    }

    // Split into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab bar
            Constraint::Length(3), // Search bar
            Constraint::Min(0),    // User list
            Constraint::Length(3), // Footer (needs 3 for border + text)
        ])
        .split(inner);

    // Render tab bar
    let selected_tab_index = match app.friends_state.selected_tab {
        crate::app::SocialTab::Following => 0,
        crate::app::SocialTab::Followers => 1,
        crate::app::SocialTab::MutualFriends => 2,
    };

    // Build tab bar as a single line with spans
    let mut tab_spans = Vec::new();

    // Following tab
    if selected_tab_index == 0 {
        tab_spans.push(Span::styled(
            " [Following] ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ));
    } else {
        tab_spans.push(Span::styled(
            "  Following  ",
            Style::default().fg(theme.text_dim),
        ));
    }
    tab_spans.push(Span::raw(" | "));

    // Followers tab
    if selected_tab_index == 1 {
        tab_spans.push(Span::styled(
            " [Followers] ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ));
    } else {
        tab_spans.push(Span::styled(
            "  Followers  ",
            Style::default().fg(theme.text_dim),
        ));
    }
    tab_spans.push(Span::raw(" | "));

    // Mutual Friends tab
    if selected_tab_index == 2 {
        tab_spans.push(Span::styled(
            " [Mutual Friends] ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ));
    } else {
        tab_spans.push(Span::styled(
            "  Mutual Friends  ",
            Style::default().fg(theme.text_dim),
        ));
    }

    let tab_bar = Paragraph::new(Line::from(tab_spans))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border)),
        );
    frame.render_widget(tab_bar, chunks[0]);

    // Render search bar
    let search_text = if app.friends_state.search_mode {
        format!("/{}", app.friends_state.search_query)
    } else if !app.friends_state.search_query.is_empty() {
        format!("Filter: {}", app.friends_state.search_query)
    } else {
        "Press / to search".to_string()
    };

    let search_bar = Paragraph::new(search_text)
        .style(Style::default().fg(if app.friends_state.search_mode {
            theme.accent
        } else {
            theme.text_dim
        }))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border)),
        );
    frame.render_widget(search_bar, chunks[1]);

    // Get filtered user list
    let filtered_users = app.get_filtered_social_list();

    if filtered_users.is_empty() {
        let empty_msg = if app.friends_state.search_query.is_empty() {
            match app.friends_state.selected_tab {
                crate::app::SocialTab::Following => "Not following anyone yet",
                crate::app::SocialTab::Followers => "No followers yet",
                crate::app::SocialTab::MutualFriends => "No mutual friends yet",
            }
        } else {
            "No users match your search"
        };

        let empty = Paragraph::new(empty_msg)
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.text_dim));
        frame.render_widget(empty, chunks[2]);
    } else {
        // Build user list
        let items: Vec<ListItem> = filtered_users
            .iter()
            .map(|user| {
                let content = format!(
                    "@{}  {} followers | {} following",
                    user.username, user.follower_count, user.following_count
                );
                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(theme.highlight_bg)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        let mut list_state = ListState::default();
        list_state.select(Some(
            app.friends_state
                .selected_index
                .min(filtered_users.len().saturating_sub(1)),
        ));

        frame.render_stateful_widget(list, chunks[2], &mut list_state);
    }

    // Render footer with context-sensitive shortcuts
    let footer_text = if app.friends_state.search_mode {
        "Type to search | Esc: Exit search"
    } else {
        "↑/↓/j/k: Navigate | p: View Profile | f: Follow/Unfollow | /: Search | Tab: Switch | Esc: Close"
    };

    let footer = Paragraph::new(footer_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.text))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border)),
        );
    frame.render_widget(footer, chunks[3]);
}

/// Render user profile view modal
pub fn render_user_profile_view(frame: &mut Frame, app: &App, area: Rect) {
    let theme = get_theme_colors(app);

    let profile = match &app.user_profile_view {
        Some(p) => p,
        None => return,
    };

    // Create centered modal area (60% width, 70% height)
    let modal_area = centered_rect(60, 70, area);

    // Clear background
    frame.render_widget(Clear, modal_area);

    let block = Block::default()
        .title(" User Profile ")
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(theme.background));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    // Split inner modal into sections
    let modal_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Header with username and stats
            Constraint::Length(4), // Bio
            Constraint::Length(3), // Relationship status
            Constraint::Min(0),    // Spacer
            Constraint::Length(3), // Actions footer (needs 3 for border + text)
        ])
        .split(inner);

    // Render header with username and stats
    let header_lines = vec![
        Line::from(Span::styled(
            format!("@{}", profile.username),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("{} ", profile.follower_count),
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Followers  ", Style::default().fg(theme.text_dim)),
            Span::styled(
                format!("{} ", profile.following_count),
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Following  ", Style::default().fg(theme.text_dim)),
            Span::styled(
                format!("{} ", profile.post_count),
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Posts", Style::default().fg(theme.text_dim)),
        ]),
    ];

    let header = Paragraph::new(header_lines)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border)),
        );
    frame.render_widget(header, modal_chunks[0]);

    // Render bio
    let bio_text = profile.bio.as_deref().unwrap_or("No bio");
    let bio = Paragraph::new(bio_text)
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(theme.text))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .title(" Bio "),
        );
    frame.render_widget(bio, modal_chunks[1]);

    // Render relationship status
    let (status_text, status_color) = match &profile.relationship {
        crate::app::RelationshipStatus::Self_ => ("This is you", theme.accent),
        crate::app::RelationshipStatus::MutualFriends => ("Mutual Friends", theme.success),
        crate::app::RelationshipStatus::Following => ("Following", theme.primary),
        crate::app::RelationshipStatus::FollowsYou => ("Follows You", theme.warning),
        crate::app::RelationshipStatus::None => ("Not Following", theme.text_dim),
    };

    let status = Paragraph::new(Line::from(Span::styled(
        status_text,
        Style::default()
            .fg(status_color)
            .add_modifier(Modifier::BOLD),
    )))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border)),
    );
    frame.render_widget(status, modal_chunks[2]);

    // Render actions footer with context-sensitive shortcuts
    let actions_text = match &profile.relationship {
        crate::app::RelationshipStatus::Self_ => "Esc: Cancel",
        crate::app::RelationshipStatus::MutualFriends => {
            "f: Follow/Unfollow | m: Message | Esc: Cancel"
        }
        crate::app::RelationshipStatus::Following => "f: Follow/Unfollow | Esc: Cancel",
        crate::app::RelationshipStatus::FollowsYou => "f: Follow/Unfollow | Esc: Cancel",
        crate::app::RelationshipStatus::None => "f: Follow/Unfollow | Esc: Cancel",
    };

    let actions = Paragraph::new(actions_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.text))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border)),
        );
    frame.render_widget(actions, modal_chunks[4]);
}

/// Render new conversation modal (matches friends modal design)
pub fn render_new_conversation_modal(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);

    // Create centered modal area (70% width, 80% height) - same as friends modal
    let modal_area = centered_rect(70, 80, area);

    // Clear background
    frame.render_widget(Clear, modal_area);

    let block = Block::default()
        .title(" New Conversation ")
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(theme.background));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    // Split into sections (same layout as friends modal)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search bar
            Constraint::Min(0),    // User list
            Constraint::Length(3), // Footer (needs 3 for border + text)
        ])
        .split(inner);

    // Render search bar
    let search_text = if app.dms_state.new_conversation_search_mode {
        format!("/{}", app.dms_state.new_conversation_search_query)
    } else if !app.dms_state.new_conversation_search_query.is_empty() {
        format!("Filter: {}", app.dms_state.new_conversation_search_query)
    } else {
        "Press / to search".to_string()
    };

    let search_bar = Paragraph::new(search_text)
        .style(
            Style::default().fg(if app.dms_state.new_conversation_search_mode {
                theme.accent
            } else {
                theme.text_dim
            }),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border)),
        );
    frame.render_widget(search_bar, chunks[0]);

    // Get filtered user list
    let filtered_users = app.get_filtered_mutual_friends();

    if filtered_users.is_empty() {
        let empty_msg = if app.dms_state.new_conversation_search_query.is_empty() {
            "No mutual friends available for messaging"
        } else {
            "No users match your search"
        };

        let empty = Paragraph::new(empty_msg)
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.text_dim));
        frame.render_widget(empty, chunks[1]);
    } else {
        // Build user list (same format as friends modal)
        let items: Vec<ListItem> = filtered_users
            .iter()
            .map(|user| {
                let content = format!(
                    "@{}  {} followers | {} following",
                    user.username, user.follower_count, user.following_count
                );
                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(theme.highlight_bg)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        let mut list_state = ListState::default();
        list_state.select(Some(
            app.dms_state
                .new_conversation_selected_index
                .min(filtered_users.len().saturating_sub(1)),
        ));

        frame.render_stateful_widget(list, chunks[1], &mut list_state);
    }

    // Render footer with context-sensitive shortcuts
    let footer_text = if app.dms_state.new_conversation_search_mode {
        "Type to search | Esc: Exit search"
    } else {
        "↑/↓/j/k: Navigate | Enter: Start Conversation | /: Search | Esc: Close"
    };

    let footer = Paragraph::new(footer_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.text))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border)),
        );
    frame.render_widget(footer, chunks[2]);
}

/// Render DM error modal
pub fn render_dm_error_modal(frame: &mut Frame, app: &App, area: Rect) {
    let theme = get_theme_colors(app);

    // Create centered modal area (50% width, 30% height)
    let modal_area = centered_rect(50, 30, area);

    // Clear background
    frame.render_widget(Clear, modal_area);

    let block = Block::default()
        .title("Error")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.error))
        .style(Style::default().bg(theme.background));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // Error message
            Constraint::Length(2), // Footer
        ])
        .split(inner);

    // Error message
    let message = Paragraph::new(app.dms_state.dm_error_message.as_str())
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(theme.text));
    frame.render_widget(message, chunks[0]);

    // Footer
    let footer = Paragraph::new("Enter: Add Friend | Esc: Cancel")
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.text))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border)),
        );
    frame.render_widget(footer, chunks[1]);
}

/// Render user search modal using shared components
pub fn render_user_search_modal(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);

    // Create modal container using shared component
    let modal_area = centered_rect(70, 80, area);
    frame.render_widget(Clear, modal_area);

    let block = Block::default()
        .title(" Search Users ")
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(theme.background));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    // Handle loading state
    if app.user_search_state.loading {
        let loading = Paragraph::new("Searching...")
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.warning));
        frame.render_widget(loading, inner);
        return;
    }

    // Split into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search bar
            Constraint::Min(0),    // User list
            Constraint::Length(3), // Footer
        ])
        .split(inner);

    // Render search bar
    let search_text = if app.user_search_state.search_query.is_empty() {
        "Type to search users...".to_string()
    } else {
        format!("Search: {}", app.user_search_state.search_query)
    };

    let search_bar = Paragraph::new(search_text)
        .style(Style::default().fg(theme.accent))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border)),
        );
    frame.render_widget(search_bar, chunks[0]);

    // Render user list or empty state
    let results = &app.user_search_state.search_results;
    if results.is_empty() {
        let empty_msg = if app.user_search_state.search_query.is_empty() {
            "Start typing to search for users"
        } else if app.user_search_state.search_query.len() < 2 {
            "Type at least 2 characters to search"
        } else {
            "No users found matching your search"
        };

        let empty = Paragraph::new(empty_msg)
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.text_dim));
        frame.render_widget(empty, chunks[1]);
    } else {
        // Build user list - simplified without stats
        let items: Vec<ListItem> = results
            .iter()
            .map(|user| ListItem::new(format!("@{}", user.username)))
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(theme.highlight_bg)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        let mut list_state = ListState::default();
        list_state.select(Some(
            app.user_search_state
                .selected_index
                .min(results.len().saturating_sub(1)),
        ));

        frame.render_stateful_widget(list, chunks[1], &mut list_state);
    }

    // Render footer
    let footer =
        Paragraph::new("↑/↓/j/k: Navigate | Enter: View Profile | d: Send DM | Esc: Close")
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.text))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border)),
            );
    frame.render_widget(footer, chunks[2]);
}

// TODO: Refactor all social modals to use shared components from social_components.rs
// This will reduce code duplication by ~280 lines across:
// - render_friends_modal
// - render_new_conversation_modal
// - render_user_search_modal
// See social_refactored_example.rs for the pattern

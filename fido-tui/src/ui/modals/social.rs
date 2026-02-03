use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use super::super::components::empty_state::render_empty_state;
use super::super::components::footer::render_footer;
use super::super::components::search_bar::{render_search_bar, SearchBarConfig, SearchBarMode};
use super::super::theme::get_theme_colors;
use super::social_components::*;
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

impl UserListItem for crate::app::UserSearchResult {
    fn username(&self) -> &str {
        &self.username
    }
}

/// Render social connections modal (Following/Followers/Mutual Friends)
pub fn render_friends_modal(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);

    let config = ModalConfig::new(" Social Connections ").with_size(70, 80);
    let inner = render_modal_container(frame, area, &config, &theme);

    if app.friends_state.loading {
        render_loading_state(frame, inner, "Loading...", &theme);
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
    let tab_config = TabBarConfig {
        tabs: &["Following", "Followers", "Mutual Friends"],
        selected_index: selected_tab_index,
    };
    render_tab_bar(frame, chunks[0], &tab_config, &theme);

    // Render search bar
    let search_config = SearchBarConfig {
        query: &app.friends_state.search_query,
        is_active: app.friends_state.search_mode,
        placeholder: "Press / to search",
        mode: SearchBarMode::Slash,
    };
    render_search_bar(frame, chunks[1], &search_config, &theme);

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

        render_empty_state(frame, chunks[2], empty_msg, &theme);
    } else {
        let list_config = UserListConfig {
            selected_index: app.friends_state.selected_index,
            show_stats: true,
        };
        render_user_list(frame, chunks[2], &filtered_users, &list_config, &theme);
    }

    // Render footer with context-sensitive shortcuts
    let footer_text = if app.friends_state.search_mode {
        "Type to search | Esc: Exit search"
    } else {
        "↑/↓/j/k: Navigate | p: View Profile | f: Follow/Unfollow | /: Search | Tab: Switch | Esc: Close"
    };

    render_footer(frame, chunks[3], footer_text, &theme);
}

/// Render user profile view modal
pub fn render_user_profile_view(frame: &mut Frame, app: &App, area: Rect) {
    let theme = get_theme_colors(app);

    let profile = match &app.user_profile_view {
        Some(p) => p,
        None => return,
    };

    let config = ModalConfig::new(" User Profile ").with_size(60, 70);
    let inner = render_modal_container(frame, area, &config, &theme);

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

    render_footer(frame, modal_chunks[4], actions_text, &theme);
}

/// Render new conversation modal (matches friends modal design)
pub fn render_new_conversation_modal(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);

    let config = ModalConfig::new(" New Conversation ").with_size(70, 80);
    let inner = render_modal_container(frame, area, &config, &theme);

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
    let search_config = SearchBarConfig {
        query: &app.dms_state.new_conversation_search_query,
        is_active: app.dms_state.new_conversation_search_mode,
        placeholder: "Press / to search",
        mode: SearchBarMode::Slash,
    };
    render_search_bar(frame, chunks[0], &search_config, &theme);

    // Get filtered user list
    let filtered_users = app.get_filtered_mutual_friends();

    if filtered_users.is_empty() {
        let empty_msg = if app.dms_state.new_conversation_search_query.is_empty() {
            "No mutual friends available for messaging"
        } else {
            "No users match your search"
        };

        render_empty_state(frame, chunks[1], empty_msg, &theme);
    } else {
        let list_config = UserListConfig {
            selected_index: app.dms_state.new_conversation_selected_index,
            show_stats: true,
        };
        render_user_list(frame, chunks[1], &filtered_users, &list_config, &theme);
    }

    // Render footer with context-sensitive shortcuts
    let footer_text = if app.dms_state.new_conversation_search_mode {
        "Type to search | Esc: Exit search"
    } else {
        "↑/↓/j/k: Navigate | Enter: Start Conversation | /: Search | Esc: Close"
    };

    render_footer(frame, chunks[2], footer_text, &theme);
}

/// Render DM error modal
pub fn render_dm_error_modal(frame: &mut Frame, app: &App, area: Rect) {
    let theme = get_theme_colors(app);

    let config = ModalConfig::new("Error")
        .with_size(50, 30)
        .with_border_color(theme.error);
    let inner = render_modal_container(frame, area, &config, &theme);

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
    render_footer(frame, chunks[1], "Enter: Add Friend | Esc: Cancel", &theme);
}

/// Render user search modal using shared components
pub fn render_user_search_modal(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);

    let config = ModalConfig::new(" Search Users ").with_size(70, 80);
    let inner = render_modal_container(frame, area, &config, &theme);

    // Handle loading state
    if app.user_search_state.loading {
        render_loading_state(frame, inner, "Searching...", &theme);
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
    let search_config = SearchBarConfig {
        query: &app.user_search_state.search_query,
        is_active: true,
        placeholder: "Type to search users...",
        mode: SearchBarMode::Search,
    };
    render_search_bar(frame, chunks[0], &search_config, &theme);

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

        render_empty_state(frame, chunks[1], empty_msg, &theme);
    } else {
        let list_config = UserListConfig {
            selected_index: app.user_search_state.selected_index,
            show_stats: false,
        };
        render_user_list(frame, chunks[1], results, &list_config, &theme);
    }

    // Render footer
    render_footer(
        frame,
        chunks[2],
        "↑/↓/j/k: Navigate | Enter: View Profile | d: Send DM | Esc: Close",
        &theme,
    );
}

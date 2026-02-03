//! Shared components for social modals to reduce code duplication
//!
//! This module provides reusable UI components for rendering social-related modals
//! in the Fido TUI application. It follows the project's principles of:
//! - Speed First: Optimized rendering with minimal allocations
//! - Keyboard-Driven: Consistent navigation patterns
//! - Developer-Centric: Clean, composable API
//!
//! # Example Usage
//!
//! ```rust
//! use super::social_components::*;
//!
//! let config = ModalConfig::new(" Friends ")
//!     .with_size(70, 80);
//! let inner = render_modal_container(frame, area, &config, &theme);
//!
//! let search_config = SearchBarConfig {
//!     query: &app.search_query,
//!     is_active: app.search_mode,
//!     placeholder: "Press / to search",
//!     mode: SearchBarMode::Slash,
//! };
//! render_search_bar(frame, search_area, &search_config, &theme);
//! ```
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, ListItem, ListState, Paragraph},
    Frame,
};

use super::super::components::list::styled_list;
pub use crate::ui::components::modal::{render_modal_container, ModalConfig};
pub use crate::ui::components::search_bar::{render_search_bar, SearchBarConfig, SearchBarMode};
pub use crate::ui::components::tab_bar::{render_tab_bar, TabBarConfig};
use super::super::theme::ThemeColors;

/// Render a loading state
pub fn render_loading_state(frame: &mut Frame, area: Rect, message: &str, theme: &ThemeColors) {
    let loading = Paragraph::new(message)
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.warning));
    frame.render_widget(loading, area);
}

/// Configuration for user list rendering
pub struct UserListConfig {
    pub selected_index: usize,
    pub show_stats: bool,
}

/// User info for rendering in lists
pub trait UserListItem {
    fn username(&self) -> &str;
    fn follower_count(&self) -> Option<usize> {
        None
    }
    fn following_count(&self) -> Option<usize> {
        None
    }

    /// Optional display name override
    fn display_name(&self) -> String {
        format!("@{}", self.username())
    }
}

/// Result type for modal operations
pub type ModalResult<T> = Result<T, ModalError>;

/// Errors that can occur in modal operations
#[derive(Debug)]
pub enum ModalError {
    InvalidIndex(usize),
    EmptyList,
    RenderError(String),
}

impl std::fmt::Display for ModalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModalError::InvalidIndex(idx) => write!(f, "Invalid index: {}", idx),
            ModalError::EmptyList => write!(f, "Cannot operate on empty list"),
            ModalError::RenderError(msg) => write!(f, "Render error: {}", msg),
        }
    }
}

impl std::error::Error for ModalError {}

/// Render a user list with consistent styling
pub fn render_user_list<T: UserListItem>(
    frame: &mut Frame,
    area: Rect,
    users: &[T],
    config: &UserListConfig,
    theme: &ThemeColors,
) {
    // Early return for empty lists to avoid unnecessary allocations
    if users.is_empty() {
        return;
    }

    let items: Vec<ListItem> = users
        .iter()
        .map(|user| {
            let content = if config.show_stats {
                if let (Some(followers), Some(following)) =
                    (user.follower_count(), user.following_count())
                {
                    format!(
                        "@{}  {} followers | {} following",
                        user.username(),
                        followers,
                        following
                    )
                } else {
                    format!("@{}", user.username())
                }
            } else {
                format!("@{}", user.username())
            };
            ListItem::new(content)
        })
        .collect();

    let list = styled_list(items, theme, Some(">> "))
        .highlight_style(
            Style::default()
                .bg(theme.highlight_bg)
                .add_modifier(Modifier::BOLD),
        );

    let mut list_state = ListState::default();
    // Safer bounds checking - only select if we have items
    if !users.is_empty() {
        list_state.select(Some(config.selected_index.min(users.len() - 1)));
    }

    frame.render_stateful_widget(list, area, &mut list_state);
}

/// Render a footer with shortcuts
pub fn render_modal_footer(frame: &mut Frame, area: Rect, shortcuts: &str, theme: &ThemeColors) {
    let footer = Paragraph::new(shortcuts)
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.text))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border)),
        );
    frame.render_widget(footer, area);
}

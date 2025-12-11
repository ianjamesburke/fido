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
//! This module provides reusable components for social modals including
//! modal containers, search bars, user lists, and tab navigation.
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::super::theme::ThemeColors;
use super::utils::centered_rect;

/// Configuration for rendering a social modal
#[derive(Clone)]
pub struct SocialModalConfig<'a> {
    pub title: &'a str,
    pub width_percent: u16,
    pub height_percent: u16,
}

impl<'a> Default for SocialModalConfig<'a> {
    fn default() -> Self {
        Self {
            title: " Modal ",
            width_percent: 70,
            height_percent: 80,
        }
    }
}

impl<'a> SocialModalConfig<'a> {
    /// Builder pattern for cleaner configuration
    pub fn new(title: &'a str) -> Self {
        Self {
            title,
            ..Default::default()
        }
    }
    
    pub fn with_size(mut self, width_percent: u16, height_percent: u16) -> Self {
        self.width_percent = width_percent;
        self.height_percent = height_percent;
        self
    }
}

/// Create and render the outer modal container
pub fn create_modal_container(
    frame: &mut Frame,
    area: Rect,
    config: &SocialModalConfig,
    theme: &ThemeColors,
) -> Rect {
    let modal_area = centered_rect(config.width_percent, config.height_percent, area);
    
    frame.render_widget(ratatui::widgets::Clear, modal_area);
    
    let block = Block::default()
        .title(config.title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(theme.background));
    
    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);
    
    inner
}

/// Render a loading state
pub fn render_loading_state(frame: &mut Frame, area: Rect, message: &str, theme: &ThemeColors) {
    let loading = Paragraph::new(message)
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.warning));
    frame.render_widget(loading, area);
}

/// Configuration for search bar rendering
pub struct SearchBarConfig<'a> {
    pub query: &'a str,
    pub is_active: bool,
    pub placeholder: &'a str,
}

/// Render a search bar
pub fn render_search_bar(
    frame: &mut Frame,
    area: Rect,
    config: &SearchBarConfig,
    theme: &ThemeColors,
) {
    let search_text = if config.is_active {
        format!("/{}", config.query)
    } else if !config.query.is_empty() {
        format!("Filter: {}", config.query)
    } else {
        config.placeholder.to_string()
    };

    let search_bar = Paragraph::new(search_text)
        .style(Style::default().fg(if config.is_active { theme.accent } else { theme.text_dim }))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(theme.border)));
    
    frame.render_widget(search_bar, area);
}

/// Render an empty state message
pub fn render_empty_state(
    frame: &mut Frame,
    area: Rect,
    message: &str,
    theme: &ThemeColors,
) {
    let empty = Paragraph::new(message)
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.text_dim));
    frame.render_widget(empty, area);
}

/// Configuration for user list rendering
pub struct UserListConfig {
    pub selected_index: usize,
    pub show_stats: bool,
}

/// Configuration for tab bar rendering
pub struct TabBarConfig<'a> {
    pub tabs: &'a [&'a str],
    pub selected_index: usize,
}

/// User info for rendering in lists
pub trait UserListItem {
    fn username(&self) -> &str;
    fn follower_count(&self) -> Option<usize> { None }
    fn following_count(&self) -> Option<usize> { None }
    
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
                if let (Some(followers), Some(following)) = (user.follower_count(), user.following_count()) {
                    format!("@{}  {} followers | {} following", user.username(), followers, following)
                } else {
                    format!("@{}", user.username())
                }
            } else {
                format!("@{}", user.username())
            };
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
    // Safer bounds checking - only select if we have items
    if !users.is_empty() {
        list_state.select(Some(config.selected_index.min(users.len() - 1)));
    }

    frame.render_stateful_widget(list, area, &mut list_state);
}

/// Render a tab bar with consistent styling
pub fn render_tab_bar(
    frame: &mut Frame,
    area: Rect,
    config: &TabBarConfig,
    theme: &ThemeColors,
) {
    use ratatui::text::{Line, Span};
    
    let mut tab_spans = Vec::new();
    
    for (i, &tab_name) in config.tabs.iter().enumerate() {
        if i > 0 {
            tab_spans.push(Span::raw(" | "));
        }
        
        if i == config.selected_index {
            tab_spans.push(Span::styled(
                format!(" [{}] ", tab_name),
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
            ));
        } else {
            tab_spans.push(Span::styled(
                format!("  {}  ", tab_name),
                Style::default().fg(theme.text_dim)
            ));
        }
    }
    
    let tab_bar = Paragraph::new(Line::from(tab_spans))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(theme.border)));
    frame.render_widget(tab_bar, area);
}

/// Render a footer with shortcuts
pub fn render_modal_footer(
    frame: &mut Frame,
    area: Rect,
    shortcuts: &str,
    theme: &ThemeColors,
) {
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

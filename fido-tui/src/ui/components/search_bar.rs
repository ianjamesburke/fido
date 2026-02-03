use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::theme::ThemeColors;

/// Display modes for the search bar.
pub enum SearchBarMode {
    /// Slash-driven search: "/query" when active, "Filter: query" when inactive.
    Slash,
    /// Explicit search label: "Search: query" when query is non-empty.
    Search,
}

/// Configuration for search bar rendering.
pub struct SearchBarConfig<'a> {
    pub query: &'a str,
    pub is_active: bool,
    pub placeholder: &'a str,
    pub mode: SearchBarMode,
}

/// Render a search bar.
pub fn render_search_bar(
    frame: &mut Frame,
    area: Rect,
    config: &SearchBarConfig,
    theme: &ThemeColors,
) {
    let search_text = if config.query.is_empty() {
        config.placeholder.to_string()
    } else if config.is_active {
        match config.mode {
            SearchBarMode::Slash => format!("/{}", config.query),
            SearchBarMode::Search => format!("Search: {}", config.query),
        }
    } else {
        match config.mode {
            SearchBarMode::Slash => format!("Filter: {}", config.query),
            SearchBarMode::Search => format!("Search: {}", config.query),
        }
    };

    let search_bar = Paragraph::new(search_text)
        .style(Style::default().fg(if config.is_active {
            theme.accent
        } else {
            theme.text_dim
        }))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border)),
        );

    frame.render_widget(search_bar, area);
}

use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::theme::ThemeColors;

/// Configuration for tab bar rendering.
pub struct TabBarConfig<'a> {
    pub tabs: &'a [&'a str],
    pub selected_index: usize,
}

/// Render a tab bar with consistent styling.
pub fn render_tab_bar(frame: &mut Frame, area: Rect, config: &TabBarConfig, theme: &ThemeColors) {
    let mut tab_spans = Vec::new();

    for (i, &tab_name) in config.tabs.iter().enumerate() {
        if i > 0 {
            tab_spans.push(Span::raw(" | "));
        }

        if i == config.selected_index {
            tab_spans.push(Span::styled(
                format!(" [{}] ", tab_name),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            tab_spans.push(Span::styled(
                format!("  {}  ", tab_name),
                Style::default().fg(theme.text_dim),
            ));
        }
    }

    let tab_bar = Paragraph::new(Line::from(tab_spans))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border)),
        );
    frame.render_widget(tab_bar, area);
}

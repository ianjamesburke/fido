use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    text::Text,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::theme::ThemeColors;

/// Render a centered empty state message.
pub fn render_empty_state(frame: &mut Frame, area: Rect, text: impl Into<Text>, theme: &ThemeColors) {
    let empty = Paragraph::new(text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.text_dim));
    frame.render_widget(empty, area);
}

/// Render a centered empty state with a standard bordered block.
pub fn render_empty_state_block(
    frame: &mut Frame,
    area: Rect,
    text: impl Into<Text>,
    theme: &ThemeColors,
    title: &str,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.background));

    let empty = Paragraph::new(text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.text_dim))
        .block(block);
    frame.render_widget(empty, area);
}

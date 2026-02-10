use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::theme::ThemeColors;

/// Render a centered panel with pre-styled lines.
pub fn render_panel_lines(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    lines: Vec<Line<'static>>,
    theme: &ThemeColors,
) {
    let panel = Paragraph::new(lines).alignment(Alignment::Center).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.background)),
    );
    frame.render_widget(panel, area);
}

use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::theme::ThemeColors;

/// Render a standard footer with centered hint text.
pub fn render_footer(frame: &mut Frame, area: Rect, text: &str, theme: &ThemeColors) {
    render_footer_with_style(
        frame,
        area,
        text,
        theme,
        Style::default().fg(theme.text),
    );
}

/// Render a footer with a custom text style while keeping standard borders/background.
pub fn render_footer_with_style(
    frame: &mut Frame,
    area: Rect,
    text: &str,
    theme: &ThemeColors,
    text_style: Style,
) {
    let footer = Paragraph::new(text)
        .alignment(Alignment::Center)
        .style(text_style.bg(theme.background))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.background)),
        );
    frame.render_widget(footer, area);
}

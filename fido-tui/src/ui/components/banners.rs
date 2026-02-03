use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::theme::ThemeColors;

pub fn render_message_banner(frame: &mut Frame, area: Rect, message: &str, theme: &ThemeColors) {
    let banner = Paragraph::new(message)
        .style(
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Message")
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.background)),
        );
    frame.render_widget(banner, area);
}

pub fn render_error_banner(frame: &mut Frame, area: Rect, message: &str, theme: &ThemeColors) {
    let banner = Paragraph::new(message)
        .style(
            Style::default()
                .fg(theme.error)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Error")
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.background)),
        );
    frame.render_widget(banner, area);
}

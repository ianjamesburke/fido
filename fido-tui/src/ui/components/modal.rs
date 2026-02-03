use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear},
    Frame,
};

use crate::ui::theme::ThemeColors;

/// Configuration for rendering modal containers.
pub struct ModalConfig<'a> {
    pub title: &'a str,
    pub width_percent: u16,
    pub height_percent: u16,
    pub border_color: Option<Color>,
}

impl<'a> ModalConfig<'a> {
    pub fn new(title: &'a str) -> Self {
        Self {
            title,
            width_percent: 70,
            height_percent: 80,
            border_color: None,
        }
    }

    pub fn with_size(mut self, width_percent: u16, height_percent: u16) -> Self {
        self.width_percent = width_percent;
        self.height_percent = height_percent;
        self
    }

    pub fn with_border_color(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }
}

/// Create a centered rectangle for modal positioning
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Clear the modal background so it renders cleanly over the UI.
pub fn clear_modal(frame: &mut Frame, area: Rect) {
    frame.render_widget(Clear, area);
}

/// Render a standard modal container and return the inner content area.
pub fn render_modal_container(
    frame: &mut Frame,
    area: Rect,
    config: &ModalConfig,
    theme: &ThemeColors,
) -> Rect {
    let modal_area = centered_rect(config.width_percent, config.height_percent, area);
    clear_modal(frame, modal_area);

    let border_color = config.border_color.unwrap_or(theme.accent);
    let block = Block::default()
        .title(config.title)
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(border_color)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(theme.background));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);
    inner
}

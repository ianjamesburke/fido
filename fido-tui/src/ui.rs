// UI module - split into cohesive submodules for maintainability
pub mod theme;
mod formatting;
mod tabs;
mod modals;

// Re-export main render function
pub use self::render_main::render;

// Main render logic
mod render_main {
    use ratatui::{
        layout::Alignment,
        style::{Modifier, Style},
        text::{Line, Span},
        widgets::{Block, Borders, Clear, Paragraph},
        Frame,
    };

    use crate::app::{App, Screen};
    use super::theme::get_theme_colors;
    use super::tabs::{render_auth_screen, render_main_screen};

    /// Render the UI
    pub fn render(app: &mut App, frame: &mut Frame) {
        let area = frame.area();
        
        let theme = get_theme_colors(app);
        
        frame.render_widget(Clear, area);
        
        let background = Block::default().style(Style::default().bg(theme.background));
        frame.render_widget(background, area);

        const MIN_WIDTH: u16 = 60;
        const MIN_HEIGHT: u16 = 20;

        if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
            let warning = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "Terminal Too Small",
                    Style::default()
                        .fg(theme.error)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!("Minimum size: {}x{}", MIN_WIDTH, MIN_HEIGHT),
                    Style::default().fg(theme.text),
                )),
                Line::from(Span::styled(
                    format!("Current size: {}x{}", area.width, area.height),
                    Style::default().fg(theme.warning),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Please resize your terminal window",
                    Style::default().fg(theme.text_dim),
                )),
            ])
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.error)),
            );

            frame.render_widget(warning, area);
            return;
        }

        match app.current_screen {
            Screen::Auth => render_auth_screen(frame, app),
            Screen::Main => render_main_screen(frame, app),
        }
    }
}

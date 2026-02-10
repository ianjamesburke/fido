use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;

pub struct AuthLayout {
    pub header: Rect,
    pub content: Rect,
    pub footer: Rect,
}

pub struct MainLayout {
    pub header: Rect,
    pub content: Rect,
    pub actions: Rect,
    pub footer: Rect,
}

pub struct BannerLayout {
    pub message: Option<Rect>,
    pub error: Option<Rect>,
    pub content: Rect,
}

pub fn auth_layout(frame: &mut Frame, app: &App) -> AuthLayout {
    let area = frame.area();

    let chunks = if app.is_demo_mode {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Demo banner
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Footer
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Footer
            ])
            .split(area)
    };

    if app.is_demo_mode {
        let demo_banner =
            Paragraph::new("🎮 DEMO MODE - Data is temporary and will be lost on refresh")
                .style(
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
                .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(demo_banner, chunks[0]);

        AuthLayout {
            header: chunks[1],
            content: chunks[2],
            footer: chunks[3],
        }
    } else {
        AuthLayout {
            header: chunks[0],
            content: chunks[1],
            footer: chunks[2],
        }
    }
}

pub fn main_layout(frame: &mut Frame, app: &App) -> MainLayout {
    let area = frame.area();

    let (header_height, footer_height) = if area.height < 30 {
        (3u16, 2u16)
    } else {
        (3u16, 3u16)
    };

    let show_update_banner = app.update_available.is_some();

    let chunks = if show_update_banner {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),             // Update banner
                Constraint::Length(header_height), // Tab header
                Constraint::Min(0),                // Content
                Constraint::Length(1),             // Page-specific actions
                Constraint::Length(footer_height), // Global footer
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(header_height), // Tab header
                Constraint::Min(0),                // Content
                Constraint::Length(1),             // Page-specific actions
                Constraint::Length(footer_height), // Global footer
            ])
            .split(area)
    };

    if show_update_banner {
        if let Some(version) = &app.update_available {
            let banner = Paragraph::new(format!(
                "🆕 Update available: v{} → Run: fido --update",
                version
            ))
            .style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(ratatui::layout::Alignment::Center);
            frame.render_widget(banner, chunks[0]);
        }

        MainLayout {
            header: chunks[1],
            content: chunks[2],
            actions: chunks[3],
            footer: chunks[4],
        }
    } else {
        MainLayout {
            header: chunks[0],
            content: chunks[1],
            actions: chunks[2],
            footer: chunks[3],
        }
    }
}

pub fn banner_layout(area: Rect, has_message: bool, has_error: bool) -> BannerLayout {
    let chunks = match (has_message, has_error) {
        (true, true) => {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Message banner
                    Constraint::Length(3), // Error banner
                    Constraint::Min(0),    // Content
                ])
                .split(area)
        }
        (true, false) => {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Message banner
                    Constraint::Min(0),    // Content
                ])
                .split(area)
        }
        (false, true) => {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Error banner
                    Constraint::Min(0),    // Content
                ])
                .split(area)
        }
        (false, false) => Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0)])
            .split(area),
    };

    match (has_message, has_error) {
        (true, true) => BannerLayout {
            message: Some(chunks[0]),
            error: Some(chunks[1]),
            content: chunks[2],
        },
        (true, false) => BannerLayout {
            message: Some(chunks[0]),
            error: None,
            content: chunks[1],
        },
        (false, true) => BannerLayout {
            message: None,
            error: Some(chunks[0]),
            content: chunks[1],
        },
        (false, false) => BannerLayout {
            message: None,
            error: None,
            content: chunks[0],
        },
    }
}

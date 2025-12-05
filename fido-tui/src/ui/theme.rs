use ratatui::style::Color;
use crate::app::App;

pub struct ThemeColors {
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
    pub text: Color,
    pub text_dim: Color,
    pub background: Color,
    pub border: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub highlight_bg: Color,
}

/// Get theme colors based on user's color scheme setting
/// Each theme is a complete color overhaul with proper contrast
pub fn get_theme_colors(app: &App) -> ThemeColors {
    let scheme = app
        .settings_state
        .config
        .as_ref()
        .map(|c| &c.color_scheme)
        .unwrap_or(&fido_types::ColorScheme::Default);

    match scheme {
        // Terminal Green - Classic hacker aesthetic
        fido_types::ColorScheme::Default => ThemeColors {
            primary: Color::Rgb(0, 255, 0),      // Bright green
            secondary: Color::Rgb(0, 200, 0),    // Medium green
            accent: Color::Rgb(0, 255, 100),     // Cyan-green
            text: Color::Rgb(0, 255, 0),         // Bright green text
            text_dim: Color::Rgb(0, 150, 0),     // Dim green
            background: Color::Black,
            border: Color::Rgb(0, 200, 0),
            success: Color::Rgb(0, 255, 0),
            warning: Color::Rgb(255, 255, 0),
            error: Color::Rgb(255, 0, 0),
            highlight_bg: Color::Rgb(0, 50, 0),  // Very dark green
        },
        
        // Dark Mode - Modern dark theme with blue accents
        fido_types::ColorScheme::Dark => ThemeColors {
            primary: Color::Rgb(100, 200, 255),  // Light blue
            secondary: Color::Rgb(150, 150, 255), // Purple-blue
            accent: Color::Rgb(255, 100, 200),   // Pink
            text: Color::Rgb(220, 220, 220),     // Light gray
            text_dim: Color::Rgb(120, 120, 120), // Medium gray
            background: Color::Rgb(20, 20, 25),  // Very dark blue-gray
            border: Color::Rgb(60, 60, 70),      // Dark gray-blue
            success: Color::Rgb(100, 255, 150),  // Bright green
            warning: Color::Rgb(255, 200, 100),  // Orange
            error: Color::Rgb(255, 100, 100),    // Bright red
            highlight_bg: Color::Rgb(40, 40, 50), // Slightly lighter than bg
        },
        
        // Light Mode - True light theme with dark text
        fido_types::ColorScheme::Light => ThemeColors {
            primary: Color::Rgb(0, 100, 200),    // Dark blue
            secondary: Color::Rgb(100, 50, 200), // Purple
            accent: Color::Rgb(200, 0, 100),     // Magenta
            text: Color::Rgb(30, 30, 30),        // Almost black
            text_dim: Color::Rgb(100, 100, 100), // Medium gray
            background: Color::Rgb(250, 250, 250), // Off-white
            border: Color::Rgb(180, 180, 180),   // Light gray
            success: Color::Rgb(0, 150, 50),     // Dark green
            warning: Color::Rgb(200, 150, 0),    // Dark yellow
            error: Color::Rgb(200, 0, 0),        // Dark red
            highlight_bg: Color::Rgb(230, 240, 255), // Light blue tint
        },
        
        // Solarized Dark - Authentic Solarized colors
        fido_types::ColorScheme::Solarized => ThemeColors {
            primary: Color::Rgb(38, 139, 210),   // Solarized blue
            secondary: Color::Rgb(42, 161, 152), // Solarized cyan
            accent: Color::Rgb(211, 54, 130),    // Solarized magenta
            text: Color::Rgb(147, 161, 161),     // Solarized base1
            text_dim: Color::Rgb(101, 123, 131), // Solarized base00
            background: Color::Rgb(0, 43, 54),   // Solarized base03
            border: Color::Rgb(7, 54, 66),       // Solarized base02
            success: Color::Rgb(133, 153, 0),    // Solarized green
            warning: Color::Rgb(181, 137, 0),    // Solarized yellow
            error: Color::Rgb(220, 50, 47),      // Solarized red
            highlight_bg: Color::Rgb(7, 54, 66), // Solarized base02
        },
    }
}

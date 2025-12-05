/// Text wrapping utilities for terminal UI
use tui_textarea::TextArea;

/// Configuration for text wrapping behavior
pub struct WrapConfig {
    /// Maximum width before wrapping (in characters)
    pub wrap_width: usize,
}

impl WrapConfig {
    /// Standard width for composer modals (70% of typical terminal)
    pub const COMPOSER: Self = Self { wrap_width: 100 };
    
    /// Standard width for DM panels (70% of typical terminal)
    pub const DM_PANEL: Self = Self { wrap_width: 110 };
}

/// Wrap text in a TextArea if the current line exceeds the configured width
/// 
/// This function:
/// - Finds the current cursor position
/// - Checks if the current line exceeds wrap_width
/// - Splits at the last space before wrap_width
/// - Rebuilds the textarea with wrapped content
/// - Repositions the cursor appropriately
pub fn wrap_textarea_if_needed(textarea: &mut TextArea<'static>, config: WrapConfig) {
    let (row, col) = textarea.cursor();
    let lines: Vec<String> = textarea.lines().to_vec();
    
    if row >= lines.len() {
        return;
    }
    
    let current_line = &lines[row];
    let char_count = current_line.chars().count();
    
    if char_count <= config.wrap_width {
        return;
    }
    
    // Collect characters for safe indexing
    let chars: Vec<char> = current_line.chars().collect();
    
    // Find the last space before wrap_width
    let mut wrap_point = config.wrap_width;
    for i in (0..config.wrap_width.min(chars.len())).rev() {
        if chars[i] == ' ' {
            wrap_point = i;
            break;
        }
    }
    
    // Split at character boundary
    let first_part: String = chars[..wrap_point].iter().collect();
    let second_part: String = chars[wrap_point..].iter().collect();
    let first_part = first_part.trim_end();
    let second_part = second_part.trim_start();
    
    // Rebuild all lines with the wrapped text
    let mut new_lines = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if i == row {
            new_lines.push(first_part.to_string());
            new_lines.push(second_part.to_string());
        } else {
            new_lines.push(line.clone());
        }
    }
    
    // Replace textarea content
    *textarea = TextArea::from(new_lines.iter().map(|s| s.as_str()));
    textarea.set_hard_tab_indent(true);
    
    // Move cursor to the second line at the appropriate position
    let new_col = if col > wrap_point {
        col - wrap_point - 1
    } else {
        second_part.chars().count()
    };
    textarea.move_cursor(tui_textarea::CursorMove::Jump(row as u16 + 1, new_col as u16));
}

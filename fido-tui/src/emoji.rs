/// Emoji utility functions for parsing and rendering emojis in posts
/// Parse emoji shortcodes (e.g., :smile:) and replace them with actual emojis
pub fn parse_emoji_shortcodes(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == ':' {
            // Try to find the closing colon
            let mut shortcode = String::new();
            let mut found_closing = false;

            // Collect characters until we find another colon or reach max length
            while let Some(&next_ch) = chars.peek() {
                if next_ch == ':' {
                    chars.next(); // consume the closing colon
                    found_closing = true;
                    break;
                } else if next_ch.is_whitespace() || shortcode.len() > 30 {
                    // Not a valid shortcode
                    break;
                } else {
                    shortcode.push(next_ch);
                    chars.next();
                }
            }

            if found_closing && !shortcode.is_empty() {
                // Try to find the emoji by shortcode
                if let Some(emoji) = emojis::get_by_shortcode(&shortcode) {
                    result.push_str(emoji.as_str());
                } else {
                    // Not a valid emoji shortcode, keep the original text
                    result.push(':');
                    result.push_str(&shortcode);
                    result.push(':');
                }
            } else {
                // Not a complete shortcode, keep the original colon and text
                result.push(':');
                result.push_str(&shortcode);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Count the actual character length including emojis
/// Emojis count as 1 character for the 280 limit
pub fn count_characters(text: &str) -> usize {
    text.chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_emoji_shortcodes() {
        assert_eq!(parse_emoji_shortcodes("Hello :smile:"), "Hello 😄");
        assert_eq!(parse_emoji_shortcodes(":heart: Rust"), "❤️ Rust");
        assert_eq!(parse_emoji_shortcodes("No emoji here"), "No emoji here");
        assert_eq!(parse_emoji_shortcodes(":invalid_code:"), ":invalid_code:");
    }

    #[test]
    fn test_count_characters() {
        assert_eq!(count_characters("Hello"), 5);
        assert_eq!(count_characters("Hello 😀"), 7);
        assert_eq!(count_characters("❤️"), 2); // Heart with variation selector
    }
}

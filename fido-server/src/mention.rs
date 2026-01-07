/// Mention extraction utilities for Fido
/// Extracts @username mentions from post content
use regex::Regex;
use std::sync::OnceLock;

/// Get the compiled regex for mention extraction
#[allow(dead_code)]
fn mention_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        // Match @username where username is alphanumeric and underscores
        // Must be preceded by whitespace or start of string
        // Must not be followed by alphanumeric characters
        Regex::new(r"(?:^|[^@\w])@([a-zA-Z0-9_]+)").unwrap()
    })
}

/// Extract all @username mentions from content
/// Returns a vector of unique usernames (without the @ symbol)
#[allow(dead_code)]
pub fn extract_mentions(content: &str) -> Vec<String> {
    let re = mention_regex();
    let mut mentions = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for cap in re.captures_iter(content) {
        if let Some(username) = cap.get(1) {
            let username_str = username.as_str().to_lowercase();
            if seen.insert(username_str.clone()) {
                mentions.push(username_str);
            }
        }
    }

    mentions
}

/// Extract the first @username mention from content
/// Returns None if no mentions found
#[allow(dead_code)]
pub fn extract_first_mention(content: &str) -> Option<String> {
    let re = mention_regex();
    re.captures(content)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_mentions() {
        assert_eq!(
            extract_mentions("Hey @alice, what do you think?"),
            vec!["alice"]
        );

        assert_eq!(
            extract_mentions("@bob and @charlie are both right"),
            vec!["bob", "charlie"]
        );

        assert_eq!(extract_mentions("No mentions here"), Vec::<String>::new());

        // Duplicate mentions should only appear once
        assert_eq!(extract_mentions("@alice @bob @alice"), vec!["alice", "bob"]);

        // Should not match email addresses
        assert_eq!(
            extract_mentions("Email me at test@example.com"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn test_extract_first_mention() {
        assert_eq!(
            extract_first_mention("Hey @alice, what do you think?"),
            Some("alice".to_string())
        );

        assert_eq!(
            extract_first_mention("@bob and @charlie are both right"),
            Some("bob".to_string())
        );

        assert_eq!(extract_first_mention("No mentions here"), None);
    }
}

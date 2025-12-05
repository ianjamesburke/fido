use once_cell::sync::Lazy;
use regex::Regex;

/// Regex pattern for matching hashtags
/// Matches: #word where word contains letters, numbers, underscores (minimum 2 chars)
static HASHTAG_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"#(\w{2,})").expect("Failed to compile hashtag regex")
});

/// Extract hashtags from post content
/// 
/// Returns a vector of unique hashtag names (without the # prefix)
/// Hashtags are normalized to lowercase and duplicates are removed
/// 
/// # Examples
/// 
/// ```
/// use fido_server::hashtag::extract_hashtags;
/// let content = "Check out #rust and #Rust! Also #web_dev";
/// let hashtags = extract_hashtags(content);
/// assert_eq!(hashtags.len(), 2);
/// assert!(hashtags.contains(&"rust".to_string()));
/// assert!(hashtags.contains(&"web_dev".to_string()));
/// ```
pub fn extract_hashtags(content: &str) -> Vec<String> {
    use std::collections::HashSet;
    
    HASHTAG_REGEX
        .captures_iter(content)
        .map(|cap| cap[1].to_lowercase())  // Normalize to lowercase
        .collect::<HashSet<_>>()  // Remove duplicates
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_single_hashtag() {
        let content = "This is a post with #rust";
        let hashtags = extract_hashtags(content);
        assert_eq!(hashtags.len(), 1);
        assert!(hashtags.contains(&"rust".to_string()));
    }

    #[test]
    fn test_extract_multiple_hashtags() {
        let content = "Learning #rust and #async programming";
        let hashtags = extract_hashtags(content);
        assert_eq!(hashtags.len(), 2);
        assert!(hashtags.contains(&"rust".to_string()));
        assert!(hashtags.contains(&"async".to_string()));
    }

    #[test]
    fn test_extract_duplicate_hashtags() {
        let content = "#rust is great! I love #rust and #Rust";
        let hashtags = extract_hashtags(content);
        assert_eq!(hashtags.len(), 1);
        assert!(hashtags.contains(&"rust".to_string()));
    }

    #[test]
    fn test_extract_hashtags_with_underscores() {
        let content = "Check out #web_dev and #rust_lang";
        let hashtags = extract_hashtags(content);
        assert_eq!(hashtags.len(), 2);
        assert!(hashtags.contains(&"web_dev".to_string()));
        assert!(hashtags.contains(&"rust_lang".to_string()));
    }

    #[test]
    fn test_extract_hashtags_with_numbers() {
        let content = "Excited for #rust2024 and #web3";
        let hashtags = extract_hashtags(content);
        assert_eq!(hashtags.len(), 2);
        assert!(hashtags.contains(&"rust2024".to_string()));
        assert!(hashtags.contains(&"web3".to_string()));
    }

    #[test]
    fn test_minimum_length_requirement() {
        let content = "Short tags: #a #ab #abc";
        let hashtags = extract_hashtags(content);
        // Only #ab and #abc should be extracted (minimum 2 chars)
        assert_eq!(hashtags.len(), 2);
        assert!(hashtags.contains(&"ab".to_string()));
        assert!(hashtags.contains(&"abc".to_string()));
        assert!(!hashtags.contains(&"a".to_string()));
    }

    #[test]
    fn test_no_hashtags() {
        let content = "This post has no hashtags";
        let hashtags = extract_hashtags(content);
        assert_eq!(hashtags.len(), 0);
    }

    #[test]
    fn test_hashtag_at_start() {
        let content = "#rust is awesome";
        let hashtags = extract_hashtags(content);
        assert_eq!(hashtags.len(), 1);
        assert!(hashtags.contains(&"rust".to_string()));
    }

    #[test]
    fn test_hashtag_at_end() {
        let content = "Learning about #rust";
        let hashtags = extract_hashtags(content);
        assert_eq!(hashtags.len(), 1);
        assert!(hashtags.contains(&"rust".to_string()));
    }

    #[test]
    fn test_case_normalization() {
        let content = "#Rust #RUST #rust";
        let hashtags = extract_hashtags(content);
        assert_eq!(hashtags.len(), 1);
        assert!(hashtags.contains(&"rust".to_string()));
    }
}

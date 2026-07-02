//! Input validation module for Fido server
//!
//! This module provides comprehensive input validation to protect against
//! malicious input and ensure data integrity. All user-provided content
//! should be validated before processing.

use thiserror::Error;

/// Maximum length for usernames (characters)
pub const MAX_USERNAME_LENGTH: usize = 32;
/// Minimum length for usernames (characters)
pub const MIN_USERNAME_LENGTH: usize = 3;
/// Maximum length for bio content (characters)
pub const MAX_BIO_LENGTH: usize = 160;
/// Maximum length for post content (characters)
pub const MAX_POST_LENGTH: usize = 280;

/// Validation error types
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ValidationError {
    /// Content is empty when it shouldn't be
    #[error("{field} cannot be empty")]
    Empty { field: String },

    /// Content exceeds maximum length
    #[error("{field} exceeds maximum length of {max} characters (current: {current})")]
    TooLong {
        field: String,
        max: usize,
        current: usize,
    },

    /// Content is below minimum length
    #[error("{field} must be at least {min} characters (current: {current})")]
    TooShort {
        field: String,
        min: usize,
        current: usize,
    },

    /// Content contains invalid characters
    #[error("{field} contains invalid characters: {details}")]
    InvalidCharacters { field: String, details: String },

    /// Content contains potentially dangerous patterns
    #[error("{field} contains potentially dangerous content")]
    DangerousContent { field: String },
}

/// Result type for validation operations
pub type ValidationResult<T> = Result<T, ValidationError>;

/// Input validator for user-provided content
///
/// Provides validation methods for various types of user input including
/// usernames, bios, and post content.
#[derive(Debug, Clone, Default)]
pub struct InputValidator;

impl InputValidator {
    /// Create a new InputValidator instance
    pub fn new() -> Self {
        Self
    }

    /// Validate a username
    ///
    /// Usernames must:
    /// - Be between 3 and 32 characters
    /// - Contain only alphanumeric characters, underscores, and hyphens
    /// - Start with an alphanumeric character
    ///
    /// # Arguments
    /// * `username` - The username to validate
    ///
    /// # Returns
    /// * `Ok(())` if valid
    /// * `Err(ValidationError)` if invalid
    pub fn validate_username(&self, username: &str) -> ValidationResult<()> {
        let trimmed = username.trim();

        // Check for empty
        if trimmed.is_empty() {
            return Err(ValidationError::Empty {
                field: "Username".to_string(),
            });
        }

        // Check minimum length
        if trimmed.len() < MIN_USERNAME_LENGTH {
            return Err(ValidationError::TooShort {
                field: "Username".to_string(),
                min: MIN_USERNAME_LENGTH,
                current: trimmed.len(),
            });
        }

        // Check maximum length
        if trimmed.len() > MAX_USERNAME_LENGTH {
            return Err(ValidationError::TooLong {
                field: "Username".to_string(),
                max: MAX_USERNAME_LENGTH,
                current: trimmed.len(),
            });
        }

        // Check first character is alphanumeric
        if let Some(first_char) = trimmed.chars().next() {
            if !first_char.is_ascii_alphanumeric() {
                return Err(ValidationError::InvalidCharacters {
                    field: "Username".to_string(),
                    details: "must start with a letter or number".to_string(),
                });
            }
        }

        // Check all characters are valid (alphanumeric, underscore, hyphen)
        for ch in trimmed.chars() {
            if !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-' {
                return Err(ValidationError::InvalidCharacters {
                    field: "Username".to_string(),
                    details: format!(
                        "only letters, numbers, underscores, and hyphens are allowed (found '{}')",
                        ch
                    ),
                });
            }
        }

        Ok(())
    }

    /// Validate bio content
    ///
    /// Bios must:
    /// - Be at most 160 characters
    /// - Not contain dangerous HTML/script content
    ///
    /// Note: Empty bios are allowed (users can clear their bio)
    ///
    /// # Arguments
    /// * `bio` - The bio content to validate
    ///
    /// # Returns
    /// * `Ok(())` if valid
    /// * `Err(ValidationError)` if invalid
    pub fn validate_bio(&self, bio: &str) -> ValidationResult<()> {
        // Check maximum length
        if bio.len() > MAX_BIO_LENGTH {
            return Err(ValidationError::TooLong {
                field: "Bio".to_string(),
                max: MAX_BIO_LENGTH,
                current: bio.len(),
            });
        }

        // Check for dangerous content
        if self.contains_dangerous_patterns(bio) {
            return Err(ValidationError::DangerousContent {
                field: "Bio".to_string(),
            });
        }

        Ok(())
    }

    /// Validate post content
    ///
    /// Posts must:
    /// - Not be empty
    /// - Be at most 280 characters
    /// - Not contain dangerous HTML/script content
    ///
    /// # Arguments
    /// * `content` - The post content to validate
    ///
    /// # Returns
    /// * `Ok(())` if valid
    /// * `Err(ValidationError)` if invalid
    pub fn validate_post_content(&self, content: &str) -> ValidationResult<()> {
        let trimmed = content.trim();

        // Check for empty
        if trimmed.is_empty() {
            return Err(ValidationError::Empty {
                field: "Post content".to_string(),
            });
        }

        // Check maximum length
        if content.len() > MAX_POST_LENGTH {
            return Err(ValidationError::TooLong {
                field: "Post content".to_string(),
                max: MAX_POST_LENGTH,
                current: content.len(),
            });
        }

        // Check for dangerous content
        if self.contains_dangerous_patterns(content) {
            return Err(ValidationError::DangerousContent {
                field: "Post content".to_string(),
            });
        }

        Ok(())
    }

    /// Check if text contains potentially dangerous patterns
    ///
    /// Detects common XSS attack patterns including:
    /// - Script tags
    /// - Event handlers (onclick, onerror, etc.)
    /// - JavaScript URLs
    /// - Data URLs with script content
    fn contains_dangerous_patterns(&self, text: &str) -> bool {
        let lower = text.to_lowercase();

        // Check for script tags
        if lower.contains("<script") || lower.contains("</script") {
            return true;
        }

        // Check for common event handlers
        let event_handlers = [
            "onclick",
            "onerror",
            "onload",
            "onmouseover",
            "onmouseout",
            "onfocus",
            "onblur",
            "onsubmit",
            "onchange",
            "onkeydown",
            "onkeyup",
            "onkeypress",
        ];

        for handler in event_handlers {
            if lower.contains(&format!("{}=", handler)) {
                return true;
            }
        }

        // Check for javascript: URLs
        if lower.contains("javascript:") {
            return true;
        }

        // Check for data: URLs that might contain scripts
        if lower.contains("data:text/html") || lower.contains("data:application/javascript") {
            return true;
        }

        // Check for expression() CSS hack (IE)
        if lower.contains("expression(") {
            return true;
        }

        // Check for vbscript: URLs
        if lower.contains("vbscript:") {
            return true;
        }

        false
    }
}

/// Convenience function to validate a username
pub fn validate_username(username: &str) -> ValidationResult<()> {
    InputValidator::new().validate_username(username)
}

/// Convenience function to validate bio content
pub fn validate_bio(bio: &str) -> ValidationResult<()> {
    InputValidator::new().validate_bio(bio)
}

/// Convenience function to validate post content
pub fn validate_post_content(content: &str) -> ValidationResult<()> {
    InputValidator::new().validate_post_content(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Username validation tests
    #[test]
    fn test_validate_username_valid() {
        let validator = InputValidator::new();
        assert!(validator.validate_username("alice").is_ok());
        assert!(validator.validate_username("bob123").is_ok());
        assert!(validator.validate_username("user_name").is_ok());
        assert!(validator.validate_username("user-name").is_ok());
        assert!(validator.validate_username("User_Name-123").is_ok());
    }

    #[test]
    fn test_validate_username_empty() {
        let validator = InputValidator::new();
        let result = validator.validate_username("");
        assert!(matches!(result, Err(ValidationError::Empty { .. })));
    }

    #[test]
    fn test_validate_username_too_short() {
        let validator = InputValidator::new();
        let result = validator.validate_username("ab");
        assert!(matches!(result, Err(ValidationError::TooShort { .. })));
    }

    #[test]
    fn test_validate_username_too_long() {
        let validator = InputValidator::new();
        let long_name = "a".repeat(33);
        let result = validator.validate_username(&long_name);
        assert!(matches!(result, Err(ValidationError::TooLong { .. })));
    }

    #[test]
    fn test_validate_username_invalid_start() {
        let validator = InputValidator::new();
        let result = validator.validate_username("_username");
        assert!(matches!(
            result,
            Err(ValidationError::InvalidCharacters { .. })
        ));
    }

    #[test]
    fn test_validate_username_invalid_chars() {
        let validator = InputValidator::new();
        let result = validator.validate_username("user@name");
        assert!(matches!(
            result,
            Err(ValidationError::InvalidCharacters { .. })
        ));

        let result = validator.validate_username("user name");
        assert!(matches!(
            result,
            Err(ValidationError::InvalidCharacters { .. })
        ));
    }

    // Bio validation tests
    #[test]
    fn test_validate_bio_valid() {
        let validator = InputValidator::new();
        assert!(validator.validate_bio("Hello, I'm a developer!").is_ok());
        assert!(validator.validate_bio("").is_ok()); // Empty bio is allowed
        assert!(validator.validate_bio("🚀 Rust enthusiast").is_ok());
    }

    #[test]
    fn test_validate_bio_too_long() {
        let validator = InputValidator::new();
        let long_bio = "a".repeat(161);
        let result = validator.validate_bio(&long_bio);
        assert!(matches!(result, Err(ValidationError::TooLong { .. })));
    }

    #[test]
    fn test_validate_bio_dangerous_content() {
        let validator = InputValidator::new();
        let result = validator.validate_bio("<script>alert('xss')</script>");
        assert!(matches!(
            result,
            Err(ValidationError::DangerousContent { .. })
        ));
    }

    // Post content validation tests
    #[test]
    fn test_validate_post_content_valid() {
        let validator = InputValidator::new();
        assert!(validator.validate_post_content("Hello world!").is_ok());
        assert!(validator
            .validate_post_content("Check out #rust and #programming")
            .is_ok());
        assert!(validator.validate_post_content("🎉 Great news!").is_ok());
    }

    #[test]
    fn test_validate_post_content_empty() {
        let validator = InputValidator::new();
        let result = validator.validate_post_content("");
        assert!(matches!(result, Err(ValidationError::Empty { .. })));

        let result = validator.validate_post_content("   ");
        assert!(matches!(result, Err(ValidationError::Empty { .. })));
    }

    #[test]
    fn test_validate_post_content_too_long() {
        let validator = InputValidator::new();
        let long_post = "a".repeat(281);
        let result = validator.validate_post_content(&long_post);
        assert!(matches!(result, Err(ValidationError::TooLong { .. })));
    }

    #[test]
    fn test_validate_post_content_dangerous() {
        let validator = InputValidator::new();
        let result = validator.validate_post_content("Click <script>evil()</script> here");
        assert!(matches!(
            result,
            Err(ValidationError::DangerousContent { .. })
        ));
    }

    // Dangerous pattern detection tests
    #[test]
    fn test_dangerous_patterns_script_tags() {
        let validator = InputValidator::new();
        assert!(validator.contains_dangerous_patterns("<script>alert(1)</script>"));
        assert!(validator.contains_dangerous_patterns("<SCRIPT>alert(1)</SCRIPT>"));
        assert!(validator.contains_dangerous_patterns("<ScRiPt>alert(1)</ScRiPt>"));
    }

    #[test]
    fn test_dangerous_patterns_event_handlers() {
        let validator = InputValidator::new();
        assert!(validator.contains_dangerous_patterns("<img onclick=alert(1)>"));
        assert!(validator.contains_dangerous_patterns("<img onerror=alert(1)>"));
        assert!(validator.contains_dangerous_patterns("<body onload=alert(1)>"));
    }

    #[test]
    fn test_dangerous_patterns_javascript_urls() {
        let validator = InputValidator::new();
        assert!(validator.contains_dangerous_patterns("<a href='javascript:alert(1)'>"));
        assert!(validator.contains_dangerous_patterns("javascript:void(0)"));
    }

    #[test]
    fn test_dangerous_patterns_safe_content() {
        let validator = InputValidator::new();
        assert!(!validator.contains_dangerous_patterns("Hello world!"));
        assert!(!validator.contains_dangerous_patterns("Check out my script for automation"));
        assert!(!validator.contains_dangerous_patterns("I love JavaScript programming"));
    }

    // Convenience function tests
    #[test]
    fn test_convenience_functions() {
        assert!(validate_username("alice").is_ok());
        assert!(validate_bio("Hello!").is_ok());
        assert!(validate_post_content("Hello world!").is_ok());
    }

    #[test]
    fn test_validation_error_display() {
        let err = ValidationError::Empty {
            field: "Username".to_string(),
        };
        assert_eq!(format!("{}", err), "Username cannot be empty");

        let err = ValidationError::TooLong {
            field: "Bio".to_string(),
            max: 160,
            current: 200,
        };
        assert_eq!(
            format!("{}", err),
            "Bio exceeds maximum length of 160 characters (current: 200)"
        );
    }
}

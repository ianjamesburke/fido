use std::env;

/// Detects the current application mode based on environment variables
#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    /// Native terminal mode - uses file-based storage
    Native,
    /// Web terminal mode - uses browser-based storage
    Web,
}

/// Mode detection system using environment variables
#[derive(Debug, Clone)]
pub struct ModeDetector {
    pub mode: AppMode,
}

impl ModeDetector {
    /// Create a new mode detector by checking environment variables
    pub fn new() -> Self {
        let mode = if env::var("FIDO_WEB_MODE").is_ok() {
            AppMode::Web
        } else {
            AppMode::Native
        };
        
        Self { mode }
    }
    
    /// Get the detected application mode
    pub fn mode(&self) -> &AppMode {
        &self.mode
    }
    
    /// Check if running in web mode
    pub fn is_web_mode(&self) -> bool {
        matches!(self.mode, AppMode::Web)
    }
    
    /// Check if running in native mode
    pub fn is_native_mode(&self) -> bool {
        matches!(self.mode, AppMode::Native)
    }
}

impl Default for ModeDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_native_mode_when_no_env_var() {
        // Store original environment state
        let original_env = env::var("FIDO_WEB_MODE").ok();
        
        // Ensure FIDO_WEB_MODE is not set
        env::remove_var("FIDO_WEB_MODE");
        
        let detector = ModeDetector::new();
        assert_eq!(detector.mode(), &AppMode::Native);
        assert!(detector.is_native_mode());
        assert!(!detector.is_web_mode());
        
        // Restore original environment state
        match original_env {
            Some(value) => env::set_var("FIDO_WEB_MODE", value),
            None => env::remove_var("FIDO_WEB_MODE"),
        }
    }

    #[test]
    fn test_web_mode_when_env_var_set() {
        // Store original environment state
        let original_env = env::var("FIDO_WEB_MODE").ok();
        
        // Set FIDO_WEB_MODE environment variable
        env::set_var("FIDO_WEB_MODE", "true");
        
        let detector = ModeDetector::new();
        assert_eq!(detector.mode(), &AppMode::Web);
        assert!(detector.is_web_mode());
        assert!(!detector.is_native_mode());
        
        // Restore original environment state
        match original_env {
            Some(value) => env::set_var("FIDO_WEB_MODE", value),
            None => env::remove_var("FIDO_WEB_MODE"),
        }
    }

    #[test]
    fn test_web_mode_with_any_value() {
        // Store original environment state
        let original_env = env::var("FIDO_WEB_MODE").ok();
        
        // Ensure clean state first
        env::remove_var("FIDO_WEB_MODE");
        
        // FIDO_WEB_MODE can have any value - just needs to exist
        env::set_var("FIDO_WEB_MODE", "false");
        let detector = ModeDetector::new();
        assert_eq!(detector.mode(), &AppMode::Web);
        
        env::set_var("FIDO_WEB_MODE", "1");
        let detector = ModeDetector::new();
        assert_eq!(detector.mode(), &AppMode::Web);
        
        env::set_var("FIDO_WEB_MODE", "");
        let detector = ModeDetector::new();
        assert_eq!(detector.mode(), &AppMode::Web);
        
        // Restore original environment state
        match original_env {
            Some(value) => env::set_var("FIDO_WEB_MODE", value),
            None => env::remove_var("FIDO_WEB_MODE"),
        }
    }

    // Property-based tests
    use proptest::prelude::*;

    // **Feature: web-terminal-interface, Property 7: Mode Detection Accuracy**
    // **Validates: Requirements 4.1**
    // For any environment variable state, the system should correctly detect web mode 
    // when FIDO_WEB_MODE environment variable is present and native mode otherwise
    proptest! {
        #[test]
        fn prop_mode_detection_accuracy(
            env_var_value in prop::option::of("[a-zA-Z0-9_-]*"),
            other_env_vars in prop::collection::vec(("[A-Z_]{1,20}", "[a-zA-Z0-9_-]{0,50}"), 0..3)
        ) {
            // Clean up any existing FIDO_WEB_MODE
            env::remove_var("FIDO_WEB_MODE");
            
            // Set up other environment variables (but not FIDO_WEB_MODE)
            // Filter out any FIDO-related environment variables to avoid conflicts
            let safe_env_vars: Vec<_> = other_env_vars.iter()
                .filter(|(key, _)| !key.starts_with("FIDO"))
                .collect();
            
            for (key, value) in &safe_env_vars {
                env::set_var(key, value);
            }
            
            // Test native mode (no FIDO_WEB_MODE set)
            let detector_native = ModeDetector::new();
            prop_assert_eq!(detector_native.mode(), &AppMode::Native);
            prop_assert!(detector_native.is_native_mode());
            prop_assert!(!detector_native.is_web_mode());
            
            // Test web mode (FIDO_WEB_MODE set to any value or empty)
            if let Some(value) = env_var_value {
                env::set_var("FIDO_WEB_MODE", &value);
            } else {
                env::set_var("FIDO_WEB_MODE", "");
            }
            
            let detector_web = ModeDetector::new();
            prop_assert_eq!(detector_web.mode(), &AppMode::Web);
            prop_assert!(detector_web.is_web_mode());
            prop_assert!(!detector_web.is_native_mode());
            
            // Clean up
            env::remove_var("FIDO_WEB_MODE");
            for (key, _) in &safe_env_vars {
                env::remove_var(key);
            }
        }
    }
}
use crate::config::{ConfigManager, ServerConfig};
use anyhow::Result;

/// Server configuration utility for managing server URL settings
pub struct ServerConfigManager {
    config_manager: ConfigManager,
}

impl ServerConfigManager {
    /// Create a new server configuration manager
    pub fn new() -> Result<Self> {
        let config_manager = ConfigManager::new()?;
        Ok(Self { config_manager })
    }

    /// Determine the server URL to use based on priority:
    /// 1. CLI argument (highest priority)
    /// 2. Environment variable FIDO_SERVER_URL
    /// 3. Saved configuration file
    /// 4. Default based on mode (lowest priority)
    pub fn determine_server_url(&self, cli_override: Option<String>) -> Result<String> {
        // 1. CLI argument has highest priority
        if let Some(url) = cli_override {
            return Ok(url);
        }

        // 2. Environment variable
        if let Ok(url) = std::env::var("FIDO_SERVER_URL") {
            return Ok(url);
        }

        // 3. Saved configuration file
        if let Some(config) = self.config_manager.load_server_config()? {
            return Ok(config.server_url);
        }

        // 4. Default based on mode
        Ok(self.get_default_server_url())
    }

    /// Get the default server URL based on the current mode
    fn get_default_server_url(&self) -> String {
        // Check if running in web mode
        if std::env::var("FIDO_WEB_MODE").is_ok() {
            // In web mode (running on server), connect to localhost API server
            "http://127.0.0.1:3000".to_string()
        } else {
            // For local TUI client, use production URL by default
            "https://fido-social.fly.dev".to_string()
        }
    }

    /// Save server URL to configuration file
    pub fn save_server_url(&self, server_url: String) -> Result<()> {
        let config = ServerConfig {
            server_url,
            last_updated: chrono::Utc::now(),
        };
        self.config_manager.save_server_config(&config)
    }

    /// Get the currently configured server URL (from file or default)
    pub fn get_configured_server_url(&self) -> Result<String> {
        if let Some(config) = self.config_manager.load_server_config()? {
            Ok(config.server_url)
        } else {
            Ok(self.get_default_server_url())
        }
    }

    /// Check if a custom server URL is configured (not using default)
    pub fn has_custom_server_url(&self) -> Result<bool> {
        if std::env::var("FIDO_SERVER_URL").is_ok() {
            return Ok(true);
        }

        if let Some(config) = self.config_manager.load_server_config()? {
            let default_url = self.get_default_server_url();
            return Ok(config.server_url != default_url);
        }

        Ok(false)
    }

    /// Get a display-friendly description of the current server configuration
    pub fn get_server_description(&self, current_url: &str) -> String {
        let default_url = self.get_default_server_url();

        if current_url == default_url {
            if current_url.contains("fido-social.fly.dev") {
                "Production Server (default)".to_string()
            } else {
                "Local Development Server (default)".to_string()
            }
        } else if current_url.contains("localhost") || current_url.contains("127.0.0.1") {
            "Local Development Server (custom)".to_string()
        } else if current_url.contains("fido-social.fly.dev") {
            "Production Server (custom)".to_string()
        } else {
            "Custom Server".to_string()
        }
    }
}

impl Default for ServerConfigManager {
    fn default() -> Self {
        Self::new().expect("Failed to create server config manager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_default_server_url() {
        // Store original environment state
        let original_web_mode = env::var("FIDO_WEB_MODE").ok();

        let manager = ServerConfigManager::new().unwrap();

        // Test production default (when not in web mode)
        env::remove_var("FIDO_WEB_MODE");
        assert_eq!(
            manager.get_default_server_url(),
            "https://fido-social.fly.dev"
        );

        // Test web mode default
        env::set_var("FIDO_WEB_MODE", "true");
        assert_eq!(manager.get_default_server_url(), "http://127.0.0.1:3000");

        // Restore original environment state
        match original_web_mode {
            Some(value) => env::set_var("FIDO_WEB_MODE", value),
            None => env::remove_var("FIDO_WEB_MODE"),
        }
    }

    #[test]
    fn test_server_description() {
        let manager = ServerConfigManager::new().unwrap();

        // Test production server
        let desc = manager.get_server_description("https://fido-social.fly.dev");
        assert!(desc.contains("Production"));

        // Test local server
        let desc = manager.get_server_description("http://localhost:3000");
        assert!(desc.contains("Local Development"));

        // Test custom server
        let desc = manager.get_server_description("https://custom.example.com");
        assert_eq!(desc, "Custom Server");
    }

    #[test]
    fn test_cli_override_priority() {
        let manager = ServerConfigManager::new().unwrap();

        // CLI override should have highest priority
        let url = manager
            .determine_server_url(Some("http://cli-override:3000".to_string()))
            .unwrap();
        assert_eq!(url, "http://cli-override:3000");
    }

    #[test]
    fn test_env_var_priority() {
        let manager = ServerConfigManager::new().unwrap();

        // Set environment variable
        env::set_var("FIDO_SERVER_URL", "http://env-override:3000");

        // Environment variable should be used when no CLI override
        let url = manager.determine_server_url(None).unwrap();
        assert_eq!(url, "http://env-override:3000");

        // Clean up
        env::remove_var("FIDO_SERVER_URL");
    }
}

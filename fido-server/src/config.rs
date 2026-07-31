use config::{Config, ConfigBuilder, ConfigError, File};
use serde::Deserialize;
use std::path::PathBuf;

// Configuration constants
const CONFIG_FILE_NAME: &str = "settings.toml";
const DEFAULT_HOST: &str = "0.0.0.0";
const DEFAULT_PORT: u16 = 4747;
const DEFAULT_DB_PATH: &str = "fido.db";
const DEV_CONFIG_DIR: &str = "fido-server";

#[derive(Debug, Deserialize, Clone)]
pub struct Server {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Database {
    pub path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OAuth {
    pub github_client_id: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub server: Server,
    pub database: Database,
    pub oauth: OAuth,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            server: Server {
                host: DEFAULT_HOST.to_string(),
                port: DEFAULT_PORT,
            },
            database: Database {
                path: DEFAULT_DB_PATH.to_string(),
            },
            oauth: OAuth {
                github_client_id: String::new(), // Will be loaded from environment
            },
        }
    }
}

impl Settings {
    /// Create a new Settings instance with configuration loaded from files and environment
    pub fn new() -> Result<Self, ConfigError> {
        let mut builder = Config::builder();

        // Load configuration files (lowest priority)
        builder = Self::load_config_files(builder)?;

        // Set defaults (medium priority)
        builder = Self::set_defaults(builder)?;

        // Apply environment overrides (highest priority)
        builder = Self::apply_env_overrides(builder)?;

        let config = builder.build()?;
        config.try_deserialize()
    }

    /// Load configuration files from standard locations
    fn load_config_files(
        mut builder: ConfigBuilder<config::builder::DefaultState>,
    ) -> Result<ConfigBuilder<config::builder::DefaultState>, ConfigError> {
        // Check in current directory (for production deployment)
        let current_dir_path = PathBuf::from(CONFIG_FILE_NAME);
        if current_dir_path.exists() {
            builder = builder.add_source(File::from(current_dir_path).required(false));
        }

        // Check in fido-server directory (for development)
        let dev_path = PathBuf::from(DEV_CONFIG_DIR).join(CONFIG_FILE_NAME);
        if dev_path.exists() {
            builder = builder.add_source(File::from(dev_path).required(false));
        }

        Ok(builder)
    }

    /// Set default configuration values
    fn set_defaults(
        builder: ConfigBuilder<config::builder::DefaultState>,
    ) -> Result<ConfigBuilder<config::builder::DefaultState>, ConfigError> {
        builder
            .set_default("server.host", DEFAULT_HOST)?
            .set_default("server.port", DEFAULT_PORT)?
            .set_default("database.path", DEFAULT_DB_PATH)?
            .set_default("oauth.github_client_id", "")
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(
        mut builder: ConfigBuilder<config::builder::DefaultState>,
    ) -> Result<ConfigBuilder<config::builder::DefaultState>, ConfigError> {
        // Environment variables take highest priority
        if let Ok(db_path) = std::env::var("DATABASE_PATH") {
            builder = builder.set_override("database.path", db_path)?;
        }
        if let Ok(port) = std::env::var("PORT") {
            builder = builder.set_override("server.port", port)?;
        }
        if let Ok(host) = std::env::var("HOST") {
            builder = builder.set_override("server.host", host)?;
        }

        // OAuth configuration from environment (required)
        if let Ok(github_client_id) = std::env::var("GITHUB_CLIENT_ID") {
            builder = builder.set_override("oauth.github_client_id", github_client_id)?;
        }

        Ok(builder)
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate port range
        if self.server.port == 0 {
            return Err(ConfigError::Message("Port cannot be 0".to_string()));
        }

        // Validate host format (basic check)
        if self.server.host.is_empty() {
            return Err(ConfigError::Message("Host cannot be empty".to_string()));
        }

        // Validate OAuth configuration
        if self.oauth.github_client_id.is_empty() {
            return Err(ConfigError::Message(
                "GITHUB_CLIENT_ID environment variable is required but not set".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::env_lock;
    use std::env;

    #[test]
    fn test_default_settings() {
        let _guard = env_lock().lock().unwrap();
        let settings = Settings::default();
        assert_eq!(settings.server.host, DEFAULT_HOST);
        assert_eq!(settings.server.port, DEFAULT_PORT);
        assert_eq!(settings.database.path, DEFAULT_DB_PATH);
        assert_eq!(settings.oauth.github_client_id, "");
    }

    #[test]
    fn test_validation_success() {
        let _guard = env_lock().lock().unwrap();
        let mut settings = Settings::default();
        settings.oauth.github_client_id = "test_client_id".to_string();
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn test_validation_zero_port() {
        let _guard = env_lock().lock().unwrap();
        let mut settings = Settings::default();
        settings.server.port = 0;
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_validation_empty_host() {
        let _guard = env_lock().lock().unwrap();
        let mut settings = Settings::default();
        settings.server.host = String::new();
        settings.oauth.github_client_id = "test_client_id".to_string();
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_validation_missing_github_client_id() {
        let _guard = env_lock().lock().unwrap();
        let settings = Settings::default(); // github_client_id is empty by default
        let result = settings.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("GITHUB_CLIENT_ID"));
    }

    #[test]
    fn test_environment_variable_overrides() {
        let _guard = env_lock().lock().unwrap();
        // Set environment variables
        env::set_var("HOST", "127.0.0.1");
        env::set_var("PORT", "8080");
        env::set_var("DATABASE_PATH", "/tmp/test.db");
        env::set_var("GITHUB_CLIENT_ID", "test_github_client_id");

        let settings = Settings::new().expect("Failed to load settings");

        assert_eq!(settings.server.host, "127.0.0.1");
        assert_eq!(settings.server.port, 8080);
        assert_eq!(settings.database.path, "/tmp/test.db");
        assert_eq!(settings.oauth.github_client_id, "test_github_client_id");

        // Clean up
        env::remove_var("HOST");
        env::remove_var("PORT");
        env::remove_var("DATABASE_PATH");
        env::remove_var("GITHUB_CLIENT_ID");
    }
}

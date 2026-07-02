//! Security configuration module for Fido server
//!
//! This module provides environment-based security configuration with validation
//! for production-ready security controls.

pub mod admin;
pub mod audit;
pub mod cookies;
pub mod cors;
pub mod errors;
pub mod headers;
pub mod validation;

use std::fmt;
use thiserror::Error;

pub use audit::{AuditEvent, AuditEventType, AuditLogger};
pub use cookies::{create_session_cookie, is_https};
pub use cors::CorsConfig;

/// Security configuration errors
#[derive(Debug, Error)]
pub enum SecurityConfigError {
    #[error("Missing required configuration: {0}")]
    MissingConfig(String),

    #[error("Invalid configuration value for {field}: {message}")]
    InvalidValue { field: String, message: String },
}

/// Application environment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Environment {
    /// Development environment - relaxed security for local testing
    #[default]
    Development,
    /// Production environment - strict security enforcement
    Production,
}

impl Environment {
    /// Parse environment from string
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "production" | "prod" => Environment::Production,
            _ => Environment::Development,
        }
    }

    /// Check if this is a production environment
    pub fn is_production(&self) -> bool {
        matches!(self, Environment::Production)
    }

    /// Check if this is a development environment
    pub fn is_development(&self) -> bool {
        matches!(self, Environment::Development)
    }
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Environment::Development => write!(f, "development"),
            Environment::Production => write!(f, "production"),
        }
    }
}

/// Security configuration for the application
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Current environment (Development or Production)
    pub environment: Environment,

    /// Allowed CORS origins
    pub allowed_origins: Vec<String>,

    /// GitHub OAuth client ID
    pub github_client_id: String,

    /// Whether to enable HTTPS redirect
    pub enable_https_redirect: bool,

    /// Maximum request body size in bytes (default: 1MB)
    pub max_request_size: usize,

    /// Maximum session age in days (default: 30)
    pub session_max_age_days: i64,

    /// Session idle timeout in hours (default: 720 / 30 days)
    pub session_idle_timeout_hours: i64,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            environment: Environment::Development,
            allowed_origins: vec![
                "http://localhost:3000".to_string(),
                "http://localhost:8080".to_string(),
                "http://127.0.0.1:3000".to_string(),
                "http://127.0.0.1:8080".to_string(),
            ],
            github_client_id: String::new(),
            enable_https_redirect: false,
            max_request_size: 1024 * 1024, // 1MB
            session_max_age_days: 30,
            session_idle_timeout_hours: 24 * 30,
        }
    }
}

impl SecurityConfig {
    /// Create a new SecurityConfig by loading from environment variables
    pub fn from_env() -> Result<Self, SecurityConfigError> {
        let environment = std::env::var("ENVIRONMENT")
            .or_else(|_| std::env::var("RUST_ENV"))
            .map(|s| Environment::parse(&s))
            .unwrap_or(Environment::Development);

        let github_client_id = std::env::var("GITHUB_CLIENT_ID").unwrap_or_default();

        let allowed_origins = Self::get_allowed_origins(&environment);

        let enable_https_redirect = environment.is_production();

        let max_request_size = std::env::var("MAX_REQUEST_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1024 * 1024); // 1MB default

        let session_max_age_days = std::env::var("SESSION_MAX_AGE_DAYS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);

        let session_idle_timeout_hours = std::env::var("SESSION_IDLE_TIMEOUT_HOURS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(24 * 30);

        Ok(Self {
            environment,
            allowed_origins,
            github_client_id,
            enable_https_redirect,
            max_request_size,
            session_max_age_days,
            session_idle_timeout_hours,
        })
    }

    /// Get allowed origins based on environment
    fn get_allowed_origins(environment: &Environment) -> Vec<String> {
        if let Ok(origins_env) = std::env::var("ALLOWED_ORIGINS") {
            let parsed = origins_env
                .split(',')
                .map(|origin| origin.trim().to_string())
                .filter(|origin| !origin.is_empty())
                .collect::<Vec<_>>();
            if !parsed.is_empty() {
                return parsed;
            }
        }

        match environment {
            Environment::Production => {
                vec!["https://fido-web-production.up.railway.app".to_string()]
            }
            Environment::Development => {
                vec![
                    "http://localhost:3000".to_string(),
                    "http://localhost:8080".to_string(),
                    "http://127.0.0.1:3000".to_string(),
                    "http://127.0.0.1:8080".to_string(),
                ]
            }
        }
    }

    /// Validate the security configuration
    ///
    /// In production, this will fail if required configuration is missing.
    /// In development, it will log warnings but allow startup.
    pub fn validate(&self) -> Result<(), SecurityConfigError> {
        // GitHub client ID is required in production
        if self.environment.is_production() && self.github_client_id.is_empty() {
            return Err(SecurityConfigError::MissingConfig(
                "GITHUB_CLIENT_ID is required in production".to_string(),
            ));
        }

        // Validate session configuration
        if self.session_max_age_days <= 0 {
            return Err(SecurityConfigError::InvalidValue {
                field: "session_max_age_days".to_string(),
                message: "Must be greater than 0".to_string(),
            });
        }

        if self.session_idle_timeout_hours <= 0 {
            return Err(SecurityConfigError::InvalidValue {
                field: "session_idle_timeout_hours".to_string(),
                message: "Must be greater than 0".to_string(),
            });
        }

        // Validate max request size
        if self.max_request_size == 0 {
            return Err(SecurityConfigError::InvalidValue {
                field: "max_request_size".to_string(),
                message: "Must be greater than 0".to_string(),
            });
        }

        // Validate allowed origins in production
        if self.environment.is_production() && self.allowed_origins.is_empty() {
            return Err(SecurityConfigError::InvalidValue {
                field: "allowed_origins".to_string(),
                message: "At least one allowed origin is required in production".to_string(),
            });
        }

        Ok(())
    }

    /// Log the security configuration (without sensitive values)
    pub fn log_config(&self) {
        tracing::info!("Security Configuration:");
        tracing::info!("  Environment: {}", self.environment);
        tracing::info!("  Allowed Origins: {:?}", self.allowed_origins);
        tracing::info!("  HTTPS Redirect: {}", self.enable_https_redirect);
        tracing::info!("  Max Request Size: {} bytes", self.max_request_size);
        tracing::info!("  Session Max Age: {} days", self.session_max_age_days);
        tracing::info!(
            "  Session Idle Timeout: {} hours",
            self.session_idle_timeout_hours
        );
        // Note: github_client_id is intentionally not logged for security
        tracing::info!(
            "  GitHub Client ID: {}",
            if self.github_client_id.is_empty() {
                "<not set>"
            } else {
                "<configured>"
            }
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_from_str() {
        assert_eq!(Environment::parse("production"), Environment::Production);
        assert_eq!(Environment::parse("prod"), Environment::Production);
        assert_eq!(Environment::parse("PRODUCTION"), Environment::Production);
        assert_eq!(Environment::parse("development"), Environment::Development);
        assert_eq!(Environment::parse("dev"), Environment::Development);
        assert_eq!(Environment::parse("anything"), Environment::Development);
    }

    #[test]
    fn test_environment_is_production() {
        assert!(Environment::Production.is_production());
        assert!(!Environment::Development.is_production());
    }

    #[test]
    fn test_environment_is_development() {
        assert!(Environment::Development.is_development());
        assert!(!Environment::Production.is_development());
    }

    #[test]
    fn test_environment_display() {
        assert_eq!(format!("{}", Environment::Production), "production");
        assert_eq!(format!("{}", Environment::Development), "development");
    }

    #[test]
    fn test_security_config_default() {
        let config = SecurityConfig::default();
        assert_eq!(config.environment, Environment::Development);
        assert!(!config.allowed_origins.is_empty());
        assert_eq!(config.max_request_size, 1024 * 1024);
        assert_eq!(config.session_max_age_days, 30);
        assert_eq!(config.session_idle_timeout_hours, 24 * 30);
    }

    #[test]
    fn test_security_config_validate_development() {
        let config = SecurityConfig::default();
        // Development mode should pass validation even without github_client_id
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_security_config_validate_production_missing_github() {
        let config = SecurityConfig {
            environment: Environment::Production,
            github_client_id: String::new(),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(result, Err(SecurityConfigError::MissingConfig(_))));
    }

    #[test]
    fn test_security_config_validate_production_success() {
        let config = SecurityConfig {
            environment: Environment::Production,
            github_client_id: "test_client_id".to_string(),
            allowed_origins: vec!["https://fido-web-production.up.railway.app".to_string()],
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_security_config_validate_invalid_session_age() {
        let config = SecurityConfig {
            session_max_age_days: 0,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(SecurityConfigError::InvalidValue { .. })
        ));
    }

    #[test]
    fn test_security_config_validate_invalid_idle_timeout() {
        let config = SecurityConfig {
            session_idle_timeout_hours: -1,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_get_allowed_origins_production() {
        let origins = SecurityConfig::get_allowed_origins(&Environment::Production);
        assert_eq!(origins.len(), 1);
        assert!(origins.contains(&"https://fido-web-production.up.railway.app".to_string()));
    }

    #[test]
    fn test_get_allowed_origins_development() {
        let origins = SecurityConfig::get_allowed_origins(&Environment::Development);
        assert!(origins.len() >= 2);
        assert!(origins.contains(&"http://localhost:3000".to_string()));
    }
}

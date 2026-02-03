//! CORS (Cross-Origin Resource Sharing) configuration module
//!
//! This module provides environment-aware CORS configuration to protect the web terminal
//! interface from CSRF attacks while allowing CLI and TUI clients to function.
//!
//! Key behaviors:
//! - Production: Only allows https://fido-social.fly.dev origin
//! - Development: Allows http://localhost:* origins for local testing
//! - Always allows requests without Origin header (CLI/TUI clients)
//! - Restricts methods to GET, POST, PUT, DELETE
//! - Restricts headers to Content-Type, X-Session-Token

use super::Environment;
use axum::http::{header, HeaderName, HeaderValue, Method};
use tower_http::cors::{AllowOrigin, CorsLayer};

/// CORS configuration for the application
#[derive(Debug, Clone)]
pub struct CorsConfig {
    /// Allowed origins for CORS requests
    allowed_origins: Vec<String>,
    /// Allowed HTTP methods
    allowed_methods: Vec<Method>,
    /// Allowed HTTP headers
    allowed_headers: Vec<HeaderName>,
    /// Current environment
    environment: Environment,
}

impl CorsConfig {
    /// Create a new CorsConfig for the given environment
    pub fn for_environment(environment: Environment) -> Self {
        let allowed_origins = match environment {
            Environment::Production => {
                vec!["https://fido-social.fly.dev".to_string()]
            }
            Environment::Development => {
                vec![
                    "http://localhost:3000".to_string(),
                    "http://localhost:8080".to_string(),
                    "http://localhost:5173".to_string(),
                    "http://127.0.0.1:3000".to_string(),
                    "http://127.0.0.1:8080".to_string(),
                    "http://127.0.0.1:5173".to_string(),
                ]
            }
        };

        let allowed_methods = vec![Method::GET, Method::POST, Method::PUT, Method::DELETE];

        let allowed_headers = vec![
            header::CONTENT_TYPE,
            HeaderName::from_static("x-session-token"),
        ];

        Self {
            allowed_origins,
            allowed_methods,
            allowed_headers,
            environment,
        }
    }

    /// Get the allowed origins
    pub fn allowed_origins(&self) -> &[String] {
        &self.allowed_origins
    }

    /// Convert to a tower-http CorsLayer
    ///
    /// This creates a CorsLayer that:
    /// - Allows configured origins based on environment
    /// - Allows requests without Origin header (for CLI/TUI clients)
    /// - Restricts methods to GET, POST, PUT, DELETE
    /// - Restricts headers to Content-Type, X-Session-Token
    pub fn to_cors_layer(&self) -> CorsLayer {
        let allowed_origins = self.allowed_origins.clone();
        let is_development = self.environment.is_development();

        // Create a custom origin predicate that:
        // 1. Allows requests without Origin header (CLI/TUI clients)
        // 2. Allows configured origins
        // 3. In development, allows any localhost origin
        let allow_origin = AllowOrigin::predicate(move |origin: &HeaderValue, _request_parts| {
            // Convert origin to string for comparison
            let origin_str = match origin.to_str() {
                Ok(s) => s,
                Err(_) => return false, // Invalid UTF-8 in origin header
            };

            // Check if origin is in the allowed list
            if allowed_origins.iter().any(|o| o == origin_str) {
                return true;
            }

            // In development, allow any localhost origin
            if is_development {
                if origin_str.starts_with("http://localhost:")
                    || origin_str.starts_with("http://127.0.0.1:")
                {
                    return true;
                }
            }

            false
        });

        CorsLayer::new()
            .allow_origin(allow_origin)
            .allow_methods(self.allowed_methods.clone())
            .allow_headers(self.allowed_headers.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cors_layer_creation() {
        // Just verify that to_cors_layer doesn't panic
        let config = CorsConfig::for_environment(Environment::Production);
        let _layer = config.to_cors_layer();

        let config = CorsConfig::for_environment(Environment::Development);
        let _layer = config.to_cors_layer();
    }

    #[test]
    fn test_cors_config_getters() {
        let config = CorsConfig::for_environment(Environment::Production);

        assert_eq!(config.allowed_origins().len(), 1);
    }
}

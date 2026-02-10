//! Security headers middleware for Fido server
//!
//! This module provides middleware that adds security headers to all HTTP responses.
//! These headers provide defense-in-depth protection against common web attacks.
//!
//! Headers added:
//! - X-Content-Type-Options: nosniff - Prevents MIME type sniffing
//! - X-Frame-Options: DENY - Prevents clickjacking attacks
//! - X-XSS-Protection: 1; mode=block - Enables XSS filtering in older browsers
//! - Content-Security-Policy: default-src 'self' - Restricts resource loading
//! - Referrer-Policy: strict-origin-when-cross-origin - Controls referrer information
//! - Strict-Transport-Security (production only) - Enforces HTTPS

use axum::{
    body::Body,
    http::{header, Request, Response},
    middleware::Next,
};

use super::Environment;

/// Security headers middleware that adds protective headers to all responses
///
/// This middleware adds the following security headers:
/// - `X-Content-Type-Options: nosniff` - Prevents MIME type sniffing
/// - `X-Frame-Options: DENY` - Prevents clickjacking by denying framing
/// - `X-XSS-Protection: 1; mode=block` - Enables XSS filtering in older browsers
/// - `Content-Security-Policy: default-src 'self'` - Restricts resource loading
/// - `Referrer-Policy: strict-origin-when-cross-origin` - Controls referrer information
/// - `Strict-Transport-Security` (production only) - Enforces HTTPS for 1 year
///
/// # Arguments
///
/// * `environment` - The current environment (Development or Production)
/// * `request` - The incoming HTTP request
/// * `next` - The next middleware/handler in the chain
///
/// # Returns
///
/// The response with security headers added
pub async fn security_headers_middleware(
    environment: Environment,
    request: Request<Body>,
    next: Next,
) -> Response<Body> {
    // Process the request through the rest of the middleware chain
    let mut response = next.run(request).await;

    // Get mutable reference to headers
    let headers = response.headers_mut();

    // X-Content-Type-Options: Prevents MIME type sniffing
    // Requirement 9.1
    headers.insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());

    // X-Frame-Options: Prevents clickjacking attacks
    // Requirement 9.2
    headers.insert(header::X_FRAME_OPTIONS, "DENY".parse().unwrap());

    // X-XSS-Protection: Enables XSS filtering in older browsers
    // Requirement 9.3
    headers.insert(header::X_XSS_PROTECTION, "1; mode=block".parse().unwrap());

    // Content-Security-Policy: Restricts resource loading to same origin
    // Requirement 9.4
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        "default-src 'self'".parse().unwrap(),
    );

    // Referrer-Policy: Controls how much referrer information is sent
    // Requirement 9.5
    headers.insert(
        header::REFERRER_POLICY,
        "strict-origin-when-cross-origin".parse().unwrap(),
    );

    // Strict-Transport-Security: Enforces HTTPS (production only)
    // Requirement 9.6
    if environment.is_production() {
        headers.insert(
            header::STRICT_TRANSPORT_SECURITY,
            "max-age=31536000; includeSubDomains".parse().unwrap(),
        );
    }

    response
}

/// Create a middleware layer for security headers
///
/// This function creates a closure that can be used with `axum::middleware::from_fn`
/// to add security headers to all responses.
///
/// # Arguments
///
/// * `environment` - The current environment (Development or Production)
///
/// # Example
///
/// ```ignore
/// use axum::{Router, middleware};
/// use fido_server::security::{Environment, headers::create_security_headers_layer};
///
/// let app = Router::new()
///     .route("/", get(handler))
///     .layer(middleware::from_fn(create_security_headers_layer(Environment::Production)));
/// ```
pub fn create_security_headers_layer(
    environment: Environment,
) -> impl Fn(
    Request<Body>,
    Next,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response<Body>> + Send>>
       + Clone
       + Send
       + 'static {
    move |request: Request<Body>, next: Next| {
        let env = environment;
        Box::pin(async move { security_headers_middleware(env, request, next).await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

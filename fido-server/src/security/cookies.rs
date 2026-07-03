use axum::http::{HeaderMap, HeaderValue};

const SESSION_COOKIE_MAX_AGE_SECONDS: i64 = 60 * 60 * 24 * 30;

/// Detects if the request is over HTTPS by checking proxy headers
///
/// Checks the following headers in order:
/// 1. X-Forwarded-Proto: https
/// 2. X-Forwarded-Ssl: on
///
/// This is necessary because when behind a reverse proxy,
/// the application sees HTTP traffic even though the client connection is HTTPS.
///
/// # Arguments
/// * `headers` - The request headers to check
///
/// # Returns
/// * `bool` - true if HTTPS is detected, false otherwise
pub fn is_https(headers: &HeaderMap) -> bool {
    // Check X-Forwarded-Proto header
    if let Some(proto) = headers.get("X-Forwarded-Proto") {
        if let Ok(proto_str) = proto.to_str() {
            if proto_str.eq_ignore_ascii_case("https") {
                return true;
            }
        }
    }

    // Check X-Forwarded-Ssl header
    if let Some(ssl) = headers.get("X-Forwarded-Ssl") {
        if let Ok(ssl_str) = ssl.to_str() {
            if ssl_str.eq_ignore_ascii_case("on") {
                return true;
            }
        }
    }

    false
}

/// Creates a Set-Cookie header value for a session token
///
/// The cookie includes security attributes:
/// - HttpOnly: Prevents JavaScript access to the cookie
/// - SameSite=Strict: Prevents CSRF attacks
/// - Secure: Only sent over HTTPS (when HTTPS is detected)
/// - Path=/: Available for all paths
/// - Max-Age: 30 days (2592000 seconds)
///
/// # Arguments
/// * `token` - The session token value
/// * `is_https` - Whether the request is over HTTPS (from spoofable proxy headers)
/// * `is_production` - Whether the server runs in production
///
/// In production the `Secure` flag is always set: the cookie must never be
/// transmitted over plaintext, and we do not trust the spoofable
/// `X-Forwarded-Proto` header to decide that. Outside production it is set
/// only when HTTPS is detected, so local HTTP development still works.
///
/// # Returns
/// * `HeaderValue` - The Set-Cookie header value
pub fn create_session_cookie(token: &str, is_https: bool, is_production: bool) -> HeaderValue {
    let secure_flag = if is_production || is_https {
        "; Secure"
    } else {
        ""
    };
    let cookie = format!(
        "session_token={}; HttpOnly; SameSite=Strict; Path=/; Max-Age={}{}",
        token, SESSION_COOKIE_MAX_AGE_SECONDS, secure_flag
    );

    HeaderValue::from_str(&cookie).unwrap_or_else(|_| HeaderValue::from_static(""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    #[test]
    fn test_is_https_with_forwarded_proto() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Forwarded-Proto", HeaderValue::from_static("https"));

        assert!(
            is_https(&headers),
            "Should detect HTTPS from X-Forwarded-Proto"
        );
    }

    #[test]
    fn test_is_https_with_forwarded_proto_case_insensitive() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Forwarded-Proto", HeaderValue::from_static("HTTPS"));

        assert!(is_https(&headers), "Should detect HTTPS case-insensitively");
    }

    #[test]
    fn test_is_https_with_forwarded_ssl() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Forwarded-Ssl", HeaderValue::from_static("on"));

        assert!(
            is_https(&headers),
            "Should detect HTTPS from X-Forwarded-Ssl"
        );
    }

    #[test]
    fn test_is_https_with_forwarded_ssl_case_insensitive() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Forwarded-Ssl", HeaderValue::from_static("ON"));

        assert!(is_https(&headers), "Should detect HTTPS case-insensitively");
    }

    #[test]
    fn test_is_https_with_http() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Forwarded-Proto", HeaderValue::from_static("http"));

        assert!(!is_https(&headers), "Should not detect HTTPS for HTTP");
    }

    #[test]
    fn test_is_https_with_no_headers() {
        let headers = HeaderMap::new();

        assert!(
            !is_https(&headers),
            "Should not detect HTTPS without headers"
        );
    }

    #[test]
    fn test_is_https_prefers_forwarded_proto() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Forwarded-Proto", HeaderValue::from_static("https"));
        headers.insert("X-Forwarded-Ssl", HeaderValue::from_static("off"));

        assert!(
            is_https(&headers),
            "Should prefer X-Forwarded-Proto over X-Forwarded-Ssl"
        );
    }

    #[test]
    fn test_create_session_cookie_with_https() {
        let token = "test-token-123";
        let cookie = create_session_cookie(token, true, false);
        let cookie_str = cookie.to_str().unwrap();

        assert!(
            cookie_str.contains("session_token=test-token-123"),
            "Should contain token"
        );
        assert!(cookie_str.contains("HttpOnly"), "Should be HttpOnly");
        assert!(
            cookie_str.contains("SameSite=Strict"),
            "Should have SameSite=Strict"
        );
        assert!(
            cookie_str.contains("Secure"),
            "Should have Secure flag for HTTPS"
        );
        assert!(cookie_str.contains("Path=/"), "Should have Path=/");
        assert!(
            cookie_str.contains("Max-Age=2592000"),
            "Should have 30-day expiry"
        );
    }

    #[test]
    fn test_create_session_cookie_without_https() {
        let token = "test-token-456";
        let cookie = create_session_cookie(token, false, false);
        let cookie_str = cookie.to_str().unwrap();

        assert!(
            cookie_str.contains("session_token=test-token-456"),
            "Should contain token"
        );
        assert!(cookie_str.contains("HttpOnly"), "Should be HttpOnly");
        assert!(
            cookie_str.contains("SameSite=Strict"),
            "Should have SameSite=Strict"
        );
        assert!(
            !cookie_str.contains("Secure"),
            "Should not have Secure flag for HTTP"
        );
        assert!(cookie_str.contains("Path=/"), "Should have Path=/");
        assert!(
            cookie_str.contains("Max-Age=2592000"),
            "Should have 30-day expiry"
        );
    }

    #[test]
    fn test_create_session_cookie_production_forces_secure() {
        // Even without HTTPS detected, production must set Secure.
        let cookie = create_session_cookie("prod-token", false, true);
        let cookie_str = cookie.to_str().unwrap();
        assert!(
            cookie_str.contains("Secure"),
            "Production cookie must always be Secure"
        );
    }

    #[test]
    fn test_create_session_cookie_format() {
        let token = "abc123";
        let cookie = create_session_cookie(token, true, false);
        let cookie_str = cookie.to_str().unwrap();

        // Verify the cookie follows the expected format
        let expected_parts = vec![
            "session_token=abc123",
            "HttpOnly",
            "SameSite=Strict",
            "Path=/",
            "Max-Age=2592000",
            "Secure",
        ];

        for part in expected_parts {
            assert!(
                cookie_str.contains(part),
                "Cookie should contain '{}', got: {}",
                part,
                cookie_str
            );
        }
    }
}

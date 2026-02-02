use axum::http::HeaderMap;
use fido_server::security::{is_https, create_session_cookie};

#[test]
fn test_https_detection_integration() {
    // Test with X-Forwarded-Proto header (Fly.io uses this)
    let mut headers = HeaderMap::new();
    headers.insert("X-Forwarded-Proto", "https".parse().unwrap());
    
    assert!(is_https(&headers), "Should detect HTTPS from X-Forwarded-Proto");
    
    // Test cookie creation with HTTPS
    let token = "test-session-token";
    let cookie = create_session_cookie(token, true);
    let cookie_str = cookie.to_str().unwrap();
    
    assert!(cookie_str.contains("Secure"), "Cookie should have Secure flag for HTTPS");
    assert!(cookie_str.contains("HttpOnly"), "Cookie should have HttpOnly flag");
    assert!(cookie_str.contains("SameSite=Strict"), "Cookie should have SameSite=Strict");
}

#[test]
fn test_http_cookie_without_secure_flag() {
    // Test without HTTPS headers
    let headers = HeaderMap::new();
    
    assert!(!is_https(&headers), "Should not detect HTTPS without headers");
    
    // Test cookie creation without HTTPS
    let token = "test-session-token";
    let cookie = create_session_cookie(token, false);
    let cookie_str = cookie.to_str().unwrap();
    
    assert!(!cookie_str.contains("Secure"), "Cookie should not have Secure flag for HTTP");
    assert!(cookie_str.contains("HttpOnly"), "Cookie should still have HttpOnly flag");
    assert!(cookie_str.contains("SameSite=Strict"), "Cookie should still have SameSite=Strict");
}

#[test]
fn test_fly_io_https_headers() {
    // Test with X-Forwarded-Ssl header (alternative header some proxies use)
    let mut headers = HeaderMap::new();
    headers.insert("X-Forwarded-Ssl", "on".parse().unwrap());
    
    assert!(is_https(&headers), "Should detect HTTPS from X-Forwarded-Ssl");
}

#[test]
fn test_cookie_attributes() {
    let token = "abc123xyz";
    let cookie = create_session_cookie(token, true);
    let cookie_str = cookie.to_str().unwrap();
    
    // Verify all required attributes are present
    assert!(cookie_str.contains("session_token=abc123xyz"), "Should contain token value");
    assert!(cookie_str.contains("HttpOnly"), "Should be HttpOnly");
    assert!(cookie_str.contains("SameSite=Strict"), "Should have SameSite=Strict");
    assert!(cookie_str.contains("Path=/"), "Should have Path=/");
    assert!(cookie_str.contains("Max-Age=604800"), "Should have 7-day expiry (604800 seconds)");
    assert!(cookie_str.contains("Secure"), "Should have Secure flag");
}

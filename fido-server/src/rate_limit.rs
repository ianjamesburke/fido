use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Simple in-memory rate limiter
/// Tracks requests per session token with a sliding window
#[derive(Clone)]
pub struct RateLimiter {
    // Map of session_token -> (request_count, window_start)
    state: Arc<Mutex<HashMap<String, (u32, Instant)>>>,
    max_requests: u32,
    window_duration: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window_seconds: u64) -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window_duration: Duration::from_secs(window_seconds),
        }
    }

    /// Check if a request should be allowed
    pub fn check_rate_limit(&self, token: &str) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        let now = Instant::now();

        // Clean up old entries periodically (simple cleanup)
        if state.len() > 10000 {
            state.retain(|_, (_, start)| now.duration_since(*start) < self.window_duration * 2);
        }

        match state.get_mut(token) {
            Some((count, window_start)) => {
                // Check if we're still in the same window
                if now.duration_since(*window_start) < self.window_duration {
                    if *count >= self.max_requests {
                        let remaining = self.window_duration - now.duration_since(*window_start);
                        return Err(format!(
                            "Rate limit exceeded. Try again in {} seconds.",
                            remaining.as_secs()
                        ));
                    }
                    *count += 1;
                } else {
                    // New window
                    *window_start = now;
                    *count = 1;
                }
            }
            None => {
                // First request from this token
                state.insert(token.to_string(), (1, now));
            }
        }

        Ok(())
    }
}

/// Middleware to apply rate limiting to all requests
pub async fn rate_limit_middleware(
    axum::Extension(limiter): axum::Extension<RateLimiter>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract session token from headers
    let token = request
        .headers()
        .get("X-Session-Token")
        .and_then(|v| v.to_str().ok());

    // Only rate limit authenticated requests
    if let Some(token) = token {
        if let Err(msg) = limiter.check_rate_limit(token) {
            return Ok((
                StatusCode::TOO_MANY_REQUESTS,
                format!("{{\"error\": \"{}\"}}", msg),
            )
                .into_response());
        }
    }

    Ok(next.run(request).await)
}

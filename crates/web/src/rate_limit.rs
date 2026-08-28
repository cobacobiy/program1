use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::{
    body::Body,
    extract::Request,
    http::{header, HeaderName, HeaderValue, Response, StatusCode},
    middleware::Next,
    response::IntoResponse,
    Json,
};
use serde_json::json;

#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    pub limit: usize,
    pub remaining: usize,
    pub reset_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct RateLimitRejection {
    pub limit: usize,
    pub retry_after_seconds: u64,
}

pub type RateLimitStore = Arc<Mutex<HashMap<(String, String), Vec<Instant>>>>;

/// Thread-safe in-memory sliding window rate limiter
#[derive(Clone, Default)]
pub struct IpRateLimiter {
    // Map of (ip, route_key) -> list of request timestamps
    records: RateLimitStore,
}

impl IpRateLimiter {
    pub fn new() -> Self {
        Self {
            records: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check and record an incoming request for a given IP and route key
    pub fn check(
        &self,
        ip: &str,
        route_key: &str,
        max_requests: usize,
        window: Duration,
    ) -> Result<RateLimitInfo, RateLimitRejection> {
        let mut map = self.records.lock().unwrap();
        let now = Instant::now();
        let cutoff = now.checked_sub(window).unwrap_or(now);

        let key = (ip.to_string(), route_key.to_string());
        let timestamps = map.entry(key).or_default();


        // Retain only timestamps within the sliding window
        timestamps.retain(|&t| t > cutoff);

        if timestamps.len() >= max_requests {
            let oldest = timestamps.first().copied().unwrap_or(now);
            let elapsed = now.duration_since(oldest);
            let retry_after = if window > elapsed {
                (window - elapsed).as_secs().max(1)
            } else {
                1
            };

            return Err(RateLimitRejection {
                limit: max_requests,
                retry_after_seconds: retry_after,
            });
        }

        timestamps.push(now);
        let remaining = max_requests.saturating_sub(timestamps.len());
        let reset_seconds = window.as_secs();

        Ok(RateLimitInfo {
            limit: max_requests,
            remaining,
            reset_seconds,
        })
    }

    /// Reset limiter state (useful for tests)
    pub fn clear(&self) {
        let mut map = self.records.lock().unwrap();
        map.clear();
    }
}

/// Helper function to extract real client IP address
pub fn extract_client_ip(req: &Request<Body>) -> String {
    // Check X-Forwarded-For first (format: client, proxy1, proxy2)
    if let Some(forwarded) = req.headers().get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        if let Some(first_ip) = forwarded.split(',').next() {
            let clean = first_ip.trim();
            if !clean.is_empty() {
                return clean.to_string();
            }
        }
    }

    // Check X-Real-IP next
    if let Some(real_ip) = req.headers().get("x-real-ip").and_then(|v| v.to_str().ok()) {
        let clean = real_ip.trim();
        if !clean.is_empty() {
            return clean.to_string();
        }
    }

    // Fallback default IP
    "127.0.0.1".to_string()
}

/// Helper to create a rate-limiting middleware for Axum routes
pub async fn rate_limit_layer(
    limiter: Arc<IpRateLimiter>,
    route_key: &'static str,
    max_requests: usize,
    window: Duration,
    req: Request<Body>,
    next: Next,
) -> Response<Body> {
    let client_ip = extract_client_ip(&req);

    match limiter.check(&client_ip, route_key, max_requests, window) {
        Ok(info) => {
            let mut response = next.run(req).await;
            let headers = response.headers_mut();

            let _ = HeaderValue::from_str(&info.limit.to_string())
                .map(|v| headers.insert(HeaderName::from_static("x-ratelimit-limit"), v));
            let _ = HeaderValue::from_str(&info.remaining.to_string())
                .map(|v| headers.insert(HeaderName::from_static("x-ratelimit-remaining"), v));
            let _ = HeaderValue::from_str(&info.reset_seconds.to_string())
                .map(|v| headers.insert(HeaderName::from_static("x-ratelimit-reset"), v));

            response
        }
        Err(rejection) => {
            let json_body = json!({
                "error": "rate_limit_exceeded",
                "message": format!("Too many requests. Please try again in {} seconds.", rejection.retry_after_seconds),
                "retry_after": rejection.retry_after_seconds
            });

            let mut response = (StatusCode::TOO_MANY_REQUESTS, Json(json_body)).into_response();
            let headers = response.headers_mut();

            let _ = HeaderValue::from_str(&rejection.retry_after_seconds.to_string())
                .map(|v| headers.insert(header::RETRY_AFTER, v));
            let _ = HeaderValue::from_str(&rejection.limit.to_string())
                .map(|v| headers.insert(HeaderName::from_static("x-ratelimit-limit"), v));
            let _ = HeaderValue::from_str("0")
                .map(|v| headers.insert(HeaderName::from_static("x-ratelimit-remaining"), v));
            let _ = HeaderValue::from_str(&rejection.retry_after_seconds.to_string())
                .map(|v| headers.insert(HeaderName::from_static("x-ratelimit-reset"), v));

            response
        }
    }
}

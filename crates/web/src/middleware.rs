use std::env;
use std::time::{Duration, Instant};

use axum::{
    extract::{FromRequestParts, Request, State},
    http::{header, request::Parts, HeaderName, HeaderValue, Method, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use program1_contracts::JwtClaims;
use serde_json::json;
use tower_http::cors::{AllowOrigin, CorsLayer};
use uuid::Uuid;

use crate::AppState;

/// Request ID extension container for request tracing
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

/// Middleware to extract or generate unique X-Request-ID and log request metrics
pub async fn request_id_middleware(mut req: Request, next: Next) -> Response {
    let start_time = Instant::now();
    let method = req.method().to_string();
    let path = req.uri().path().to_string();

    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    req.extensions_mut().insert(RequestId(request_id.clone()));

    let mut response = next.run(req).await;
    let latency = start_time.elapsed();
    let status = response.status();

    if let Ok(header_val) = HeaderValue::from_str(&request_id) {
        response
            .headers_mut()
            .insert(HeaderName::from_static("x-request-id"), header_val);
    }

    tracing::info!(
        request_id = %request_id,
        method = %method,
        path = %path,
        status = %status.as_u16(),
        latency_ms = %latency.as_millis(),
        "HTTP Request completed"
    );

    response
}

/// Extractor for authenticated user claims
#[derive(Debug, Clone)]
pub struct AuthUser(pub JwtClaims);

#[axum::async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|val| val.to_str().ok());

        let token = match auth_header {
            Some(header_val) if header_val.starts_with("Bearer ") => {
                &header_val["Bearer ".len()..]
            }
            _ => {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(json!({
                        "error": "authentication_required",
                        "message": "Missing or invalid Authorization header"
                    })),
                ));
            }
        };

        match state.auth_contract.validate_token(token) {
            Ok(claims) => Ok(AuthUser(claims)),
            Err(e) => {
                let err_msg = e.to_string();
                if err_msg.to_lowercase().contains("expired") {
                    Err((
                        StatusCode::UNAUTHORIZED,
                        Json(json!({
                            "error": "token_expired",
                            "message": "Your session has expired. Please login again"
                        })),
                    ))
                } else {
                    Err((
                        StatusCode::UNAUTHORIZED,
                        Json(json!({
                            "error": "authentication_required",
                            "message": "Invalid token"
                        })),
                    ))
                }
            }
        }
    }
}

/// Middleware to enforce authentication on protected endpoints
pub async fn require_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|val| val.to_str().ok());

    let token = match auth_header {
        Some(header_val) if header_val.starts_with("Bearer ") => {
            &header_val["Bearer ".len()..]
        }
        _ => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "authentication_required",
                    "message": "Missing or invalid Authorization header"
                })),
            ));
        }
    };

    match state.auth_contract.validate_token(token) {
        Ok(claims) => {
            req.extensions_mut().insert(claims);
            Ok(next.run(req).await)
        }
        Err(e) => {
            let err_msg = e.to_string();
            if err_msg.to_lowercase().contains("expired") {
                Err((
                    StatusCode::UNAUTHORIZED,
                    Json(json!({
                        "error": "token_expired",
                        "message": "Your session has expired. Please login again"
                    })),
                ))
            } else {
                Err((
                    StatusCode::UNAUTHORIZED,
                    Json(json!({
                        "error": "authentication_required",
                        "message": "Missing or invalid Authorization header"
                    })),
                ))
            }
        }
    }
}

/// Middleware to enforce Admin-only RBAC access
pub async fn require_admin(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|val| val.to_str().ok());

    let token = match auth_header {
        Some(header_val) if header_val.starts_with("Bearer ") => {
            &header_val["Bearer ".len()..]
        }
        _ => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "authentication_required",
                    "message": "Missing or invalid Authorization header"
                })),
            ));
        }
    };

    match state.auth_contract.validate_token(token) {
        Ok(claims) => {
            let role = claims.role.to_lowercase();
            if role.contains("admin") {
                req.extensions_mut().insert(claims);
                Ok(next.run(req).await)
            } else {
                Err((
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "error": "insufficient_permissions",
                        "message": "Your role does not have access to this resource"
                    })),
                ))
            }
        }
        Err(e) => {
            let err_msg = e.to_string();
            if err_msg.to_lowercase().contains("expired") {
                Err((
                    StatusCode::UNAUTHORIZED,
                    Json(json!({
                        "error": "token_expired",
                        "message": "Your session has expired. Please login again"
                    })),
                ))
            } else {
                Err((
                    StatusCode::UNAUTHORIZED,
                    Json(json!({
                        "error": "authentication_required",
                        "message": "Missing or invalid Authorization header"
                    })),
                ))
            }
        }
    }
}

/// Middleware to attach hardened security headers
pub async fn security_headers(req: Request, next: Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    // Prevent clickjacking
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));

    // Prevent MIME type sniffing
    headers.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));

    // XSS protection (legacy browsers)
    headers.insert(HeaderName::from_static("x-xss-protection"), HeaderValue::from_static("1; mode=block"));

    // Referrer policy
    headers.insert(header::REFERRER_POLICY, HeaderValue::from_static("strict-origin-when-cross-origin"));

    // API Version header
    headers.insert(
        HeaderName::from_static("x-api-version"),
        HeaderValue::from_static("1.0.0"),
    );

    // Content Security Policy
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com; img-src 'self' https: data:; connect-src 'self'"
        ),
    );

    // Strict Transport Security (HSTS for production / HTTPS)
    if env::var("APP_ENV").unwrap_or_default().to_lowercase() == "production" {
        headers.insert(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );
    }

    response
}

/// Build hardened CORS configuration layer
pub fn build_cors_layer() -> CorsLayer {
    let default_origins = "http://localhost:8080,http://localhost:6090,http://localhost:3000,http://127.0.0.1:8080,http://127.0.0.1:6090,http://127.0.0.1:3000".to_string();
    let allowed_origins_str = env::var("ALLOWED_ORIGINS").unwrap_or(default_origins);

    let origins: Vec<HeaderValue> = allowed_origins_str
        .split(',')
        .filter_map(|o| HeaderValue::from_str(o.trim()).ok())
        .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::ACCEPT,
            HeaderName::from_static("x-requested-with"),
            HeaderName::from_static("x-request-id"),
        ])
        .allow_credentials(true)
        .max_age(Duration::from_secs(3600))
}

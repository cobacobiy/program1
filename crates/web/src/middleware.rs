use axum::{
    extract::{FromRequestParts, Request, State},
    http::{header, request::Parts, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use program1_contracts::JwtClaims;
use serde_json::json;

use crate::AppState;

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
                            "message": "Missing or invalid Authorization header"
                        })),
                    ))
                }
            }
        }
    }
}

/// Middleware to require valid JWT authentication
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

/// Middleware to require admin access (Super Admin or Admin)
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

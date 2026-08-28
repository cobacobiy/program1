use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::{AppState, ValidatedJson};
use program1_contracts::{
    AuditLogEntry, AuthTokenResponse, LoginRequest, RegisterUserRequest, UserAccountDto,
};

/// Authenticate user credentials and issue JWT bearer token
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = AuthTokenResponse),
        (status = 400, description = "Bad credentials or validation error", body = ApiError),
        (status = 429, description = "Rate limit exceeded")
    ),
    tag = "Auth"
)]
pub async fn login_handler(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<LoginRequest>,
) -> Result<Json<AuthTokenResponse>, ApiError> {
    match state.user_contract.authenticate(&payload.username, &payload.password).await {
        Ok(user) => {
            let token = state.auth_contract.generate_token(&user)?;

            let _ = state.audit_contract.log_action(AuditLogEntry {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                actor_id: Some(user.id),
                actor_username: user.username.clone(),
                action: "LOGIN_SUCCESS".to_string(),
                resource_type: "user".to_string(),
                resource_id: Some(user.id),
                details: json!({ "role": user.role }).to_string(),
                ip_address: None,
            }).await;

            let token_resp = AuthTokenResponse {
                access_token: token,
                token_type: "Bearer".to_string(),
                expires_in: 86400,
                user,
            };
            Ok(Json(token_resp))
        }
        Err(e) => {
            let _ = state.audit_contract.log_action(AuditLogEntry {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                actor_id: None,
                actor_username: payload.username.clone(),
                action: "LOGIN_FAILED".to_string(),
                resource_type: "user".to_string(),
                resource_id: None,
                details: json!({ "attempted_username": payload.username }).to_string(),
                ip_address: None,
            }).await;

            Err(ApiError::from(e))
        }
    }
}

/// Register a new system user account (Admin only)
#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    request_body = RegisterUserRequest,
    responses(
        (status = 201, description = "User successfully registered", body = UserAccountDto),
        (status = 400, description = "Username duplicate or validation error", body = ApiError),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Admin role required")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Auth"
)]
pub async fn register_handler(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<RegisterUserRequest>,
) -> Result<(StatusCode, Json<UserAccountDto>), ApiError> {
    let user = state.user_contract.register(payload).await?;

    let _ = state.audit_contract.log_action(AuditLogEntry {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        actor_id: Some(user.id),
        actor_username: user.username.clone(),
        action: "USER_REGISTERED".to_string(),
        resource_type: "user".to_string(),
        resource_id: Some(user.id),
        details: json!({ "role": user.role }).to_string(),
        ip_address: None,
    }).await;

    Ok((StatusCode::CREATED, Json(user)))
}

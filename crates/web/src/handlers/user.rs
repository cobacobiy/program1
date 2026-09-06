use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::{AppState, ValidatedJson};
use program1_contracts::{
    ActivateBreakGlassRequest, AuditLogEntry, BreakGlassStatusDto, CreateUserAccountRequest,
    DeactivateBreakGlassRequest, JwtClaims, UpdateUserPermissionsRequest, UserAccountDto,
};

/// List all registered user accounts with RBAC assignments (Admin only)
#[utoipa::path(
    get,
    path = "/api/v1/users/accounts",
    responses(
        (status = 200, description = "List of user accounts", body = Vec<UserAccountDto>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Users"
)]
pub async fn list_user_accounts(
    State(state): State<AppState>,
) -> Result<Json<Vec<UserAccountDto>>, ApiError> {
    let accounts = state.user_contract.list_accounts().await?;
    Ok(Json(accounts))
}

/// Create a new user account with specified RBAC role and menu access (Admin only)
#[utoipa::path(
    post,
    path = "/api/v1/users/accounts",
    request_body = CreateUserAccountRequest,
    responses(
        (status = 201, description = "User account created", body = UserAccountDto),
        (status = 400, description = "Validation error", body = ApiError),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Users"
)]
pub async fn create_user_account(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<CreateUserAccountRequest>,
) -> Result<(StatusCode, Json<UserAccountDto>), ApiError> {
    let acc = state.user_contract.create_account(payload).await?;

    let _ = state
        .audit_contract
        .log_action(AuditLogEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor_id: Some(acc.id),
            actor_username: acc.username.clone(),
            action: "USER_CREATED".to_string(),
            resource_type: "user".to_string(),
            resource_id: Some(acc.id),
            details: json!({ "role": acc.role }).to_string(),
            ip_address: None,
        })
        .await;

    Ok((StatusCode::CREATED, Json(acc)))
}

/// Update accessible menu permissions for a specific user (Admin only)
#[utoipa::path(
    post,
    path = "/api/v1/users/accounts/{id}/permissions",
    params(
        ("id" = Uuid, Path, description = "Target user account ID")
    ),
    request_body = UpdateUserPermissionsRequest,
    responses(
        (status = 200, description = "User permissions updated", body = UserAccountDto),
        (status = 404, description = "User not found", body = ApiError),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Users"
)]
pub async fn update_user_permissions(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Json(payload): Json<UpdateUserPermissionsRequest>,
) -> Result<Json<UserAccountDto>, ApiError> {
    let acc = state
        .user_contract
        .update_permissions(id, payload.accessible_menus.clone())
        .await?;

    let _ = state
        .audit_contract
        .log_action(AuditLogEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor_id: Some(id),
            actor_username: acc.username.clone(),
            action: "PERMISSIONS_UPDATED".to_string(),
            resource_type: "user".to_string(),
            resource_id: Some(id),
            details: json!({ "accessible_menus": payload.accessible_menus }).to_string(),
            ip_address: None,
        })
        .await;

    Ok(Json(acc))
}

/// Activate temporary emergency break-glass developer access (Admin/Owner only)
#[utoipa::path(
    post,
    path = "/api/v1/admin/break-glass/activate",
    request_body = ActivateBreakGlassRequest,
    responses(
        (status = 200, description = "Break-glass developer access activated", body = BreakGlassStatusDto),
        (status = 400, description = "Validation error", body = ApiError),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Admin Break-Glass"
)]
pub async fn activate_break_glass(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    ValidatedJson(payload): ValidatedJson<ActivateBreakGlassRequest>,
) -> Result<Json<BreakGlassStatusDto>, ApiError> {
    let status = state
        .user_contract
        .activate_break_glass(payload.clone(), &claims.username)
        .await?;

    state
        .audit_contract
        .log_action(AuditLogEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor_id: Some(claims.sub),
            actor_username: claims.username,
            action: "BREAK_GLASS_ACTIVATED".to_string(),
            resource_type: "break_glass".to_string(),
            resource_id: None,
            details:
                json!({ "reason": payload.reason, "duration_minutes": payload.duration_minutes })
                    .to_string(),
            ip_address: None,
        })
        .await?;

    Ok(Json(status))
}

/// Deactivate emergency break-glass developer access immediately (Admin/Owner only)
#[utoipa::path(
    post,
    path = "/api/v1/admin/break-glass/deactivate",
    responses(
        (status = 200, description = "Break-glass developer access deactivated", body = BreakGlassStatusDto),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Admin Break-Glass"
)]
pub async fn deactivate_break_glass(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    body: Option<Json<DeactivateBreakGlassRequest>>,
) -> Result<Json<BreakGlassStatusDto>, ApiError> {
    let reason = body.and_then(|b| b.0.reason);
    let status = state
        .user_contract
        .deactivate_break_glass(&claims.username, reason.clone())
        .await?;

    state
        .audit_contract
        .log_action(AuditLogEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor_id: Some(claims.sub),
            actor_username: claims.username,
            action: "BREAK_GLASS_DEACTIVATED".to_string(),
            resource_type: "break_glass".to_string(),
            resource_id: None,
            details: json!({ "reason": reason }).to_string(),
            ip_address: None,
        })
        .await?;

    Ok(Json(status))
}

/// Get current emergency break-glass developer access status (Admin/Owner only)
#[utoipa::path(
    get,
    path = "/api/v1/admin/break-glass/status",
    responses(
        (status = 200, description = "Break-glass status", body = BreakGlassStatusDto),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Admin Break-Glass"
)]
pub async fn get_break_glass_status(
    State(state): State<AppState>,
    Extension(_claims): Extension<JwtClaims>,
) -> Result<Json<BreakGlassStatusDto>, ApiError> {
    let status = state.user_contract.get_break_glass_status().await?;
    Ok(Json(status))
}

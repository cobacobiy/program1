use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::{AppState, ValidatedJson};
use program1_contracts::{
    AuditLogEntry, CreateUserAccountRequest, UpdateUserPermissionsRequest, UserAccountDto,
};

pub async fn list_user_accounts(
    State(state): State<AppState>,
) -> Result<Json<Vec<UserAccountDto>>, ApiError> {
    let accounts = state.user_contract.list_accounts().await?;
    Ok(Json(accounts))
}

pub async fn create_user_account(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<CreateUserAccountRequest>,
) -> Result<(StatusCode, Json<UserAccountDto>), ApiError> {
    let acc = state.user_contract.create_account(payload).await?;

    let _ = state.audit_contract.log_action(AuditLogEntry {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        actor_id: Some(acc.id),
        actor_username: acc.username.clone(),
        action: "USER_CREATED".to_string(),
        resource_type: "user".to_string(),
        resource_id: Some(acc.id),
        details: json!({ "role": acc.role }).to_string(),
        ip_address: None,
    }).await;

    Ok((StatusCode::CREATED, Json(acc)))
}

pub async fn update_user_permissions(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Json(payload): Json<UpdateUserPermissionsRequest>,
) -> Result<Json<UserAccountDto>, ApiError> {
    let acc = state
        .user_contract
        .update_permissions(id, payload.accessible_menus.clone())
        .await?;

    let _ = state.audit_contract.log_action(AuditLogEntry {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        actor_id: Some(id),
        actor_username: acc.username.clone(),
        action: "PERMISSIONS_UPDATED".to_string(),
        resource_type: "user".to_string(),
        resource_id: Some(id),
        details: json!({ "accessible_menus": payload.accessible_menus }).to_string(),
        ip_address: None,
    }).await;

    Ok(Json(acc))
}

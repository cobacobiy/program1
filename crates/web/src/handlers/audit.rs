use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;
use program1_contracts::AuditLogEntry;

#[derive(Debug, Deserialize)]
pub struct AuditQueryParam {
    pub resource_type: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

pub async fn list_audit_logs(
    State(state): State<AppState>,
    Query(params): Query<AuditQueryParam>,
) -> Result<Json<Vec<AuditLogEntry>>, ApiError> {
    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);
    let res_type = params.resource_type.as_deref();

    let logs = state.audit_contract.get_logs(res_type, limit, offset).await?;
    Ok(Json(logs))
}

pub async fn get_user_audit_logs(
    Path(user_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<Vec<AuditLogEntry>>, ApiError> {
    let logs = state.audit_contract.get_logs_by_actor(user_id).await?;
    Ok(Json(logs))
}

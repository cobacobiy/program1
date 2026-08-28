use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;
use program1_contracts::AuditLogEntry;

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct AuditQueryParam {
    pub resource_type: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// Query system audit logs with optional resource type filtering and pagination (Admin only)
#[utoipa::path(
    get,
    path = "/api/v1/audit/logs",
    params(
        AuditQueryParam
    ),
    responses(
        (status = 200, description = "List of audit logs", body = Vec<AuditLogEntry>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Audit"
)]
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

/// Retrieve all audit trail records performed by a specific user actor (Admin only)
#[utoipa::path(
    get,
    path = "/api/v1/audit/logs/user/{id}",
    params(
        ("id" = Uuid, Path, description = "Target actor user ID")
    ),
    responses(
        (status = 200, description = "Audit trail for the given actor", body = Vec<AuditLogEntry>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Audit"
)]
pub async fn get_user_audit_logs(
    Path(user_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<Vec<AuditLogEntry>>, ApiError> {
    let logs = state.audit_contract.get_logs_by_actor(user_id).await?;
    Ok(Json(logs))
}

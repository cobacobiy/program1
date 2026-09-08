use axum::{
    extract::{Query, State},
    Json,
};

use crate::error::ApiError;
use crate::state::AppState;
use program1_contracts::{SalesAnalyticsDto, SalesReportQuery, SalesReportResponse};

/// Retrieve sales analytics and channel revenue breakdown (Admin only)
#[utoipa::path(
    get,
    path = "/api/v1/analytics",
    responses(
        (status = 200, description = "Aggregated sales analytics", body = SalesAnalyticsDto),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Analytics"
)]
pub async fn get_analytics(
    State(state): State<AppState>,
) -> Result<Json<SalesAnalyticsDto>, ApiError> {
    let analytics = state.analytics_contract.get_sales_analytics().await?;
    Ok(Json(analytics))
}

/// Retrieve detailed sales report with date range & status filters (Admin only)
#[utoipa::path(
    get,
    path = "/api/v1/analytics/report",
    params(
        ("date_from" = Option<String>, Query, description = "Filter start date (YYYY-MM-DD)"),
        ("date_to" = Option<String>, Query, description = "Filter end date (YYYY-MM-DD)"),
        ("status_filter" = Option<String>, Query, description = "Filter by order status (e.g. delivered, paid, pending, all)")
    ),
    responses(
        (status = 200, description = "Sales report summary and detailed transaction rows", body = SalesReportResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Analytics"
)]
pub async fn get_sales_report_handler(
    State(state): State<AppState>,
    Query(query): Query<SalesReportQuery>,
) -> Result<Json<SalesReportResponse>, ApiError> {
    let report = state
        .analytics_contract
        .generate_sales_report(query)
        .await?;
    Ok(Json(report))
}

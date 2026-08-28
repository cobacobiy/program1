use axum::{
    extract::State,
    Json,
};

use crate::error::ApiError;
use crate::state::AppState;
use program1_contracts::SalesAnalyticsDto;

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

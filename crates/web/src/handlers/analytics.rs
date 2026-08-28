use axum::{
    extract::State,
    Json,
};

use crate::error::ApiError;
use crate::state::AppState;
use program1_contracts::SalesAnalyticsDto;

pub async fn get_analytics(
    State(state): State<AppState>,
) -> Result<Json<SalesAnalyticsDto>, ApiError> {
    let analytics = state.analytics_contract.get_sales_analytics().await?;
    Ok(Json(analytics))
}

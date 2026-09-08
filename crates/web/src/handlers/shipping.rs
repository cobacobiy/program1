use axum::{
    extract::{Query, State},
    Json,
};
use program1_contracts::{
    ShippingCity, ShippingCost, ShippingCostRequest, ShippingCourier,
};
use serde::Deserialize;

use crate::error::ApiError;
use crate::state::{AppState, ValidatedJson};

#[derive(Debug, Deserialize)]
pub struct CitySearchQuery {
    pub q: Option<String>,
}

/// Calculate shipping cost for a given destination, weight, and courier
#[utoipa::path(
    post,
    path = "/api/v1/shipping/cost",
    request_body = ShippingCostRequest,
    responses(
        (status = 200, description = "Shipping cost options", body = Vec<ShippingCost>),
        (status = 400, description = "Validation error", body = ApiError),
        (status = 422, description = "Payload validation failed", body = ApiError)
    ),
    tag = "Shipping"
)]
pub async fn calculate_shipping_handler(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<ShippingCostRequest>,
) -> Result<Json<Vec<ShippingCost>>, ApiError> {
    let costs = state.shipping_contract.calculate_cost(payload).await?;
    Ok(Json(costs))
}

/// List supported domestic couriers
#[utoipa::path(
    get,
    path = "/api/v1/shipping/couriers",
    responses(
        (status = 200, description = "List of supported couriers", body = Vec<ShippingCourier>)
    ),
    tag = "Shipping"
)]
pub async fn list_couriers_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<ShippingCourier>>, ApiError> {
    let couriers = state.shipping_contract.list_couriers().await?;
    Ok(Json(couriers))
}

/// Search cities by name or province for shipping destination autocomplete
#[utoipa::path(
    get,
    path = "/api/v1/shipping/cities",
    params(
        ("q" = Option<String>, Query, description = "Search query for city or province name")
    ),
    responses(
        (status = 200, description = "List of matching cities", body = Vec<ShippingCity>)
    ),
    tag = "Shipping"
)]
pub async fn search_cities_handler(
    State(state): State<AppState>,
    Query(query): Query<CitySearchQuery>,
) -> Result<Json<Vec<ShippingCity>>, ApiError> {
    let search_term = query.q.as_deref().unwrap_or("");
    let cities = state.shipping_contract.search_cities(search_term).await?;
    Ok(Json(cities))
}

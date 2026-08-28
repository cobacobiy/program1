use std::sync::Arc;

use axum::{
    extract::{FromRequest, Request},
    http::StatusCode,
    Json,
};
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::error::ApiError;
use crate::rate_limit::IpRateLimiter;
use program1_contracts::{
    AnalyticsContract, AuditContract, AuthContract, CatalogContract, ChannelSyncContract,
    ErrorCode, InventoryContract, OrderContract, UserContract,
};

#[derive(Clone)]
pub struct AppState {
    pub store_name: String,
    pub store_currency: String,
    pub user_contract: Arc<dyn UserContract>,
    pub auth_contract: Arc<dyn AuthContract>,
    pub catalog_contract: Arc<dyn CatalogContract>,
    pub inventory_contract: Arc<dyn InventoryContract>,
    pub channel_contract: Arc<dyn ChannelSyncContract>,
    pub order_contract: Arc<dyn OrderContract>,
    pub analytics_contract: Arc<dyn AnalyticsContract>,
    pub audit_contract: Arc<dyn AuditContract>,
    pub rate_limiter: Arc<IpRateLimiter>,
    pub started_at: std::time::Instant,
}


/// Custom Axum extractor that parses JSON and automatically runs validation
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedJson<T>(pub T);

#[axum::async_trait]
impl<T> FromRequest<AppState> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &AppState) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|e| ApiError::new(ErrorCode::InvalidRequest, e.to_string(), StatusCode::BAD_REQUEST))?;

        value.validate().map_err(|e| {
            let mut details = Vec::new();
            for (field, err_kind) in e.field_errors() {
                for err in err_kind {
                    let msg = err
                        .message
                        .as_ref()
                        .map(|m| m.to_string())
                        .unwrap_or_else(|| format!("Invalid value for {}", field));
                    details.push(format!("{}: {}", field, msg));
                }
            }

            ApiError::with_details(
                ErrorCode::ValidationFailed,
                "Payload validation failed",
                StatusCode::UNPROCESSABLE_ENTITY,
                details,
            )
        })?;

        Ok(ValidatedJson(value))
    }
}

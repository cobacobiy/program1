use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use program1_contracts::{ContractError, ErrorCode};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: String,
    pub status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Vec<String>>,
}

impl ApiError {
    pub fn new(code: ErrorCode, message: impl Into<String>, status: StatusCode) -> Self {
        Self {
            code,
            message: message.into(),
            status: status.as_u16(),
            details: None,
        }
    }

    pub fn with_details(
        code: ErrorCode,
        message: impl Into<String>,
        status: StatusCode,
        details: Vec<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            status: status.as_u16(),
            details: Some(details),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status_code =
            StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        let mut err_obj = json!({
            "code": self.code.as_str(),
            "message": self.message,
            "status": self.status,
        });

        if let Some(details) = self.details {
            err_obj["details"] = json!(details);
        }

        // Standardized envelope with top-level error object (and backwards-compatible "error" string in root if needed)
        let body = json!({
            "error": err_obj
        });

        (status_code, Json(body)).into_response()
    }
}

impl From<ContractError> for ApiError {
    fn from(err: ContractError) -> Self {
        let code = err.code();
        match err {
            ContractError::NotFound(msg) => ApiError::new(code, msg, StatusCode::NOT_FOUND),
            ContractError::ValidationError(msg) => {
                ApiError::new(code, msg, StatusCode::BAD_REQUEST)
            }
            ContractError::InsufficientStock {
                product_id,
                requested,
                available,
            } => {
                let msg = format!(
                    "Insufficient stock for product {}: requested {}, available {}",
                    product_id, requested, available
                );
                ApiError::new(code, msg, StatusCode::CONFLICT)
            }
            ContractError::ChannelSyncError(msg) => {
                ApiError::new(code, msg, StatusCode::BAD_GATEWAY)
            }
            ContractError::Internal(msg) => {
                tracing::error!(error = %msg, "Internal module error occurred");
                let client_msg = if cfg!(debug_assertions) {
                    msg
                } else {
                    "An internal server error occurred".to_string()
                };
                ApiError::new(code, client_msg, StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contract_error_to_api_error_conversion() {
        let err = ContractError::NotFound("Item 123".to_string());
        let api_err = ApiError::from(err);
        assert_eq!(api_err.code, ErrorCode::ResourceNotFound);
        assert_eq!(api_err.status, 404);
        assert_eq!(api_err.message, "Item 123");
    }

    #[test]
    fn test_validation_error_conversion() {
        let err = ContractError::ValidationError("Price cannot be negative".to_string());
        let api_err = ApiError::from(err);
        assert_eq!(api_err.code, ErrorCode::ValidationFailed);
        assert_eq!(api_err.status, 400);
    }
}

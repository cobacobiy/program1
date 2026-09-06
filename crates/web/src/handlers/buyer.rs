use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use program1_contracts::{
    BuyerAccountDto, BuyerAddressDto, BuyerAuthResponse, CreateBuyerAddressRequest,
    GoogleAuthRequest, JwtClaims, OtpRequest, OtpVerifyRequest, UpdateBuyerAddressRequest,
};
use serde_json::json;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::{AppState, ValidatedJson};

/// Authenticate Buyer using Google OAuth/OIDC ID token
#[utoipa::path(
    post,
    path = "/api/v1/buyer/auth/google",
    request_body = GoogleAuthRequest,
    responses(
        (status = 200, description = "Google authentication successful", body = BuyerAuthResponse),
        (status = 400, description = "Invalid token or verification failed", body = ApiError),
        (status = 429, description = "Rate limit exceeded")
    ),
    tag = "Buyer Auth"
)]
pub async fn google_auth_handler(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<GoogleAuthRequest>,
) -> Result<Json<BuyerAuthResponse>, ApiError> {
    let auth_resp = state
        .buyer_contract
        .authenticate_google(&payload.id_token)
        .await?;
    Ok(Json(auth_resp))
}

/// Request SMS OTP code for buyer phone number verification
#[utoipa::path(
    post,
    path = "/api/v1/buyer/otp/request",
    request_body = OtpRequest,
    responses(
        (status = 200, description = "OTP sent successfully"),
        (status = 400, description = "Invalid phone number or cooldown active", body = ApiError),
        (status = 401, description = "Unauthorized"),
        (status = 429, description = "Rate limit exceeded")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Buyer Auth"
)]
pub async fn request_otp_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    ValidatedJson(payload): ValidatedJson<OtpRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .buyer_contract
        .request_phone_otp(claims.sub, &payload.phone_number)
        .await?;
    Ok(Json(json!({
        "status": "success",
        "message": "Kode OTP berhasil dikirim melalui SMS/WhatsApp. Berlaku 5 menit."
    })))
}

/// Verify phone number OTP code
#[utoipa::path(
    post,
    path = "/api/v1/buyer/otp/verify",
    request_body = OtpVerifyRequest,
    responses(
        (status = 200, description = "Phone number verified", body = BuyerAccountDto),
        (status = 400, description = "Invalid OTP code or expired", body = ApiError),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Buyer Auth"
)]
pub async fn verify_otp_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    ValidatedJson(payload): ValidatedJson<OtpVerifyRequest>,
) -> Result<Json<BuyerAccountDto>, ApiError> {
    let updated = state
        .buyer_contract
        .verify_phone_otp(claims.sub, &payload.phone_number, &payload.code)
        .await?;
    Ok(Json(updated))
}

/// Get authenticated buyer profile
#[utoipa::path(
    get,
    path = "/api/v1/buyer/profile",
    responses(
        (status = 200, description = "Buyer profile", body = BuyerAccountDto),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Buyer not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Buyer Profile"
)]
pub async fn get_buyer_profile_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
) -> Result<Json<BuyerAccountDto>, ApiError> {
    let buyer = state.buyer_contract.get_buyer_profile(claims.sub).await?;
    Ok(Json(buyer))
}

/// List all shipping addresses for authenticated buyer
#[utoipa::path(
    get,
    path = "/api/v1/buyer/addresses",
    responses(
        (status = 200, description = "List of shipping addresses", body = Vec<BuyerAddressDto>),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Buyer Addresses"
)]
pub async fn list_addresses_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
) -> Result<Json<Vec<BuyerAddressDto>>, ApiError> {
    let addresses = state.buyer_contract.list_addresses(claims.sub).await?;
    Ok(Json(addresses))
}

/// Add a new shipping address
#[utoipa::path(
    post,
    path = "/api/v1/buyer/addresses",
    request_body = CreateBuyerAddressRequest,
    responses(
        (status = 201, description = "Address created", body = BuyerAddressDto),
        (status = 400, description = "Validation error", body = ApiError),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Buyer Addresses"
)]
pub async fn create_address_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    ValidatedJson(payload): ValidatedJson<CreateBuyerAddressRequest>,
) -> Result<(StatusCode, Json<BuyerAddressDto>), ApiError> {
    let addr = state
        .buyer_contract
        .create_address(claims.sub, payload)
        .await?;
    Ok((StatusCode::CREATED, Json(addr)))
}

/// Update an existing shipping address
#[utoipa::path(
    put,
    path = "/api/v1/buyer/addresses/{id}",
    params(
        ("id" = Uuid, Path, description = "Address ID")
    ),
    request_body = UpdateBuyerAddressRequest,
    responses(
        (status = 200, description = "Address updated", body = BuyerAddressDto),
        (status = 404, description = "Address not found", body = ApiError),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Buyer Addresses"
)]
pub async fn update_address_handler(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    ValidatedJson(payload): ValidatedJson<UpdateBuyerAddressRequest>,
) -> Result<Json<BuyerAddressDto>, ApiError> {
    let addr = state
        .buyer_contract
        .update_address(claims.sub, id, payload)
        .await?;
    Ok(Json(addr))
}

/// Delete a shipping address
#[utoipa::path(
    delete,
    path = "/api/v1/buyer/addresses/{id}",
    params(
        ("id" = Uuid, Path, description = "Address ID")
    ),
    responses(
        (status = 204, description = "Address deleted"),
        (status = 404, description = "Address not found", body = ApiError),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Buyer Addresses"
)]
pub async fn delete_address_handler(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
) -> Result<StatusCode, ApiError> {
    state.buyer_contract.delete_address(claims.sub, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Set a shipping address as default
#[utoipa::path(
    post,
    path = "/api/v1/buyer/addresses/{id}/default",
    params(
        ("id" = Uuid, Path, description = "Address ID")
    ),
    responses(
        (status = 200, description = "Address set as default", body = BuyerAddressDto),
        (status = 404, description = "Address not found", body = ApiError),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Buyer Addresses"
)]
pub async fn set_default_address_handler(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
) -> Result<Json<BuyerAddressDto>, ApiError> {
    let addr = state
        .buyer_contract
        .set_default_address(claims.sub, id)
        .await?;
    Ok(Json(addr))
}

/// Retrieve public Buyer Auth configuration (Google Client ID)
#[utoipa::path(
    get,
    path = "/api/v1/buyer/auth/config",
    responses(
        (status = 200, description = "Buyer auth configuration", body = serde_json::Value)
    ),
    tag = "Buyer Auth"
)]
pub async fn get_buyer_auth_config_handler(
    State(state): State<AppState>,
) -> impl axum::response::IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "google_client_id": state.google_client_id,
        })),
    )
}

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use program1_contracts::{
    AuditLogEntry, BuyerAccountDto, BuyerAddressDto, BuyerAuthResponse, BuyerLoginRequest,
    CreateBuyerAddressRequest, ErrorCode, GoogleAuthRequest, JwtClaims, OtpRequest,
    OtpVerifyRequest, PaginatedResponse, PaginationParams, RegisterBuyerRequest,
    UpdateBuyerAddressRequest, UpdateBuyerProfileRequest, UpdateBuyerStatusRequest,
};
use serde_json::json;
use uuid::Uuid;
use validator::Validate;

use crate::error::ApiError;
use crate::state::{AppState, ValidatedJson};

/// Register a new Buyer account with Email & Password
#[utoipa::path(
    post,
    path = "/api/v1/buyer/auth/register",
    request_body = RegisterBuyerRequest,
    responses(
        (status = 201, description = "Buyer registration successful", body = BuyerAuthResponse),
        (status = 400, description = "Validation error", body = ApiError),
        (status = 409, description = "Email already registered", body = ApiError),
        (status = 429, description = "Rate limit exceeded")
    ),
    tag = "Buyer Auth"
)]
pub async fn buyer_register_handler(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<RegisterBuyerRequest>,
) -> Result<(StatusCode, Json<BuyerAuthResponse>), ApiError> {
    let auth_resp = state.buyer_contract.register(payload).await?;
    Ok((StatusCode::CREATED, Json(auth_resp)))
}

/// Login Buyer account with Email & Password
#[utoipa::path(
    post,
    path = "/api/v1/buyer/auth/login",
    request_body = BuyerLoginRequest,
    responses(
        (status = 200, description = "Buyer login successful", body = BuyerAuthResponse),
        (status = 400, description = "Validation error", body = ApiError),
        (status = 401, description = "Invalid email or password", body = ApiError),
        (status = 429, description = "Rate limit exceeded")
    ),
    tag = "Buyer Auth"
)]
pub async fn buyer_login_handler(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<BuyerLoginRequest>,
) -> Result<Json<BuyerAuthResponse>, ApiError> {
    let auth_resp = state.buyer_contract.login(payload).await?;
    Ok(Json(auth_resp))
}

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
    let dev_code = state
        .buyer_contract
        .request_phone_otp(claims.sub, &payload.phone_number)
        .await?;

    let mut resp = json!({
        "status": "success",
        "message": "Kode OTP berhasil dikirim melalui SMS/WhatsApp. Berlaku 5 menit."
    });

    if let Some(code) = dev_code {
        resp["dev_otp"] = serde_json::Value::String(code);
        resp["is_simulation"] = serde_json::Value::Bool(true);
    }

    Ok(Json(resp))
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

/// Update authenticated buyer profile
#[utoipa::path(
    put,
    path = "/api/v1/buyer/profile",
    request_body = UpdateBuyerProfileRequest,
    responses(
        (status = 200, description = "Buyer profile updated", body = BuyerAccountDto),
        (status = 400, description = "Validation error", body = ApiError),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Buyer not found", body = ApiError)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Buyer Profile"
)]
pub async fn update_buyer_profile_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    ValidatedJson(payload): ValidatedJson<UpdateBuyerProfileRequest>,
) -> Result<Json<BuyerAccountDto>, ApiError> {
    if !claims.is_buyer() {
        return Err(ApiError::new(
            ErrorCode::InsufficientPermissions,
            "Akses perbarui profil pembeli hanya untuk akun buyer",
            StatusCode::FORBIDDEN,
        ));
    }

    let updated = state
        .buyer_contract
        .update_buyer_profile(claims.sub, payload.full_name, payload.avatar_url)
        .await?;
    Ok(Json(updated))
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

/// List all registered buyers with pagination and search (Admin only)
#[utoipa::path(
    get,
    path = "/api/v1/admin/buyers",
    params(
        ("page" = Option<i64>, Query, description = "Page number (min: 1)"),
        ("page_size" = Option<i64>, Query, description = "Page size (1-100, default: 20)"),
        ("search" = Option<String>, Query, description = "Search by email, name, or phone")
    ),
    responses(
        (status = 200, description = "Paginated list of all buyers", body = PaginatedResponse<BuyerAccountDto>),
        (status = 400, description = "Validation error", body = ApiError),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Admin Buyers"
)]
pub async fn admin_list_buyers_handler(
    Query(params): Query<PaginationParams>,
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
) -> Result<Json<PaginatedResponse<BuyerAccountDto>>, ApiError> {
    if !claims.is_seller_staff() {
        return Err(ApiError::new(
            ErrorCode::InsufficientPermissions,
            "Hanya staf/penjual yang dapat mengakses direktori pelanggan",
            StatusCode::FORBIDDEN,
        ));
    }
    params.validate().map_err(|e| {
        ApiError::new(
            ErrorCode::ValidationFailed,
            format!("Invalid pagination parameters: {}", e),
            StatusCode::BAD_REQUEST,
        )
    })?;

    let buyers = state
        .buyer_contract
        .list_buyers_paginated(
            params.page(),
            params.page_size(),
            params.search.as_deref(),
        )
        .await?;
    Ok(Json(buyers))
}

/// Set buyer active status (Admin only)
#[utoipa::path(
    patch,
    path = "/api/v1/admin/buyers/{id}/status",
    request_body = UpdateBuyerStatusRequest,
    params(
        ("id" = Uuid, Path, description = "Buyer ID")
    ),
    responses(
        (status = 200, description = "Buyer status updated", body = BuyerAccountDto),
        (status = 404, description = "Buyer not found"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Admin Buyers"
)]
pub async fn admin_set_buyer_status_handler(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    ValidatedJson(payload): ValidatedJson<UpdateBuyerStatusRequest>,
) -> Result<Json<BuyerAccountDto>, ApiError> {
    if !claims.is_seller_staff() {
        return Err(ApiError::new(
            ErrorCode::InsufficientPermissions,
            "Hanya staf/penjual yang dapat mengubah status akun pelanggan",
            StatusCode::FORBIDDEN,
        ));
    }
    let buyer = state
        .buyer_contract
        .set_buyer_active_status(id, payload.is_active)
        .await?;
    Ok(Json(buyer))
}

/// List recent buyer activities (Admin only)
#[utoipa::path(
    get,
    path = "/api/v1/admin/buyers/activity",
    responses(
        (status = 200, description = "List of buyer activity logs", body = Vec<AuditLogEntry>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Admin Buyers"
)]
pub async fn admin_list_buyer_activity_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
) -> Result<Json<Vec<AuditLogEntry>>, ApiError> {
    if !claims.is_seller_staff() {
        return Err(ApiError::new(
            ErrorCode::InsufficientPermissions,
            "Hanya staf/penjual yang dapat melihat log aktivitas pelanggan",
            StatusCode::FORBIDDEN,
        ));
    }
    let logs = state.audit_contract.get_logs(Some("buyer"), 100, 0).await?;
    Ok(Json(logs))
}

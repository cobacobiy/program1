use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::{AppState, ValidatedJson};
use program1_contracts::{
    CreateReviewRequest, ErrorCode, JwtClaims, PaginatedResponse, PaginationParams,
    ProductRatingSummaryDto, ProductReviewDto, PublicReviewDto, UpdateReviewVisibilityRequest,
};

use validator::Validate;

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AdminReviewQuery {
    pub product_id: Option<Uuid>,
    pub is_visible: Option<bool>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// Create a product review for a delivered order (Buyer)
#[utoipa::path(
    post,
    path = "/api/v1/buyer/reviews",
    request_body = CreateReviewRequest,
    responses(
        (status = 201, description = "Review submitted successfully", body = ProductReviewDto),
        (status = 400, description = "Validation error", body = ApiError),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden (only buyers can review)", body = ApiError),
        (status = 404, description = "Order or product not found", body = ApiError),
        (status = 409, description = "Conflict (duplicate review or invalid status)", body = ApiError)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Reviews"
)]
pub async fn create_review_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    ValidatedJson(payload): ValidatedJson<CreateReviewRequest>,
) -> Result<(StatusCode, Json<ProductReviewDto>), ApiError> {
    if !claims.is_buyer() {
        return Err(ApiError::new(
            ErrorCode::InsufficientPermissions,
            "Hanya akun pembeli yang dapat memberikan ulasan produk",
            StatusCode::FORBIDDEN,
        ));
    }

    let review = state
        .review_contract
        .create_review(claims.sub, payload)
        .await?;

    Ok((StatusCode::CREATED, Json(review)))
}

/// List visible reviews for a product with pagination (Public)
#[utoipa::path(
    get,
    path = "/api/v1/catalog/{id}/reviews",
    params(
        ("id" = Uuid, Path, description = "Product identifier"),
        ("page" = Option<i64>, Query, description = "Page number (min: 1)"),
        ("page_size" = Option<i64>, Query, description = "Page size (1-100, default: 20)")
    ),
    responses(
        (status = 200, description = "List of product reviews", body = PaginatedResponse<PublicReviewDto>),
        (status = 400, description = "Validation error", body = ApiError)
    ),
    tag = "Reviews"
)]
pub async fn get_product_reviews_handler(
    Path(product_id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
    State(state): State<AppState>,
) -> Result<Json<PaginatedResponse<PublicReviewDto>>, ApiError> {
    params.validate().map_err(|e| {
        ApiError::new(
            ErrorCode::ValidationFailed,
            format!("Invalid pagination parameters: {}", e),
            StatusCode::BAD_REQUEST,
        )
    })?;

    let reviews = state
        .review_contract
        .get_reviews_for_product(product_id, params.page(), params.page_size())
        .await?;
    Ok(Json(reviews))
}

/// Get rating summary and star distribution for a product (Public)
#[utoipa::path(
    get,
    path = "/api/v1/catalog/{id}/rating",
    params(
        ("id" = Uuid, Path, description = "Product identifier")
    ),
    responses(
        (status = 200, description = "Product rating summary", body = ProductRatingSummaryDto)
    ),
    tag = "Reviews"
)]
pub async fn get_product_rating_handler(
    Path(product_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<ProductRatingSummaryDto>, ApiError> {
    let summary = state.review_contract.get_rating_summary(product_id).await?;
    Ok(Json(summary))
}

/// List reviews created by currently authenticated buyer
#[utoipa::path(
    get,
    path = "/api/v1/buyer/reviews",
    responses(
        (status = 200, description = "Buyer reviews", body = Vec<ProductReviewDto>),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Reviews"
)]
pub async fn list_buyer_reviews_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
) -> Result<Json<Vec<ProductReviewDto>>, ApiError> {
    let reviews = state.review_contract.list_buyer_reviews(claims.sub).await?;
    Ok(Json(reviews))
}

/// Admin list all product reviews with filters and pagination
#[utoipa::path(
    get,
    path = "/api/v1/admin/reviews",
    params(
        ("product_id" = Option<Uuid>, Query, description = "Filter by product UUID"),
        ("is_visible" = Option<bool>, Query, description = "Filter by visibility (true/false)"),
        ("page" = Option<i64>, Query, description = "Page number (min: 1)"),
        ("page_size" = Option<i64>, Query, description = "Page size (1-100, default: 20)")
    ),
    responses(
        (status = 200, description = "Paginated reviews list", body = PaginatedResponse<ProductReviewDto>),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Reviews"
)]
pub async fn admin_list_reviews_handler(
    State(state): State<AppState>,
    Query(query): Query<AdminReviewQuery>,
) -> Result<Json<PaginatedResponse<ProductReviewDto>>, ApiError> {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    let reviews = state
        .review_contract
        .admin_list_reviews(query.product_id, query.is_visible, page, page_size)
        .await?;

    Ok(Json(reviews))
}

/// Admin toggle or update review visibility
#[utoipa::path(
    patch,
    path = "/api/v1/admin/reviews/{id}/visibility",
    params(
        ("id" = Uuid, Path, description = "Review identifier")
    ),
    request_body = UpdateReviewVisibilityRequest,
    responses(
        (status = 200, description = "Review visibility updated", body = ProductReviewDto),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Review not found", body = ApiError)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Reviews"
)]
pub async fn admin_moderate_review_handler(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Json(payload): Json<UpdateReviewVisibilityRequest>,
) -> Result<Json<ProductReviewDto>, ApiError> {
    let review = state
        .review_contract
        .admin_update_visibility(
            id,
            payload.is_visible,
            Some(claims.sub),
            Some(claims.username),
        )
        .await?;

    Ok(Json(review))
}

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use program1_contracts::{
    ContractError, CouponContract, CouponDto, CouponValidationResult, CreateCouponRequest,
    DiscountType, ValidateCouponRequest,
};
use program1_core::database::DbPool;
use sqlx::Row;
use uuid::Uuid;

#[derive(Clone)]
pub struct CouponModule {
    pool: DbPool,
}

impl CouponModule {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    fn row_to_dto(row: &sqlx::sqlite::SqliteRow) -> Result<CouponDto, ContractError> {
        let id_str: String = row.get("id");
        let id = Uuid::parse_str(&id_str)
            .map_err(|e| ContractError::Internal(format!("Corrupt UUID in coupon id: {}", e)))?;
        let code: String = row.get("code");
        let description: Option<String> = row.get("description");
        let discount_type_str: String = row.get("discount_type");
        let discount_type =
            DiscountType::from_str(&discount_type_str).unwrap_or(DiscountType::Percentage);
        let discount_value: f64 = row.get("discount_value");
        let min_order_amount: f64 = row.get("min_order_amount");
        let max_discount_amount: Option<f64> = row.get("max_discount_amount");
        let usage_limit_val: Option<i64> = row.get("usage_limit");
        let usage_limit = usage_limit_val.map(|v| v as u32);
        let usage_count_val: i64 = row.get("usage_count");
        let usage_count = usage_count_val as u32;
        let is_active_int: i64 = row.get("is_active");
        let is_active = is_active_int != 0;

        let valid_from_str: String = row.get("valid_from");
        let valid_from = DateTime::parse_from_rfc3339(&valid_from_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let valid_until_str: String = row.get("valid_until");
        let valid_until = DateTime::parse_from_rfc3339(&valid_until_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let created_at_str: String = row.get("created_at");
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let updated_at_str: String = row.get("updated_at");
        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(CouponDto {
            id,
            code,
            description,
            discount_type,
            discount_value,
            min_order_amount,
            max_discount_amount,
            usage_limit,
            usage_count,
            valid_from,
            valid_until,
            is_active,
            created_at,
            updated_at,
        })
    }
}

#[async_trait]
impl CouponContract for CouponModule {
    async fn list_coupons(&self) -> Result<Vec<CouponDto>, ContractError> {
        let rows = sqlx::query(
            "SELECT id, code, description, discount_type, discount_value, min_order_amount,
                    max_discount_amount, usage_limit, usage_count, valid_from, valid_until,
                    is_active, created_at, updated_at
             FROM coupons
             ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        rows.iter().map(Self::row_to_dto).collect()
    }

    async fn get_coupon(&self, id: Uuid) -> Result<CouponDto, ContractError> {
        let row = sqlx::query(
            "SELECT id, code, description, discount_type, discount_value, min_order_amount,
                    max_discount_amount, usage_limit, usage_count, valid_from, valid_until,
                    is_active, created_at, updated_at
             FROM coupons
             WHERE id = $1",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        match row {
            Some(r) => Self::row_to_dto(&r),
            None => Err(ContractError::NotFound(format!("Coupon {}", id))),
        }
    }

    async fn get_coupon_by_code(&self, code: &str) -> Result<CouponDto, ContractError> {
        let clean_code = code.trim().to_uppercase();
        let row = sqlx::query(
            "SELECT id, code, description, discount_type, discount_value, min_order_amount,
                    max_discount_amount, usage_limit, usage_count, valid_from, valid_until,
                    is_active, created_at, updated_at
             FROM coupons
             WHERE code = $1",
        )
        .bind(&clean_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        match row {
            Some(r) => Self::row_to_dto(&r),
            None => Err(ContractError::NotFound(format!(
                "Coupon code {}",
                clean_code
            ))),
        }
    }

    async fn create_coupon(&self, req: CreateCouponRequest) -> Result<CouponDto, ContractError> {
        let clean_code = req.code.trim().to_uppercase();
        if clean_code.len() < 3 || clean_code.len() > 30 {
            return Err(ContractError::ValidationError(
                "Coupon code must be 3-30 characters".to_string(),
            ));
        }

        // Check if code already exists
        let exists = sqlx::query("SELECT id FROM coupons WHERE code = $1")
            .bind(&clean_code)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        if exists.is_some() {
            return Err(ContractError::ValidationError(format!(
                "Coupon with code '{}' already exists",
                clean_code
            )));
        }

        let now = Utc::now();
        let valid_from = req.valid_from.unwrap_or(now);
        if req.valid_until <= valid_from {
            return Err(ContractError::ValidationError(
                "valid_until must be after valid_from".to_string(),
            ));
        }

        let id = Uuid::new_v4();
        let min_order = req.min_order_amount.unwrap_or(0.0);
        let usage_limit = req.usage_limit.map(|v| v as i64);

        sqlx::query(
            "INSERT INTO coupons (id, code, description, discount_type, discount_value, min_order_amount,
                                 max_discount_amount, usage_limit, usage_count, valid_from, valid_until,
                                 is_active, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0, $9, $10, 1, $11, $12)",
        )
        .bind(id.to_string())
        .bind(&clean_code)
        .bind(req.description.as_deref())
        .bind(req.discount_type.as_str())
        .bind(req.discount_value)
        .bind(min_order)
        .bind(req.max_discount_amount)
        .bind(usage_limit)
        .bind(valid_from.to_rfc3339())
        .bind(req.valid_until.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        tracing::info!(id = %id, code = %clean_code, "Coupon created successfully");

        Ok(CouponDto {
            id,
            code: clean_code,
            description: req.description,
            discount_type: req.discount_type,
            discount_value: req.discount_value,
            min_order_amount: min_order,
            max_discount_amount: req.max_discount_amount,
            usage_limit: req.usage_limit,
            usage_count: 0,
            valid_from,
            valid_until: req.valid_until,
            is_active: true,
            created_at: now,
            updated_at: now,
        })
    }

    async fn toggle_coupon_active(
        &self,
        id: Uuid,
        is_active: bool,
    ) -> Result<CouponDto, ContractError> {
        let existing = self.get_coupon(id).await?;
        let now = Utc::now();

        sqlx::query("UPDATE coupons SET is_active = $1, updated_at = $2 WHERE id = $3")
            .bind(if is_active { 1 } else { 0 })
            .bind(now.to_rfc3339())
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        Ok(CouponDto {
            is_active,
            updated_at: now,
            ..existing
        })
    }

    async fn delete_coupon(&self, id: Uuid) -> Result<(), ContractError> {
        let res = sqlx::query("DELETE FROM coupons WHERE id = $1")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        if res.rows_affected() == 0 {
            return Err(ContractError::NotFound(format!("Coupon {}", id)));
        }

        Ok(())
    }

    async fn validate_coupon(
        &self,
        req: ValidateCouponRequest,
    ) -> Result<CouponValidationResult, ContractError> {
        let clean_code = req.code.trim().to_uppercase();

        let coupon = match self.get_coupon_by_code(&clean_code).await {
            Ok(c) => c,
            Err(ContractError::NotFound(_)) => {
                return Ok(CouponValidationResult {
                    is_valid: false,
                    coupon: None,
                    discount_amount: 0.0,
                    final_amount: req.order_amount,
                    error_message: Some("Kode promo tidak ditemukan".to_string()),
                });
            }
            Err(e) => return Err(e),
        };

        if !coupon.is_active {
            return Ok(CouponValidationResult {
                is_valid: false,
                coupon: Some(coupon),
                discount_amount: 0.0,
                final_amount: req.order_amount,
                error_message: Some("Kupon promo sudah tidak aktif".to_string()),
            });
        }

        let now = Utc::now();
        if now < coupon.valid_from {
            return Ok(CouponValidationResult {
                is_valid: false,
                coupon: Some(coupon),
                discount_amount: 0.0,
                final_amount: req.order_amount,
                error_message: Some("Kupon promo belum mulai berlaku".to_string()),
            });
        }

        if now > coupon.valid_until {
            return Ok(CouponValidationResult {
                is_valid: false,
                coupon: Some(coupon),
                discount_amount: 0.0,
                final_amount: req.order_amount,
                error_message: Some("Kupon promo sudah kedaluwarsa".to_string()),
            });
        }

        if let Some(limit) = coupon.usage_limit {
            if coupon.usage_count >= limit {
                return Ok(CouponValidationResult {
                    is_valid: false,
                    coupon: Some(coupon),
                    discount_amount: 0.0,
                    final_amount: req.order_amount,
                    error_message: Some("Kuota pemakaian kupon ini telah habis".to_string()),
                });
            }
        }

        if req.order_amount < coupon.min_order_amount {
            return Ok(CouponValidationResult {
                is_valid: false,
                coupon: Some(coupon.clone()),
                discount_amount: 0.0,
                final_amount: req.order_amount,
                error_message: Some(format!(
                    "Minimum order untuk kupon ini adalah Rp{:.0}",
                    coupon.min_order_amount
                )),
            });
        }

        // Calculate discount
        let raw_discount = match coupon.discount_type {
            DiscountType::Percentage => {
                let disc = req.order_amount * (coupon.discount_value / 100.0);
                if let Some(max_cap) = coupon.max_discount_amount {
                    disc.min(max_cap)
                } else {
                    disc
                }
            }
            DiscountType::Fixed => coupon.discount_value,
        };

        let discount_amount = raw_discount.min(req.order_amount);
        let final_amount = (req.order_amount - discount_amount).max(0.0);

        Ok(CouponValidationResult {
            is_valid: true,
            coupon: Some(coupon),
            discount_amount,
            final_amount,
            error_message: None,
        })
    }

    async fn record_usage(
        &self,
        coupon_id: Uuid,
        buyer_id: Option<Uuid>,
        order_id: Option<Uuid>,
        discount_amount: f64,
    ) -> Result<(), ContractError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        let now = Utc::now();
        let res = sqlx::query(
            "UPDATE coupons SET usage_count = usage_count + 1, updated_at = $1 WHERE id = $2",
        )
        .bind(now.to_rfc3339())
        .bind(coupon_id.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        if res.rows_affected() == 0 {
            return Err(ContractError::NotFound(format!("Coupon {}", coupon_id)));
        }

        let usage_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO coupon_usages (id, coupon_id, buyer_id, order_id, discount_amount, used_at)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(usage_id.to_string())
        .bind(coupon_id.to_string())
        .bind(buyer_id.map(|b| b.to_string()))
        .bind(order_id.map(|o| o.to_string()))
        .bind(discount_amount)
        .bind(now.to_rfc3339())
        .execute(&mut *tx)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use program1_core::init_database;

    async fn create_test_coupon_module() -> CouponModule {
        let pool = init_database("sqlite::memory:")
            .await
            .expect("In-memory SQLite init failed");
        CouponModule::new(pool)
    }

    #[tokio::test]
    async fn test_create_and_validate_percentage_coupon() {
        let module = create_test_coupon_module().await;

        let coupon = module
            .create_coupon(CreateCouponRequest {
                code: "diskon20".to_string(),
                description: Some("Diskon 20% max 50rb".to_string()),
                discount_type: DiscountType::Percentage,
                discount_value: 20.0,
                min_order_amount: Some(100000.0),
                max_discount_amount: Some(50000.0),
                usage_limit: Some(10),
                valid_from: None,
                valid_until: Utc::now() + Duration::days(7),
            })
            .await
            .unwrap();

        assert_eq!(coupon.code, "DISKON20");

        // Order amount 200_000: 20% is 40_000 (below cap 50_000)
        let res = module
            .validate_coupon(ValidateCouponRequest {
                code: "DISKON20".to_string(),
                order_amount: 200000.0,
                buyer_id: None,
            })
            .await
            .unwrap();

        assert!(res.is_valid);
        assert_eq!(res.discount_amount, 40000.0);
        assert_eq!(res.final_amount, 160000.0);

        // Order amount 500_000: 20% is 100_000 -> capped at 50_000
        let res_capped = module
            .validate_coupon(ValidateCouponRequest {
                code: "diskon20".to_string(),
                order_amount: 500000.0,
                buyer_id: None,
            })
            .await
            .unwrap();

        assert!(res_capped.is_valid);
        assert_eq!(res_capped.discount_amount, 50000.0);
        assert_eq!(res_capped.final_amount, 450000.0);
    }

    #[tokio::test]
    async fn test_create_and_validate_fixed_coupon() {
        let module = create_test_coupon_module().await;

        let _ = module
            .create_coupon(CreateCouponRequest {
                code: "HEMAT25K".to_string(),
                description: Some("Potongan Rp25.000".to_string()),
                discount_type: DiscountType::Fixed,
                discount_value: 25000.0,
                min_order_amount: Some(50000.0),
                max_discount_amount: None,
                usage_limit: None,
                valid_from: None,
                valid_until: Utc::now() + Duration::days(30),
            })
            .await
            .unwrap();

        let res = module
            .validate_coupon(ValidateCouponRequest {
                code: "hemat25k".to_string(),
                order_amount: 100000.0,
                buyer_id: None,
            })
            .await
            .unwrap();

        assert!(res.is_valid);
        assert_eq!(res.discount_amount, 25000.0);
        assert_eq!(res.final_amount, 75000.0);
    }

    #[tokio::test]
    async fn test_expired_coupon_rejected() {
        let module = create_test_coupon_module().await;

        let _ = module
            .create_coupon(CreateCouponRequest {
                code: "KADALUARSA".to_string(),
                description: None,
                discount_type: DiscountType::Fixed,
                discount_value: 10000.0,
                min_order_amount: None,
                max_discount_amount: None,
                usage_limit: None,
                valid_from: Some(Utc::now() - Duration::days(10)),
                valid_until: Utc::now() - Duration::days(1), // already expired
            })
            .await
            .unwrap();

        let res = module
            .validate_coupon(ValidateCouponRequest {
                code: "KADALUARSA".to_string(),
                order_amount: 100000.0,
                buyer_id: None,
            })
            .await
            .unwrap();

        assert!(!res.is_valid);
        assert!(res.error_message.unwrap().contains("kedaluwarsa"));
    }

    #[tokio::test]
    async fn test_min_order_not_met() {
        let module = create_test_coupon_module().await;

        let _ = module
            .create_coupon(CreateCouponRequest {
                code: "MIN100K".to_string(),
                description: None,
                discount_type: DiscountType::Fixed,
                discount_value: 15000.0,
                min_order_amount: Some(100000.0),
                max_discount_amount: None,
                usage_limit: None,
                valid_from: None,
                valid_until: Utc::now() + Duration::days(7),
            })
            .await
            .unwrap();

        let res = module
            .validate_coupon(ValidateCouponRequest {
                code: "MIN100K".to_string(),
                order_amount: 80000.0,
                buyer_id: None,
            })
            .await
            .unwrap();

        assert!(!res.is_valid);
        assert!(res.error_message.unwrap().contains("Minimum order"));
    }

    #[tokio::test]
    async fn test_usage_limit_exceeded() {
        let module = create_test_coupon_module().await;

        let coupon = module
            .create_coupon(CreateCouponRequest {
                code: "LIMIT1".to_string(),
                description: None,
                discount_type: DiscountType::Fixed,
                discount_value: 5000.0,
                min_order_amount: None,
                max_discount_amount: None,
                usage_limit: Some(1),
                valid_from: None,
                valid_until: Utc::now() + Duration::days(7),
            })
            .await
            .unwrap();

        // 1st validation ok
        let res1 = module
            .validate_coupon(ValidateCouponRequest {
                code: "LIMIT1".to_string(),
                order_amount: 50000.0,
                buyer_id: None,
            })
            .await
            .unwrap();
        assert!(res1.is_valid);

        // Record usage
        module
            .record_usage(coupon.id, None, None, 5000.0)
            .await
            .unwrap();

        // 2nd validation should be rejected
        let res2 = module
            .validate_coupon(ValidateCouponRequest {
                code: "LIMIT1".to_string(),
                order_amount: 50000.0,
                buyer_id: None,
            })
            .await
            .unwrap();
        assert!(!res2.is_valid);
        assert!(res2.error_message.unwrap().contains("Kuota pemakaian"));
    }
}

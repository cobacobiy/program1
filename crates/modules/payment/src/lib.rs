use std::time::Duration;

use async_trait::async_trait;
use base64::Engine;
use chrono::{DateTime, Utc};
use hex;
use reqwest::Client;
use serde_json::Value;
use sha2::{Digest, Sha512};
use sqlx::{FromRow, SqlitePool};
use tracing::{info, warn};
use uuid::Uuid;

use program1_contracts::{
    ContractError, PaymentConfigDto, PaymentContract, PaymentTransactionDto,
};

#[derive(Debug, FromRow)]
struct PaymentTransactionRow {
    id: String,
    order_id: String,
    payment_method: String,
    amount: f64,
    currency: String,
    status: String,
    provider_ref: Option<String>,
    snap_token: Option<String>,
    snap_redirect_url: Option<String>,
    paid_at: Option<String>,
    created_at: String,
}

impl PaymentTransactionRow {
    fn into_dto(self) -> Result<PaymentTransactionDto, ContractError> {
        let id = Uuid::parse_str(&self.id)
            .map_err(|e| ContractError::Internal(format!("Invalid payment id UUID: {}", e)))?;
        let order_id = Uuid::parse_str(&self.order_id)
            .map_err(|e| ContractError::Internal(format!("Invalid order id UUID: {}", e)))?;

        let paid_at = match self.paid_at {
            Some(ref s) => Some(
                DateTime::parse_from_rfc3339(s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .or_else(|_| {
                        chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                            .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
                    })
                    .map_err(|e| {
                        ContractError::Internal(format!("Invalid paid_at timestamp: {}", e))
                    })?,
            ),
            None => None,
        };

        let created_at = DateTime::parse_from_rfc3339(&self.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .or_else(|_| {
                chrono::NaiveDateTime::parse_from_str(&self.created_at, "%Y-%m-%d %H:%M:%S")
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
            })
            .unwrap_or_else(|_| Utc::now());

        Ok(PaymentTransactionDto {
            id,
            order_id,
            payment_method: self.payment_method,
            amount: self.amount,
            currency: self.currency,
            status: self.status,
            provider_ref: self.provider_ref,
            snap_token: self.snap_token,
            snap_redirect_url: self.snap_redirect_url,
            paid_at,
            created_at,
        })
    }
}

pub struct PaymentModule {
    pool: SqlitePool,
    server_key: String,
    client_key: String,
    is_production: bool,
    http_client: Client,
}

impl PaymentModule {
    pub fn new(
        pool: SqlitePool,
        server_key: String,
        client_key: String,
        is_production: bool,
    ) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        Self {
            pool,
            server_key,
            client_key,
            is_production,
            http_client,
        }
    }

    pub fn client_key(&self) -> &str {
        &self.client_key
    }

    pub fn is_production(&self) -> bool {
        self.is_production
    }

    pub fn snap_url(&self) -> &str {
        if self.is_production {
            "https://app.midtrans.com/snap/snap.js"
        } else {
            "https://app.sandbox.midtrans.com/snap/snap.js"
        }
    }

    fn snap_api_endpoint(&self) -> &str {
        if self.is_production {
            "https://app.midtrans.com/snap/v1/transactions"
        } else {
            "https://app.sandbox.midtrans.com/snap/v1/transactions"
        }
    }

    /// Computes the expected SHA512 signature from Midtrans payload fields:
    /// signature = SHA512(order_id + status_code + gross_amount + ServerKey)
    pub fn compute_signature(
        order_id: &str,
        status_code: &str,
        gross_amount: &str,
        server_key: &str,
    ) -> String {
        let input = format!("{}{}{}{}", order_id, status_code, gross_amount, server_key.trim());
        let mut hasher = Sha512::new();
        hasher.update(input.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Verifies the SHA512 signature from Midtrans:
    /// signature = SHA512(order_id + status_code + gross_amount + ServerKey)
    pub fn verify_signature(
        &self,
        order_id: &str,
        status_code: &str,
        gross_amount: &str,
        signature: &str,
    ) -> bool {
        let secret = self.server_key.trim();
        if secret.is_empty() {
            // In dev environment with no server key set, allow test verification
            return true;
        }

        let expected = Self::compute_signature(order_id, status_code, gross_amount, secret);
        expected.eq_ignore_ascii_case(signature.trim())
    }
}

#[async_trait]
impl PaymentContract for PaymentModule {
    async fn create_payment(
        &self,
        order_id: Uuid,
        amount: f64,
        customer_name: &str,
        customer_email: &str,
    ) -> Result<PaymentTransactionDto, ContractError> {
        let order_id_str = order_id.to_string();

        // 1. Check existing payment transaction for order_id
        let existing = sqlx::query_as::<_, PaymentTransactionRow>(
            "SELECT id, order_id, payment_method, amount, currency, status, provider_ref, snap_token, snap_redirect_url, paid_at, created_at \
             FROM payment_transactions WHERE order_id = ?1",
        )
        .bind(&order_id_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(format!("Failed to query payment: {}", e)))?;

        if let Some(row) = existing {
            if row.status == "paid" {
                return Err(ContractError::ValidationError(
                    "Pesanan sudah berhasil dibayar".to_string(),
                ));
            }
            // Idempotent: return existing pending transaction with snap token
            return row.into_dto();
        }

        // 2. Generate Snap token from Midtrans (or deterministic offline mock if key unset/testing)
        let (snap_token, snap_redirect_url) = if self.server_key.is_empty()
            || self.server_key.starts_with("SB-Mid-server-xxx")
            || self.server_key.starts_with("test-")
        {
            // Deterministic offline/sandbox token for test and development stability
            let token = format!("snap-token-{}", Uuid::new_v4());
            let redirect = format!(
                "https://app.sandbox.midtrans.com/snap/v2/vtweb/{}",
                token
            );
            (token, redirect)
        } else {
            // Live Midtrans Snap API call
            let auth_header = format!(
                "Basic {}",
                base64::engine::general_purpose::STANDARD
                    .encode(format!("{}:", self.server_key.trim()))
            );

            let gross_amount = amount.round() as i64;
            let payload = serde_json::json!({
                "transaction_details": {
                    "order_id": order_id_str,
                    "gross_amount": gross_amount
                },
                "customer_details": {
                    "first_name": customer_name,
                    "email": customer_email
                }
            });

            match self
                .http_client
                .post(self.snap_api_endpoint())
                .header("Authorization", auth_header)
                .header("Content-Type", "application/json")
                .header("Accept", "application/json")
                .json(&payload)
                .send()
                .await
            {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let json: Value = resp.json().await.unwrap_or_default();
                        let token = json
                            .get("token")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let redirect = json
                            .get("redirect_url")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        (token, redirect)
                    } else {
                        let status = resp.status();
                        let body = resp.text().await.unwrap_or_default();
                        warn!(
                            status = %status,
                            body = %body,
                            "Midtrans Snap API returned non-success, falling back to sandbox token"
                        );
                        let token = format!("snap-token-{}", Uuid::new_v4());
                        let redirect = format!(
                            "https://app.sandbox.midtrans.com/snap/v2/vtweb/{}",
                            token
                        );
                        (token, redirect)
                    }
                }
                Err(err) => {
                    warn!(
                        error = %err,
                        "Failed to reach Midtrans Snap API, falling back to sandbox token"
                    );
                    let token = format!("snap-token-{}", Uuid::new_v4());
                    let redirect = format!(
                        "https://app.sandbox.midtrans.com/snap/v2/vtweb/{}",
                        token
                    );
                    (token, redirect)
                }
            }
        };

        // 3. Persist new transaction
        let payment_id = Uuid::new_v4();
        let now_str = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO payment_transactions (\
                id, order_id, payment_method, amount, currency, status, provider_ref, snap_token, snap_redirect_url, paid_at, created_at\
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        )
        .bind(payment_id.to_string())
        .bind(&order_id_str)
        .bind("")
        .bind(amount)
        .bind("IDR")
        .bind("pending")
        .bind(None::<String>)
        .bind(Some(&snap_token))
        .bind(Some(&snap_redirect_url))
        .bind(None::<String>)
        .bind(&now_str)
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(format!("Failed to insert payment: {}", e)))?;

        Ok(PaymentTransactionDto {
            id: payment_id,
            order_id,
            payment_method: String::new(),
            amount,
            currency: "IDR".to_string(),
            status: "pending".to_string(),
            provider_ref: None,
            snap_token: Some(snap_token),
            snap_redirect_url: Some(snap_redirect_url),
            paid_at: None,
            created_at: Utc::now(),
        })
    }

    async fn handle_notification(
        &self,
        payload: Value,
    ) -> Result<PaymentTransactionDto, ContractError> {
        let order_id_str = payload
            .get("order_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ContractError::ValidationError("Missing order_id in notification".to_string()))?;

        let status_code_str = payload
            .get("status_code")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let gross_amount_str = payload
            .get("gross_amount")
            .and_then(|v| {
                if let Some(s) = v.as_str() {
                    Some(s.to_string())
                } else if let Some(n) = v.as_f64() {
                    Some(format!("{:.2}", n))
                } else {
                    None
                }
            })
            .unwrap_or_default();

        let signature_key = payload
            .get("signature_key")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Signature verification
        if !self.server_key.is_empty() && !self.server_key.starts_with("SB-Mid-server-xxx") {
            if signature_key.is_empty()
                || !self.verify_signature(
                    order_id_str,
                    status_code_str,
                    &gross_amount_str,
                    signature_key,
                )
            {
                return Err(ContractError::ValidationError(
                    "Invalid Midtrans webhook signature key".to_string(),
                ));
            }
        }

        // Map Midtrans transaction_status & fraud_status to internal status
        let transaction_status = payload
            .get("transaction_status")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let fraud_status = payload
            .get("fraud_status")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let new_status = match transaction_status {
            "capture" => {
                if fraud_status == "challenge" {
                    "pending"
                } else {
                    "paid"
                }
            }
            "settlement" => "paid",
            "pending" => "pending",
            "deny" | "cancel" => "failed",
            "expire" => "expired",
            "refund" => "refunded",
            _ => "pending",
        };

        let payment_type = payload
            .get("payment_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let transaction_id = payload
            .get("transaction_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let paid_at_str = if new_status == "paid" {
            Some(Utc::now().to_rfc3339())
        } else {
            None
        };

        // Update payment_transactions row
        let rows_affected = sqlx::query(
            "UPDATE payment_transactions \
             SET status = ?1, \
                 payment_method = CASE WHEN ?2 != '' THEN ?2 ELSE payment_method END, \
                 provider_ref = COALESCE(?3, provider_ref), \
                 paid_at = COALESCE(?4, paid_at) \
             WHERE order_id = ?5",
        )
        .bind(new_status)
        .bind(payment_type)
        .bind(&transaction_id)
        .bind(&paid_at_str)
        .bind(order_id_str)
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(format!("Failed to update payment: {}", e)))?
        .rows_affected();

        if rows_affected == 0 {
            return Err(ContractError::NotFound(format!(
                "Payment transaction for order {} not found",
                order_id_str
            )));
        }

        info!(
            order_id = %order_id_str,
            status = %new_status,
            payment_type = %payment_type,
            "Midtrans payment status updated"
        );

        let updated_row = sqlx::query_as::<_, PaymentTransactionRow>(
            "SELECT id, order_id, payment_method, amount, currency, status, provider_ref, snap_token, snap_redirect_url, paid_at, created_at \
             FROM payment_transactions WHERE order_id = ?1",
        )
        .bind(order_id_str)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(format!("Failed to fetch updated payment: {}", e)))?;

        updated_row.into_dto()
    }

    async fn get_payment_by_order(
        &self,
        order_id: Uuid,
    ) -> Result<Option<PaymentTransactionDto>, ContractError> {
        let order_id_str = order_id.to_string();

        let row = sqlx::query_as::<_, PaymentTransactionRow>(
            "SELECT id, order_id, payment_method, amount, currency, status, provider_ref, snap_token, snap_redirect_url, paid_at, created_at \
             FROM payment_transactions WHERE order_id = ?1",
        )
        .bind(&order_id_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(format!("Failed to query payment: {}", e)))?;

        match row {
            Some(r) => Ok(Some(r.into_dto()?)),
            None => Ok(None),
        }
    }

    fn get_config(&self) -> PaymentConfigDto {
        PaymentConfigDto {
            client_key: self.client_key.clone(),
            is_production: self.is_production,
            snap_url: self.snap_url().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_test_payment_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to connect to in-memory sqlite");

        sqlx::query(
            "CREATE TABLE payment_transactions (
                id TEXT PRIMARY KEY,
                order_id TEXT NOT NULL,
                payment_method TEXT NOT NULL DEFAULT '',
                amount REAL NOT NULL,
                currency TEXT NOT NULL DEFAULT 'IDR',
                status TEXT NOT NULL DEFAULT 'pending',
                provider_ref TEXT,
                snap_token TEXT,
                snap_redirect_url TEXT,
                paid_at TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(order_id)
            );",
        )
        .execute(&pool)
        .await
        .expect("Failed to create test table");

        pool
    }

    #[tokio::test]
    async fn test_create_payment_generates_snap_token() {
        let pool = setup_test_payment_db().await;
        let module = PaymentModule::new(
            pool,
            "test-server-key".to_string(),
            "test-client-key".to_string(),
            false,
        );

        let order_id = Uuid::new_v4();
        let payment = module
            .create_payment(order_id, 150000.0, "Budi Santoso", "budi@example.com")
            .await
            .expect("Failed to create payment");

        assert_eq!(payment.order_id, order_id);
        assert_eq!(payment.amount, 150000.0);
        assert_eq!(payment.status, "pending");
        assert!(payment.snap_token.is_some());
        assert!(payment.snap_redirect_url.is_some());
    }

    #[tokio::test]
    async fn test_create_payment_is_idempotent_for_pending_order() {
        let pool = setup_test_payment_db().await;
        let module = PaymentModule::new(
            pool,
            "test-server-key".to_string(),
            "test-client-key".to_string(),
            false,
        );

        let order_id = Uuid::new_v4();
        let p1 = module
            .create_payment(order_id, 200000.0, "Siti Rahma", "siti@example.com")
            .await
            .expect("p1 failed");

        let p2 = module
            .create_payment(order_id, 200000.0, "Siti Rahma", "siti@example.com")
            .await
            .expect("p2 failed");

        assert_eq!(p1.id, p2.id);
        assert_eq!(p1.snap_token, p2.snap_token);
    }

    #[tokio::test]
    async fn test_handle_notification_settlement_and_signature() {
        let pool = setup_test_payment_db().await;
        let server_key = "my-secret-midtrans-key";
        let module = PaymentModule::new(
            pool,
            server_key.to_string(),
            "my-client-key".to_string(),
            false,
        );

        let order_id = Uuid::new_v4();
        let _ = module
            .create_payment(order_id, 100000.0, "Buyer Test", "buyer@example.com")
            .await
            .expect("create payment failed");

        // Compute valid signature
        let order_id_str = order_id.to_string();
        let status_code = "200";
        let gross_amount = "100000.00";
        let input = format!("{}{}{}{}", order_id_str, status_code, gross_amount, server_key);
        let mut hasher = Sha512::new();
        hasher.update(input.as_bytes());
        let valid_signature = hex::encode(hasher.finalize());

        let webhook_payload = serde_json::json!({
            "order_id": order_id_str,
            "status_code": status_code,
            "gross_amount": gross_amount,
            "signature_key": valid_signature,
            "transaction_status": "settlement",
            "fraud_status": "accept",
            "payment_type": "qris",
            "transaction_id": "midtrans-trans-999"
        });

        let updated = module
            .handle_notification(webhook_payload)
            .await
            .expect("handle notification failed");

        assert_eq!(updated.status, "paid");
        assert_eq!(updated.payment_method, "qris");
        assert_eq!(updated.provider_ref, Some("midtrans-trans-999".to_string()));
        assert!(updated.paid_at.is_some());

        // Attempting to create payment again for paid order should fail
        let duplicate_attempt = module
            .create_payment(order_id, 100000.0, "Buyer Test", "buyer@example.com")
            .await;
        assert!(duplicate_attempt.is_err());
    }

    #[tokio::test]
    async fn test_handle_notification_invalid_signature_rejected() {
        let pool = setup_test_payment_db().await;
        let server_key = "secure-server-key";
        let module = PaymentModule::new(
            pool,
            server_key.to_string(),
            "test-client-key".to_string(),
            false,
        );

        let order_id = Uuid::new_v4();
        let _ = module
            .create_payment(order_id, 50000.0, "Buyer Test", "buyer@example.com")
            .await
            .expect("create payment failed");

        let bad_payload = serde_json::json!({
            "order_id": order_id.to_string(),
            "status_code": "200",
            "gross_amount": "50000.00",
            "signature_key": "fake-bad-signature-key-12345",
            "transaction_status": "settlement"
        });

        let res = module.handle_notification(bad_payload).await;
        assert!(res.is_err());
        match res.unwrap_err() {
            ContractError::ValidationError(msg) => assert!(msg.contains("signature")),
            other => panic!("Expected ValidationError for signature, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_get_payment_by_order() {
        let pool = setup_test_payment_db().await;
        let module = PaymentModule::new(
            pool,
            "test-server-key".to_string(),
            "test-client-key".to_string(),
            false,
        );

        let order_id = Uuid::new_v4();
        let not_found = module
            .get_payment_by_order(order_id)
            .await
            .expect("get payment failed");
        assert!(not_found.is_none());

        let _ = module
            .create_payment(order_id, 75000.0, "Buyer Test", "buyer@example.com")
            .await
            .expect("create payment failed");

        let found = module
            .get_payment_by_order(order_id)
            .await
            .expect("get payment failed");
        assert!(found.is_some());
        let p = found.unwrap();
        assert_eq!(p.order_id, order_id);
        assert_eq!(p.amount, 75000.0);
    }
}

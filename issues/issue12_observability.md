# Issue #12 — Health Check & Observability Enhancement

> **Prioritas**: 🟢 MEDIUM
> **Estimasi**: 1-2 hari
> **Depends On**: —
> **Blocks**: —

---

## 🔍 Masalah Saat Ini

Health check saat ini **terlalu sederhana**:

```rust
async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({
        "status": "healthy",
        "app": "program1",
        "architecture": "modular_monolith",
        "version": "0.1.0"
    })))
}
```

Masalah:
- Tidak cek koneksi database (setelah Issue #3)
- Tidak cek module readiness
- Tidak ada metrics (request count, latency, error rate)
- Tidak ada request ID untuk tracing
- Docker healthcheck di `docker-compose.yml` cek `/api/v1/store/info` bukan `/health`
- Version hardcoded — harusnya dari `Cargo.toml`

---

## ✅ Acceptance Criteria

### 1. Enhanced Health Check

```rust
async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let mut checks = serde_json::Map::new();
    let mut overall_healthy = true;

    // Check database connectivity (setelah Issue #3)
    // let db_ok = state.database.ping().await.is_ok();
    // checks.insert("database".into(), json!(if db_ok { "ok" } else { "fail" }));
    // if !db_ok { overall_healthy = false; }

    // Check module counts
    let catalog_ok = state.catalog_contract.list_items().await.is_ok();
    checks.insert("catalog_module".into(), json!(if catalog_ok { "ok" } else { "fail" }));

    (
        if overall_healthy { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE },
        Json(json!({
            "status": if overall_healthy { "healthy" } else { "degraded" },
            "app": "program1",
            "version": env!("CARGO_PKG_VERSION"),
            "architecture": "modular_monolith",
            "uptime_seconds": state.started_at.elapsed().as_secs(),
            "checks": checks,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })),
    )
}
```

### 2. Request ID Middleware

Setiap request harus memiliki unique ID untuk tracing:

```rust
use uuid::Uuid;

async fn request_id_middleware<B>(
    mut request: Request<B>,
    next: Next<B>,
) -> Response {
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    
    request.extensions_mut().insert(RequestId(request_id.clone()));
    
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        "x-request-id",
        request_id.parse().unwrap(),
    );
    response
}
```

### 3. Request Logging Enhancement

Tambahkan structured logging di middleware:

```rust
tracing::info!(
    request_id = %request_id,
    method = %method,
    path = %path,
    status = %status,
    latency_ms = %latency.as_millis(),
    "HTTP Request"
);
```

### 4. Readiness vs Liveness Probes

```
GET /health          → Liveness probe (always responds if server is up)
GET /health/ready    → Readiness probe (checks dependencies like DB)
```

### 5. Update Docker Healthcheck

```yaml
healthcheck:
  test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
  interval: 10s
  timeout: 5s
  retries: 3
  start_period: 15s
```

### 6. Track Uptime

Tambahkan `started_at` ke `AppState`:

```rust
pub struct AppState {
    pub started_at: std::time::Instant,
    // ... existing fields
}
```

---

## 🧪 Unit Test

```rust
#[cfg(test)]
mod tests {
    // 1. Test health check returns expected fields
    #[tokio::test]
    async fn test_health_check_fields() { ... }
    
    // 2. Test request ID propagation
    #[tokio::test]
    async fn test_request_id_header() { ... }
    
    // 3. Test version not hardcoded
    #[test]
    fn test_version_from_cargo() { ... }
}
```

---

## ⚠️ Peringatan

- Health check JANGAN melakukan operasi berat — harus cepat (<100ms)
- Request ID middleware harus di-apply di layer paling luar
- `env!("CARGO_PKG_VERSION")` di-resolve saat compile time dari `Cargo.toml`
- Semua existing test `cargo test --workspace` harus tetap PASS

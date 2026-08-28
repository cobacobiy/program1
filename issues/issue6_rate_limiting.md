# Issue #6 — Rate Limiting & Abuse Protection

> **Prioritas**: 🟡 HIGH
> **Estimasi**: 1-2 hari
> **Depends On**: —
> **Blocks**: —

---

## 🔍 Masalah Saat Ini

Tidak ada rate limiting sama sekali. Artinya:
- Attacker bisa brute-force login endpoint tanpa batas
- Bot bisa flood API dengan request, menyebabkan DoS
- Seseorang bisa membuat ribuan order palsu dalam hitungan detik
- In-memory `HashMap` bisa membengkak tanpa batas

---

## ✅ Acceptance Criteria

### 1. Tambahkan Dependencies

```toml
# Cargo.toml [workspace.dependencies]
tower = { version = "0.4", features = ["util", "limit"] }
governor = "0.6"   # Rate limiting middleware
```

### 2. Global Rate Limit

Tambahkan tower rate limit layer untuk semua request:

```rust
use tower::limit::RateLimitLayer;
use std::time::Duration;

// Global: max 100 requests per second per IP
let app = Router::new()
    // ... routes
    .layer(RateLimitLayer::new(100, Duration::from_secs(1)));
```

### 3. Endpoint-Specific Rate Limits

| Endpoint | Limit | Alasan |
|----------|-------|--------|
| `POST /api/v1/auth/login` | 5 req/menit per IP | Anti brute-force |
| `POST /api/v1/auth/register` | 3 req/menit per IP | Anti spam account |
| `POST /api/v1/orders` | 10 req/menit per IP | Anti fake orders |
| `POST /api/v1/catalog` | 20 req/menit per IP | Anti spam |
| `GET /api/v1/*` | 60 req/menit per IP | General API |
| `GET /health` | Unlimited | Health check |

### 4. Implementasi dengan Governor

```rust
use governor::{Governor, GovernorConfigBuilder, KeyExtractor};
use tower_governor::GovernorLayer;

// Per-IP rate limiting
fn rate_limit_layer(requests_per_minute: u64) -> GovernorLayer<...> {
    let config = GovernorConfigBuilder::default()
        .per_second(requests_per_minute / 60)
        .burst_size(requests_per_minute as u32)
        .key_extractor(PeerIpKeyExtractor)
        .finish()
        .unwrap();
    
    GovernorLayer { config: Arc::new(config) }
}
```

### 5. Response Headers untuk Rate Limit

Saat rate limited, return:
```
HTTP/1.1 429 Too Many Requests
Retry-After: 60
X-RateLimit-Limit: 5
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1693000000

{
    "error": "rate_limit_exceeded",
    "message": "Terlalu banyak request. Coba lagi dalam 60 detik.",
    "retry_after": 60
}
```

### 6. Request Body Size Limit (already in Issue #4 but reinforce here)

```rust
.layer(DefaultBodyLimit::max(1024 * 1024)) // 1MB
```

---

## 🧪 Unit Test

```rust
#[cfg(test)]
mod tests {
    // 1. Test rate limit response code (429)
    #[tokio::test]
    async fn test_rate_limit_returns_429() { ... }
    
    // 2. Test Retry-After header present
    #[tokio::test]
    async fn test_retry_after_header() { ... }
}
```

---

## ⚠️ Peringatan

- Rate limit harus berdasarkan IP address (bukan random)
- Di belakang reverse proxy (NPM), gunakan `X-Forwarded-For` atau `X-Real-IP` header
- Health check endpoint JANGAN di-rate limit
- Jangan terlalu ketat di development (bisa pakai env-based config)
- Semua existing test `cargo test --workspace` harus tetap PASS

---

## 📎 Referensi

- [tower-governor crate](https://docs.rs/tower_governor/latest/tower_governor/)
- [governor crate](https://docs.rs/governor/latest/governor/)

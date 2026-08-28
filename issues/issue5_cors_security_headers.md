# Issue #5 — CORS Hardening & Security Headers

> **Prioritas**: 🟡 HIGH
> **Estimasi**: 1 hari
> **Depends On**: —
> **Blocks**: —

---

## 🔍 Masalah Saat Ini

CORS saat ini dikonfigurasi sebagai **"allow everything"**:

```rust
// crates/web/src/main.rs baris 74-77
let cors = CorsLayer::new()
    .allow_origin(Any)      // ❌ SEMUA domain bisa akses
    .allow_methods(Any)     // ❌ SEMUA HTTP method diizinkan
    .allow_headers(Any);    // ❌ SEMUA header diizinkan
```

Ini berarti:
- Website manapun bisa melakukan API request ke server
- Tidak ada proteksi terhadap CSRF (Cross-Site Request Forgery)
- Tidak ada security headers (HSTS, CSP, X-Frame-Options, dll)

---

## ✅ Acceptance Criteria

### 1. CORS Configuration yang Proper

```rust
use tower_http::cors::{CorsLayer, AllowOrigin};
use axum::http::{Method, HeaderName, HeaderValue};

fn build_cors_layer() -> CorsLayer {
    let allowed_origins = env::var("ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:8080,http://localhost:3000".to_string());
    
    let origins: Vec<HeaderValue> = allowed_origins
        .split(',')
        .filter_map(|o| o.trim().parse().ok())
        .collect();

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            HeaderName::from_static("content-type"),
            HeaderName::from_static("authorization"),
            HeaderName::from_static("x-requested-with"),
        ])
        .allow_credentials(true)
        .max_age(std::time::Duration::from_secs(3600))
}
```

### 2. Security Headers Middleware

Buat middleware atau layer:

```rust
use axum::middleware::{self, Next};
use axum::http::{Request, Response, header};

async fn security_headers<B>(request: Request<B>, next: Next<B>) -> Response<B> {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    
    // Prevent clickjacking
    headers.insert("X-Frame-Options", "DENY".parse().unwrap());
    
    // Prevent MIME type sniffing
    headers.insert("X-Content-Type-Options", "nosniff".parse().unwrap());
    
    // XSS protection (legacy browsers)
    headers.insert("X-XSS-Protection", "1; mode=block".parse().unwrap());
    
    // Referrer policy
    headers.insert("Referrer-Policy", "strict-origin-when-cross-origin".parse().unwrap());
    
    // Content Security Policy (basic)
    headers.insert(
        "Content-Security-Policy",
        "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com; img-src 'self' https: data:; connect-src 'self'".parse().unwrap()
    );
    
    // Strict Transport Security (production only)
    if env::var("APP_ENV").unwrap_or_default() == "production" {
        headers.insert(
            "Strict-Transport-Security",
            "max-age=31536000; includeSubDomains".parse().unwrap()
        );
    }
    
    response
}
```

### 3. Apply Middleware di Router

```rust
let app = Router::new()
    // ... routes
    .layer(middleware::from_fn(security_headers))
    .layer(build_cors_layer());
```

### 4. Update `.env.example`

```env
# CORS — comma-separated list of allowed origins
ALLOWED_ORIGINS=http://localhost:8080,http://localhost:3000
# Untuk production:
# ALLOWED_ORIGINS=https://yourdomain.com,https://admin.yourdomain.com
```

### 5. Considerations untuk Storefront

Karena `/store` dan `/` adalah public storefront yang di-serve dari same origin, pastikan:
- Static assets (`/assets/*`) bisa diakses tanpa CORS issue
- API calls dari storefront JavaScript tetap berjalan (same-origin request tidak terkena CORS)

---

## 🧪 Unit Test

```rust
#[cfg(test)]
mod tests {
    // 1. Test CORS header present pada response
    #[tokio::test]
    async fn test_cors_headers_present() { ... }
    
    // 2. Test security headers pada response
    #[tokio::test]
    async fn test_security_headers_present() { ... }
    
    // 3. Test preflight OPTIONS request
    #[tokio::test]
    async fn test_cors_preflight_response() { ... }
}
```

---

## ⚠️ Peringatan

- Di development, boleh keep `localhost` origins
- Di production via NPM reverse proxy, origin harus match HTTPS domain
- `allow_credentials(true)` dan `allow_origin(Any)` **tidak bisa dipakai bersamaan** — ini sudah benar di solusi di atas
- Semua existing test `cargo test --workspace` harus tetap PASS

---

## 📎 Referensi

- [MDN CORS guide](https://developer.mozilla.org/en-US/docs/Web/HTTP/CORS)
- [OWASP Secure Headers](https://owasp.org/www-project-secure-headers/)
- [tower-http CORS docs](https://docs.rs/tower-http/latest/tower_http/cors/)

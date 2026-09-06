# 📋 Program1 — Development Issue Tracker

> Master index untuk semua issue pengembangan Program1 Modular Monolith.
> Setiap issue dirancang agar bisa dikerjakan secara **independen** oleh programmer junior atau AI agent yang lebih murah.

---

## 🏗️ Arsitektur Saat Ini

```
program1/
├── crates/contracts/       ← Shared trait interfaces & DTOs
├── crates/core/            ← Tracing, DB init, auth helpers (Argon2id)
├── crates/modules/
│   ├── user/               ← UserContract (RBAC, SQLite-backed)
│   ├── auth/               ← AuthContract (JWT HS256)
│   ├── buyer/              ← BuyerContract (Google OAuth, Email/Pass, OTP)
│   ├── catalog/            ← CatalogContract (product catalog, SQLite)
│   ├── inventory/          ← InventoryContract (Ginee OMS multi-stock)
│   ├── channel/            ← ChannelSyncContract (marketplace sync)
│   ├── order/              ← OrderContract (omni-channel orders)
│   ├── analytics/          ← AnalyticsContract (sales analytics)
│   └── audit/              ← AuditContract (immutable audit log)
└── crates/web/             ← Axum HTTP server + static UI (storefront + admin)
```

---

## 🚦 Status & Prioritas Issue

### ✅ Phase 0 — Foundation (DONE)

| # | GitHub Issue | Status |
|---|--------------|--------|
| 1 | [#1 Authentication & Password Hashing](https://github.com/cobacobiy/program1/issues/1) | `[x]` DONE — Argon2id di `core/src/auth.rs`, UserModule uses password_hash |
| 2 | [#2 JWT Middleware & Route Protection](https://github.com/cobacobiy/program1/issues/2) | `[x]` DONE — AuthModule + require_auth/require_buyer_auth/require_admin middleware |
| 3 | [#3 Database Persistence (SQLite)](https://github.com/cobacobiy/program1/issues/3) | `[x]` DONE — Semua module pakai `sqlx` + 9 migrations |
| 4 | [#4 Input Validation & Sanitization](https://github.com/cobacobiy/program1/issues/4) | `[x]` DONE — `validator` crate + `ValidatedJson` extractor |
| 5 | [#5 CORS Hardening & Security Headers](https://github.com/cobacobiy/program1/issues/5) | `[x]` DONE — CorsLayer + security_headers middleware |
| 6 | [#6 Rate Limiting & Abuse Protection](https://github.com/cobacobiy/program1/issues/6) | `[x]` DONE — Per-endpoint IP rate limiting |
| 7 | [#7 Audit Logging & Activity Trail](https://github.com/cobacobiy/program1/issues/7) | `[x]` DONE — AuditModule + SQLite persistence |
| 8 | [#8 Error Handling Standardization](https://github.com/cobacobiy/program1/issues/8) | `[x]` DONE — ApiError + ErrorCode enum + structured JSON responses |
| 9 | [#9 Legacy Cleanup & Code Hygiene](https://github.com/cobacobiy/program1/issues/9) | `[x]` DONE — Legacy product module removed |
| 10 | [#10 Environment Config & Secrets Management](https://github.com/cobacobiy/program1/issues/10) | `[x]` DONE — AppConfig with prod safety checks |
| 11 | [#11 API Versioning & Documentation (OpenAPI)](https://github.com/cobacobiy/program1/issues/11) | `[x]` DONE — utoipa + Swagger UI at /swagger-ui |
| 12 | [#12 Health Check & Observability](https://github.com/cobacobiy/program1/issues/12) | `[x]` DONE — /health + /health/ready + diagnostics workflow |

### 🆕 Phase 1 — Core E-Commerce Features (GitHub Issues #34 – #42)

| Local # | GitHub Issue | Prioritas | Estimasi | Status |
|---------|--------------|-----------|----------|--------|
| 13 | [#34 Live Chat Module (Backend)](https://github.com/cobacobiy/program1/issues/34) | 🟡 HIGH | 3-4 hari | `[x]` DONE (In PR) — Persistent Live Chat module & REST API (#34) |
| 14 | [#35 Payment Gateway Integration (Midtrans)](https://github.com/cobacobiy/program1/issues/35) | 🔴 CRITICAL | 4-5 hari | `[x]` DONE (In PR) — Payment Gateway Integration (Midtrans) |
| 15 | [#36 Order Status Workflow & State Machine](https://github.com/cobacobiy/program1/issues/36) | 🟡 HIGH | 2-3 hari | `[ ]` Open |
| 16 | [#37 Product Image Upload & File Storage](https://github.com/cobacobiy/program1/issues/37) | 🟡 HIGH | 2-3 hari | `[ ]` Open |
| 17 | [#38 Catalog Pagination, Search & Filtering](https://github.com/cobacobiy/program1/issues/38) | 🟡 HIGH | 2 hari | `[ ]` Open |
| 18 | [#39 Email Notification System](https://github.com/cobacobiy/program1/issues/39) | 🟢 MEDIUM | 2-3 hari | `[ ]` Open |
| 19 | [#40 Frontend Modularization (store.js Refactor)](https://github.com/cobacobiy/program1/issues/40) | 🟢 MEDIUM | 2-3 hari | `[ ]` Open |
| 20 | [#41 Buyer Order History & Profile Page](https://github.com/cobacobiy/program1/issues/41) | 🟡 HIGH | 2 hari | `[ ]` Open |
| 21 | [#42 CI/CD Pipeline Optimization](https://github.com/cobacobiy/program1/issues/42) | 🟡 HIGH | 1-2 hari | `[x]` DONE — Parallel validate + build-ghcr with cargo caching (#43) |

---

## 📐 Dependency Graph (Urutan Pengerjaan Phase 1)

```mermaid
graph TD
    I13["#34: Live Chat Module"]
    I14["#35: Payment Gateway"]
    I15["#36: Order Status Workflow"]
    I16["#37: Image Upload"]
    I17["#38: Pagination & Search"]
    I18["#39: Email Notifications"]
    I19["#40: Frontend Modularization"]
    I20["#41: Buyer Order History"]
    I21["#42: CI/CD Optimization"]

    I14 --> I15
    I14 --> I18
    I15 --> I20

    style I14 fill:#ff4444,color:#fff
    style I13 fill:#ffaa00,color:#000
    style I15 fill:#ffaa00,color:#000
    style I16 fill:#ffaa00,color:#000
    style I17 fill:#ffaa00,color:#000
    style I20 fill:#ffaa00,color:#000
    style I21 fill:#ffaa00,color:#000
    style I18 fill:#44bb44,color:#fff
    style I19 fill:#44bb44,color:#fff
```

### Urutan yang Disarankan:

1. **Parallel Batch A (Tidak saling depend)**:
   - Issue #34 (Live Chat) — bisa dikerjakan independen
   - Issue #35 (Payment Gateway) — **CRITICAL**, kerjakan pertama
   - Issue #37 (Image Upload) — bisa dikerjakan independen
   - Issue #38 (Pagination) — bisa dikerjakan independen
   - Issue #40 (Frontend Refactor) — bisa dikerjakan independen
   - Issue #42 (CI/CD Optimization) — **KERJAKAN DULUAN** supaya semua issue berikutnya deploy lebih cepat

2. **Sequential Batch B (Depend ke Batch A)**:
   - Issue #36 (Order Status) — depend ke Issue #35
   - Issue #39 (Email Notifications) — depend ke Issue #35
   - Issue #41 (Buyer Order History) — depend ke Issue #36

---

## 📏 Aturan Pengerjaan

1. **Setiap issue adalah Pull Request terpisah** — jangan gabung banyak issue dalam 1 PR
2. **Wajib `cargo test --workspace`** sebelum push — tidak boleh ada test yang gagal
3. **Ikuti Contract Isolation** — modul tidak boleh depend langsung ke internal modul lain
4. **Update `.env.example`** jika menambah environment variable baru
5. **Tulis unit test minimal 1** untuk setiap fitur/fungsi baru
6. **Ikuti aturan AGENTS.md** — No monolithic HTML, no inline Python overwrites

---

## 💰 Estimasi Budget (AI Agent / Junior Dev)

| Batch | Issues | Estimasi Total | Bisa Parallel |
|-------|--------|----------------|---------------|
| A | #13, #14, #16, #17, #19 | 13-17 hari | ✅ Ya (5 orang/agent) |
| B | #15, #18, #20 | 6-8 hari | ⚠️ Sebagian |
| **Total** | **8 issues** | **~19-25 hari** | — |

> Jika dikerjakan oleh 3 junior developer / AI agents secara parallel:
> **Estimasi selesai: ~2-3 minggu**

# Changelog

All notable changes to **Program1** (Rust Modular Monolith E-Commerce Platform) will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [1.2.0] - 2026-09-13

### 🏭 Supplier & Procurement Management (Issue #21)
- **Supplier Registry**: Full CRUD for supplier master data (`name`, `contact_person`, `phone`, `email`, `address`, `is_active`) via `crates/modules/supplier`.
- **Purchase Order (PO) Lifecycle**: Draft → Ordered → Received / Cancelled workflow with auto-generated `PO-XXXXXXXX` numbers.
- **PO Line Items**: Multi-item purchase orders linked to catalog products and variants with per-unit cost tracking.
- **REST API**: `GET/POST /api/v1/admin/suppliers`, `GET/PUT/DELETE /api/v1/admin/suppliers/:id`, `GET/POST /api/v1/admin/purchase-orders`, `GET /api/v1/admin/purchase-orders/:id`, `PUT /api/v1/admin/purchase-orders/:id/receive`, `PUT /api/v1/admin/purchase-orders/:id/cancel`.
- **Database Migrations**: `024_create_suppliers_and_po.sql` — `suppliers`, `purchase_orders`, `purchase_order_items` tables with foreign key constraints and indexes.

### 🔐 Granular Staff RBAC & Permission System (Issue #24)
- **Permission Table & Seeds**: `025_create_staff_permissions.sql` — `permissions` and `user_permissions` junction tables with 7 seeded permission types (`orders:manage`, `inventory:manage`, `catalog:write`, `chat:support`, `reports:view`, `promotions:manage`, `suppliers:manage`).
- **Axum Permission Middleware**: `require_permission("perm_name")` middleware layer on protected routes — Super Admin bypasses all permission checks; Staff users must hold the specific permission.
- **JWT Claims Extension**: `JwtClaims` now includes a `permissions: Vec<String>` field with `has_permission()` helper method for granular access control.
- **Staff Permission Management API**: `GET /api/v1/admin/permissions` (list available permissions), `GET/PUT /api/v1/users/accounts/:id/staff-permissions` (view/update staff user permissions).
- **Contract Integration**: `UserContract` extended with `list_permissions()`, `get_user_permissions()`, and `update_user_permissions()` methods; `SupplierContract` added to `crates/contracts`.

### 🏗️ Architecture Improvements
- **New Module**: `crates/modules/supplier` — implements `SupplierContract` with SQLx persistence.
- **AppState Extension**: `supplier_contract: Arc<dyn SupplierContract>` added to the web orchestrator state.
- **Admin Hub Integration**: Supplier management and staff RBAC permission panels integrated into the Svelte 5 SPA dashboard.

---

## [1.1.0] - 2026-09-12

### ⚡ Flash Sale Campaigns (Issue #18)
- **Session Management**: Create time-bound flash sale sessions with `start_time`, `end_time`, and `is_active` toggle.
- **Flash Sale Items**: Add products to sessions with `flash_price`, `flash_stock`, and `sold_count` tracking.
- **Storefront Integration**: Active flash sales auto-display on customer storefront with countdown timers.
- **REST API**: `GET/POST /api/v1/admin/flash-sales`, `POST /api/v1/admin/flash-sales/:id/items`, `PUT /api/v1/admin/flash-sales/:id/toggle`.
- **Database Migration**: `021_create_flash_sales.sql` — `flash_sale_sessions` and `flash_sale_items` tables.

### 🎯 Loyalty Points & Rewards Program (Issue #19)
- **Point Accumulation**: Automatic loyalty point credits on completed orders with configurable earning rate.
- **Point Redemption**: Buyers can redeem accumulated points for order discounts.
- **Points Ledger**: Full transaction history with `earn`, `redeem`, and `expire` event types.
- **Database Migration**: `022_create_loyalty_points.sql` — `loyalty_points_ledger` table.

### 🔍 Full-Text Search & Autocomplete (Issue #20)
- **SQLite FTS5**: Content-sync FTS5 virtual table for instant product search across `name`, `description`, and `sku`.
- **Search Autocomplete API**: `GET /api/v1/search?q=...` with relevance-ranked results and highlighting.
- **Storefront Search Bar**: Integrated real-time search with autocomplete dropdown in the SPA.
- **Database Migration**: `023_create_search_fts.sql` — FTS5 virtual table and content-sync triggers.

### 💾 Database Backup & Recovery Manager (Issue #22)
- **On-Demand Backup**: Create SQLite database snapshots with timestamped filenames via admin API.
- **Backup Listing**: Browse available backups with file size and creation metadata.
- **Backup Download**: Download backup files directly from the admin dashboard.
- **Database Health Check**: Subsystem health verification endpoint reporting storage metrics.
- **REST API**: `POST /api/v1/admin/database/backup`, `GET /api/v1/admin/database/backups`, `GET /api/v1/admin/database/backups/:filename/download`, `GET /api/v1/admin/database/health`.

### 🌐 Dynamic SEO & Sitemap (Issue #23)
- **Dynamic Sitemap.xml**: Auto-generated sitemap including product pages, category pages, and store URLs.
- **SEO Meta Tags**: Server-side rendered `<title>`, `<meta description>`, and Open Graph tags for product and storefront pages.

---

## [1.0.1] - 2026-09-12

### 🎨 UI & UX Improvements
- **Tactile Buy Button Feedback**: Added realistic press-down effect (`translateY(2px) scale(0.92)`), dynamic hover elevation, and smooth cubic-bezier transitions on all "Beli" / "Tambah ke Keranjang" action buttons across both Svelte 5 SPA and `/store` storefront.
- **Post-Click Confirmation**: Implemented instant emerald green transition with animated `✓ Ditambahkan!` label feedback for 1 second upon adding items.
- **Cart Badge Animation**: Added dynamic bounce & shake micro-animations to the cart trigger and count badge upon successful cart additions.
- **Wishlist & Modal Enhancements**: Applied enhanced tactile styles to product detail modals and wishlist cart action buttons.
- **i18n Dictionary Parity**: Synchronized updated Indonesian and English translation dictionaries (`id.json`, `en.json`) across backend static assets and frontend SPA.

---

## [1.0.0] - 2026-09-11

### 🎉 Initial Production Release

Program1 is an enterprise-grade high-performance **Modular Monolith** e-commerce application built in **Rust** (Axum, SQLx, Tokio) with a modern **Svelte 5** Single Page Application (SPA) frontend.

---

### ✨ Core Highlights

#### 🏢 Architecture & Foundation (Phase 0)
- **Modular Monolith Design**: Decoupled domain crates (`user`, `auth`, `catalog`, `inventory`, `order`, `payment`, `chat`, `review`, `shipping`, `notification`, `return`, `coupon`, `analytics`, `audit`) interacting solely through async traits defined in `crates/contracts`.
- **Database Architecture**: SQLite / PostgreSQL support via SQLx with automated migrations.
- **Security & Cryptography**: Password hashing using Argon2id, JWT bearer token authentication, secure input sanitization.
- **Automated CI/CD**: Multiplatform Docker builds, GitHub Actions pipelines, deployment to self-hosted production runner.

#### 🛒 E-Commerce & Marketplace Operations (Phase 1)
- **Catalog Management**: Hierarchical product categories, multi-variant products (SKU, attributes, pricing), stock alerts.
- **Order Lifecycle**: Multi-item cart, checkout workflows, atomic stock reservation, status transitions (`pending` → `processing` → `shipped` → `delivered` / `cancelled`).
- **Inventory & Channel Sync**: Real-time stock reservation, safety stock management, marketplace sync simulation.
- **Audit Logging**: Immutable event auditing tracking administrative and transactional operations.

#### 🚀 Advanced Domain Capabilities (Phase 2)
- **Payment Gateway**: Midtrans integration with Snap token generation and automated webhook callback handling.
- **Real-Time Customer Support**: WebSocket-powered live chat between buyers and admin support.
- **Product Reviews & Ratings**: Verified purchase rating system with text reviews, media attachments, and moderation.
- **Logistics & Shipping**: Automated shipping rate calculation and tracking status updates.
- **Marketing & Coupons**: Percentage and fixed discount promo codes with usage limit enforcement.
- **Notification Engine**: In-app notifications and simulated WhatsApp dispatch for critical order updates.
- **Return & Refund System**: Comprehensive buyer return request lifecycle with seller approval/rejection workflows (#61).

#### 🎨 Modern Frontend Experience (Phase 3)
- **Svelte 5 SPA**: Built with modern Runes (`$state`, `$derived`, `$effect`) and Vite.
- **Rich Dashboard UI**: Interactive catalog browsing, multi-lingual interface (ID/EN i18n), buyer profile management, responsive modal flows.
- **Zero Monolithic Bloat**: Strictly modularized HTML, CSS, and component structure.

---

### 📦 Installation & Deployment
```bash
# Clone the repository
git clone https://github.com/cobacobiy/program1.git
cd program1

# Deploy via Docker Compose
docker compose up -d --build
```
Web application dashboard is served at `http://localhost:6090` (or `http://localhost:8080` internally).

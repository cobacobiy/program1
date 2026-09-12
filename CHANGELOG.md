# Changelog

All notable changes to **Program1** (Rust Modular Monolith E-Commerce Platform) will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

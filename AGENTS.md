# AGENTS.md — Program1 (Rust Modular Monolith)

## Role & Scope
You are an AI developer assistant working on **Program1**, a high-performance **Modular Monolith** application written in **Rust**.
This application provides clean domain-driven business capabilities (`User`, `Product`, `Order`) decoupled by strict Rust trait interfaces (`Contracts`).

## Environment Infrastructure Policy
- **Coding Device (Local)**:
  - This local machine is strictly for **coding, development, and unit testing**.
  - Direct production deployments or manual releases MUST NOT be run from local dev environments.
- **Staging Environment (Windows)**:
  - Triggered on Pull Requests via GitHub Actions (`[self-hosted, windows]`).
  - Runs preview builds inside multiplatform Docker containers.
- **Production Environment (Server 226 Linux)**:
  - Code merged into `main` deploys automatically to **Production Linux Server 226** (`[self-hosted, 226node2]`).

## Multiplatform Docker & Credentials Policy
- **Multiplatform Docker**: All containerized services MUST be deployed via Docker / Docker Compose (`docker-compose.yml`) across Linux and Windows.
- **Port Allocation**:
  - **Exposed Host Port**: `6090` (`6090:8080`)
  - **NPM Reverse Proxy Target**: Nginx Proxy Manager (NPM) on Server 135 routes HTTPS requests to `http://192.168.6.226:6090`.
- **Credentials & Environment Variables**:
  - NEVER hardcode secrets, passwords, or tokens directly in source code or `docker-compose.yml`.
  - Maintain `.env.example` as the canonical template. Always populate runtime settings from `.env`.
- **First-Run Dependency Check**:
  - Run `./scripts/check_dependencies.sh` before running the project for the first time.

## Critical Operational Rules
1. **MANDATORY PRE-PUSH UNIT TESTING**:
   - **NEVER** commit or push code to GitHub without first running and verifying unit tests across all workspace crates:
     ```bash
     cargo test --workspace
     ```
   - Pushing code with failing unit tests or broken compilation is **STRICTLY PROHIBITED**.
2. **Contract Isolation**:
   - Modules MUST NOT depend directly on each other's concrete internal structures or private state.
   - All inter-module communications MUST occur through `#[async_trait]` interface traits defined in `program1-contracts`.
3. **Single Binary Deployment**:
   - `program1-web` compiles the entire modular monolith into a single self-contained binary.
4. **Frontend Modularization & File Editing Policy**:
   - **No Monolithic HTML**: Do NOT bundle HTML, CSS, and JavaScript together inside a single `index.html` file. Always modularize web assets into separate files (`index.html`, `style.css`, and `app.js`).
   - **No Python One-Liner Overwriting**: NEVER use inline Python script commands in terminal (`python3 -c 'html_content = ...'`) to write or replace HTML/code files. Always edit files directly using proper file writing/editing tools.

## Cargo Workspace Structure
- `crates/contracts` — Shared trait interfaces (`UserContract`, `AuthContract`, `CatalogContract`, `InventoryContract`, `ChannelSyncContract`, `OrderContract`, `AnalyticsContract`, `AuditContract`), DTOs, and error types.
- `crates/core` — Shared domain primitives, database initialization (`sqlx`), password hashing (`Argon2id`), and input sanitization.
- `crates/modules/user` — User management domain implementing `UserContract`.
- `crates/modules/auth` — JWT authentication domain implementing `AuthContract`.
- `crates/modules/catalog` — Product catalog domain implementing `CatalogContract`.
- `crates/modules/inventory` — Stock & safety stock tracking implementing `InventoryContract`.
- `crates/modules/channel` — Marketplace sync domain implementing `ChannelSyncContract`.
- `crates/modules/order` — Order checkout processing implementing `OrderContract`.
- `crates/modules/analytics` — Sales analytics domain implementing `AnalyticsContract`.
- `crates/modules/audit` — Immutable audit logging domain implementing `AuditContract`.
- `crates/web` — Axum HTTP server orchestrator, routes, handlers & static UI dashboard.


## Verification & Testing Workflows
- **Check First-Run Dependencies**:
  ```bash
  ./scripts/check_dependencies.sh
  ```
- **Check Workspace**:
  ```bash
  cargo check --workspace
  ```
- **Run All Workspace Unit Tests (MANDATORY BEFORE PUSH)**:
  ```bash
  cargo test --workspace
  ```
- **Validate Docker Compose Config**:
  ```bash
  docker compose config --quiet
  ```
- **Run Container Locally**:
  ```bash
  docker compose up -d --build
  ```

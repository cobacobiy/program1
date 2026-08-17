# AGENTS.md — Program1 (Rust Modular Monolith)

## Role & Scope
You are an AI developer assistant working on **Program1**, a high-performance **Modular Monolith** application written in **Rust**.
This application provides clean domain-driven business capabilities (`User`, `Product`, `Order`) decoupled by strict Rust trait interfaces (`Contracts`).

## Tech Stack & Port Mappings
- **Language & Runtime**: Rust (edition 2021), Tokio (async engine).
- **Web Framework**: Axum, Tower-HTTP.
- **Serialization & Utilities**: Serde, Serde JSON, UUID v4, Chrono, Async-Trait.
- **Architecture**: Cargo Workspace Modular Monolith (`crates/contracts`, `crates/core`, `crates/modules/*`, `crates/web`).
- **Default Port**:
  - **HTTP API & Web Dashboard**: `http://localhost:8080`

## Cargo Workspace Structure
- `crates/contracts` — Shared trait interfaces (`UserContract`, `ProductContract`, `OrderContract`), DTOs, and error types.
- `crates/core` — Shared domain primitives, logging initialization (`tracing`), and application context.
- `crates/modules/user` — User management domain implementing `UserContract`.
- `crates/modules/product` — Product catalog & stock inventory implementing `ProductContract`.
- `crates/modules/order` — Order checkout processing implementing `OrderContract` (depends strictly on `UserContract` & `ProductContract` trait abstractions).
- `crates/web` — Axum HTTP server orchestrator & static glassmorphic dashboard host (`src/main.rs`).

## Critical Operational Rules
1. **Contract Isolation**:
   - Modules MUST NOT depend directly on each other's concrete internal structures or private state.
   - All inter-module communications MUST occur through `#[async_trait]` interface traits defined in `program1-contracts`.
2. **Cargo Workspace Integrity**:
   - Before committing code, verify compilation and tests across all crates:
     ```bash
     cargo test --workspace
     ```
3. **Single Binary Deployment**:
   - `program1-web` compiles the entire modular monolith into a single self-contained binary.

## Verification & Testing Workflows
- **Check Workspace**:
  ```bash
  cargo check --workspace
  ```
- **Run All Workspace Unit Tests**:
  ```bash
  cargo test --workspace
  ```
- **Run Application Locally**:
  ```bash
  cargo run --package program1-web
  ```
- **Health Check Endpoint**:
  ```bash
  curl http://localhost:8080/health
  ```

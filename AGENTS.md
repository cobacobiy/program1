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
- **Production Environment (Server 135 Linux)**:
  - Code merged into `main` deploys automatically to **Production Linux Server 135** (`[self-hosted, 135gemini2]`).

## Multiplatform Docker & Credentials Policy
- **Multiplatform Docker**: All containerized services MUST be deployed via Docker / Docker Compose (`docker-compose.yml`) across Linux and Windows.
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

## Tech Stack & Port Mappings
- **Language & Runtime**: Rust (edition 2021), Tokio (async engine).
- **Web Framework**: Axum, Tower-HTTP.
- **Serialization & Utilities**: Serde, Serde JSON, UUID v4, Chrono, Async-Trait.
- **Architecture**: Cargo Workspace Modular Monolith (`crates/contracts`, `crates/core`, `crates/modules/*`, `crates/web`).
- **Default Port**:
  - **HTTP API & Web Dashboard**: `http://localhost:8080`

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

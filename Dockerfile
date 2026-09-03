# Multi-stage Dockerfile for Program1 Rust Modular Monolith
# Uses cargo-chef for dependency caching to avoid rebuilding 200+ crates on every edit

# --- Stage 1: Chef base ---
FROM lukemathwalker/cargo-chef:latest-rust-bookworm AS chef
WORKDIR /usr/src/program1

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# --- Stage 2: Recipe planner ---
FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo chef prepare --recipe-path recipe.json

# --- Stage 3: Builder ---
FROM chef AS builder
COPY --from=planner /usr/src/program1/recipe.json recipe.json
# Build & cache dependencies (this layer is CACHED as long as Cargo.lock doesn't change!)
RUN cargo chef cook --release --package program1-web --recipe-path recipe.json

# Copy full source tree and migrations
COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY crates ./crates

# Build only workspace application code (takes ~15 seconds instead of 4 minutes!)
RUN cargo build --release --package program1-web

# --- Stage 4: Minimal Runtime Image ---
FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy compiled binary from builder
COPY --from=builder /usr/src/program1/target/release/program1 /app/program1

# Copy static Web UI assets
COPY crates/web/static /app/crates/web/static

ENV RUST_LOG=info,program1=debug
ENV APP_PORT=8080

EXPOSE 8080

CMD ["/app/program1"]

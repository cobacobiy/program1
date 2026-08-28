# Multi-stage Dockerfile for Program1 Rust Modular Monolith

# --- Stage 1: Build binary ---
FROM rust:slim-bookworm AS builder

WORKDIR /usr/src/program1

# Install build dependencies including curl, pkg-config, OpenSSL, and ca-certificates
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy Cargo manifests, migrations, and crate sources
COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY crates ./crates

# Build release binary
RUN cargo build --release --package program1-web

# --- Stage 2: Minimal Runtime Image ---
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

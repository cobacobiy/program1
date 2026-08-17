#!/usr/bin/env bash
set -e

echo "=============================================="
echo " Program1 Dependency & First-Run Checker"
echo "=============================================="

# 1. Check Cargo / Rust
if command -v cargo &> /dev/null; then
    echo "[OK] Cargo found: $(cargo --version)"
else
    echo "[WARN] Cargo not found. Please install Rust toolchain (https://rustup.rs)."
fi

# 2. Check Docker
if command -v docker &> /dev/null; then
    echo "[OK] Docker found: $(docker --version)"
else
    echo "[WARN] Docker not found. Multiplatform Docker deployments will be unavailable."
fi

# 3. Check Docker Compose
if docker compose version &> /dev/null; then
    echo "[OK] Docker Compose found: $(docker compose version)"
else
    echo "[WARN] Docker Compose plugin not found."
fi

# 4. Check .env file
if [ ! -f ".env" ]; then
    echo "[INFO] .env file not found. Copying from .env.example..."
    cp .env.example .env
    echo "[OK] .env file created."
else
    echo "[OK] .env file exists."
fi

echo "----------------------------------------------"
echo " All dependency checks complete."
echo "=============================================="

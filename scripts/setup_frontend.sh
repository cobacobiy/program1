#!/usr/bin/env bash
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$DIR"

echo "=== Program1 Frontend Setup ==="

# 1. Ensure Bun is available
if ! command -v bun &> /dev/null && [ ! -f "$HOME/.bun/bin/bun" ]; then
    echo "[1/4] Installing Bun..."
    curl -fsSL https://bun.sh/install | bash
fi

export PATH="$HOME/.bun/bin:$PATH"

if ! command -v bun &> /dev/null; then
    echo "Failed to find bun in PATH"
    exit 1
fi

echo "[2/4] Bun ready: $(bun --version)"

# 2. Setup frontend directory if not exists
if [ ! -d "frontend" ] || [ ! -f "frontend/package.json" ]; then
    echo "[3/4] Initializing frontend with Svelte 5 + Vite..."
    # Create frontend using bun create vite with svelte-ts
    bun create vite frontend --template svelte-ts
fi

cd "$DIR/frontend"

# 3. Install dependencies
echo "[4/4] Installing frontend dependencies via Bun..."
bun install

echo "Frontend setup complete!"

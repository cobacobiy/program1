# Issue #21 — CI/CD Pipeline Optimization (Faster Deployments)

> **Prioritas**: 🟡 HIGH
> **Estimasi**: 1-2 hari
> **Depends On**: —
> **Skill Level**: Junior DevOps / CI-CD

---

## 🔍 Masalah Saat Ini

Deployment ke production **sangat lambat** karena pipeline CI/CD melakukan **Rust compilation 2x lipat** secara sequential:

### Current Pipeline Flow (Serial):
```
┌─────────────────────────────────────────────────────────────────────────┐
│ validate (~5-8 min)                                                      │
│   ├── cargo check --workspace       ← Compile #1 (check)               │
│   └── cargo test --workspace        ← Compile #1 (test, full build)    │
└──────────────────────────────────┬──────────────────────────────────────┘
                                   ↓ waits
┌──────────────────────────────────┴──────────────────────────────────────┐
│ build-ghcr (~5-10 min)                                                   │
│   └── Docker Build (cargo-chef)  ← Compile #2 (release, inside Docker) │
└──────────────────────────────────┬──────────────────────────────────────┘
                                   ↓ waits
┌──────────────────────────────────┴──────────────────────────────────────┐
│ deploy-production (~1-2 min)                                             │
│   └── docker pull + docker compose up                                    │
└─────────────────────────────────────────────────────────────────────────┘

Total: ~11-20 menit per push ke main 😱
```

### Root Causes:

| Masalah | Impact |
|---------|--------|
| **Double compilation** — `cargo test` di ubuntu + `cargo build --release` di Docker | +5-8 menit sia-sia |
| **Serial execution** — `build-ghcr` menunggu `validate` selesai | Blocking total |
| **`cargo check` + `cargo test`** — `cargo check` redundant karena `cargo test` sudah compile | +1-2 menit |
| **Tidak ada test caching antar run** — walaupun pakai `rust-cache`, tetap re-compile setiap push | Slow CI |

---

## ✅ Acceptance Criteria

### Option A: Test Di Dalam Docker Build (Recommended — Paling Simpel)

Pindahkan `cargo test` ke dalam Dockerfile sebagai build stage, sehingga hanya ada **1x compilation**:

```yaml
# ci-cd.yml — SIMPLIFIED
jobs:
  build-and-test:
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write
    steps:
      - uses: actions/checkout@v4

      - uses: docker/setup-buildx-action@v3

      - uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Build, Test & Push Docker Image
        uses: docker/build-push-action@v6
        with:
          context: .
          file: ./Dockerfile
          push: true
          provenance: false
          tags: |
            ghcr.io/cobacobiy/program1:latest
            ghcr.io/cobacobiy/program1:${{ github.sha }}
          cache-from: type=gha
          cache-to: type=gha,mode=max

  deploy-production:
    needs: build-and-test
    # ... same as before
```

**Dockerfile perubahan — tambahkan test stage:**

```dockerfile
# --- Stage 3: Builder ---
FROM chef AS builder
COPY --from=planner /usr/src/program1/recipe.json recipe.json
RUN cargo chef cook --release --package program1-web --recipe-path recipe.json

COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY crates ./crates

# Run tests FIRST (fails build if tests fail)
RUN cargo test --workspace

# Then build release binary
RUN cargo build --release --package program1-web
```

**Benefit**: Compile sekali, test + build release sekaligus. ~5-8 menit total.

### Option B: Parallel Jobs (Lebih Cepat, Tapi Test Terpisah)

Jalankan `validate` dan `build-ghcr` **secara parallel**, deploy hanya jika keduanya pass:

```yaml
jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      # Skip cargo check — cargo test already compiles
      - run: cargo test --workspace

  build-ghcr:
    # REMOVE "needs: validate" — run in parallel!
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write
    steps:
      - uses: actions/checkout@v4
      - uses: docker/setup-buildx-action@v3
      - uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}
      - uses: docker/build-push-action@v6
        with:
          context: .
          push: true
          tags: ghcr.io/cobacobiy/program1:latest
          cache-from: type=gha
          cache-to: type=gha,mode=max

  deploy-production:
    needs: [validate, build-ghcr]   # ← Wait for BOTH
    runs-on: [self-hosted, 226node2]
    # ... same deploy steps
```

**Benefit**: Validate dan Build jalan bersamaan. Deploy dimulai setelah keduanya selesai.  
**Total time**: max(validate, build-ghcr) + deploy = ~7-10 menit (bukan 15-20 menit).

### Option C: Skip Test di CI, Hanya Test Lokal (Paling Cepat, Tapi Risiko Lebih Tinggi)

Jika ingin deploy secepat mungkin dan percaya bahwa developer sudah `cargo test --workspace` sebelum push (sesuai aturan AGENTS.md):

```yaml
jobs:
  build-and-deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: docker/setup-buildx-action@v3
      # ... login to GHCR
      - uses: docker/build-push-action@v6
        with:
          push: true
          tags: ghcr.io/cobacobiy/program1:latest
          cache-from: type=gha
          cache-to: type=gha,mode=max

  deploy:
    needs: build-and-deploy
    runs-on: [self-hosted, 226node2]
    # ... deploy
```

**Benefit**: Paling cepat (~5-7 menit total).  
**Risiko**: Jika developer lupa test lokal, broken code bisa masuk production.

---

## 🏆 Rekomendasi: **Option A** (Test Di Dalam Docker Build)

| Aspek | Option A ✅ | Option B | Option C |
|-------|------------|----------|----------|
| Kecepatan | ~5-8 min | ~7-10 min | ~5-7 min |
| Safety | ✅ Test wajib pass | ✅ Test wajib pass | ⚠️ Tergantung developer |
| Complexity | Simpel (1 job) | Medium (3 jobs) | Simpel (2 jobs) |
| Double compile | ❌ Tidak | ✅ Ya (2x) | ❌ Tidak |

---

## 📋 Langkah Implementasi (Option A)

### Step 1: Update `Dockerfile`

Tambahkan `RUN cargo test --workspace` sebelum `cargo build --release`:

```dockerfile
# --- Stage 3: Builder ---
FROM chef AS builder
COPY --from=planner /usr/src/program1/recipe.json recipe.json
RUN cargo chef cook --release --package program1-web --recipe-path recipe.json

COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY crates ./crates

# ✅ NEW: Run unit tests inside Docker (fails build if tests fail)
RUN cargo test --workspace 2>&1 | tail -50

# Build optimized release binary
RUN cargo build --release --package program1-web
```

### Step 2: Simplify `ci-cd.yml`

Gabungkan `validate` + `build-ghcr` menjadi 1 job:

```yaml
name: CI/CD Pipeline — Program1

on:
  push:
    branches: [ main ]
    paths-ignore:
      - '**.md'
      - '.dockerignore'
      - '.gitignore'
      - 'docs/**'
      - 'issues/**'
  pull_request:
    branches: [ main ]
    paths-ignore:
      - '**.md'
      - 'issues/**'
  workflow_dispatch:

jobs:
  build-test-push:
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write
    steps:
      - name: Checkout Code
        uses: actions/checkout@v4

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Log in to GHCR
        uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Build, Test & Push Image
        uses: docker/build-push-action@v6
        with:
          context: .
          file: ./Dockerfile
          push: true
          provenance: false
          tags: |
            ghcr.io/cobacobiy/program1:latest
            ghcr.io/cobacobiy/program1:${{ github.sha }}
          cache-from: type=gha
          cache-to: type=gha,mode=max

  # PR validation (no push, no deploy)
  validate-pr:
    if: github.event_name == 'pull_request' && github.event.action != 'closed'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --workspace

  deploy-production:
    needs: build-test-push
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'
    runs-on: [self-hosted, 226node2]
    steps:
      - name: Checkout Code
        uses: actions/checkout@v4

      - name: Deploy Container from GHCR
        shell: bash
        run: |
          echo "Deploying Production Container from GHCR..."
          if [ ! -f ".env" ]; then
              cp .env.example .env
          fi
          sed -i 's/6281234567890/085810007735/g' .env || true
          docker pull ghcr.io/cobacobiy/program1:latest || true
          docker compose up -d --force-recreate --remove-orphans
          echo "Verifying container state..."
          sleep 3
          docker ps -a --filter "name=program1"
          echo "=== CONTAINER LOGS ==="
          docker logs --tail 100 program1-app || true
          echo "=== CURL LOCAL 6090 HEALTH ==="
          curl -ivs --max-time 5 http://localhost:6090/health || true
          echo "Production deployment complete!"
```

### Step 3: Tambahkan `issues/**` ke `paths-ignore`

Supaya push issue files `.md` tidak trigger CI pipeline:

```yaml
paths-ignore:
  - '**.md'
  - 'issues/**'       # ← NEW
```

### Step 4: Hapus Redundant `cargo check`

`cargo test` sudah melakukan kompilasi penuh, jadi `cargo check` sebelumnya adalah **redundant** dan buang waktu ~1-2 menit.

---

## 📁 File Yang Harus Diubah

| Action | File |
|--------|------|
| **MODIFY** | `.github/workflows/ci-cd.yml` (merge validate + build, add paths-ignore) |
| **MODIFY** | `Dockerfile` (add `cargo test` stage) |

---

## ⚠️ Catatan Penting

- **`cargo test` di dalam Docker** berarti test environment = production environment. Ini lebih reliable.
- Jika test gagal, Docker build gagal → image TIDAK di-push → deploy TIDAK terjadi. Safe.
- `cargo chef cook` sudah caching dependencies, jadi `cargo test` cuma compile application code (~15-30 detik setelah chef cache hit).
- PR tetap punya `validate-pr` job terpisah untuk developer feedback cepat.
- Pastikan `cargo test --workspace` pass sebelum push.

---

## 📊 Expected Improvement

| Metric | Before | After (Option A) |
|--------|--------|-------------------|
| Total pipeline time | ~15-20 min | ~7-10 min |
| Rust compilations | 2x (validate + Docker) | 1x (Docker only) |
| Jobs count (push to main) | 3 sequential | 2 sequential |
| CI minutes cost | ~15-20 min/push | ~7-10 min/push |

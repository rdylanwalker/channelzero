set shell := ["bash", "-euo", "pipefail", "-c"]

rust_image := "channelzero-rust-dev"
node_image := "node:24-slim"
playwright_image := "mcr.microsoft.com/playwright:v1.65.0-noble"
cargo_cache := "channelzero-cargo-cache"

# Build the cached Rust dev container (rustfmt, clippy, cargo-llvm-cov)
dev-image:
    docker build -f Dockerfile.dev -t {{rust_image}} .

# --- Rust (containerized) ---

rust-fmt:
    docker run --rm --user "$(id -u):$(id -g)" -e HOME=/tmp \
        -v "$PWD:/work" -v "{{cargo_cache}}:/usr/local/cargo/registry" -w /work \
        {{rust_image}} cargo fmt --check

rust-fmt-fix:
    docker run --rm --user "$(id -u):$(id -g)" -e HOME=/tmp \
        -v "$PWD:/work" -v "{{cargo_cache}}:/usr/local/cargo/registry" -w /work \
        {{rust_image}} cargo fmt

rust-clippy:
    docker run --rm --user "$(id -u):$(id -g)" -e HOME=/tmp \
        -v "$PWD:/work" -v "{{cargo_cache}}:/usr/local/cargo/registry" -w /work \
        {{rust_image}} cargo clippy --all-targets -- -D warnings

rust-test:
    docker run --rm --user "$(id -u):$(id -g)" -e HOME=/tmp \
        -v "$PWD:/work" -v "{{cargo_cache}}:/usr/local/cargo/registry" -w /work \
        {{rust_image}} cargo test

rust-cov:
    docker run --rm --user "$(id -u):$(id -g)" -e HOME=/tmp \
        -v "$PWD:/work" -v "{{cargo_cache}}:/usr/local/cargo/registry" -w /work \
        {{rust_image}} cargo llvm-cov --summary-only

rust-build:
    docker run --rm --user "$(id -u):$(id -g)" -e HOME=/tmp \
        -v "$PWD:/work" -v "{{cargo_cache}}:/usr/local/cargo/registry" -w /work \
        {{rust_image}} cargo build --release

# --- Frontend & Formatting (Node 24 container) ---

fe-install:
    docker run --rm --user "$(id -u):$(id -g)" -e HOME=/tmp \
        -v "$PWD:/work" -w /work/frontend \
        {{node_image}} npm ci

fe-lint:
    docker run --rm --user "$(id -u):$(id -g)" -e HOME=/tmp \
        -v "$PWD:/work" -w /work/frontend \
        {{node_image}} sh -c "npm run lint && npm run typecheck"

fe-test:
    docker run --rm --user "$(id -u):$(id -g)" -e HOME=/tmp \
        -v "$PWD:/work" -w /work/frontend \
        {{node_image}} npm test

fe-cov:
    docker run --rm --user "$(id -u):$(id -g)" -e HOME=/tmp \
        -v "$PWD:/work" -w /work/frontend \
        {{node_image}} npx vitest run --coverage

fe-build:
    docker run --rm --user "$(id -u):$(id -g)" -e HOME=/tmp \
        -v "$PWD:/work" -w /work/frontend \
        {{node_image}} npm run build

prettier:
    docker run --rm --user "$(id -u):$(id -g)" -e HOME=/tmp \
        -v "$PWD:/work" -w /work \
        {{node_image}} frontend/node_modules/.bin/prettier --check .

prettier-fix:
    docker run --rm --user "$(id -u):$(id -g)" -e HOME=/tmp \
        -v "$PWD:/work" -w /work \
        {{node_image}} frontend/node_modules/.bin/prettier --write .

# --- Playwright E2E (Playwright container) ---

e2e: fe-build rust-build
    docker run --rm --ipc=host --user "$(id -u):$(id -g)" -e HOME=/tmp -e CI=1 \
        -v "$PWD:/work" -w /work/playwright \
        {{playwright_image}} sh -c "npm run typecheck && npm run test:e2e"

# Capture README screenshot from the running local instance (e.g. `docker compose up -d`)
screenshot:
    docker run --rm --ipc=host --network=host --user "$(id -u):$(id -g)" -e HOME=/tmp -e CI=1 \
        -e CHANNELZERO_BASE_URL=http://127.0.0.1:8080 \
        -v "$PWD:/work" -w /work/playwright \
        {{playwright_image}} npm run screenshot

# --- Aggregate Workflows ---

lint: rust-fmt rust-clippy fe-lint prettier

test: rust-test fe-test

ci: lint test e2e

# --- Local Container Watch ---

watch:
    docker compose up --watch --build

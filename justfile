# Developer entry points. CI calls the same recipes so "green locally" means "green in CI".
set shell := ["bash", "-euo", "pipefail", "-c"]

default:
    @just --list

# --- local environment ---------------------------------------------------------------------

# Create .env with a random local Postgres password (never committed; see .env.example).
env:
    @if [ ! -f .env ]; then \
      printf 'POSTGRES_PASSWORD=%s\nPOSTGRES_PORT=5432\n' "$(od -An -N16 -tx1 /dev/urandom | tr -d ' \n')" > .env; \
      echo "created .env"; \
    fi

# Start local Postgres 17 (127.0.0.1 only) and wait until it is healthy.
db-up: env
    docker compose -f compose.dev.yml up -d --wait postgres

db-down:
    docker compose -f compose.dev.yml down

# Postgres + web + worker with auto-reload (needs `cargo install cargo-watch`).
dev: db-up
    #!/usr/bin/env bash
    set -euo pipefail
    command -v cargo-watch >/dev/null || { echo "cargo-watch not installed: cargo install --locked cargo-watch"; exit 1; }
    trap 'kill 0' EXIT
    cargo watch -q -x 'run -p web' &
    cargo watch -q -x 'run -p worker' &
    wait

# --- quality gates -------------------------------------------------------------------------

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all --check

clippy:
    cargo clippy --workspace --all-targets --locked -- -D warnings

# Architecture guard-rails: dependency direction, no string-built SQL, and their self-tests.
arch-checks:
    scripts/check-deps.sh
    scripts/check-no-sql-format.sh
    scripts/tests/selftest.sh

deny:
    cargo deny --locked check

audit:
    cargo audit --deny warnings

lint: fmt-check clippy arch-checks deny audit

test:
    cargo nextest run --workspace --locked --no-tests=pass
    cargo test --workspace --locked --doc
    scripts/sqlx-check.sh

# --- UI assets -----------------------------------------------------------------------------

# Regenerate crates/ui/assets/app.css from crates/ui/style/tailwind.css (commit the result).
css:
    "$(scripts/install-tailwind.sh)" -i crates/ui/style/tailwind.css -o crates/ui/assets/app.css --minify

# Fail if the committed app.css is stale (CI). Also runs after any change to a component's classes.
css-check:
    #!/usr/bin/env bash
    set -euo pipefail
    tmp="$(mktemp)"
    trap 'rm -f "$tmp"' EXIT
    "$(scripts/install-tailwind.sh)" -i crates/ui/style/tailwind.css -o "$tmp" --minify
    if ! cmp -s "$tmp" crates/ui/assets/app.css; then
      echo "crates/ui/assets/app.css is stale: run 'just css' and commit the result" >&2
      exit 1
    fi

# Playwright + axe against the real server (builds and starts `web` itself). Needs Chrome and
# `npm ci --prefix e2e` once. SCREENSHOTS=1 also writes e2e/screenshots/.
e2e:
    cd e2e && npx playwright test

# Lighthouse mobile run against a release build of the shell page (Perf >= 90, A11y >= 95).
lighthouse:
    cargo build --release --locked -p web
    cd e2e && node lighthouse-check.mjs

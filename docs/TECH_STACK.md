# Technology Stack

Status: PROPOSED. Each choice links to an ADR in `docs/adr/`.

| Layer | Choice | Why | Rejected alternatives | ADR |
|---|---|---|---|---|
| Language | Rust stable (pinned via `rust-toolchain.toml`) | Mandated; strong types for money and state machines | — | 0001 |
| Architecture | Modular monolith, 2 binaries (`web`, `worker`) | ACID across the money path; low ops cost | Microservices (premature); single binary (job crashes would take down HTTP) | 0002 |
| HTTP | Axum 0.8 + Tower + Tokio | De-facto standard; middleware ecosystem (timeouts, limits, tracing) | Actix-web (fine, smaller tower ecosystem) | 0001 |
| UI | Leptos 0.8 SSR + hydration (`cargo-leptos`) | Single language; SSR for SEO/first paint; islands keep WASM small | HTMX + Askama (simpler, but weaker for interactive wallet/admin); separate SPA (would need Node build + 2nd language) | 0003 |
| CSS | Tailwind CSS v4 via standalone CLI binary | No Node runtime in prod or CI; design tokens as CSS variables | Hand-written CSS | 0003 |
| DB | PostgreSQL 17 | Transactions, constraints, `SKIP LOCKED`, partitions, JSONB for raw events | MySQL; event store | 0004 |
| DB access | SQLx (compile-time checked, offline mode in CI) | SQL stays explicit and reviewable for money; no ORM magic | Diesel, SeaORM | 0004 |
| Queue | Postgres `jobs` table + `SKIP LOCKED` | One stateful dependency; transactional enqueue | Redis/Sidekiq-style, RabbitMQ, Kafka | 0005 |
| Cache / rate limit | Postgres tables + in-process `moka` for hot read-only config | Redis is unjustified at V1 volumes | Redis (deferred; revisit at ~50 req/s sustained on rate-limit paths) | 0004 |
| HTTP client | Reqwest + rustls; custom resolver for SSRF guard | No OpenSSL; we control DNS for `safe_fetch` | hyper directly | 0007 |
| Serialization | Serde, `serde_json` | Standard | — | — |
| IDs | UUIDv7 (`uuid` crate) newtypes; opaque click tokens = 96-bit random, Crockford base32 | Time-ordered PKs; tokens unguessable and unlinkable to users | Sequential IDs in URLs (IDOR/enumeration) | 0006 |
| Money | `i64` minor units (VND has no minor unit → 1 đồng); rates in basis points `u32` | No floats; exact | `rust_decimal` (unnecessary for VND; allowed for FX later) | 0006 |
| Time | `time` crate + `time-tz` for `Asia/Ho_Chi_Minh` | Correct day boundaries | chrono (fine; one crate chosen for consistency) | — |
| Auth | `argon2` (Argon2id), `openidconnect` (Google), `totp-rs` (MFA), server-side sessions in Postgres | Revocable sessions; no JWT footguns | JWT access tokens for browser sessions | 0008 |
| Crypto at rest | `aes-gcm` envelope encryption with keys from env/secret files, key IDs stored per row | Bank account numbers, provider credentials | DB-level pgcrypto (keys would sit next to data) | 0008 |
| Observability | `tracing`, `tracing-opentelemetry`, OTLP → otel-collector → Prometheus + Loki + Grafana (+ Tempo optional) | Vendor-neutral, cheap to self-host | SaaS APM (cost; revisit) | 0009 |
| API docs | `utoipa` → `docs/api/openapi.yaml` generated and diffed in CI | Spec can't drift from code | Hand-maintained only | — |
| Reverse proxy | Caddy 2 | Auto TLS, HTTP/3, simple config | Nginx + certbot | 0009 |
| Packaging | Multi-stage Dockerfile (`cargo-chef`), distroless/cc runtime, non-root | Small, reproducible images | Alpine/musl (DNS and allocator quirks) | 0009 |
| CI/CD | GitHub Actions; `cargo fmt`, `clippy -D warnings`, `nextest`, `sqlx prepare --check`, `cargo deny`, `cargo audit`, Trivy, Playwright, k6 smoke | Mandated; free tier sufficient | — | 0009 |
| E2E | Playwright (test tooling only; Node is allowed in CI, never in prod) | Mobile Safari/Chrome emulation | Cypress | — |
| Load | k6 | Scriptable thresholds in CI | Locust (Python) | — |
| Email | Provider-agnostic `EmailSender` trait; SMTP adapter first | Avoid lock-in; choose provider in onboarding | — | — |

## Version policy
- Pin exact minor versions in `Cargo.lock`. Renovate opens grouped weekly PRs. Security patches get merged within 72h.
- MSRV = pinned stable. Bump it quarterly in a dedicated PR.
- Any new dependency needs one PR comment justifying it. Prefer crates with >1 maintainer and an audited `unsafe` footprint.

## What we deliberately do not use in V1
Redis, Kafka, Kubernetes, GraphQL server, ORM, JWT browser sessions, a Node/Python backend, server-side headless browsers for scraping product pages.

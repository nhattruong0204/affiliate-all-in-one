# Architecture Decision Records

Format: Context → Decision → Consequences → Alternatives. Status values: Proposed, Accepted, Superseded.
All ADRs below are **Proposed** until the owner approves the architecture PR.

| # | Title | Status |
|---|---|---|
| 0001 | Rust + Axum + Tokio for all production logic | Proposed |
| 0002 | Modular monolith with separate web and worker processes | Proposed |
| 0003 | Leptos SSR with hydration and Tailwind standalone CLI | Proposed |
| 0004 | PostgreSQL as the only stateful dependency in V1 (no Redis) | Proposed |
| 0005 | Postgres-backed job queue with transactional outbox | Proposed |
| 0006 | Integer VND money, basis-point rates, versioned commission rules | Proposed |
| 0007 | Capability-based Affiliate Gateway with provider routing | Proposed |
| 0008 | Server-side sessions, Argon2id, Google OIDC, envelope encryption | Proposed |
| 0009 | Single-VPS Docker Compose deployment with Caddy and self-hosted observability | Proposed |
| 0010 | Double-entry immutable ledger as financial source of truth | Proposed |
| 0011 | Settlement-gated availability with capped advance funding | Proposed |
| 0012 | Manual-review payouts first; automated disbursement disabled | Proposed |
| 0013 | ACCESSTRADE as first provider; direct integrations behind disabled adapters | Proposed |
| 0014 | Single-level referrals only; multi-level hard-disabled | Proposed |

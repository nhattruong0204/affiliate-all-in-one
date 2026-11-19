# CLAUDE.md — persistent project context

Read `AGENTS.md` first; it is the binding rule set. This file is the short orientation.

## What this is
Cashback platform for Vietnam (VND, `Asia/Ho_Chi_Minh`, Vietnamese default, English secondary). Revenue = affiliate commission; users get a configurable share. Reference product: hoantien.com (parity of public workflows, original implementation and branding).

## Current phase
Architecture approved (PR #52 merged 2026-11-18). Implementation proceeds release by release (`docs/RELEASE_PLAN.md`). **Only work on an issue assigned to you** whose dependencies are merged; one issue per PR. Workspace skeleton and CI (AAO-001) are in place.
Delivery is incremental (V0.1 → V0.5 → V1.0 → V1.x, see `docs/RELEASE_PLAN.md`). Build for the issue's release only. Money and attribution features must stay behind their flags until the release gate passes.

## Hard rules (summary)
- Rust only for production logic. No Node/Python backend.
- Money: `domain::Money` (i64 VND). No `f32`/`f64` anywhere near amounts or rates. Rates are basis points (`u32`, 10 000 = 100%).
- Ledger rows are immutable. Corrections are compensating entries.
- Every state-changing handler/job is idempotent and has a uniqueness constraint backing it.
- Provider facts must come from official docs or live contract tests. Never invent endpoints, fields, rate limits or signatures. Unknown → disabled adapter + labeled mock.
- Estimated cashback is never displayed as guaranteed.
- Secrets come from the environment / secret files, never from the repo.
- One migration per PR, numbered by the issue; never edit a merged migration.
- Never bypass failing CI. Never merge without owner approval where `AGENTS.md` requires it.

## Where things live
- Crate map and dependency rules: `docs/ARCHITECTURE.md` §4
- Schema: `docs/DATABASE_DESIGN.md`
- Ledger accounts and postings: `docs/FINANCIAL_LEDGER.md`
- Provider facts: `docs/AFFILIATE_PROVIDER_MATRIX.md`
- Open owner decisions: `docs/PROJECT_STATUS.md` §4

## Commands
```
just dev          # compose up postgres, run web + worker with watch (needs cargo-watch)
just test         # cargo nextest + doctests + sqlx offline check
just lint         # fmt --check, clippy -D warnings, arch-checks, cargo deny, cargo audit
just arch-checks  # dependency-direction check, no string-built SQL, and their self-tests
just css          # regenerate crates/ui/assets/app.css (Tailwind standalone); `just css-check` verifies it
just e2e          # Playwright + axe + strict-CSP tests against the real server (npm ci --prefix e2e first)
just lighthouse   # mobile Lighthouse on a release build (Perf >= 90, A11y >= 95)
```
Tooling: `just`, `cargo-nextest`, `cargo-deny`, `cargo-audit`, `jq`, `perl`, Docker. CI installs pinned versions (`.github/workflows/ci.yml`).

## When unsure
Stop and write the question into the PR description under "Owner decision needed". Do not guess on money, legal, or provider behaviour.

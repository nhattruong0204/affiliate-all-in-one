# Project Status

**Phase:** Architecture approved (PR #52 merged 2026-11-18); V0.1 implementation in progress: AAO-001 and AAO-014 (UI shell, PR #58) merged; AAO-005 (observability, PR #59) merged; AAO-002 (container build, PR #60) in review. Last updated 2026-11-19.

## 1. Executive summary

**Recommended architecture.** A Rust modular monolith (Axum + Leptos SSR) with two processes, `web` and `worker`, sharing nine crates. PostgreSQL is the only stateful dependency (data, queue, outbox, rate limits). An immutable double-entry ledger is the financial source of truth. Everything runs on Docker Compose on one VPS behind Caddy. Kubernetes, Kafka, Redis and microservices are all deferred until measurements justify them. → `ARCHITECTURE.md`, ADR-0001…0014.

**V1 scope.** Shopee + TikTok Shop through ACCESSTRADE: paste → preview/estimate → tracked link → `/go` redirect → polling-based conversion sync → versioned cashback rules → ledger → settlement-gated availability → manually reviewed bank withdrawals (VietQR payload) → admin console with RBAC, MFA, maker-checker and audit. Plus vi/en, dark/light, legal pages and data-subject rights. V1.x: referral (single-level), check-in, notifications centre, blog, PWA, missing-cashback tickets. → `PRODUCT_REQUIREMENTS.md` §4.

**Technology decisions.** Rust stable, Axum, Tokio, Leptos, Tailwind standalone, PostgreSQL 17, SQLx, Reqwest+rustls, tracing/OTel, Prometheus/Loki/Grafana, utoipa, Caddy, Docker Compose, GitHub Actions, Playwright, k6. Redis is deliberately excluded from V1 (ADR-0004). → `TECH_STACK.md`.

**Release strategy (2026-11-18).** 52 issues across four gated releases. → `RELEASE_PLAN.md`.
| Release | Issues | Ships | Calendar estimate (2 sessions per model / 1) |
|---|---|---|---|
| V0.1 Public Website Preview | 13 | landing, analyzer preview, waitlist, legal | ~4 / ~7 weeks |
| V0.5 Private Alpha | 16 | invite accounts, real tracked links, cashback tracking (no withdrawals) | ~10 / ~16 weeks cumulative |
| V1.0 Commercial | 15 | open sign-up, settlement-gated balances, manual-review withdrawals | ~15 / ~23 weeks cumulative |
| V1.x Growth | 8 | referral, check-in, tickets, blog, PWA, extra providers | rolling |

**Critical path.** AAO-001 → 004 → 010 → 013 → 023 → 025 → 031 → 033 → 034 → 042 → 043 → 045 → 090 → 096. The V0.1 branch is AAO-001 → 003 → 020 → 100. → `IMPLEMENTATION_ROADMAP.md`, `ENGINEERING_BACKLOG.md`.

**Commercial onboarding requirements.** Legal entity; ACCESSTRADE VN publisher account; Shopee + TikTok Shop campaign approval; **written per-campaign permission for cashback/incentive traffic**; settlement cycle, clawback window and statement format; VN lawyer and tax advisor. → `AFFILIATE_PROVIDER_MATRIX.md` §6.

**Critical blockers.**
1. Cashback permission on ACCESSTRADE's Shopee/TikTok campaigns is **UNKNOWN** (OB-4). Without it, there's no launch.
2. Settlement cycle and clawback rules are **UNKNOWN**, so we can't calibrate the default user payout speed.
3. Tax characterization of cashback paid to users needs professional advice (C-TAX-2).
4. Applicability of the E-commerce Law 122/2025 affiliate provisions to a cashback platform needs legal advice (C-CORP-2).

**Estimated infrastructure cost.** About USD 16–60/month up to 1 000 users, 40–140 at 10 000, and 215–640 at 100 000 (indicative). Infrastructure isn't the economic constraint; acquisition cost and commission rates are. → `COST_ESTIMATION.md`.

**GitHub issues.** #1–#51 plus #53 (AAO-100). Every issue has `release:*`, `launch-blocker`, `parallelizable`, `commercial-blocker`, `model:*`, `effort:*` and `size:*` labels and a release milestone. The old M0–M6 milestones are closed and kept for history. → `ENGINEERING_BACKLOG.md` §5.

**The exact first issue Claude Opus 5.5 should implement:** **AAO-003 (#3) — Domain core: Money, Bps, IDs, BusinessClock, share split.** It starts as soon as Claude Sonnet 5.5 merges **AAO-001 (#1)** (workspace skeleton + CI). AAO-003 has no I/O, unblocks AAO-020/022/023/030, and is the smallest piece where correctness matters most. Opus then takes AAO-004 and AAO-021 in the same wave.

**Next five issues:** #1 AAO-001 (Sonnet) → #3 AAO-003 (Opus), #14 AAO-021 (Opus), #10 AAO-014 (Sonnet), #4 AAO-004 (Opus). All five are V0.1 launch blockers. #14 and #10 can start as soon as #1 merges.

## 2. Top 20 risks

| ID | Risk | Type | L | I | Mitigation | Owner |
|---|---|---|---|---|---|---|
| R-01 | ACCESSTRADE/Shopee/TikTok forbid cashback traffic on needed campaigns | Commercial | M | Critical | OB-4 written permission before launch; `cashback_allowed` gates routing; pursue Shopee direct as second path | Owner |
| R-02 | Single-provider dependency (key revoked, policy change, account ban) | Commercial | M | High | Gateway abstraction; AAO-060/061; keep the publisher account compliant | Owner/Arch |
| R-03 | Commission rate cuts (platform-wide, as happened with Shopee in the past) | Commercial | H | High | Versioned rules allow rapid share changes; unit-economics monitoring | Owner |
| R-04 | Paying out before provider settlement → losses on clawbacks | Financial | M | High | Settlement gating; advance off by default with caps; negative balances | Opus |
| R-05 | Ledger bug creates or loses money | Financial | L | Critical | Property tests, DB-level balance triggers, nightly recomputation, owner review on ledger PRs | Opus |
| R-06 | Double payout from races or retried unknown outcomes | Financial | L | Critical | Unique indexes, row locks, idempotency, never auto-retry `unknown` | Opus |
| R-07 | Attribution loss (in-app browsers, ad blockers, multi-publisher overrides) → user complaints | Product | H | Medium | Interstitials, purchase rules, missing-cashback flow, transparent "estimate" language | Sonnet/Opus |
| R-08 | Self-referral / fake-account farming | Fraud | H | Medium | Rewards only on settled orders, clustering, budgets, holds | Opus |
| R-09 | Account takeover → bank account swap → theft | Security | M | High | Step-up, cooldown, notifications, review | Opus |
| R-10 | Insider misuse (admin approves own payouts, edits rules) | Security | L | High | Maker-checker CHECKs, audit hash chain, owner daily report | Opus |
| R-11 | Provider API undocumented changes or inconsistencies (already seen in docs) | Technical | H | Medium | Contract errors alert; fixtures from live data; lenient parsing with strict mapping | Opus |
| R-12 | Unknown settlement statement format makes allocation manual and error-prone | Operational | H | Medium | Suspense account; CSV importer per format; OB-5 | Finance |
| R-13 | Regulatory: e-commerce law obligations, PDPL, tax withholding | Legal | M | High | ⚖️ items in COMPLIANCE_CHECKLIST before launch | Owner |
| R-14 | Multi-level referral pressure (competitor parity) violates MLM rules | Legal | M | High | ADR-0014 hard-off; legal opinion required | Owner |
| R-15 | Leptos maturity / hydration bugs slow UI delivery | Technical | M | Medium | Keep JSON API independent; HTMX fallback documented in ADR-0003 | Sonnet |
| R-16 | Single VPS outage during mega-sale days | Operational | M | Medium | RTO 4h runbook; scale-up before 11.11/12.12; status page | Sonnet |
| R-17 | Manual payout capacity bottleneck at scale | Operational | M | Medium | Batch UI, limits; MoMo/bank API adapters post-V1 | Owner |
| R-18 | AI-agent changes bypass architecture rules | Process | M | Medium | CODEOWNERS, CI dependency check, AGENTS.md, small PRs | Arch |
| R-19 | Customer acquisition cost exceeds contribution per user | Commercial | H | High | Measure funnel from day one; referral only if CAC < LTV | Owner |
| R-20 | Brand/trademark confusion with marketplaces or the reference product | Legal | L | Medium | Original brand, disclaimers, no marketplace marks in the logo | Owner |

L = likelihood, I = impact (H/M/L).

## 3. Self-review

**Decisions challenged**
- *Leptos vs HTMX.* Leptos adds WASM and hydration complexity for a mostly-forms product. We keep it, because islands cover the interactive wallet and admin and the brief prefers it. HTMX + Askama is the documented fallback if AAO-014 shows hydration pain.
- *Polling vs postbacks.* Polling at 10 req/min is enough at our scale and authoritative. Postbacks only make it faster, and they're unsigned (Global docs), so they serve as triggers only.
- *Settlement gating* makes us slower than the reference product's "~7 days" claim. That's a commercial disadvantage we accept deliberately, and the owner can opt into capped advances.
- *Contingent ledger accounts for pending.* Some accountants would keep pending off-ledger. We keep them on-ledger in contingent accounts because users must see them and it simplifies transitions. Statutory reporting can exclude them (FINANCIAL_LEDGER §9).

**Possible over-engineering (cut if time-pressed)**
- Hash-chained audit log: a plain append-only table plus off-site export may be enough for V1. Low cost, so it stays.
- Routing scores with weights: V1 has one provider per merchant. Implement the eligibility filter fully and keep scoring trivial until a second provider exists.
- Monthly partitions on `clicks`: defer until volume needs it, but keep the PK shape partition-ready.
- OpenTelemetry traces + Tempo: logs + metrics are enough at launch, so traces stay optional.
- Envelope encryption with rotation: required for bank numbers, but rotation automation can come after launch.

**Missing invariants identified and added**
- I12 (settlement allocation + suspense = amount) and I13 (reward budgets) were added during review.
- Still open: an invariant tying `withdrawals.status=paid` to exactly one confirmed `payout_attempt` **and** one 2400→1100 posting. Added as part of I10's test scope in AAO-043.
- Still open: provider-level reconciliation tolerance (how much drift between our totals and provider dashboards is acceptable). Owner decision D-11.

**Security review summary.** The SSRF guard, open-redirect prevention, IDOR scoping, maker-checker and step-up are specified with tests. The biggest residual risks are staff phishing (TOTP only, WebAuthn later) and supply chain (mitigated, not eliminated). → `SECURITY_THREAT_MODEL.md`.

**Unit economics and cash-flow exposure.** Each order contributes ~2.9–3.5k VND at a 6% commission. Advance funding at 100k users could tie up ~660M VND. Both are explicit owner levers. → `COST_ESTIMATION.md` §4.

## 4. Decisions requiring owner approval

| ID | Decision | Options / recommendation |
|---|---|---|
| D-1 | Approve architecture, stack, ADR-0001…0014 | Recommend approve as-is |
| D-2 | Advance funding policy | Recommend **off** at launch; revisit with 60 days of settlement data |
| D-3 | Default user share, withdrawal minimum and fee | Recommend 70% share (matches the brief example), 50 000đ minimum, 0 fee |
| D-4 | Legal entity, brand name, domain | Needed for OB-1 and legal pages |
| D-5 | Apply to Shopee direct affiliate in parallel with ACCESSTRADE | Recommend yes (R-02) |
| D-6 | Hosting provider and region (VN vs Singapore) | Recommend a VN data-centre VPS if PDPL counsel flags cross-border transfer concerns; otherwise Singapore |
| D-7 | Transactional email provider | Any SMTP provider with VN deliverability; decide at AAO-011 |
| D-8 | Referral reward economics (V1.x) | e.g. 10% of referee cashback for 90 days, budget-capped |
| D-9 | External penetration test before public launch | Recommend yes, scoped to auth, payouts, admin |
| D-10 | Tax treatment of cashback and rewards | After advisor (C-TAX-*) |
| D-11 | Reconciliation tolerance and negative-balance write-off period | Recommend 0đ tolerance on ledger, 1% on provider dashboards, 180-day write-off |
| D-13 | Supply-chain exceptions for the Leptos dependency tree (AAO-014): allow licences `CC0-1.0` and `BSL-1.0`; ignore RUSTSEC-2024-0436 (`paste`, unmaintained compile-time macro, required by every Leptos 0.8 release) in `deny.toml` and `.cargo/audit.toml`, review by 2027-02-18 | Recommend approve; the alternative is a different UI stack (ADR-0003 fallback HTMX + Askama) |
| D-14 | V0.1 UI is server-rendered only (no WASM/hydration) until the first interactive island needs it; `cargo-leptos` is not set up yet (deviation from AAO-014's technical notes, consistent with ADR-0003 "selective hydration") | Recommend approve: best mobile performance, strict CSP, faster V0.1 |
| D-15 | AAO-005 readiness scope: until AAO-004 lands, `/readyz` checks only TCP reachability of the `DATABASE_URL` host:port behind a `ReadinessCheck` seam; "migrations current" and `SELECT 1` are AAO-004's job (it must register the real check and remove `TcpProbe`). OTLP export is deferred (JSON logs + Prometheus suffice for V0.1 per the issue) | Recommend approve: keeps sqlx confined to `db` and unblocks AAO-091/092/100 now |
| D-16 | Config policy (AAO-005): `APP_ENV` defaults to `development` (deployments must set `production`); in production `DATABASE_URL_FILE` is required and inline secret env vars are rejected; `WEB_UI_GALLERY` is refused in production; metrics are on a separate internal listener (`METRICS_BIND`, default `127.0.0.1:9100`) that must never be published | Recommend approve; AAO-092 compose must follow `docs/OBSERVABILITY.md` §7 |
| D-17 | AAO-002 Trivy exception: `CVE-2026-84782` (libssl3 HIGH, fixed in Debian 3.0.22-1~deb12u2) is present in the *latest* `gcr.io/distroless/cc-debian12:nonroot`; time-boxed in `.trivyignore.yaml` until 2026-12-03 because our binaries do not link OpenSSL and nothing speaks DTLS. The gate fails again on that date if the entry is not removed | Recommend approve (the alternative is a red `image` job until distroless republishes) |
| D-18 | AAO-002 deviations: (a) no Tailwind CLI / `cargo-leptos` / `site/` in the image (assets are committed and embedded; `css-check` already guards them); (b) Trivy is a checksum-pinned binary instead of a third-party Action; (c) Renovate is configured but `.github/dependabot.yml` stays until the Renovate GitHub App is installed; (d) cosign + SBOM deferred to AAO-090 as the issue's MVP scope says | Recommend approve; **owner action:** install the Renovate app, then delete `dependabot.yml` |
| D-12 | GitHub settings: branch protection, required checks, CODEOWNERS enforcement | Owner sets these in repository settings after AAO-001; also require the new `e2e` (and later `lighthouse`) jobs once stable |

## 5. Session log
| Date | Agent | Issue / branch | State | Next |
|---|---|---|---|---|
| 2026-11-17 | Claude Opus 5.5 (acting as Principal Architect) | `docs/architecture-v1` | Architecture package + backlog written; PR opened; GitHub issues created | Owner review of PR; decisions D-1…D-12; start OB-1…OB-6 |
| 2026-11-18 | Claude Opus 5.5 (acting as CTO/Architect) | `docs/architecture-v1` | Re-planned into V0.1/V0.5/V1.0/V1.x (ADR-0015, RELEASE_PLAN.md); added AAO-100 (#53); relabelled and re-milestoned all 52 issues; 5 dependency edits; reasoning effort reassessed | Owner approves PR + D-4/D-6 so G-0.1 work can start; Sonnet starts #1 |
| 2026-11-18 | Claude Sonnet 5.5 | `feat/AAO-001-workspace-skeleton` (#1) | Workspace skeleton, toolchain pin, lints, guard-rail scripts + self-test, justfile, compose.dev.yml, CODEOWNERS, PR template, `deny.toml`, `ci.yml` written; local fmt/clippy/build/arch-checks/gitleaks pass. `deny`, `audit`, `nextest` could not run in the sandbox (no network/tools); CI is the first real run | Owner: review/merge, then set branch protection (D-12: require `lint`, `test`, `supply-chain`, `secrets`; code-owner review). Next: AAO-003, AAO-021, AAO-014, AAO-004 can start once merged |
| 2026-11-18 | Claude Sonnet 5.5 | `feat/AAO-014-ui-shell` (#10, PR #58) | UI foundations: design tokens (light/dark, contrast-tested), `AppShell` with mobile bottom / desktop top nav, theme toggle without flash, vi/en Fluent catalogs, buttons, forms, cards, alerts, toasts, skeletons, empty/error states, component gallery, `web` SSR host with strict CSP, committed Tailwind CSS (`just css`), docs/UI_FOUNDATIONS.md. CI green: fmt, clippy, nextest, deny, audit, gitleaks, css-check, Playwright + axe (WCAG 2.2 A/AA, 0 violations, both themes, desktop + mobile), strict-CSP test, Lighthouse mobile 100/100. `Modal`, `MoneyText`, `StatePill` deferred (AAO-036). Brand name/logo are placeholders (D-4) | Owner: review/merge PR #58, decide D-13/D-14. Next: AAO-005 (#5, Sonnet) then AAO-002 (#2, Sonnet) in parallel; AAO-091 and AAO-100 build on 005 + 014 |
| 2026-11-19 | Claude Sonnet 5.5 | `feat/AAO-005-observability` (#5, PR #59) | Typed config (`APP_ENV` profiles, `*_FILE` secrets, all errors reported at once), JSON logs with field-name redaction layer, W3C trace ids (`X-Trace-Id`, request span), hand-rolled Prometheus RED metrics on an internal-only listener, `/healthz` + `/readyz` (readiness trait seam, interim TCP probe, 503 problem+json), `AppError` → RFC 9457 with stable `code` and vi/en `detail`, docs/OBSERVABILITY.md, openapi `/readyz`. 113 tests pass (48 new incl. redaction, readiness against a stopped listener, metrics-not-on-public-port, trace-id in logs); CI green: fmt, clippy, nextest, deny, audit, gitleaks, e2e, lighthouse. No new crates beyond what Leptos already pulled in (lock diff: `nu-ansi-term` dropped). No coverage tool in CI yet, so the ≥ 90% criterion is argued from tests, not measured | Owner: review/merge PR #59, decide D-15/D-16. Next: AAO-002 (#2, container build, independent); then AAO-091/092/100 once 005 is merged and their other deps are met |
| 2026-11-19 | Claude Sonnet 5.5 | `feat/AAO-002-container-build` (#2, PR #60) | Multi-stage `Dockerfile` (cargo-chef, cargo-auditable, digest-pinned distroless `cc-debian12:nonroot`, UID 10001, no shell; web 26 MiB, worker 22 MiB), `.dockerignore`, `image` CI job (build both targets, `scripts/image-smoke.sh`: size, UID, no shell, `/healthz` 200, `/readyz` 200, `/metrics` 404 on the public port, worker starts; Rust-toolchain sync check), checksum-pinned Trivy gate (`scripts/install-trivy.sh`, fixable HIGH/CRITICAL fail the job), `renovate.json`. Seeded-vulnerability proof: scratch branch `scratch/aao-002-seeded-vuln` adds `hyper =0.14.9`; Trivy reports GHSA-f67m-9j94-qv9j in `usr/local/bin/web (rustbinary)` and the gate blocks (run 44485920456). CI green on the PR. Cosign + SBOM deferred (AAO-090) | Owner: review/merge PR #60, decide D-17/D-18, install the Renovate app then delete `dependabot.yml`, delete the scratch branches. Next: AAO-091 (security headers/CSP middleware) and AAO-092 (compose/Caddy) now that 005 and 002 are merged-or-in-review; AAO-100 per its other deps |

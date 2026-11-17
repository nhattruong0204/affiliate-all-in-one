"""Single source of truth for the V1 backlog.
Generates docs/ENGINEERING_BACKLOG.md and issues.json (for GitHub issue creation)."""
import json, sys, pathlib

OPUS, SONNET = "Claude Opus 5.5", "Claude Sonnet 5.5"
M = {
    "M0": "M0 Foundations",
    "M1": "M1 Identity & Admin base",
    "M2": "M2 Link engine",
    "M3": "M3 Ledger & Conversions",
    "M4": "M4 Withdrawals",
    "M5": "M5 Launch hardening",
    "M6": "M6 V1.x Engagement",
}

STD_DOD = [
    "CI green (fmt, clippy -D warnings, nextest, sqlx check, deny, audit, gitleaks)",
    "Tests listed below exist and pass; coverage of new domain logic ≥ 90% lines",
    "Docs referenced above updated if behaviour differs; PROJECT_STATUS session log entry added",
    "No secrets, PII, or real provider data committed",
    "PR reviewed (owner review required on protected paths per CODEOWNERS)",
]

I = []
def issue(id, title, ms, model, effort, cx, deps, objective, business, tech, refs, files, ac, security, tests, extra_dod=(), parallel=""):
    I.append(dict(id=id, title=title, milestone=ms, model=model, effort=effort, complexity=cx, deps=deps,
                  objective=objective, business=business, tech=tech, refs=refs, files=files, ac=ac,
                  security=security, tests=tests, dod=list(extra_dod) + STD_DOD, parallel=parallel))

# ---------------- M0 ----------------
issue("AAO-001", "Cargo workspace skeleton, toolchain, CI baseline", "M0", SONNET, "medium", "M", [],
  "Create the empty-but-compiling workspace and CI so all later issues have a home.",
  "Every other issue depends on a consistent workspace layout and enforced quality gates.",
  ["Workspace with crates: domain, db, ledger, affiliate, app, ui, web (bin), worker (bin), testkit (dev) — stubs only",
   "rust-toolchain.toml pinned stable; workspace lints: forbid(unsafe_code), clippy::float_arithmetic denied in domain/ledger/app",
   "justfile: dev, test, lint, e2e; compose.dev.yml with postgres:17",
   ".github/workflows/ci.yml: fmt, clippy, nextest, cargo deny, cargo audit, gitleaks, grep-check forbidding format! into SQL",
   "deny.toml (licenses allowlist, bans), CODEOWNERS (owner on crates/ledger, migrations, .github, infra, docs/adr), PR template with 'Owner decision needed' section",
   "migrations/README.md describing numbering rule (see AGENTS.md §8); dependency-direction check script over cargo metadata"],
  ["ARCHITECTURE.md §4", "TECH_STACK.md", "AGENTS.md"],
  ["Cargo.toml", "rust-toolchain.toml", "crates/*/Cargo.toml", "crates/*/src/lib.rs|main.rs", "justfile", "compose.dev.yml", ".github/workflows/ci.yml", "deny.toml", ".github/CODEOWNERS", ".github/pull_request_template.md", "migrations/README.md", "scripts/check-deps.sh"],
  ["`just lint` and `just test` pass on a clean checkout", "CI runs on PRs and blocks merge on failure", "Dependency check fails if e.g. domain depends on sqlx"],
  ["Actions pinned by SHA; workflow permissions read-only by default", "gitleaks blocks committed secrets"],
  ["CI self-test: a deliberately failing lint in a scratch branch is caught (documented in PR)"],
  parallel="First issue. Blocks everything.")

issue("AAO-002", "Container build, image signing, SBOM, dependency automation", "M0", SONNET, "medium", "M", ["AAO-001"],
  "Produce reproducible, minimal, signed images for web and worker.",
  "Supply-chain integrity for a product handling money.",
  ["Multi-stage Dockerfile with cargo-chef; distroless cc nonroot runtime; Tailwind standalone CLI in build stage",
   "Trivy scan (fail on fixable HIGH/CRITICAL); cosign keyless signing; SBOM (cargo-cyclonedx) as artifact",
   "Renovate config: weekly grouped Rust deps, actions digests, Docker base images"],
  ["INFRASTRUCTURE.md §3–4", "SECURITY_THREAT_MODEL.md E-2"],
  ["Dockerfile", ".dockerignore", ".github/workflows/ci.yml (image job)", "renovate.json"],
  ["Images < 60 MB, run as UID 10001, no shell", "Signature verifiable with cosign verify", "Trivy job blocks on seeded vulnerable dep in test branch"],
  ["No secrets in build args or layers"], ["Image smoke: container starts and /healthz returns 200 (once AAO-005 lands; stub endpoint until then)"],
  parallel="Parallel with AAO-003/004/005/014/021.")

issue("AAO-003", "Domain core: Money, Bps, IDs, BusinessClock, share split", "M0", OPUS, "high", "M", ["AAO-001"],
  "Implement the pure financial and identity primitives every other crate uses.",
  "Correct money math is the foundation of user trust and cash-flow safety.",
  ["`Money{amount:i64,currency}` with checked ops; `Bps(u32)` 0..=10000; `Rounding{Floor,HalfEven}`",
   "`split_commission(commission, share_bps, rounding, cap) -> (user, platform)` with platform taking remainder (I7)",
   "UUIDv7 newtype IDs (UserId, ClickId, ConversionId, …) with serde + sqlx Type behind feature flag",
   "`BusinessClock` trait: now(), vn_date(), vn_day_bounds(); fake clock for tests",
   "State enums for Conversion, Entitlement, Withdrawal with `can_transition(from,to)` tables (no I/O)"],
  ["ADR-0006", "FINANCIAL_LEDGER.md §5, §7", "ORDER_RECONCILIATION.md §4", "WITHDRAWAL_DESIGN.md §3"],
  ["crates/domain/src/{money.rs,bps.rs,ids.rs,clock.rs,split.rs,state/*.rs}"],
  ["No f32/f64 in crate (lint)", "split satisfies user+platform==commission for all inputs", "Illegal transitions return Err"],
  ["Overflow returns error, never wraps or panics"],
  ["proptest: split invariants, rounding bounds, cap respected", "exhaustive transition table tests", "serde round-trip"],
  parallel="Parallel with AAO-002/004/005/014/021.")

issue("AAO-004", "DB foundation: pool, migrations runner, DB roles, job queue, outbox, idempotency", "M0", OPUS, "high", "L", ["AAO-001"],
  "Provide transactional infrastructure: migrations, a durable job queue, outbox/inbox, idempotency keys.",
  "Reliable background processing without double financial effects.",
  ["sqlx pool config from env; `migrator` binary subcommand; roles app_rw/app_ro/migrator with grants",
   "Tables: jobs, outbox, processed_events, idempotency_keys, rate_limit_buckets, config_values (effective-dated)",
   "Job API: enqueue(tx, kind, payload, dedupe_key, run_at); lease(n) via SKIP LOCKED; heartbeat; complete; fail with backoff; dead-letter",
   "Outbox writer (in caller tx) + publisher loop skeleton in worker; inbox dedupe helper",
   "testkit: Postgres testcontainer with template DB per test"],
  ["ADR-0004", "ADR-0005", "DATABASE_DESIGN.md §4.2, §4.9, §6, §8"],
  ["crates/db/src/{pool.rs,jobs.rs,outbox.rs,idempotency.rs,config.rs}", "crates/worker/src/runner.rs", "crates/testkit/src/pg.rs", "migrations/0001_AAO-004_foundation.sql"],
  ["Two workers never lease the same job", "Expired lease is reclaimed", "dedupe_key prevents duplicate queued jobs", "Outbox event published exactly once per consumer (inbox)"],
  ["app_rw cannot ALTER/DROP; least-privilege grants"],
  ["integration: concurrent leasing (50 tasks)", "crash simulation: drop connection mid-job → retried", "migration from zero in CI"],
  parallel="Parallel with AAO-002/003/005/014/021.")

issue("AAO-005", "Telemetry, health/readiness, problem+json errors, typed config", "M0", SONNET, "medium", "M", ["AAO-001"],
  "Make both binaries observable and fail predictably.",
  "Operability from day one; trace ids help support resolve user issues.",
  ["tracing + OTLP exporter, JSON logs with redaction layer (deny-list of field names: email, phone, account_number, token, password)",
   "Prometheus metrics endpoint on internal port; RED middleware for Axum",
   "/healthz (process) and /readyz (DB ping + migrations current)",
   "`AppError` → RFC 9457 problem+json with stable `code`, localized `detail`, `trace_id`",
   "Typed config (figment or envy) with validation at startup; secrets read from /run/secrets/*"],
  ["ARCHITECTURE.md §7", "API_SPECIFICATION.md §2", "INFRASTRUCTURE.md §8"],
  ["crates/app/src/telemetry.rs", "crates/app/src/error.rs", "crates/web/src/{health.rs,middleware/*.rs}", "crates/app/src/config.rs"],
  ["Error responses match schema in docs/api/openapi.yaml", "Logs never contain redacted fields (test)", "/readyz 503 when DB down"],
  ["Metrics port not exposed publicly (documented for AAO-092)"],
  ["unit: redaction layer", "integration: readiness with stopped DB"],
  parallel="Parallel with AAO-002/003/004/014/021.")

# ---------------- M1 ----------------
issue("AAO-010", "Identity core: registration, login, sessions, CSRF, Argon2id, rate limits", "M1", OPUS, "high", "L", ["AAO-004", "AAO-005"],
  "Secure email/password accounts with revocable server-side sessions.",
  "Accounts gate link attribution and payouts; ATO is the main fraud vector.",
  ["Tables users, user_credentials, sessions, email_tokens (migration)", "Argon2id params per ADR-0008 with rehash-on-login",
   "Session cookie __Host-, token hash stored, idle+absolute expiry, revoke-all", "CSRF middleware (synchronizer token)",
   "Postgres token-bucket rate limiter (per IP, per account); generic auth errors; lockout backoff",
   "Consent capture fields at registration (document versions)", "referral_code generation (random, unique)"],
  ["ADR-0008", "SECURITY_THREAT_MODEL.md S-1,S-2,I-4", "DATABASE_DESIGN.md §4.1", "API_SPECIFICATION.md §3"],
  ["crates/app/src/identity/*", "crates/web/src/routes/auth.rs", "crates/web/src/middleware/{session.rs,csrf.rs,ratelimit.rs}", "migrations/NNNN_AAO-010_identity.sql"],
  ["Register/login/logout via API", "Session revoked on logout and password change", "Enumeration-safe responses", "Rate limits enforced with 429 + Retry-After"],
  ["Timing-safe comparisons; password max length 128; breached-password check hook (offline list)"],
  ["integration: auth flows", "security: CSRF missing token rejected, session fixation prevented", "property: rate limiter never exceeds budget"],
  parallel="Parallel with AAO-020/021/022/030 once deps merged.")

issue("AAO-011", "Email verification, password reset, EmailSender (SMTP)", "M1", SONNET, "medium", "M", ["AAO-010"],
  "Complete account lifecycle emails.", "Verified email is required to withdraw.",
  ["EmailSender trait + SMTP adapter (lettre, rustls) + dev mailbox (log)", "Bilingual templates (vi/en)",
   "Verify-email and reset flows with single-use hashed tokens (30 min reset, 24h verify); reset revokes sessions"],
  ["ADR-0008", "SECURITY_THREAT_MODEL.md S-2"],
  ["crates/app/src/identity/{verify.rs,reset.rs}", "crates/app/src/email/*", "crates/ui/locales/*/email.ftl"],
  ["Tokens single-use and expire", "Same response whether or not account exists"],
  ["Email content contains no secrets except the one-time link"], ["integration: token reuse rejected, expired rejected"])

issue("AAO-012", "Google Sign-In (OIDC + PKCE) and account linking", "M1", OPUS, "medium", "M", ["AAO-010"],
  "Allow one-tap Google login.", "Parity feature F-09; reduces signup friction.",
  ["openidconnect crate; state+nonce+PKCE; require email_verified", "oauth_identities table",
   "Linking rules: auto-link only when logged in or after email confirmation; never silently merge"],
  ["ADR-0008", "SECURITY_THREAT_MODEL.md S-3"],
  ["crates/app/src/identity/oidc.rs", "crates/web/src/routes/oauth.rs", "migrations/NNNN_AAO-012_oauth.sql"],
  ["New user via Google gets verified email", "Existing email account requires confirmation to link"],
  ["Client secret from secret file; redirect URI exact match"], ["integration with stub IdP (testkit)"])

issue("AAO-013", "Staff RBAC, TOTP MFA, step-up, hash-chained audit log, feature flags, approvals", "M1", OPUS, "high", "L", ["AAO-010"],
  "Provide the privileged-access backbone for all admin and money actions.", "Insider and ATO risk control; maker-checker for money.",
  ["roles/role_grants; owner seeded via CLI only", "TOTP enrolment/verify with replay protection (last_used_step)",
   "step-up guard (≤5 min) as Axum extractor", "audit_log with SHA-256 hash chain + verifier job",
   "feature_flags with 30s cache; dangerous flags require approvals row by a different staff user",
   "approvals table + service (maker≠checker DB CHECK)", "`app::authz` policy functions per permission (ADMIN_DASHBOARD.md §2)"],
  ["ADMIN_DASHBOARD.md §2", "ADR-0008", "SECURITY_THREAT_MODEL.md S-4,T-3,E-1,A-9"],
  ["crates/app/src/{authz.rs,audit.rs,flags.rs,approvals.rs,identity/mfa.rs}", "crates/web/src/extract/{staff.rs,step_up.rs}", "migrations/NNNN_AAO-013_admin.sql"],
  ["Every staff route requires TOTP-verified session", "Audit chain verification detects tampering", "Dangerous flag cannot be enabled by a single person"],
  ["Owner role not grantable via API"], ["authz matrix test generated from ADMIN_DASHBOARD.md table", "audit chain tamper test"])

issue("AAO-014", "UI shell: design tokens, AppShell, theme, i18n (vi/en), base components", "M1", SONNET, "medium", "L", ["AAO-001"],
  "Create the Leptos SSR app shell and component library.", "Mobile-first, Vietnamese-first UX parity (F-22, F-23).",
  ["cargo-leptos setup with islands; Tailwind v4 standalone; tokens from UI_UX_SPECIFICATION.md §2",
   "AppShell (bottom nav mobile / top nav desktop), ThemeToggle without FOUC, LanguageSwitch, MoneyText, StatePill, EmptyState, ErrorState, Toast, Modal",
   "Fluent catalogs vi/en; CI missing-key check; self-hosted Be Vietnam Pro subset"],
  ["UI_UX_SPECIFICATION.md", "ADR-0003"],
  ["crates/ui/src/{app.rs,components/*,theme.rs,i18n.rs}", "crates/ui/style/tailwind.css", "crates/ui/locales/{vi,en}/*.ftl", "crates/web/src/ssr.rs"],
  ["Lighthouse mobile: Perf ≥ 90, A11y ≥ 95 on shell page", "No hydration warnings", "Dark/light respects prefers-color-scheme"],
  ["CSP-compatible (no inline scripts except hashed)"], ["Playwright: theme + language persist; axe no serious violations"],
  parallel="Parallel with M0 issues after AAO-001.")

issue("AAO-015", "Auth pages: register, login, Google, verify, forgot/reset, profile basics", "M1", SONNET, "medium", "M", ["AAO-011", "AAO-012", "AAO-014"],
  "User-facing authentication UI.", "Parity F-07..F-10.",
  ["Pages per UI spec §5 with inline validation, password strength (zxcvbn), consent checkboxes linking legal docs",
   "Session list + revoke on profile page"],
  ["UI_UX_SPECIFICATION.md §5", "API_SPECIFICATION.md §3"],
  ["crates/ui/src/pages/auth/*", "crates/ui/src/pages/profile.rs"],
  ["All flows work without JS for submit (progressive enhancement)", "Errors localized"],
  ["No credentials in URLs; autocomplete attributes correct"], ["Playwright: register→verify→login; Google stub; reset"])

issue("AAO-016", "Admin console shell and Users module", "M1", SONNET, "medium", "M", ["AAO-013", "AAO-014"],
  "Admin layout, navigation by role, users search/detail/actions.", "Operators need tooling before launch.",
  ["admin subdomain routing; role-aware nav", "Users: search, detail, lock/suspend, revoke sessions, view-as (read-only)", "Audit log viewer"],
  ["ADMIN_DASHBOARD.md §1–2"], ["crates/ui/src/admin/{shell.rs,users/*,audit.rs}", "crates/web/src/routes/admin/users.rs"],
  ["Each action audited with reason", "Support role sees masked bank data"], ["CSRF + step-up on mutations"], ["Playwright admin smoke; authz tests per role"])

# ---------------- M2 ----------------
issue("AAO-020", "URL engine: extraction, merchant detection, canonicalization, product IDs", "M2", OPUS, "medium", "M", ["AAO-003"],
  "Turn arbitrary user input into a canonical merchant product.", "Parity F-02; robust parsing drives conversion.",
  ["extract_urls from text (≤4 KB, ≤5 URLs)", "Merchant modules: Shopee VN, TikTok Shop VN (hosts, short hosts, patterns, app schemes)",
   "Canonical URL builder; strip tracking/other-publisher params", "Fixture corpus of real public URL shapes (no PII)"],
  ["TRACKING_AND_ATTRIBUTION.md §2"], ["crates/affiliate/src/url/{extract.rs,merchants/shopee.rs,merchants/tiktok.rs,canonical.rs}", "crates/affiliate/tests/url_corpus.rs"],
  ["≥ 40 fixture inputs pass incl. share-text with emoji/Vietnamese", "Unknown hosts rejected"], ["No network calls here (short links delegated to safe_fetch)"],
  ["unit + proptest (never panics on arbitrary input)"], parallel="Parallel with AAO-021/022/030.")

issue("AAO-021", "safe_fetch: SSRF-hardened short-link resolver", "M2", OPUS, "high", "M", ["AAO-001"],
  "Resolve short links safely.", "Short/shared mobile links are the common input; SSRF is the main risk of resolving them.",
  ["reqwest+rustls with custom resolver; allowlisted hosts; IP range blocklist (v4/v6); connect to vetted IP", "Manual redirect following with per-hop validation, ≤5 hops, loop detection",
   "HEAD then ranged GET; size cap 512 KB; total timeout 4s; no cookies/proxy env", "24h cache by input hash (product_cache or dedicated table)"],
  ["TRACKING_AND_ATTRIBUTION.md §2.1", "SECURITY_THREAT_MODEL.md D-2"], ["crates/affiliate/src/safe_fetch/*"],
  ["Blocks private/loopback/link-local/metadata/rebinding cases", "Resolves allowlisted short links in fixtures"], ["Primary SSRF control — reviewed by owner"],
  ["SSRF corpus tests with local DNS stub and redirect servers (T-SSRF-1)"], parallel="Parallel with AAO-003/004/005.")

issue("AAO-022", "Affiliate Gateway traits, provider registry, catalog tables, routing", "M2", OPUS, "high", "L", ["AAO-003", "AAO-004"],
  "Provider-independent gateway with capability discovery and route selection.", "Lets us add providers without touching business logic; protects history.",
  ["Traits per AFFILIATE_INTEGRATION_GUIDE.md §1; ProviderError taxonomy", "Registry with modes Disabled/Mock/Sandbox/Live; prod refuses Mock",
   "Tables providers, merchants, campaigns, provider_merchant_routes, product_cache", "Routing eligibility + scoring (§3), decision persisted", "Per-provider rate limiter + circuit breaker + health score"],
  ["ADR-0007", "AFFILIATE_INTEGRATION_GUIDE.md", "DATABASE_DESIGN.md §4.3"],
  ["crates/affiliate/src/{gateway.rs,registry.rs,routing.rs,resilience.rs}", "crates/app/src/catalog/*", "migrations/NNNN_AAO-022_catalog.sql"],
  ["Route never chosen when cashback_allowed≠yes", "Mock providers rejected at startup in prod profile"], ["Credentials encrypted (uses AAO-040 when available; until then field remains null and provider disabled)"],
  ["unit: routing table-driven", "integration: registry + DB"], parallel="Parallel with AAO-020/021/030.")

issue("AAO-023", "Commission rules engine: versions, resolver, approval, simulation", "M2", OPUS, "high", "M", ["AAO-003", "AAO-004", "AAO-013"],
  "Configurable, versioned, reproducible cashback share rules.", "Margin control; no hardcoded rates (brief §3).",
  ["Tables commission_rules, commission_rule_versions (immutable after approval trigger)", "Pure resolver in domain: specificity, tier/promo overlays, priority, effective dates",
   "Admin API: create version, approve (maker≠checker), simulate on last N conversions", "Ceiling alert when share_bps > config max"],
  ["ADR-0006", "DATABASE_DESIGN.md §4.4", "ADMIN_DASHBOARD.md"],
  ["crates/domain/src/rules.rs", "crates/app/src/rules/*", "crates/web/src/routes/admin/rules.rs", "migrations/NNNN_AAO-023_rules.sql"],
  ["Same inputs → same version (deterministic)", "Unapproved versions never resolve"], ["Rule edits audited; diff stored"],
  ["proptest resolver determinism", "integration approval flow"])

issue("AAO-024", "ACCESSTRADE adapter: link creation (generic + TikTok v2), campaign catalog, product feed", "M2", OPUS, "high", "L", ["AAO-022"],
  "First real provider integration for link generation.", "Unlocks Shopee and TikTok Shop cashback links via one network (ADR-0013).",
  ["Client with Token auth, timeouts, limiter", "LinkBuilder: POST /v1/product_link/create; TikTok: POST /v2/tiktokshop_product_feeds/create_link?minify=true",
   "SubIds: utm_source=aaio, utm_campaign, utm_content=token, sub1=token", "CampaignCatalog: GET /v1/campaigns?approval=successful (daily job)",
   "ProductLookup for TikTok via product feed when possible; Shopee preview = none (capability false)", "Mock adapter + wiremock contract tests; fixtures only from real redacted responses once OB-2 done"],
  ["AFFILIATE_PROVIDER_MATRIX.md §2", "AFFILIATE_INTEGRATION_GUIDE.md §7"], ["crates/affiliate/src/providers/accesstrade/*"],
  ["Ineligible/suspended URLs map to ProviderError::Ineligible", "Never logs token header", "Live verification checklist completed in staging before mode=Live"],
  ["Credentials only via admin, encrypted"], ["contract tests; live verification evidence (manual) — required before enabling"],
  extra_dod=["Owner confirms OB-2/OB-3 (account + campaign approval) before live verification"])

issue("AAO-025", "Tracking: /analyze, /links, opaque tokens, /go redirect, clicks", "M2", OPUS, "high", "L", ["AAO-020", "AAO-021", "AAO-022", "AAO-023", "AAO-024", "AAO-010"],
  "End-to-end link generation and click tracking.", "Core parity journey F-02..F-06, F-28.",
  ["Tables link_requests, clicks (monthly partitions + creator job)", "Token: 96-bit OsRng, Crockford base32", "Estimate via rules (labelled estimate, basis recorded)",
   "/go/{token}: validate, record click, 302 or interstitial for in-app UAs; redirect only to stored provider URL on allowlisted host", "Link TTL config; expired page"],
  ["TRACKING_AND_ATTRIBUTION.md", "ARCHITECTURE.md §6.1", "API_SPECIFICATION.md"],
  ["crates/app/src/tracking/*", "crates/web/src/routes/{analyze.rs,links.rs,go.rs}", "migrations/NNNN_AAO-025_tracking.sql"],
  ["Provider/campaign/rule version pinned on link", "No PII in any outbound URL", "/go p95 < 50 ms in k6 smoke"],
  ["Open-redirect impossible (T-REDIR-1)", "Rate limits per API spec"], ["integration with mock provider", "T-REDIR-1, k6 smoke"])

issue("AAO-026", "Landing, analyzer result, link result pages", "M2", SONNET, "medium", "L", ["AAO-014", "AAO-025"],
  "User-facing link flow UI.", "Primary conversion funnel; parity F-01, F-03..F-06, F-25, F-28.",
  ["Pages per UI spec §5–6; PasteBox with clipboard; preview card; EstimateBadge; purchase-rules checklist; in-app browser notice; desktop QR", "Support contact links from config", "SEO meta + JSON-LD for landing"],
  ["UI_UX_SPECIFICATION.md §5–6"], ["crates/ui/src/pages/{landing.rs,analyze.rs,link_result.rs}"],
  ["Works without JS for paste+submit", "Never shows estimate without 'ước tính'"], ["No third-party scripts"], ["Playwright mobile+desktop journey 1 (TEST_STRATEGY §3)"])

issue("AAO-027", "Admin: providers, merchants, campaigns, routing, rules UI", "M2", SONNET, "medium", "M", ["AAO-016", "AAO-022", "AAO-023"],
  "Operate catalog and rules without SQL.", "Commission config must not be hardcoded.",
  ["CRUD + simulation views; cashback_allowed toggle requires evidence ref; rules diff + approve"],
  ["ADMIN_DASHBOARD.md §1"], ["crates/ui/src/admin/{providers,merchants,campaigns,routes,rules}/*"],
  ["Optimistic concurrency (412) handled in UI"], ["Credentials write-only"], ["Playwright admin flows; authz per role"])

# ---------------- M3 ----------------
issue("AAO-030", "Ledger core: accounts, post(), guards, immutability, balances, invariant checker", "M3", OPUS, "max", "XL", ["AAO-003", "AAO-004"],
  "Implement the double-entry ledger exactly as specified.", "Financial source of truth; every cashback đồng flows through it.",
  ["Tables ledger_accounts, ledger_transactions, ledger_entries, ledger_balances", "Deferred balanced-transaction constraint trigger; UPDATE/DELETE blocking triggers; app_rw grants without UPDATE/DELETE",
   "`post(tx, Posting, guards)` with deterministic lock order, idempotency key + payload hash", "Account resolver/creator (lazy per user/provider)",
   "Nightly invariant job I1–I6, I11 with metrics + alerts", "Views v_user_wallet, v_platform_position"],
  ["ADR-0010", "FINANCIAL_LEDGER.md (all)", "DATABASE_DESIGN.md §4.7, §6"],
  ["crates/ledger/src/*", "migrations/NNNN_AAO-030_ledger.sql", "crates/worker/src/jobs/ledger_invariants.rs"],
  ["All §5 worked examples reproduced as tests with exact balances", "Unbalanced tx rejected at commit", "Replay returns same TxId; mismatched payload errors"],
  ["Owner approval required (protected path)"], ["T-LEDGER-1/2, proptest random posting sequences keep I1/I3/I4", "concurrency: parallel posts to same account"],
  extra_dod=["Owner signs off chart of accounts"], parallel="Parallel with AAO-020/021/022 after M0.")

issue("AAO-031", "Conversion ingestion pipeline: raw events, normalizer, monotonic upsert, state machine", "M3", OPUS, "high", "L", ["AAO-022", "AAO-030", "AAO-025"],
  "One idempotent pipeline for poll/webhook/CSV/backfill.", "Accurate order tracking (F-12) and protection from duplicates/out-of-order data.",
  ["Tables provider_events_raw, conversions, conversion_status_history, sync_checkpoints", "Per-provider redaction allowlist",
   "Upsert with provider_updated_at guard + status rank; row lock for transitions", "Attribution by token → matched/unmatched", "Emits outbox events"],
  ["ORDER_RECONCILIATION.md §1–2, §4.1, §5", "ARCHITECTURE.md §6.2"],
  ["crates/app/src/conversions/*", "migrations/NNNN_AAO-031_conversions.sql"],
  ["T-DUP-1, T-OOO-1/2, T-MULTI-1, T-CRASH-1/2, T-CONC-2 pass"], ["Raw payloads redacted before insert"], ["listed scenario tests"])

issue("AAO-032", "ACCESSTRADE conversion sync (transactions API) with schedules and quota control", "M3", OPUS, "high", "M", ["AAO-024", "AAO-031"],
  "Poll ACCESSTRADE transactions reliably within 10 req/min.", "Primary source of order status.",
  ["ConversionSource: GET /v1/transactions with update_time windows, limit=300, paging", "Status mapping table (0/1/2 + is_confirmed) — confirm against live data",
   "Streams recent/daily/monthly per ORDER_RECONCILIATION §2; 429 Retry-After handling", "Quota metrics + alert at 70%"],
  ["AFFILIATE_PROVIDER_MATRIX.md §2", "ORDER_RECONCILIATION.md §2"], ["crates/affiliate/src/providers/accesstrade/conversions.rs", "crates/worker/src/schedules/accesstrade.rs"],
  ["Sync freshness SLO measurable", "Never exceeds 8 req/min in tests"], ["Token never logged"], ["contract tests with paging, 429, timeouts"])

issue("AAO-033", "Cashback entitlements: compute, transitions, ledger postings, freeze", "M3", OPUS, "xhigh", "L", ["AAO-023", "AAO-030", "AAO-031"],
  "Translate conversion changes into user obligations.", "What users see as their cashback.",
  ["Table cashback_entitlements", "Recompute on each conversion revision with pinned rule; delta postings with ent:{id}:rev:{n} keys", "Transitions per §4.2; frozen_reason handling", "Outbox notifications"],
  ["FINANCIAL_LEDGER.md §5–6", "ORDER_RECONCILIATION.md §4.2"], ["crates/app/src/entitlements/*", "migrations/NNNN_AAO-033_entitlements.sql"],
  ["T-REF-1, T-RULE-1 pass; I6/I7 hold under proptest sequences"], ["Fraud void requires approval"], ["proptest: random conversion event sequences → ledger consistent"])

issue("AAO-034", "Settlements, allocation, suspense, advance policy, coverage invariant", "M3", OPUS, "max", "L", ["AAO-033"],
  "Make cashback available only when funded or explicitly advanced.", "Cash-flow protection (brief §3, ADR-0011).",
  ["Tables provider_settlements, settlement_allocations, advance_policy", "Finance records settlement + uploads statement; matcher; allocation postings; suspense",
   "Advance availability job under policy lock; exposure calc", "Coverage invariant I8 daily + auto payout suspension"],
  ["ADR-0011", "FINANCIAL_LEDGER.md §5.1, §7"], ["crates/app/src/settlement/*", "migrations/NNNN_AAO-034_settlement.sql"],
  ["T-SET-1, T-ADV-1, T-COV-1 pass", "Advance off by default"], ["Maker-checker on settlement recording"], ["integration + proptest on allocation sums (I12)"],
  extra_dod=["Owner decision D-2 recorded (advance policy values)"])

issue("AAO-035", "Reconciliation jobs and cases", "M3", OPUS, "high", "M", ["AAO-034", "AAO-032"],
  "Detect and track discrepancies.", "Catches provider/ledger drift before it costs money.",
  ["reconciliation_cases table", "Daily provider-vs-local totals per status/day", "Invariant results surfaced; case workflow with adjustment link"],
  ["ORDER_RECONCILIATION.md §3"], ["crates/app/src/reconciliation/*", "crates/worker/src/jobs/recon_*.rs", "migrations/NNNN_AAO-035_recon.sql"],
  ["Seeded discrepancy opens a case"], ["Adjustments require approval"], ["integration"])

issue("AAO-036", "User dashboard, orders, order detail, wallet pages", "M3", SONNET, "medium", "L", ["AAO-033", "AAO-026"],
  "Users see orders and balances accurately.", "Parity F-11..F-13; trust.",
  ["APIs /wallet, /wallet/transactions, /orders, /orders/{id} (keyset pagination)", "Pages per UI spec with Timeline and state colours", "Owed (negative) banner"],
  ["UI_UX_SPECIFICATION.md §5", "API_SPECIFICATION.md"], ["crates/web/src/routes/{wallet.rs,orders.rs}", "crates/ui/src/pages/{dashboard.rs,orders/*,wallet.rs}"],
  ["Totals equal v_user_wallet", "IDOR tests pass"], ["Owner-scoped queries"], ["T-AUTHZ-1; Playwright journey 3 (partial)"])

issue("AAO-037", "Admin: conversions, ledger, settlements, reconciliation UI", "M3", SONNET, "medium", "L", ["AAO-016", "AAO-034", "AAO-035"],
  "Finance and support tooling for money states.", "Operability of reconciliation.",
  ["Views per ADMIN_DASHBOARD.md; raw event viewer (redacted); settlement upload; suspense; invariant status"],
  ["ADMIN_DASHBOARD.md"], ["crates/ui/src/admin/{conversions,ledger,settlements,recon}/*"],
  ["Role permissions enforced"], ["Exports audited"], ["Playwright admin"])

# ---------------- M4 ----------------
issue("AAO-040", "Envelope encryption at rest and key rotation", "M4", OPUS, "high", "M", ["AAO-004"],
  "Encrypt bank numbers, TOTP secrets, provider credentials.", "Limits blast radius of DB/backup leaks (I-2).",
  ["AES-256-GCM with per-row DEK wrapped by KEK from secret file; key_id column", "Rotation job re-wraps/re-encrypts; KEK loading fails closed"],
  ["ADR-0008", "SECURITY_THREAT_MODEL.md I-2"], ["crates/app/src/crypto.rs", "crates/worker/src/jobs/rekey.rs"],
  ["Ciphertext differs per row; tampering detected"], ["Keys never logged; zeroize"], ["unit + rotation integration"], parallel="Can start right after AAO-004.")

issue("AAO-041", "Bank accounts: add/change with step-up and cooldown, bank list", "M4", OPUS, "medium", "M", ["AAO-040", "AAO-013"],
  "Users register a payout destination safely.", "Parity F-15; ATO defence.",
  ["Table bank_accounts + banks seed (source documented)", "Encrypted number, last4, dedupe hash; holder name normalization", "Step-up, cooldown, old-email notification"],
  ["WITHDRAWAL_DESIGN.md §6"], ["crates/app/src/payouts/bank_accounts.rs", "migrations/NNNN_AAO-041_bank_accounts.sql"],
  ["Only one active account; cooldown enforced"], ["A-6 controls"], ["integration + authz"])

issue("AAO-042", "Withdrawals: state machine, reservation, limits, risk checks", "M4", OPUS, "max", "L", ["AAO-030", "AAO-041", "AAO-034"],
  "Request→review→approve flow with no double spend.", "Parity F-14; highest fraud surface.",
  ["Tables withdrawals, withdrawal_status_history", "Reservation under lock + NonNegative guard + partial unique index + Idempotency-Key", "Config limits (effective-dated); risk flags §4", "Cancel/reject releases"],
  ["WITHDRAWAL_DESIGN.md §2–4", "FINANCIAL_LEDGER.md §5"], ["crates/app/src/payouts/withdrawals.rs", "crates/web/src/routes/withdrawals.rs", "migrations/NNNN_AAO-042_withdrawals.sql"],
  ["T-CONC-1, T-DUP-2 pass", "Negative available blocks requests"], ["Step-up required; owner approval on migration"], ["concurrency + property tests"])

issue("AAO-043", "Payout operations: VietQR payload, batches, statement import, unknown handling, suspension", "M4", OPUS, "high", "L", ["AAO-042"],
  "Execute and reconcile manual payouts.", "Pay users correctly and provably.",
  ["VietQR (EMVCo) payload generator with CRC, validated against official spec examples", "payout_attempts, bank_statement_lines; CSV importers per bank (pluggable)",
   "Submit (≠ approver), match, paid, failed, unknown transitions + postings", "payouts.suspended kill switch", "PayoutRail trait; MoMo adapter Disabled stub"],
  ["WITHDRAWAL_DESIGN.md §1, §3, §5, §7", "ADR-0012"], ["crates/app/src/payouts/{rails.rs,vietqr.rs,statements.rs}", "migrations/NNNN_AAO-043_payout_ops.sql"],
  ["T-PAY-1, T-PAY-2 pass", "Unknown never auto-retried"], ["Evidence hashes stored; maker-checker"], ["integration + fixtures of statement CSV"])

issue("AAO-044", "User withdrawal and bank account UI", "M4", SONNET, "medium", "M", ["AAO-036", "AAO-041", "AAO-042"],
  "Self-service cash-out UI.", "Parity F-14/F-15.", ["Pages per UI spec; StepUpDialog; client-generated Idempotency-Key per form; suspended banner"],
  ["UI_UX_SPECIFICATION.md §5"], ["crates/ui/src/pages/{withdraw.rs,withdrawals.rs,bank_account.rs}"],
  ["Double-click cannot create two requests"], ["Masked account display"], ["Playwright journey 3"])

issue("AAO-045", "Admin withdrawal review queue and payout batch UI", "M4", SONNET, "medium", "M", ["AAO-043", "AAO-037"],
  "Finance UI for review, batch, VietQR display, statement import.", "Operational payout capacity.",
  ["Review queue with risk flags; batch view with QR; mark submitted with evidence; unknown queue"],
  ["ADMIN_DASHBOARD.md", "WITHDRAWAL_DESIGN.md §5"], ["crates/ui/src/admin/withdrawals/*"],
  ["Approver cannot submit same withdrawal (UI + server)"], ["Step-up on actions"], ["Playwright finance flow"])

# ---------------- M5 ----------------
issue("AAO-050", "Notifications: outbox consumer, email templates, in-app centre", "M5", SONNET, "medium", "M", ["AAO-011", "AAO-033", "AAO-042"],
  "Notify users of order and withdrawal changes exactly once.", "Parity F-20; reduces support load.",
  ["notifications table; consumer with inbox dedupe; vi/en templates; bell + list UI"],
  ["ARCHITECTURE.md §7", "DATABASE_DESIGN.md §4.9"], ["crates/app/src/notifications/*", "crates/ui/src/pages/notifications.rs", "migrations/NNNN_AAO-050_notifications.sql"],
  ["One notification per state change (dedupe_key)"], ["No sensitive data in email bodies beyond masked values"], ["integration dedupe"])

issue("AAO-051", "Legal documents, consents, data export and account closure", "M5", SONNET, "medium", "M", ["AAO-010"],
  "Versioned ToS/Privacy and PDPL rights.", "Parity F-27 plus compliance gaps closed.",
  ["legal_documents, consents; re-consent on new version", "/me/data-export (async JSON), /me/close with pseudonymization job keeping financial records"],
  ["COMPLIANCE_CHECKLIST.md", "DATABASE_DESIGN.md §7"], ["crates/app/src/privacy/*", "crates/ui/src/pages/legal.rs", "migrations/NNNN_AAO-051_legal.sql"],
  ["Export contains all user data categories", "Closure keeps ledger integrity"], ["Export download link expires; audited"], ["integration"],
  extra_dod=["Legal text supplied/approved by owner's counsel (placeholder until then)"])

issue("AAO-090", "Security review: authz matrix, threat model re-check, abuse tests", "M5", OPUS, "high", "M", ["AAO-045", "AAO-036", "AAO-050"],
  "Independent pass over the complete V1.", "Launch gate.", ["Generate authz tests from role table; review every route; update threat model; fix or ticket findings"],
  ["SECURITY_THREAT_MODEL.md"], ["crates/web/tests/authz_matrix.rs", "docs/SECURITY_THREAT_MODEL.md"],
  ["No open High findings"], ["—"], ["security regression suite"])

issue("AAO-091", "Security headers, CSP, CAPTCHA hook, rate-limit tuning", "M5", SONNET, "medium", "S", ["AAO-026"],
  "Harden HTTP surface.", "XSS/clickjacking defence.", ["Headers per threat model §5; Turnstile (or equivalent) behind config; rate-limit values from config"],
  ["SECURITY_THREAT_MODEL.md §5"], ["crates/web/src/middleware/headers.rs", "infra/caddy/Caddyfile"],
  ["Mozilla Observatory A+ on staging"], ["—"], ["header tests"])

issue("AAO-092", "Production compose, Caddy, observability stack, alerts", "M5", SONNET, "medium", "L", ["AAO-002", "AAO-005"],
  "Deployable stack on a VPS.", "Go-live infrastructure.",
  ["compose.prod.yml per INFRASTRUCTURE.md §2; networks; secrets; hardening flags", "otel-collector, Prometheus, Loki, Grafana dashboards (RED, provider, business), alert rules §8", "VPS bootstrap script (ufw, users, fail2ban, unattended-upgrades)"],
  ["INFRASTRUCTURE.md", "ADR-0009"], ["infra/compose.prod.yml", "infra/caddy/*", "infra/observability/*", "infra/bootstrap/*.sh"],
  ["Staging up from scratch with one command + documented secrets"], ["Only 80/443/22 open"], ["staging smoke"])

issue("AAO-093", "Backups: WAL archiving, encrypted off-site, automated restore test", "M5", SONNET, "medium", "M", ["AAO-092", "AAO-030"],
  "Meet RPO 15 min / RTO 4 h.", "Survive host loss.", ["wal-g or pgBackRest; client-side encryption; weekly restore + invariant check workflow"],
  ["INFRASTRUCTURE.md §7"], ["infra/backup/*", ".github/workflows/restore-test.yml"],
  ["Restore drill documented with timings"], ["Backup key not on VPS"], ["weekly restore job green"])

issue("AAO-094", "Deploy/rollback workflows and runbooks", "M5", SONNET, "medium", "M", ["AAO-092"],
  "Safe releases and incident playbooks.", "Operational readiness.", ["deploy-staging/prod workflows with environment approval, migrate job, smoke, auto-rollback", "Runbooks RB-01..RB-09"],
  ["INFRASTRUCTURE.md §4–6, §9"], [".github/workflows/deploy-*.yml", "scripts/rollback.sh", "docs/runbooks/*.md"],
  ["Rollback exercised on staging"], ["Prod deploy requires owner approval"], ["staging rehearsal"])

issue("AAO-095", "Full E2E, accessibility and load test suites", "M5", SONNET, "medium", "M", ["AAO-044", "AAO-045"],
  "Automate journeys and performance gates.", "Regression safety before launch.", ["Playwright journeys 1–5, axe, k6 scenarios with thresholds (TEST_STRATEGY §3–4)"],
  ["TEST_STRATEGY.md"], ["e2e/*", "load/*", ".github/workflows/nightly.yml"],
  ["Thresholds met on staging"], ["Test-only fixture endpoints compiled out of prod"], ["—"])

issue("AAO-096", "Launch readiness: live provider verification, financial dry run, staging soak", "M5", OPUS, "high", "M", ["AAO-090", "AAO-093", "AAO-094", "AAO-095", "AAO-032"],
  "Prove the money path end-to-end with real providers before opening to users.", "Launch gate; integrations must not be declared verified from mocks.",
  ["Real low-value purchases via ACCESSTRADE links (Shopee + TikTok) → sync → approve → settlement record → payout to staff account", "7-day staging soak with invariant checks", "Go/no-go checklist"],
  ["AFFILIATE_INTEGRATION_GUIDE.md §6", "TEST_STRATEGY.md §6"], ["docs/LAUNCH_CHECKLIST.md", "docs/PROJECT_STATUS.md"],
  ["Evidence attached; all invariants green; owner go decision"], ["Real credentials only in staging/prod secrets"], ["manual + automated"],
  extra_dod=["Owner go/no-go recorded"])

# ---------------- M6 (V1.x) ----------------
issue("AAO-052", "Support tickets and missing-cashback claims", "M6", SONNET, "medium", "M", ["AAO-031", "AAO-016"],
  "Let users report missing cashback with evidence.", "Parity F-26; trust.", ["tickets, ticket_messages, missing_cashback_claims; auto-search; private uploads (image only, 2 MB, scanned)"],
  ["ORDER_RECONCILIATION.md §6"], ["crates/app/src/support/*", "crates/ui/src/pages/support/*", "migrations/NNNN_AAO-052_support.sql"],
  ["Claim links to click and found conversion"], ["Upload validation; no public URLs"], ["integration + Playwright journey 4"])

issue("AAO-060", "P1 provider research: Lazada, Ecomobi, MasOffer, Adpia, Permate", "M6", SONNET, "medium", "M", [],
  "Fill provider matrix with verified facts.", "Diversify beyond one network (R-02).", ["Read official docs/terms; update matrix with labels and sources; onboarding questions list"],
  ["AFFILIATE_PROVIDER_MATRIX.md §5"], ["docs/AFFILIATE_PROVIDER_MATRIX.md"], ["Each row has sources or UNKNOWN"], ["No credentials in docs"], ["—"],
  parallel="Can run any time; docs only.")

issue("AAO-061", "Shopee direct adapter (disabled + mock) and readiness checklist", "M6", OPUS, "high", "M", ["AAO-022", "AAO-031"],
  "Prepare a redundant Shopee route.", "Reduce single-provider dependency.", ["Adapter skeleton behind Disabled; mock clearly labelled; enable only after Open API docs + permission"],
  ["AFFILIATE_PROVIDER_MATRIX.md §3"], ["crates/affiliate/src/providers/shopee_direct/*"], ["Cannot be enabled in prod without credentials + approval"], ["Secret signing per official docs only"], ["contract tests from real responses once available"])

issue("AAO-070", "Single-level referral programme", "M6", OPUS, "high", "M", ["AAO-034", "AAO-013"],
  "Referral codes, attribution, rewards on settled orders.", "Parity F-16 with anti-abuse.",
  ["referrals, reward_campaigns; attribution at signup only; reward postings from budget; reversal propagation; clustering signals", "Flag referrals.enabled; multilevel hard-off"],
  ["ADR-0014", "FINANCIAL_LEDGER.md §5.1", "SECURITY_THREAT_MODEL.md A-2/A-3"], ["crates/app/src/referrals/*", "crates/ui/src/pages/referral.rs", "migrations/NNNN_AAO-070_referrals.sql"],
  ["No self/circular referral; budget never exceeded (I13)"], ["Fraud signals hold rewards"], ["proptest budget; integration reversal"])

issue("AAO-071", "Blog CMS, FAQ and SEO (sitemap, canonical, JSON-LD)", "M6", SONNET, "medium", "M", ["AAO-016", "AAO-014"],
  "Content for acquisition.", "Parity F-21.", ["posts with Markdown (pulldown-cmark + ammonia), categories/tags, views, sitemap.xml, robots, hreflang"],
  ["UI_UX_SPECIFICATION.md"], ["crates/app/src/content/*", "crates/ui/src/pages/blog/*", "migrations/NNNN_AAO-071_content.sql"],
  ["Lighthouse SEO ≥ 95"], ["Sanitized HTML (XSS tests)"], ["Playwright blog"])

issue("AAO-072", "Daily check-in and reward campaigns", "M6", OPUS, "medium", "M", ["AAO-034"],
  "Engagement rewards within budget.", "Parity F-18.", ["checkins per VN day; streaks; promo postings (5110); budget lock"],
  ["FINANCIAL_LEDGER.md §5.1"], ["crates/app/src/rewards/*", "crates/ui/src/pages/checkin.rs", "migrations/NNNN_AAO-072_checkins.sql"],
  ["One check-in per VN day even across midnight races"], ["Bot farming limits"], ["concurrency + clock tests"])

issue("AAO-073", "PWA: manifest, service worker (static only), install prompt", "M6", SONNET, "low", "S", ["AAO-026"],
  "Installable app experience.", "Parity F-24.", ["Manifest + icons; SW caches static assets only; versioned"],
  ["UI_UX_SPECIFICATION.md §8"], ["crates/ui/public/{manifest.webmanifest,sw.js}"], ["Installable on Android Chrome and iOS"], ["Never caches authenticated HTML/API"], ["Playwright SW check"])

issue("AAO-075", "Leaderboard (flag-gated)", "M6", SONNET, "low", "S", ["AAO-036"],
  "Optional ranking page.", "Parity F-19 (reference currently disabled).", ["Materialized ranks, masked names, period selector"],
  ["FEATURE_PARITY_MATRIX.md F-19"], ["crates/app/src/leaderboard.rs", "crates/ui/src/pages/leaderboard.rs"], ["Names masked"], ["No PII leakage"], ["integration"])

# ---------------- render ----------------
def md():
    out = []
    out.append("# Engineering Backlog\n")
    out.append("Generated by `python3 scripts/backlog/generate.py . /tmp/issues.json` — edit the generator, not this file. GitHub issues mirror it. Status: PROPOSED until the architecture PR is approved.\n")
    out.append("Model policy: complex financial, security, and architectural work → **Claude Opus 5.5**; well-defined frontend, standard APIs, docs, tests, infra → **Claude Sonnet 5.5**.\n")
    out.append("## 1. Milestones\n")
    out.append("| Milestone | Goal | Issues |\n|---|---|---|")
    goals = {"M0": "Compiling workspace, CI, primitives, DB infra", "M1": "Accounts, staff security, UI shell", "M2": "Paste → link → redirect working (mock + ACCESSTRADE)",
             "M3": "Conversions flow into a correct ledger", "M4": "Users can withdraw; finance can pay and reconcile", "M5": "Production-ready, verified with real providers", "M6": "V1.x engagement features"}
    for k, v in M.items():
        ids = ", ".join(i["id"] for i in I if i["milestone"] == k)
        out.append(f"| {v} | {goals[k]} | {ids} |")
    out.append("\n## 2. Dependency graph\n")
    out.append("```mermaid\nflowchart LR")
    for i in I:
        for d in i["deps"]:
            out.append(f"    {d.replace('-', '')}[{d}] --> {i['id'].replace('-', '')}[{i['id']}]")
    out.append("```\n")
    out.append("## 3. Parallel lanes (safe to run concurrently once deps are merged)\n")
    out.append("| Wave | Opus lane | Sonnet lane |\n|---|---|---|")
    waves = [
        ("1", "—", "AAO-001"),
        ("2", "AAO-003, AAO-004, AAO-021", "AAO-002, AAO-005, AAO-014, AAO-060"),
        ("3", "AAO-010, AAO-020, AAO-022, AAO-030, AAO-040", "—"),
        ("4", "AAO-012, AAO-013, AAO-024", "AAO-011, AAO-051"),
        ("5", "AAO-023, AAO-041", "AAO-015, AAO-016, AAO-092"),
        ("6", "AAO-025", "AAO-027, AAO-093, AAO-094"),
        ("7", "AAO-031", "AAO-026"),
        ("8", "AAO-032, AAO-033", "AAO-091, AAO-052"),
        ("9", "AAO-034", "AAO-036, AAO-071, AAO-073"),
        ("10", "AAO-035, AAO-042", "—"),
        ("11", "AAO-043", "AAO-037, AAO-044, AAO-050"),
        ("12", "—", "AAO-045"),
        ("13", "AAO-090", "AAO-095"),
        ("14", "AAO-096 (launch gate)", "—"),
        ("V1.x", "AAO-061, AAO-070, AAO-072", "AAO-075"),
    ]
    for w in waves:
        out.append(f"| {w[0]} | {w[1]} | {w[2]} |")
    out.append("\n**File-ownership rule:** two concurrently open PRs must not list the same file under *Expected files* except `Cargo.toml` workspace members (append-only) and `crates/web/src/routes/mod.rs` (one line each). Migrations are numbered at rebase time (AGENTS.md §8); each migration's tables are owned by exactly one issue (see table owner column below).\n")
    out.append("## 4. Issue index\n")
    nums_path = pathlib.Path(__file__).with_name("issue_numbers.json")
    nums = json.loads(nums_path.read_text()) if nums_path.exists() else {}
    out.append("| ID | GitHub | Title | Milestone | Model | Effort | Size | Depends on |\n|---|---|---|---|---|---|---|---|")
    for i in I:
        gh = f"#{nums[i['id']]}" if i['id'] in nums else "—"
        out.append(f"| {i['id']} | {gh} | {i['title']} | {M[i['milestone']]} | {i['model'].replace('Claude ', '')} | {i['effort']} | {i['complexity']} | {', '.join(i['deps']) or '—'} |")
    out.append("\n## 5. V1 requirement → issue coverage\n")
    cov = [
        ("F-01 Landing", "AAO-026"), ("F-02 URL submission", "AAO-020, AAO-021, AAO-025, AAO-026"), ("F-03 Preview", "AAO-024, AAO-025, AAO-026"),
        ("F-04 Estimate", "AAO-023, AAO-025"), ("F-05 Link generation", "AAO-022, AAO-024, AAO-025"), ("F-06 App redirection", "AAO-025, AAO-026"),
        ("F-07 Registration", "AAO-010, AAO-011, AAO-015"), ("F-08 Login", "AAO-010, AAO-015"), ("F-09 Google", "AAO-012, AAO-015"), ("F-10 Recovery", "AAO-011, AAO-015"),
        ("F-11 Dashboard", "AAO-036"), ("F-12 Order history", "AAO-031, AAO-032, AAO-033, AAO-036"), ("F-13 Wallet", "AAO-030, AAO-036"),
        ("F-14 Withdrawal", "AAO-042, AAO-043, AAO-044, AAO-045"), ("F-15 Bank account", "AAO-041, AAO-044"), ("F-22 Theme", "AAO-014"), ("F-23 i18n", "AAO-014"),
        ("F-25 Support channels", "AAO-026"), ("F-27 ToS/Privacy", "AAO-051"), ("F-28 Purchase rules", "AAO-026"),
        ("Admin console (PRD §4)", "AAO-013, AAO-016, AAO-027, AAO-037, AAO-045"), ("Settlement gating (ADR-0011)", "AAO-034"), ("Reconciliation", "AAO-035"),
        ("Security / launch", "AAO-090, AAO-091, AAO-095, AAO-096"), ("Infra / backups / deploy", "AAO-002, AAO-092, AAO-093, AAO-094"),
        ("Enablers (workspace, primitives, DB infra, telemetry, encryption)", "AAO-001, AAO-003, AAO-004, AAO-005, AAO-040"),
        ("Provider diversification (post-V1)", "AAO-060, AAO-061"),
        ("V1.x: F-16 Referral", "AAO-070"), ("V1.x: F-18 Check-in", "AAO-072"), ("V1.x: F-20 Notifications", "AAO-050"), ("V1.x: F-21 Blog", "AAO-071"),
        ("V1.x: F-24 PWA", "AAO-073"), ("V1.x: F-26 Missing cashback", "AAO-052"), ("P2: F-19 Leaderboard", "AAO-075"), ("F-17 Multi-level", "none — disabled by ADR-0014"),
    ]
    out.append("| Requirement | Issues |\n|---|---|")
    for r, ids in cov:
        out.append(f"| {r} | {ids} |")
    out.append("\n## 6. Issue details\n")
    for i in I:
        out.append(render_issue(i, heading=True))
    return "\n".join(out) + "\n"

def render_issue(i, heading=False):
    b = []
    if heading:
        b.append(f"### {i['id']}: {i['title']}\n")
    b.append(f"**Milestone:** {M[i['milestone']]} · **Assigned AI model:** {i['model']} · **Reasoning effort:** {i['effort']} · **Estimated complexity:** {i['complexity']}\n")
    b.append(f"**Objective.** {i['objective']}\n")
    b.append(f"**Business context.** {i['business']}\n")
    b.append("**Technical requirements**\n" + "\n".join(f"- {t}" for t in i["tech"]) + "\n")
    b.append(f"**Dependencies.** {', '.join(i['deps']) or 'None'}" + (f" · *Parallelism:* {i['parallel']}" if i["parallel"] else "") + "\n")
    b.append("**Architecture references.** " + "; ".join(f"`{r}`" if r.endswith('.md') or '/' in r else r for r in i["refs"]) + "\n")
    b.append("**Expected files or modules**\n" + "\n".join(f"- `{f}`" for f in i["files"]) + "\n")
    b.append("**Acceptance criteria**\n" + "\n".join(f"- [ ] {a}" for a in i["ac"]) + "\n")
    b.append("**Security considerations**\n" + "\n".join(f"- {s}" for s in i["security"]) + "\n")
    b.append("**Test requirements**\n" + "\n".join(f"- {t}" for t in i["tests"]) + "\n")
    b.append("**Definition of Done**\n" + "\n".join(f"- [ ] {d}" for d in i["dod"]) + "\n")
    return "\n".join(b)

if __name__ == "__main__":
    root = pathlib.Path(sys.argv[1])
    (root / "docs/ENGINEERING_BACKLOG.md").write_text(md())
    issues = [dict(id=i["id"], title=f"{i['id']}: {i['title']}", milestone=M[i["milestone"]],
                   labels=["model:opus" if i["model"] == OPUS else "model:sonnet", f"effort:{i['effort']}", f"size:{i['complexity']}", i["milestone"].lower()],
                   deps=i["deps"], body=render_issue(i)) for i in I]
    pathlib.Path(sys.argv[2]).write_text(json.dumps(issues, ensure_ascii=False, indent=1))
    print(len(I), "issues")

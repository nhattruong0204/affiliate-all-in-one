"""Release overlay (2026-11-18): maps every backlog issue to its earliest required release.

Fields
  rel         earliest release that requires the issue: v0.1 | v0.5 | v1.0 | v1.x
  blocker     True  -> the release cannot ship without it (label launch-blocker)
  parallel    True  -> off the release's critical path; safe to run concurrently (label parallelizable)
  commercial  non-engineering prerequisite (label commercial-blocker), else None
  simplify    MVP scope reduction for this release ("" = full scope required)
  flag        feature flag / readiness gate protecting it in production ("—" = none needed)
  effort      reassessed reasoning effort (model is never changed)
  deps        optional dependency override (only where the release split required it)
"""

RELEASES = {
    "v0.1": "V0.1 — Public Website Preview",
    "v0.5": "V0.5 — Private Alpha",
    "v1.0": "V1.0 — Commercial Cashback Platform",
    "v1.x": "V1.x — Growth and Feature Expansion",
}

R = {
  # ---------------- V0.1 Public Website Preview ----------------
  "AAO-001": dict(rel="v0.1", blocker=True,  parallel=False, commercial=None, effort="medium",
                  simplify="", flag="—"),
  "AAO-002": dict(rel="v0.1", blocker=True,  parallel=True,  commercial=None, effort="low",
                  simplify="V0.1: multi-stage Dockerfile, non-root distroless image, Trivy gate, Renovate. Cosign signing + SBOM move to the V1.0 gate (tracked in AAO-090 checklist).",
                  flag="—"),
  "AAO-003": dict(rel="v0.1", blocker=True,  parallel=False, commercial=None, effort="high",
                  simplify="", flag="—"),
  "AAO-004": dict(rel="v0.1", blocker=True,  parallel=True,  commercial=None, effort="high",
                  simplify="V0.1 needs pool, migrations, roles, rate_limit_buckets, idempotency_keys, jobs. Outbox/inbox may land in the same PR or a follow-up before AAO-031.",
                  flag="—"),
  "AAO-005": dict(rel="v0.1", blocker=True,  parallel=True,  commercial=None, effort="medium",
                  simplify="OTLP exporter optional in V0.1 (JSON logs + Prometheus metrics suffice).", flag="—"),
  "AAO-014": dict(rel="v0.1", blocker=True,  parallel=False, commercial=None, effort="medium",
                  simplify="V0.1 components: AppShell, ThemeToggle, LanguageSwitch, EmptyState/ErrorState, Toast. Money/StatePill components may land with AAO-036.",
                  flag="—"),
  "AAO-020": dict(rel="v0.1", blocker=True,  parallel=False, commercial=None, effort="medium",
                  simplify="", flag="—"),
  "AAO-021": dict(rel="v0.1", blocker=True,  parallel=True,  commercial=None, effort="high",
                  simplify="", flag="—"),
  "AAO-091": dict(rel="v0.1", blocker=True,  parallel=True,  commercial=None, effort="low",
                  simplify="V0.1: security headers + CSP + Turnstile on waitlist/analyze + rate limits for public endpoints.",
                  flag="—", deps=["AAO-005", "AAO-014"]),
  "AAO-092": dict(rel="v0.1", blocker=True,  parallel=True,  commercial="D-6 hosting provider/region (PDPL cross-border question C-PD-7); D-4 domain",
                  effort="medium",
                  simplify="V0.1: Caddy + web + postgres + backup on one VPS, external uptime monitor, Prometheus + Grafana minimal (RED + uptime). Loki/OTel collector and business dashboards are required by the V0.5 gate.",
                  flag="—"),
  "AAO-093": dict(rel="v0.1", blocker=True,  parallel=True,  commercial=None, effort="medium",
                  simplify="Restore test checks row counts in V0.1; the ledger invariant check is added by AAO-030.",
                  flag="—", deps=["AAO-092"]),
  "AAO-094": dict(rel="v0.1", blocker=True,  parallel=True,  commercial=None, effort="low",
                  simplify="V0.1 runbooks: RB-01 site down, RB-07 data breach, RB-08 restore, RB-09 secret rotation. Money runbooks (RB-02/03/05) are added before V1.0.",
                  flag="—"),
  "AAO-100": dict(rel="v0.1", blocker=True,  parallel=False, commercial="D-4 legal entity/brand/domain; counsel-reviewed privacy notice for waitlist (C-PD-1)",
                  effort="medium", simplify="", flag="`signup.mode=waitlist` (default)"),

  # ---------------- V0.5 Private Alpha ----------------
  "AAO-010": dict(rel="v0.5", blocker=True,  parallel=False, commercial=None, effort="high",
                  simplify="Adds invite-only registration (`signup.mode=invite`, single-use invite codes issued from the waitlist by CLI/admin).",
                  flag="`signup.mode` = waitlist | invite | open"),
  "AAO-011": dict(rel="v0.5", blocker=True,  parallel=True,  commercial="D-7 transactional email provider", effort="low",
                  simplify="", flag="—"),
  "AAO-013": dict(rel="v0.5", blocker=True,  parallel=False, commercial=None, effort="high",
                  simplify="Audit log is append-only with daily off-site export in V0.5; the hash chain moves to the V1.0 gate. Approvals table needed for rule versions only.",
                  flag="—"),
  "AAO-015": dict(rel="v0.5", blocker=True,  parallel=True,  commercial=None, effort="medium",
                  simplify="Google button is added by AAO-012 (V1.0); this issue no longer depends on it.",
                  flag="—", deps=["AAO-011", "AAO-014"]),
  "AAO-016": dict(rel="v0.5", blocker=True,  parallel=True,  commercial=None, effort="low",
                  simplify="V0.5: staff login, users search/detail/lock, audit viewer, waitlist → invite issuing. 'View as' deferred.",
                  flag="—"),
  "AAO-022": dict(rel="v0.5", blocker=True,  parallel=False, commercial=None, effort="medium",
                  simplify="One provider per merchant in V0.5: implement eligibility filter fully; scoring is a constant until a second provider exists. Provider credentials from Docker secret files (DB-encrypted credentials via AAO-040 later).",
                  flag="per-provider `mode` (disabled/mock/sandbox/live); prod refuses mock"),
  "AAO-023": dict(rel="v0.5", blocker=True,  parallel=False, commercial="D-3 default cashback share", effort="high",
                  simplify="V0.5: versions, resolver, approval via API/CLI; simulation and admin UI move to AAO-027 (V1.0). Initial rules seeded by owner-reviewed migration.",
                  flag="—"),
  "AAO-024": dict(rel="v0.5", blocker=True,  parallel=False, commercial="OB-1/OB-2/OB-3: entity, ACCESSTRADE publisher account, Shopee + TikTok Shop campaign approval",
                  effort="high",
                  simplify="TikTok product-feed lookup optional; Shopee preview stays capability=false.",
                  flag="provider `mode=live` only after live verification evidence in PR"),
  "AAO-025": dict(rel="v0.5", blocker=True,  parallel=False, commercial=None, effort="high",
                  simplify="Click partitions deferred (partition-ready PK only).",
                  flag="`cashback.links_enabled` (off until V0.5 gate G-0.5)"),
  "AAO-026": dict(rel="v0.5", blocker=True,  parallel=True,  commercial=None, effort="medium",
                  simplify="Landing and analyzer preview already ship in AAO-100; this issue adds logged-in analyzer → link result → /go flow, purchase rules, in-app browser notice, desktop QR.",
                  flag="`cashback.links_enabled`", deps=["AAO-014", "AAO-025", "AAO-100"]),
  "AAO-030": dict(rel="v0.5", blocker=True,  parallel=True,  commercial=None, effort="max",
                  simplify="", flag="—"),
  "AAO-031": dict(rel="v0.5", blocker=True,  parallel=False, commercial=None, effort="high",
                  simplify="Polling path only; webhook and CSV sources stay behind capability=false until a provider supports them.",
                  flag="—"),
  "AAO-032": dict(rel="v0.5", blocker=True,  parallel=False, commercial="Live ACCESSTRADE data (OB-2) to confirm status mapping", effort="high",
                  simplify="", flag="sync job enabled per provider"),
  "AAO-033": dict(rel="v0.5", blocker=True,  parallel=False, commercial=None, effort="xhigh",
                  simplify="Fraud 'void' transition via CLI + approval in V0.5; UI later.", flag="—"),
  "AAO-036": dict(rel="v0.5", blocker=True,  parallel=True,  commercial=None, effort="medium",
                  simplify="Wallet shows pending/approved only in V0.5 with an 'alpha: withdrawals open at commercial launch' banner.",
                  flag="`withdrawals.enabled` (off) hides withdraw CTA"),
  "AAO-040": dict(rel="v0.5", blocker=True,  parallel=True,  commercial=None, effort="medium",
                  simplify="Encryption + key_id only; automated rotation job deferred to V1.x (manual rotation runbook RB-09).",
                  flag="—"),

  # ---------------- V1.0 Commercial Cashback Platform ----------------
  "AAO-012": dict(rel="v1.0", blocker=False, parallel=True,  commercial="Google OAuth client verification (brand, domain, privacy URL)", effort="medium",
                  simplify="Adds Google button to AAO-015 pages.", flag="`auth.google_enabled`", deps=["AAO-010", "AAO-015"]),
  "AAO-027": dict(rel="v1.0", blocker=True,  parallel=True,  commercial=None, effort="medium",
                  simplify="Forms + rule diff/approve; routing simulation deferred to V1.x.", flag="—"),
  "AAO-034": dict(rel="v1.0", blocker=True,  parallel=False, commercial="OB-5 settlement cycle + statement format from ACCESSTRADE", effort="xhigh",
                  simplify="Settlement recording, allocation, suspense and coverage invariant are V1.0. Advance-funding code path moves to V1.x behind `advance.enabled` (schema row exists, enabled=false).",
                  flag="`advance.enabled` (off; owner decision D-2)"),
  "AAO-035": dict(rel="v1.0", blocker=True,  parallel=True,  commercial=None, effort="medium",
                  simplify="Daily totals diff + invariant results + manual cases; no auto-resolution.", flag="—"),
  "AAO-037": dict(rel="v1.0", blocker=True,  parallel=True,  commercial=None, effort="medium",
                  simplify="", flag="—"),
  "AAO-041": dict(rel="v1.0", blocker=True,  parallel=True,  commercial=None, effort="medium",
                  simplify="", flag="`withdrawals.enabled`"),
  "AAO-042": dict(rel="v1.0", blocker=True,  parallel=False, commercial=None, effort="max",
                  simplify="All withdrawals manually reviewed (auto_approve_max=0); no fee in V1.0.", flag="`withdrawals.enabled` (off until gate G-1.0)"),
  "AAO-043": dict(rel="v1.0", blocker=True,  parallel=False, commercial="OB-1 company bank account; statement CSV export from that bank", effort="high",
                  simplify="One bank statement CSV format; MoMo/bank-API rails not built (trait only). `payouts.automated` stays off.",
                  flag="`payouts.suspended` kill switch; `payouts.automated` off"),
  "AAO-044": dict(rel="v1.0", blocker=True,  parallel=True,  commercial=None, effort="medium",
                  simplify="", flag="`withdrawals.enabled`"),
  "AAO-045": dict(rel="v1.0", blocker=True,  parallel=False, commercial=None, effort="medium",
                  simplify="", flag="`withdrawals.enabled`"),
  "AAO-050": dict(rel="v1.0", blocker=False, parallel=True,  commercial=None, effort="low",
                  simplify="V1.0: transactional emails (order approved/reversed, withdrawal paid/rejected). In-app notification centre moves to V1.x.",
                  flag="per-template enable"),
  "AAO-051": dict(rel="v1.0", blocker=True,  parallel=True,  commercial="Counsel-approved ToS + privacy policy (C-PD-1, C-CON-*)", effort="medium",
                  simplify="Versioned documents + consent capture + account closure are V1.0. Data export may be fulfilled manually via email support within the legal deadline; self-service export is V1.x.",
                  flag="—"),
  "AAO-090": dict(rel="v1.0", blocker=True,  parallel=False, commercial="D-9 external pen-test decision", effort="high",
                  simplify="", flag="—"),
  "AAO-095": dict(rel="v1.0", blocker=True,  parallel=True,  commercial=None, effort="medium",
                  simplify="Journeys 1–3 + axe + k6 smoke are blockers; journeys 4–5 and full load profile may follow in V1.x.", flag="—"),
  "AAO-096": dict(rel="v1.0", blocker=True,  parallel=False, commercial="OB-4 written cashback permission; tax + e-commerce law advice (C-TAX-2, C-CORP-2)", effort="high",
                  simplify="", flag="gate G-1.0 flips `withdrawals.enabled` and `signup.mode=open`"),

  # ---------------- V1.x Growth ----------------
  "AAO-052": dict(rel="v1.x", blocker=False, parallel=True,  commercial=None, effort="medium",
                  simplify="Until shipped, missing-cashback claims are handled by email support with a documented SLA.", flag="`support.tickets_enabled`"),
  "AAO-060": dict(rel="v1.x", blocker=False, parallel=True,  commercial=None, effort="low", simplify="", flag="—"),
  "AAO-061": dict(rel="v1.x", blocker=False, parallel=True,  commercial="Shopee Affiliate Open API access + cashback permission", effort="medium",
                  simplify="", flag="provider `mode=disabled`"),
  "AAO-070": dict(rel="v1.x", blocker=False, parallel=True,  commercial="Legal opinion C-MLM-1; reward budget D-8", effort="high",
                  simplify="", flag="`referrals.enabled`; `referrals.multilevel` hard-off"),
  "AAO-071": dict(rel="v1.x", blocker=False, parallel=True,  commercial=None, effort="low",
                  simplify="Static FAQ ships in AAO-100; CMS later.", flag="`content.blog_enabled`"),
  "AAO-072": dict(rel="v1.x", blocker=False, parallel=True,  commercial=None, effort="medium", simplify="", flag="`rewards.checkin_enabled`"),
  "AAO-073": dict(rel="v1.x", blocker=False, parallel=True,  commercial=None, effort="low", simplify="", flag="—"),
  "AAO-075": dict(rel="v1.x", blocker=False, parallel=True,  commercial=None, effort="low", simplify="", flag="`leaderboard.enabled`"),
}

# Critical paths per release (longest dependency chain to the release gate)
CRITICAL = {
  "v0.1": ["AAO-001", "AAO-003", "AAO-020", "AAO-100"],
  "v0.5": ["AAO-010", "AAO-013", "AAO-023", "AAO-025", "AAO-031", "AAO-033", "AAO-036"],
  "v1.0": ["AAO-034", "AAO-042", "AAO-043", "AAO-045", "AAO-090", "AAO-096"],
}

# Size -> agent-days (implementation + CI iterations, excluding owner review wait)
SIZE_DAYS = {"S": 1.5, "M": 3, "L": 5, "XL": 8}

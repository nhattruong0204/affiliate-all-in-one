# ADR-0015: Incremental releases with readiness gates

Status: Proposed · Date: 2026-11-18

## Context
The original plan shipped nothing public until all V1 features (withdrawals, reconciliation, admin money tooling) were done, about 15–23 weeks of implementation. The owner wants earlier customer feedback and a lower time-to-market without weakening financial integrity, security or compliance.

## Decision
Ship four releases: **V0.1 Public Website Preview → V0.5 Private Alpha → V1.0 Commercial Cashback Platform → V1.x Growth**. Every issue is mapped to the earliest release that needs it (`scripts/backlog/releases.py`). Each release must be dependency-complete. Capabilities that move money or create obligations are built earlier but only switched on by readiness gates G-0.1, G-0.5a, G-0.5b and G-1.0 (`RELEASE_PLAN.md` §4) through feature flags: `signup.mode`, `cashback.links_enabled`, `withdrawals.enabled`, `auth.google_enabled`, `advance.enabled`, `payouts.automated`, `referrals.*`.

Defaults favour manual operation: manually reviewed payouts, email-based support and manual data-subject requests within legal deadlines, and config seeded through reviewed migrations until the admin UI exists.

## Consequences
- (+) The public site ships in roughly 4–7 weeks. Attribution quality is measured with real users before any payout code is live.
- (+) There's no financial exposure before V1.0. Alpha balances are pending/approved obligations only, never withdrawable.
- (−) Alpha users wait for V1.0 to withdraw. Alpha terms must say so plainly.
- (−) Flag discipline is now safety-critical. Prod config is reviewed at every gate, and flags are audited (AAO-013).

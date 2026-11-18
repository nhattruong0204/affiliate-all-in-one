# ADR-0011: Settlement-gated availability with capped advance funding

Status: Proposed · Date: 2026-11-17

## Context
Providers mark commissions "approved" before they actually pay the publisher, and approved commissions can still be clawed back. Paying users on approval exposes the company to cash-flow and reversal losses.

## Decision
- Cashback becomes **available** (withdrawable) only when (a) the corresponding provider commission is **settled** (cash received and allocated), or (b) an owner-configured **advance policy** covers it.
- Advance policy: `{enabled, max_total_exposure_vnd, max_per_user_vnd, min_conversion_age_days, eligible_providers, eligible_user_tiers}`. Default `enabled=false`. Exposure = Σ available + reserved + paid amounts whose source conversions are not yet settled. It is checked under a row lock on `advance_policy` inside the availability transaction.
- A daily job checks that the **liability coverage** invariant holds (see `FINANCIAL_LEDGER.md` §7) and pages the owner if it doesn't.

## Consequences
User payout is slower than the reference product's claimed ~7 days unless the owner opts into advance funding. That is a deliberate commercial lever, not a technical constraint.

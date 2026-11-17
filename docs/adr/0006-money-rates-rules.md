# ADR-0006: Integer VND money, basis-point rates, versioned commission rules

Status: Proposed · Date: 2026-11-17

## Context
Floating point is unacceptable for money. Cashback shares vary by provider, merchant, campaign, category, user tier and promo period, and every past calculation must be reproducible.

## Decision
- `Money { amount: i64, currency: Currency }`. VND minor unit = 1 đồng. Arithmetic is checked (`checked_add` etc.). Overflow is an error, never wraps.
- Rates are `Bps(u32)`, where 10 000 = 100%. Provider rates in hundredths of a percent (e.g. TikTok `commission.rate` 3587 = 35.87%) normalize to bps (3587 → 3587).
- `cashback = min(cap, round(commission × share_bps / 10 000, mode))`. The default mode is `Floor` (in the platform's favour by < 1đ). The mode is stored on the rule version.
- Commission rules are immutable **versions** with `effective_from`/`effective_to`, a match specificity, and priority. A conversion pins the `rule_version_id` resolved **at link creation**. If no link exists, it pins the rule resolved at conversion time.
- Resolution order: most specific match wins (campaign > merchant > provider), then user tier and promo overlays. Ties break on priority, then newest `effective_from`. The resolver is a pure function in `domain` with property tests.

## Consequences
- (+) Any historic cashback can be recomputed exactly from the stored inputs.
- (−) Rule changes don't apply retroactively. That's intended; the admin UI makes it explicit.

## Alternatives
`rust_decimal` everywhere: unnecessary for VND. If multi-currency is ever needed, `Money` gains a scale per currency.

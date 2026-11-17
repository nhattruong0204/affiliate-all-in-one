# ADR-0013: ACCESSTRADE as first provider; direct integrations behind disabled adapters

Status: Proposed · Date: 2026-11-17

## Context
ACCESSTRADE VN publishes a public publisher API with token auth, tracking-link creation with `utm_*` and `sub1..4` parameters, transaction and order reports (10 req/min), campaign lists with approval state, and TikTok Shop product search and link creation (VERIFIED, `AFFILIATE_PROVIDER_MATRIX.md`). Whether cashback/incentivized traffic is permitted on the Shopee and TikTok Shop campaigns is **UNKNOWN**.

## Decision
Build the ACCESSTRADE adapter first: links, polling-based conversion sync, campaign catalog, and TikTok product feed. Shopee direct (Open API) and TikTok Shop direct adapters exist as `Disabled` with mocks until credentials and written cashback permission exist.

## Consequences
One integration covers both P0 merchants. The commercial risk concentrates on a single provider's approval (Risk R-01).

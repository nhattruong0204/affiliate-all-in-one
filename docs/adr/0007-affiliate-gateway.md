# ADR-0007: Capability-based Affiliate Gateway with provider routing

Status: Proposed · Date: 2026-11-17

## Context
Merchants (Shopee, TikTok Shop) can be reached through several providers (ACCESSTRADE, direct programs, other networks), and those providers differ widely in what they expose.

## Decision
- Separate domain concepts: `Merchant` (where the user buys) and `Provider` (who pays us), joined by `Campaign` and `provider_merchant_routes`.
- Each provider implements small async traits per capability (`LinkBuilder`, `ConversionSource`, `WebhookVerifier`, `ProductLookup`, `CampaignCatalog`, `SettlementSource`, `HealthCheck`), registered in a `ProviderRegistry` that reports `Capabilities`. Callers check capability and never assume it.
- Routing picks the highest-scoring eligible route **before** link creation, based on enabled, campaign approved, cashback allowed, commission estimate, health score and attribution-quality score. The choice is persisted on the link and click.
- Adapters with unverified access compile as `Disabled` and expose a `Mock` variant clearly marked `is_mock = true`. Mock data never reaches production users (enforced at startup in `prod` profile).
- All user-URL fetching goes through `affiliate::safe_fetch` (SSRF-hardened).

## Consequences
- (+) Adding a provider means one adapter crate module, a migration for its config rows, and contract fixtures.
- (−) Some indirection. The trait surface stays small and capability-scoped to compensate.

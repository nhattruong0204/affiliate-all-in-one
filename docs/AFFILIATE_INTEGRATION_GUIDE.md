# Affiliate Integration Guide

How to build and enable a provider adapter. Status: PROPOSED.

## 1. Gateway traits (crate `affiliate`)

```rust
pub struct Capabilities {
    pub link_builder: bool, pub deep_link: bool, pub product_lookup: bool, pub commission_estimate: bool,
    pub conversion_poll: bool, pub webhook: bool, pub settlement_report: bool, pub campaign_catalog: bool,
}

#[async_trait]
pub trait Provider: Send + Sync {
    fn code(&self) -> ProviderCode;
    fn mode(&self) -> AdapterMode;                       // Disabled | Mock | Sandbox | Live
    fn capabilities(&self) -> Capabilities;
    async fn health(&self) -> HealthReport;
    fn link_builder(&self) -> Option<&dyn LinkBuilder> { None }
    fn conversion_source(&self) -> Option<&dyn ConversionSource> { None }
    fn webhook_verifier(&self) -> Option<&dyn WebhookVerifier> { None }
    fn product_lookup(&self) -> Option<&dyn ProductLookup> { None }
    fn campaign_catalog(&self) -> Option<&dyn CampaignCatalog> { None }
    fn settlement_source(&self) -> Option<&dyn SettlementSource> { None }
}

#[async_trait] pub trait LinkBuilder {
    async fn create(&self, req: LinkRequest) -> Result<ProviderLink, ProviderError>;
    // LinkRequest { campaign: ProviderCampaignId, canonical_url: Url, product_id: Option<String>, sub_ids: SubIds }
    // ProviderLink { affiliate_url: Url, short_url: Option<Url>, raw: RedactedJson }
}
#[async_trait] pub trait ConversionSource {
    async fn fetch(&self, window: SyncWindow, page: PageCursor) -> Result<ConversionPage, ProviderError>;
    // ConversionPage { items: Vec<NormalizedConversion>, next: Option<PageCursor>, raw: Vec<RawEvent> }
}
pub trait WebhookVerifier { fn verify(&self, req: &InboundRequest, now: OffsetDateTime) -> Result<Vec<RawEvent>, WebhookError>; }
#[async_trait] pub trait ProductLookup { async fn lookup(&self, p: &CanonicalProduct) -> Result<Option<ProductInfo>, ProviderError>; }
#[async_trait] pub trait CampaignCatalog { async fn list(&self) -> Result<Vec<CampaignInfo>, ProviderError>; }
#[async_trait] pub trait SettlementSource { async fn statements(&self, period: DateRange) -> Result<Vec<SettlementLine>, ProviderError>; }
```

`ProviderError` distinguishes `RateLimited { retry_after }`, `Transient`, `Auth`, `Ineligible { reason }` (e.g. "link not part of campaign"), `InvalidInput`, `Contract { detail }` (unexpected payload, which raises an alert), and `Disabled`.

## 2. Merchant URL engine
Merchant recognition and product-ID extraction are **provider-independent** (`affiliate::url`). Each merchant module defines its hostnames, short-link hosts, path patterns and app schemes. See `TRACKING_AND_ATTRIBUTION.md` §2.

## 3. Routing
```
eligible(route) = route.enabled && provider.mode == Live && campaign.approval == successful
                  && campaign.cashback_allowed == yes && now in campaign window && provider.health_score >= 50
score(route)    = w1*est_commission_norm + w2*health_score + w3*attribution_quality_score + route.priority_bonus
```
Weights live in `config_values`. Ties go to the lowest route priority number. If none are eligible, return `NoEligibleRoute`. The UI then explains that cashback is unavailable for this product, and we **do not** generate a link. The routing decision and its inputs are saved in `link_requests.estimate_basis`.

## 4. Resilience defaults (per provider, configurable)
| Setting | Default | Note |
|---|---|---|
| connect / total timeout | 3 s / 8 s | link creation is user-facing |
| retries | 2 for idempotent reads, 0 for link creation unless provider guarantees idempotency (AT: creating twice just returns another link, so 1 retry allowed) | |
| client rate limiter | token bucket per provider+endpoint; AT reports 8/min | below documented 10/min |
| circuit breaker | open after 5 consecutive failures or 50% over 20 calls; half-open after 30 s | health score updated |
| response size cap | 2 MB | |
| TLS | rustls, system roots, no custom CA | |

## 5. Credentials
Provider credentials are stored encrypted in `providers.credentials_enc` (ADR-0008) and entered via admin by the owner role only. They're never logged, never returned by any API (the admin UI shows only the last 4 characters), and rotated through admin with an audit entry.

## 6. Adding a provider: checklist
1. Research → add a row to `AFFILIATE_PROVIDER_MATRIX.md` with sources and labels.
2. Create `affiliate/src/providers/<code>/` with `client.rs`, `mapping.rs`, `mock.rs`, `fixtures/`.
3. Map provider statuses to our `ConversionStatus` in `mapping.rs`. Unmapped statuses → `Contract` error (never silently default).
4. Contract tests: recorded **real** responses (redacted) as fixtures, plus a mock server (wiremock). Mock-only tests mark the adapter as `verification: mock_only`, and admin shows a red badge.
5. Live verification (staging, real credentials): create one link, click it, place a real low-value order, observe conversion in sync, record evidence in the PR. Only then may `mode=Live` be set in prod. That needs owner approval.
6. Migration to insert the provider row (status `disabled`) + campaigns sync job.

## 7. ACCESSTRADE adapter specifics
- Base `https://api.accesstrade.vn`. Headers `Authorization: Token {key}`, `Content-Type: application/json`.
- Links: `POST /v1/product_link/create` with `{campaign_id, urls:[canonical], utm_source:"aaio", utm_campaign:<campaign_code>, utm_content:<token>, sub1:<token>}`. Use `success_link[0].short_link` if present, else `aff_link`. A URL found in `suspend_url` or `error_link` → `Ineligible`.
- TikTok Shop: `POST /v2/tiktokshop_product_feeds/create_link?minify=true` with `{product_url, utm_content:<token>, sub1:<token>}` (send both `sub1` and `sub_1` until the field name is confirmed live).
- Conversions: `GET /v1/transactions?update_time_start&update_time_end&limit=300&page=n`. Normalize one row per `conversion_id`. `status` 0 → pending, 1 → approved (if `is_confirmed=1`, else pending-approved per mapping table, confirmed in AAO-032), 2 → rejected. The commission is `commission`.
- Campaigns: `GET /v1/campaigns?approval=successful` daily, upsert `campaigns`. Never auto-set `cashback_allowed` (it's human-confirmed).
- Postback (if VN supports it after OB-5): treat as a **trigger** to fetch `GET /v1/transactions?transaction_id=…` and never trust postback amounts directly, because postbacks are unsigned.

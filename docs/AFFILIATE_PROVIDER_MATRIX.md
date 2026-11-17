# Affiliate Provider Matrix

Access date: 2026-11-17. Labels: **VERIFIED** = read in official provider docs or terms on that date. **INFERRED** = from third-party or indirect evidence. **UNKNOWN** = not determinable publicly. **PROPOSED** = our plan.
Rule: an adapter may only call endpoints marked VERIFIED. Everything else needs credentials and a contract test against the real API before it can be enabled.

## 1. Classification

| Provider | Kind | Relevance |
|---|---|---|
| ACCESSTRADE VN | Third-party affiliate **aggregator/network** (publisher API) | P0 path to Shopee + TikTok Shop |
| Shopee Affiliate (affiliate.shopee.vn) | **Direct** affiliate program with an Open API (GraphQL) | P0 alternative/redundancy |
| TikTok Shop Partner Center | **Seller/creator/partner management** APIs (incl. affiliate collaboration for sellers, creators, TAP partners) | Not a publisher-cashback API. Direct route UNKNOWN |
| Lazada Affiliate | Direct program (Lazada portal), also via networks (ACCESSTRADE, Involve Asia) | P1 |
| Ecomobi, MasOffer, Adpia, Permate | Networks/aggregators | P1, research pending |
| Trip.com, Klook, Agoda, Booking.com, Traveloka | Travel, usually via their own partner programs or networks | P2 |
| Tiki, FPT Shop, ShopeeFood | Merchants reachable via networks (campaign availability UNKNOWN) | P1/P2 |

## 2. ACCESSTRADE VN (P0)

| Capability | Finding | Status | Source |
|---|---|---|---|
| Publisher registration | Free self-serve sign-up; campaign-level approval (`approval`: unregistered/pending/successful) | VERIFIED | developers.accesstrade.vn `lay-danh-sach-campaigns` |
| Business eligibility for cashback sites | Not stated in VN API docs. ACCESSTRADE **Global** support has an article on integrating reward/point/cashback sites (postback `pbid`) | VN: **UNKNOWN**; Global: VERIFIED | support.accesstrade.global |
| Cashback permission per campaign | Not in API fields. `traffic_building_policy` HTML per campaign lists prohibited traffic. Violations can void that month's conversions | **UNKNOWN** → written confirmation required per campaign | campaign API sample |
| Auth | Header `Authorization: Token <access_key>`. Key from publisher profile page | VERIFIED | `/authentication` |
| Campaign list | `GET https://api.accesstrade.vn/v1/campaigns` `?approval=successful`, `?campaign_id=`. Fields: id, name, approval, status, merchant, cookie_duration (secs), cookie_policy, start/end, category, type, url, scope; commission policy only inside description HTML | VERIFIED (pagination params undocumented) | `/lay-danh-sach-campaigns` |
| Link generation | `POST https://api.accesstrade.vn/v1/product_link/create` body: `campaign_id` (req), `urls[]`, `utm_source/medium/campaign/content`, `sub1..sub4`, `short_domain`. Response: `success_link[{aff_link, first_link, short_link, url_origin}]`, `error_link[]`, `suspend_url[]` | VERIFIED (no limits stated) | `/tao-tracking-link` |
| TikTok Shop link | `POST https://api.accesstrade.vn/v2/tiktokshop_product_feeds/create_link` body: `product_url`, `product_id?`, `utm_*`, `sub_1..sub_4` (docs inconsistent: `sub1`), `?minify=true`. Response `aff_short_url`, `aff_url`, plus product fields. "The link is not part of the campaign" on ineligible products | VERIFIED | TikTok v2 create-link |
| TikTok product search | `GET https://api.accesstrade.vn/v2/tiktokshop_product_feeds` (`sort_field`, `limit`, `title_keywords`, `page_token`). `commission.rate` in hundredths of a percent (3587 = 35.87%), `sales_price`, `original_price` | VERIFIED (docs internally inconsistent on method/rate range) | TikTok v2 product search |
| Product metadata (generic) | `GET https://api.accesstrade.vn/v1/product_detail?merchant&product_id&transaction_id`. Returns name, price, discount, image, link, shop. **Requires transaction_id**, so it's not usable for pre-purchase preview | VERIFIED | `/lay-thong-tin-chi-tiet-san-pham` |
| Shopee product metadata pre-purchase | Not found in VN docs. Datafeeds/top-products endpoints exist; Shopee coverage UNKNOWN | **UNKNOWN** | index |
| Commission rate API | No per-product rate for Shopee. Campaign commission only as HTML text. TikTok product feed has a rate | Shopee: UNKNOWN; TikTok: VERIFIED | — |
| Sub-ID tracking | `utm_*` + `sub1..4` on link creation. `utm_*` returned and filterable on order and transaction APIs. Sub params appear in `_extra.parameters` | VERIFIED | order-list, transactions |
| Conversion reporting | `GET https://api.accesstrade.vn/v1/transactions`: `since/until` by sale time (≤ 366 days) or `update_time_start/end` (≤ 31 days, takes priority), `page/offset/limit` (≤ 300 recommended), filters incl. `status` (0 hold, 1 approved, 2 rejected), `is_confirmed`, `transaction_id`. Fields: `conversion_id`, `transaction_id`, status, is_confirmed, commission, transaction_value, product_*, click_time, transaction_time, update_time, confirmed_time, reason_reject(ed), `_extra`, `_utm*` | VERIFIED | `/lay-danh-sach-giao-dich` |
| Order report | `GET https://api.accesstrade.vn/v1/order-list` (since/until, page, limit ≤ 300, status, merchant, utm_*). `is_confirmed`, per-item counts | VERIFIED | `/lay-danh-sach-don-hang-v2` |
| Rate limits | transactions & order-list: **10 requests/minute**, 1-min cache. 429 with `retry_after` + `Retry-After`. Timeouts: backoff 2s/4s/8s, narrow range | VERIFIED | transactions, order-list |
| Webhook/postback | ACCESSTRADE **Global** documents GET postbacks per conversion item, up to 3 tries, 200 OK required, custom params echoed. **VN publisher platform postback: UNKNOWN** | VN: UNKNOWN | support.accesstrade.global |
| Webhook signature | None documented (Global postback is unsigned GET) | UNKNOWN → treat as untrusted hint, confirm via API | — |
| Settlement cycle / min payout | Not in API docs | **UNKNOWN** → onboarding question | — |
| Reversals | `status=2` rejected with `reason_reject`; approval via `is_confirmed`. Post-approval clawback behaviour UNKNOWN | Partially VERIFIED | transactions |
| Stable conversion ID | `conversion_id` per product line. `transaction_id` = order | VERIFIED (field exists); uniqueness guarantee INFERRED | transactions |

**Integration decision (PROPOSED):** use `utm_content = <click token>` (filterable and returned) **and** `sub1 = <click token>` (redundant). `utm_source = "aaio"`, `utm_campaign = <campaign code>`. Sync by `update_time` windows of ≤ 1 day with 2h overlap, `limit=300`, under a client-side token bucket of 8 req/min (headroom under 10).

## 3. Shopee Affiliate direct (P0 alternative)

| Capability | Finding | Status |
|---|---|---|
| Registration | Self-serve at affiliate.shopee.vn (login with Shopee account, channel info, approval) | VERIFIED (Shopee blog) |
| Terms | Shopee VN affiliate T&C (updated 2025-12-01) bind affiliates. Our fetch of the current page failed. Cashback/incentive clauses **UNKNOWN** | UNKNOWN |
| Open API | GraphQL endpoint per region (`open-api.affiliate.shopee.<tld>/graphql`). App ID + Secret, SHA-256 signed requests. Operations seen in SDKs: `generateShortLink`, `productOfferV2`, `conversionReport` | **INFERRED** (third-party SDKs; official VN doc is behind the affiliate login) |
| Sub-IDs | `subIds` on `generateShortLink` | INFERRED |
| Rate limits, webhooks, settlement | — | UNKNOWN |
| Decision | `ShopeeDirectAdapter` = Disabled + Mock until owner obtains Open API access and docs. Contract fixtures built from real responses only | PROPOSED |

## 4. TikTok Shop (P0 merchant)

| Capability | Finding | Status |
|---|---|---|
| Path via ACCESSTRADE | Product search + create-link v2 | VERIFIED |
| TikTok Shop Partner API | OAuth app in Partner Center. Affiliate APIs serve sellers, creators and affiliate partners (TAP): collaborations, promotion links, affiliate orders | VERIFIED existence (TikTok developer blog). Eligibility of a cashback platform: **UNKNOWN** |
| Decision | Use ACCESSTRADE. `TikTokDirectAdapter` Disabled. Evaluate the TAP partner route in P1 | PROPOSED |

## 5. P1/P2 providers (summary)

| Provider | Kind | Link API | Conversion API | Postback | Cashback allowed | Status |
|---|---|---|---|---|---|---|
| Lazada Affiliate | direct + via networks | UNKNOWN (portal) | UNKNOWN | UNKNOWN | UNKNOWN | research in AAO-060 |
| Ecomobi | network (publisher API page exists) | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | research |
| MasOffer | network | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | research |
| Adpia | network | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | research |
| Permate | network | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | research |
| Trip.com / Klook / Agoda / Booking / Traveloka | own partner programs or networks | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | P2 |
| Tiki / FPT Shop / ShopeeFood | merchant campaigns, possibly on ACCESSTRADE | via AT if campaign exists | via AT | — | UNKNOWN | check `GET /v1/campaigns` after onboarding |

## 6. Commercial onboarding checklist (owner actions)

| # | Item | Blocks |
|---|---|---|
| OB-1 | Register a legal entity (or confirm the existing one) to contract with networks and pay users. Get tax code and bank account | everything commercial |
| OB-2 | Create an ACCESSTRADE VN publisher account under the entity. Register the website property | AAO-024 live verification |
| OB-3 | Apply to Shopee and TikTok Shop campaigns. Record `approval=successful` | V1 launch |
| OB-4 | **Get written confirmation** from ACCESSTRADE (per campaign) that a cashback/incentive website is permitted, including any conditions (disclosure, no coupon sites, no brand bidding) | **Launch blocker** (R-01) |
| OB-5 | Ask ACCESSTRADE: payout cycle, minimum payout, invoice requirements, clawback window after approval, whether VN postback exists and its format, settlement statement format (CSV with conversion IDs?) | ledger settlement import |
| OB-6 | Ask about API limits beyond 10 req/min for transactions, and allowed polling frequency | sync design |
| OB-7 | Shopee Affiliate: apply directly. Request Open API access. Ask explicitly about cashback sites | redundancy |
| OB-8 | Decide whether to pursue TikTok TAP partnership | P1 |
| OB-9 | Store all permissions as PDFs in the company drive. Record a reference (not the file) in `campaigns.cashback_evidence_ref` | audit |
| OB-10 | Engage a VN lawyer and tax advisor (COMPLIANCE_CHECKLIST) | launch |

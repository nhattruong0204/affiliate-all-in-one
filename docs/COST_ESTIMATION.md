# Cost Estimation and Capacity

Status: ESTIMATE. Prices are **indicative ranges as of late 2026, not quotes**. Verify with providers at purchase (owner decision D-6 on host and region). Exchange rate assumption: 1 USD ≈ 26 000 VND (verify).

## 1. Traffic assumptions
| Assumption | Value | Rationale |
|---|---|---|
| Monthly active / registered | 35% | cashback apps are used around purchases |
| Links per MAU per month | 6 | ~1.5 shopping sessions/week |
| Link → click | 80% | |
| Click → tracked order | 25% | many users don't complete |
| Avg order value | 250 000đ | reference demo uses 250k (VERIFIED demo) |
| Avg commission rate | 6% (scenario range 3–12%) | **UNKNOWN**; Shopee/TikTok rates vary by category and change often |
| User share | 70% | brief example |
| Reversal/reject rate | 15% | INFERRED |
| Peak factor | 10× average (mega-sale days 11.11, 12.12) | |
| Page views per link | ~8 | |

## 2. Load by tier (monthly)
| Registered users | MAU | Links | Orders | Avg req/s | Peak req/s | DB size after 1 yr |
|---|---|---|---|---|---|---|
| 100 | 35 | 210 | 42 | <0.01 | 0.1 | < 1 GB |
| 1 000 | 350 | 2 100 | 420 | 0.01 | 0.5 | ~1 GB |
| 10 000 | 3 500 | 21 000 | 4 200 | 0.1 | 5 | ~5 GB |
| 100 000 | 35 000 | 210 000 | 42 000 | 1 | 30 | ~40 GB |

The provider rate limit (10 req/min for ACCESSTRADE reports) caps sync throughput before our infrastructure does: at 300 rows/page, it's ~4.3M conversions/day, which is ample.

## 3. Infrastructure cost (USD/month, indicative)
| Item | 100 | 1 000 | 10 000 | 100 000 |
|---|---|---|---|---|
| Prod VPS | 2 vCPU/4 GB: 8–25 | 2 vCPU/4 GB: 8–25 | 4 vCPU/8 GB: 15–50 | App 4 vCPU/8 GB + DB 8 vCPU/32 GB or managed PG: 120–350 |
| Staging VPS | 5–15 | 5–15 | 8–25 | 15–50 |
| Object storage (backups, uploads) | 1–5 | 1–5 | 5–10 | 15–40 |
| Email (transactional) | 0–10 | 0–10 | 10–20 | 30–90 |
| Domain, DNS, uptime monitor | 2–5 | 2–5 | 2–10 | 10–30 |
| Error tracking / extra (optional) | 0 | 0 | 0–26 | 26–80 |
| **Total** | **~16–60** | **~16–60** | **~40–140** | **~215–640** |

Hosting in Vietnam (VN data-centre VPS) may cost more per vCPU but reduces cross-border data questions (C-PD-7).

## 4. Unit economics (scenario, per tracked order)
| Line | Value |
|---|---|
| Order value | 250 000đ |
| Commission @ 6% | 15 000đ |
| User cashback @ 70% | 10 500đ |
| Gross retained | 4 500đ |
| Expected reversals (15% of retained) | −675đ |
| Payout cost (bank transfer fee ≈ 0–3 300đ per payout; avg 5 orders per payout) | −0 to −660đ |
| Referral reward (if enabled, e.g. 10% of referee cashback, 30% of orders referred) | −315đ |
| **Contribution per order** | **≈ 2 850–3 500đ** |

Monthly contribution vs infra (mid-point):
| Users | Orders | Contribution (VND) | Infra (VND, mid) | Result |
|---|---|---|---|---|
| 1 000 | 420 | ~1.3M | ~1M | ~break-even on infra only |
| 10 000 | 4 200 | ~13M | ~2.3M | positive before staff and marketing |
| 100 000 | 42 000 | ~134M | ~11M | positive before staff and marketing |

Takeaways: (1) the business only works at volume. Customer acquisition cost, not infrastructure, dominates. (2) At a 3% commission rate, contribution roughly halves. (3) **Advance funding** ties up roughly (avg settlement lag days ÷ 30) × monthly user cashback in working capital. At 100k users and a 45-day lag, that's ~1.5 × 441M ≈ 660M VND of exposure, which is why it's capped and off by default.

## 5. Capacity and performance targets
| Metric | Target |
|---|---|
| Availability | 99.5% monthly (V1 single host); 99.9% after split hosts |
| Page p95 (server) | < 300 ms |
| Link creation p95 | < 2.5 s (provider-bound) |
| Redirect p95 | < 50 ms |
| Throughput headroom | 10× average at each tier on the listed hardware (k6-validated) |
| RPO / RTO | 15 min / 4 h |
| Sync freshness | 95% of provider updates within 30 min |

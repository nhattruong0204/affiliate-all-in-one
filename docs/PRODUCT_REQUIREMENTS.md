# Product Requirements

## 1. Vision
The most trustworthy cashback service in Vietnam: paste a link, buy as usual, and see exactly where your money is at every step.

## 2. Users
- **Shopper** (primary): mobile-first, shares links from Shopee/TikTok apps, wants money in a bank account.
- **Referrer**: shares the tool with friends.
- **Operators**: finance, support, admin, owner, auditor.

## 3. Business model
Platform receives affiliate commission on approved orders and shares a configurable percentage with the user.

Worked example: order 500,000đ → commission 50,000đ → user share 70% → cashback 35,000đ → platform gross retained 15,000đ, before payout costs, tax, referral and promo rewards.

Share is resolved by a versioned rule matched on provider, merchant, campaign, category, user tier, and promo period (`DATABASE_DESIGN.md` §4.4). No rate, window, or cycle is hardcoded.

### Money states shown to users
| State | Meaning | Withdrawable |
|---|---|---|
| Ước tính (estimated) | Our preview before purchase | No |
| Chờ duyệt (pending) | Provider reported the order | No |
| Đã duyệt (approved) | Provider approved the commission | No |
| Khả dụng (available) | Commission settled to us, or covered by an authorized advance | Yes |
| Đang rút (reserved) | Locked for a withdrawal | — |
| Đã trả (paid) | Transferred | — |
| Đã huỷ (reversed) | Cancelled/returned/rejected | — |

## 4. V1 scope
**In:** landing; link analyzer; preview + estimate; link generation via ACCESSTRADE for Shopee and TikTok Shop (subject to campaign approval); click tracking; conversion polling; order history; ledger wallet; manual-review withdrawals to bank; bank account settings; email/password + Google auth; vi/en; dark/light; ToS/privacy; support links; purchase rules; admin console (users, providers, campaigns, rules, conversions, ledger view, withdrawals, audit log, flags).

**V1.x:** single-level referral, daily check-in, notifications centre, blog CMS, PWA install, missing-cashback tickets.

**Later:** leaderboard, tiers, MoMo payouts, direct Shopee/TikTok adapters, P1/P2 providers, Telegram/Zalo bot, automated disbursement.

**Out:** multi-level rewards, custodial wallet features (top-up, P2P transfer, payments), native apps.

## 5. Non-functional requirements
| Area | Target |
|---|---|
| Availability | 99.5% monthly (single VPS, V1) |
| Latency | p95 < 300 ms server time for pages; link generation p95 < 2.5 s (provider-bound) |
| RPO / RTO | 15 min / 4 h |
| Correctness | Zero unbalanced ledger transactions; zero duplicate payouts |
| Accessibility | WCAG 2.1 AA |
| Browsers | iOS Safari 16+, Android Chrome 110+, desktop evergreen |
| Privacy | Vietnam PDPL-aligned (see compliance checklist) |

## 6. Success metrics (first 90 days after launch)
Link → click rate; click → tracked order rate; tracked order → approved rate; median days order → available; withdrawal review time; support tickets per 100 orders; contribution margin per order.

## 7. Principles
1. Never promise money we have not been promised.
2. Never pay money we do not have, unless the owner has set an explicit limit.
3. Show the user the truth, including "we don't know yet".

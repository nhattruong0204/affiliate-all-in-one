# Admin Dashboard

Status: PROPOSED. Served by the same `web` binary under `admin.<domain>` with separate staff sessions (ADR-0008). Server-rendered tables, islands only for filters and bulk actions.

## 1. Modules
| Module | Capabilities |
|---|---|
| Users | Search (email, id, referral code), profile, status (lock/suspend/close), sessions (revoke), risk flags, linked bank accounts (masked), wallet view, impersonation **not** provided (read-only "view as" instead) |
| Roles | Grant/revoke (owner only), list grants with history |
| Providers | Status (disabled/mock/sandbox/live), credentials (write-only), capabilities, health, rate-limit usage, verification badge (mock-only vs live-verified) |
| Merchants | Hostnames, app schemes, enable/disable |
| Campaigns | Synced list, approval, **cashback_allowed + evidence ref** (human-set), cookie duration, window |
| Routing | Routes per merchant, priority/weights, simulation ("which route would this URL use now?") |
| Commission rules | Versions with effective dates, diff view, maker-checker approval, simulation on historical conversions |
| Conversions | Search/filter by state, provider, user, attribution. Raw event viewer (redacted). Unmatched queue |
| Reconciliation | Cases, daily diffs, settlements (record, upload statement, allocate, suspense view) |
| Ledger | Account browser, transaction viewer, platform position, invariant check results |
| Adjustments | Create (maker) / approve (checker), reason codes, ticket reference |
| Withdrawals | Review queue with risk flags, approve/reject, payout batch with VietQR, mark submitted, statement import, unknown queue |
| Fraud | Clusters (device/IP/bank hash), freeze/unfreeze entitlements, user risk |
| Support | Tickets, missing cashback claims with auto-search results, macros |
| Referral & rewards | Campaigns, budgets, spend, rules (flag-gated) |
| Feature flags | Toggle with reason; dangerous flags need second approval |
| Content | Blog posts (Markdown, preview, SEO fields), FAQ, legal documents (versioned) |
| Notifications | Templates (vi/en), broadcast (rate-limited, audited) |
| Jobs | Queue depth, running, dead letters with requeue/cancel, schedules |
| Audit log | Filter by actor, action, target. Hash-chain verification status |
| Integration health | Per provider: last success, error rate, latency, quota usage |
| Exports | Async CSV (orders, withdrawals, ledger, users) with PII level chosen by role. Download links expire in 24h and are audited |

## 2. Roles and permissions
| Permission | owner | admin | finance | support | auditor |
|---|---|---|---|---|---|
| View users / PII | ✔ | ✔ | ✔ (payout context) | ✔ (masked bank) | ✔ (masked) |
| Lock / suspend users | ✔ | ✔ | — | request | — |
| Grant roles | ✔ | — | — | — | — |
| Provider credentials | ✔ | — | — | — | — |
| Providers/merchants/campaigns/routes | ✔ | ✔ | view | view | view |
| Commission rules: create version | ✔ | ✔ | ✔ | — | view |
| Commission rules: approve version | ✔ | — | ✔ (≠ creator) | — | — |
| Conversions view | ✔ | ✔ | ✔ | ✔ | ✔ |
| Manual attribution | approve | — | approve | propose | — |
| Settlements record/allocate | ✔ | — | ✔ | — | view |
| Ledger view | ✔ | view | ✔ | — | ✔ |
| Adjustments create / approve | ✔ / ✔ | — | ✔ / ✔ (≠ maker; > 1M needs owner) | — | — |
| Withdrawals review/approve | ✔ | — | ✔ | view | view |
| Withdrawals submit/mark paid | ✔ | — | ✔ (≠ approver) | — | — |
| Payout suspension switch | ✔ | ✔ (on only) | ✔ (on only) | — | — |
| Feature flags (normal / dangerous) | ✔ / ✔+2nd | ✔ / request | — | — | view |
| Content, notifications | ✔ | ✔ | — | ✔ (FAQ/macros) | — |
| Jobs requeue | ✔ | ✔ | — | — | — |
| Audit log | ✔ | ✔ | own area | — | ✔ |
| Exports with full PII | ✔ | — | payout export | — | — |

All staff need TOTP. Money actions need step-up. Every mutating action writes `audit_log`.

## 3. Business analytics (reports module)
Definitions are fixed and documented in-app so GMV is never confused with revenue.

| Metric | Definition | Source |
|---|---|---|
| Registered / active users | Users created; users with ≥1 session in 30d | users, sessions |
| Links generated / clicks | `link_requests` ready; `clicks` | tracking |
| Conversion rate | conversions (matched) ÷ clicks, by merchant | conversions, clicks |
| **GMV** | Σ `order_value_vnd` of matched conversions (not revenue) | conversions |
| Gross affiliate commission | Σ `commission_vnd` by state (pending/approved/settled/reversed) | conversions |
| Settled commission | Σ settlement allocations | settlements |
| Cashback liabilities | Σ 2100/2110/2120⁺/2130 + 2400 | ledger |
| Outstanding withdrawals | Count/Σ open withdrawals by status | withdrawals |
| **Platform retained commission (revenue)** | 4100 − 4110 (+ 4200) per period | ledger |
| Operating expenses | 5100 + 5110 + 5200 + 5300 + manual opex entries (infra, staff) imported monthly | ledger + opex table |
| **Contribution margin** | Revenue − (rewards + payout fees + bad debt) − variable infra | derived |
| Provider attribution quality | matched ÷ total conversions; median report lag; rejection rate | conversions |
| Fraud / rejection rates | void entitlements ÷ total; provider rejections by reason | entitlements, conversions |
| Advance exposure, coverage ratio | Per FINANCIAL_LEDGER I8/I9 | ledger |

Reports run on read-only queries (REPEATABLE READ) or the replica when one exists. Daily snapshots go to `report_daily_metrics` so dashboards don't scan raw tables.

# Implementation Roadmap

Status: PROPOSED, re-planned 2026-11-18 into incremental releases (ADR-0015). Release contents, gates and acceptance criteria are in `RELEASE_PLAN.md`. Issue-level detail is in `ENGINEERING_BACKLOG.md`.

Durations below assume two concurrent sessions per model on `parallelizable` issues, with owner review taking ≤ 2 working days per PR. With one session per model, roughly add 60% (see `RELEASE_PLAN.md` §5). These are planning estimates, not commitments.

```mermaid
gantt
    title Release plan (relative weeks from first implementation PR)
    dateFormat  YYYY-MM-DD
    axisFormat  W%W
    section Owner / Commercial
    D-4 entity + domain, D-6 hosting, privacy notice     :crit, c1, 2026-11-23, 21d
    OB-1..OB-3 ACCESSTRADE account + campaigns           :crit, c2, 2026-11-23, 42d
    OB-4 cashback permission, alpha terms                :crit, c3, 2026-12-07, 42d
    OB-5 settlement info, ToS/tax/e-commerce advice      :crit, c4, 2026-12-21, 56d
    section V0.1 Public Website Preview
    Foundations, UI shell, URL engine, SSRF guard, infra :v01, 2026-11-23, 21d
    AAO-100 site + G-0.1                                 :milestone, g01, after v01, 0d
    section V0.5 Private Alpha
    Identity, staff security, gateway, ACCESSTRADE       :v05a, 2026-12-07, 28d
    Ledger, ingestion, entitlements, wallet              :v05b, after v05a, 21d
    G-0.5a staff alpha, G-0.5b invite alpha              :milestone, g05, after v05b, 0d
    section V1.0 Commercial
    Settlement, reconciliation, withdrawals, payout ops  :v10a, after v05b, 28d
    Security review, E2E, live dry run                   :v10b, after v10a, 10d
    G-1.0 commercial launch                              :milestone, g10, after v10b, 0d
    section V1.x Growth
    Referral, check-in, tickets, blog, PWA, providers    :v1x, after g10, 42d
```

## Release exit criteria (summary)
| Release | Exit |
|---|---|
| V0.1 | Gate G-0.1: public site live, analyzer + waitlist, no cashback claims, security headers, restore-tested backups |
| V0.5 | G-0.5a (staff, live provider verified, invariants green for 7 nights), then G-0.5b (OB-4 permission, alpha terms, invites). Withdrawals off |
| V1.0 | G-1.0: settlement-gated balances, manual-review withdrawals end to end, security review closed, counsel-approved legal docs, two finance staff |
| V1.x | Per-feature flag gates with owner (and, for referral, legal) approval |

## Critical path
`AAO-001 → AAO-004 → AAO-010 → AAO-013 → AAO-023 → AAO-025 → AAO-031 → AAO-033 → AAO-034 → AAO-042 → AAO-043 → AAO-045 → AAO-090 → AAO-096`. The V0.1 branch is `AAO-001 → AAO-003 → AAO-020 → AAO-100`. The commercial path (OB-1…OB-5, counsel) runs in parallel and may be longer than engineering. If OB-4 is refused, the invite alpha and V1.0 are blocked whatever the state of the code.

## Release strategy
1. **V0.1 preview**: public, indexable, waitlist only.
2. **V0.5a staff alpha**: real links and orders by staff, disclosed to ACCESSTRADE.
3. **V0.5b invite alpha**: ≤ 100 waitlist users, cashback accrues, no withdrawals.
4. **V1.0 GA**: open sign-up and withdrawals. Advance funding stays off.
5. **V1.x**: features enabled one flag at a time, measured against the funnel.

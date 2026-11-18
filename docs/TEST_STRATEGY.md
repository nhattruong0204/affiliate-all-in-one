# Test Strategy

Status: PROPOSED. Principle: **money and state machines are proven by property tests and real-Postgres integration tests. Mocks never certify a provider integration.**

## 1. Pyramid and tooling
| Layer | Tool | Scope | Gate |
|---|---|---|---|
| Unit | `cargo nextest` | `domain` (money, rounding, rule resolution, state machines), URL parsing | every PR |
| Property | `proptest` | money arithmetic, share split (I7), rule resolver determinism, state machine never reaches illegal states, ledger postings always balanced | every PR (256 cases), nightly (10k) |
| Integration | `testcontainers` Postgres 17 (real) | repositories, ledger `post`, triggers, constraints, job queue leasing, migrations | every PR |
| Provider contract | `wiremock` + recorded fixtures (redacted real responses) | adapters' request shape, mapping, error handling | every PR |
| Live provider verification | staging, real credentials | create link, sync, end-to-end real purchase | before enabling `Live` |
| Concurrency | tokio multi-task + real Postgres | double withdrawal, concurrent webhook+poll, concurrent rule edits | every PR |
| Security regression | integration + scripted | authz matrix, IDOR, CSRF, SSRF corpus, open redirect, rate limits, headers | every PR |
| E2E browser | Playwright (Chromium, WebKit mobile emulation: iPhone 13, Pixel 7) | critical journeys | PR smoke, nightly full |
| Accessibility | `@axe-core/playwright` | each page | nightly |
| Performance | k6 | analyze, link create, dashboard, redirect | PR smoke (thresholds), nightly load |
| Backup/restore | script + CI scheduled | restore + invariants | weekly |

## 2. Mandatory scenario list (each must exist as a named test)
| ID | Scenario | Expected |
|---|---|---|
| T-DUP-1 | Same conversion event delivered 3× (poll, poll, webhook) | 1 conversion row, 1 entitlement, 1 ledger tx per revision |
| T-DUP-2 | Same `Idempotency-Key` withdrawal request twice | same withdrawal returned, one reservation |
| T-OOO-1 | `approved` processed before `pending` | final state approved; pending ignored (older `provider_updated_at`) |
| T-OOO-2 | `rejected` and `approved` same timestamp | rank rule applies deterministically |
| T-REF-1 | Partial refund on approved conversion | entitlement reduced via pinned rule, delta posting, I6/I7 hold |
| T-REF-2 | Full reversal after payout | user available negative, withdrawals blocked, I4/I5 hold |
| T-MULTI-1 | Order with 3 identical items (3 conversion_ids) | 3 conversions, 3 entitlements |
| T-PAY-1 | Payout fails with bank return | clearing → reserved/available per choice; no double credit |
| T-PAY-2 | Payout outcome unknown | no retry possible until reconciled; funds stay in clearing |
| T-CONC-1 | 20 concurrent withdrawals for the same user, balance for 1 | exactly 1 succeeds |
| T-CONC-2 | Webhook and poll for the same conversion concurrently | consistent final state, single posting per revision |
| T-CONC-3 | Two finance users approve same rule version / withdrawal | one wins, other gets 409/412 |
| T-CRASH-1 | Kill worker after raw insert, before upsert | retry completes, no duplicates |
| T-CRASH-2 | Kill worker mid-page | checkpoint unchanged, reprocess idempotent |
| T-CRASH-3 | Crash between withdrawal state change and ledger post | impossible by design (same tx); test asserts atomicity via fault injection |
| T-ADV-1 | Advance enabled, exposure at cap | next availability stays approved |
| T-COV-1 | Coverage < 1 | payouts auto-suspended, alert emitted |
| T-SET-1 | Settlement with unmatched amount | suspense posting, case opened, I12 holds |
| T-RULE-1 | Rule version changed after link creation | conversion uses pinned version |
| T-SSRF-1 | Short link redirecting to 127.0.0.1 / 169.254.169.254 / internal hostname / DNS rebinding | blocked |
| T-AUTHZ-1 | User A reads user B's order/withdrawal/ticket by id | 404 |
| T-REDIR-1 | `/go/` with tampered token or stored non-allowlisted URL | 404 / refuse |
| T-LEDGER-1 | Attempt UPDATE/DELETE on ledger tables as `app_rw` | permission denied |
| T-LEDGER-2 | Unbalanced posting | rejected at commit by trigger |

## 3. E2E journeys (Playwright)
1. Anonymous paste → preview → register → link → `/go` redirect (stubbed provider).
2. Login with Google (stub IdP in CI) → dashboard.
3. Mock conversion lifecycle (admin fixture endpoint, test env only) → order timeline updates → available → withdraw → admin approve → submit → statement import → paid.
4. Missing cashback claim submission.
5. Theme and language switch persist without FOUC. In-app browser interstitial (UA override).

## 4. Performance targets (k6 thresholds)
| Endpoint | Load (staging, 2 vCPU) | Threshold |
|---|---|---|
| `GET /` | 100 rps | p95 < 200 ms, errors < 0.1% |
| `POST /api/v1/analyze` (cached) | 30 rps | p95 < 300 ms |
| `POST /api/v1/links` (mock provider 300 ms) | 10 rps | p95 < 800 ms |
| `GET /go/{token}` | 100 rps | p95 < 50 ms |
| `GET /api/v1/wallet` | 50 rps | p95 < 150 ms |

## 5. Test data & fixtures
`testkit` provides builders (`UserBuilder`, `ConversionBuilder`), a deterministic clock, a seeded RNG, the Postgres container (one per test binary, schema per test via template DB), provider fixtures under `crates/affiliate/src/providers/*/fixtures/` (redacted, with a README stating source and date). Fixtures with real data need owner sign-off that PII is removed.

## 6. Definition of verified integration
A provider capability is `verified` only when the live verification checklist in `AFFILIATE_INTEGRATION_GUIDE.md` §6.5 has evidence attached in the PR. CI cannot set this flag.

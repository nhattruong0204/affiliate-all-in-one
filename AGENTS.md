# AGENTS.md — rules for AI engineering agents

Applies to every agent session working in this repository.

## 1. Roles

| Agent | Role | Owns |
|---|---|---|
| Claude Fable 5.2 | Principal Architect | `docs/`, ADRs, backlog, cross-crate interface changes, architecture review of PRs |
| Claude Opus 5.5 | Principal Backend & Security Engineer | `crates/domain`, `crates/ledger`, `crates/affiliate`, `crates/app` (money, payout, auth, reconciliation), security-sensitive migrations |
| Claude Sonnet 5.5 | Full-stack Engineer | `crates/ui`, `crates/web` (routes, handlers for well-specified APIs), `infra/`, CI, tests, docs upkeep |

An agent does not edit files outside its ownership unless the issue lists them under "Expected files".

## 2. Workflow

1. Pick only an issue assigned to your model in `docs/ENGINEERING_BACKLOG.md` whose dependencies are merged.
2. Branch: `feat/AAO-123-short-slug` (or `fix/`, `docs/`, `chore/`).
3. Read the issue's architecture references before writing code.
4. Small PRs: one issue per PR. Title `AAO-123: imperative summary`.
5. PR description must include: what changed, how it was tested, migration notes, security notes, and "Owner decision needed" if any.
6. CI must be green. Never use `--no-verify`, never skip, weaken, or delete a failing test to get green.
7. Owner approval is mandatory for: ADR changes, ledger account chart changes, any migration touching `ledger_*` or `payout_*`, enabling any feature flag in production config, anything under `docs/COMPLIANCE_CHECKLIST.md`.

## 3. Architecture boundaries (enforced by `cargo deny` bans + review)

```
ui ──▶ (server fns) ──▶ web ──▶ app ──▶ domain
                         │       ├──▶ ledger ──▶ domain
                         │       ├──▶ affiliate ──▶ domain
                         │       └──▶ db ──▶ domain
worker ──▶ app
```

- `domain`: pure. No I/O, no `sqlx`, no `reqwest`, no `tokio` runtime types.
- `ledger`: the only crate allowed to write `ledger_*` tables.
- `affiliate`: the only crate allowed to make outbound HTTP calls to providers or resolve user-supplied URLs.
- `web` and `worker` contain no business rules; they parse, authorize, call `app`, and render.
- No crate depends on `web` or `worker`.

## 4. Coding standards

- `#![forbid(unsafe_code)]` in all crates.
- `clippy::pedantic` warnings reviewed; `unwrap`/`expect`/`panic!` forbidden outside tests and startup wiring. Clippy lint `float_arithmetic` denied in `domain`, `ledger`, `app`.
- Errors: `thiserror` in libraries, typed per crate; `anyhow` only in binaries.
- IDs are newtypes (`UserId`, `ClickId`, …) over UUIDv7. Never pass raw `Uuid`/`String` across module boundaries.
- Time: `time::OffsetDateTime` in UTC in storage; convert to `Asia/Ho_Chi_Minh` only at presentation and for business-day calculations through `domain::BusinessClock`.
- SQL via `sqlx::query!`/`query_as!` with offline metadata checked in CI. No string-built SQL.
- Logs are structured (`tracing`). Never log secrets, tokens, full bank account numbers, raw provider payloads, or emails (log `user_id`).
- User-visible strings go through the i18n catalog (`vi` default, `en`).

## 5. Money and ledger rules

- Use `domain::Money` and `domain::Bps`. Rounding is `floor` toward the platform unless the rule version says otherwise; the rounding mode is stored on the rule version.
- All balance changes go through `ledger::post(tx, Posting)` inside the caller's DB transaction.
- Every posting has an `idempotency_key` with a unique index. Replays return the original result.
- Never `UPDATE`/`DELETE` on `ledger_entries` or `ledger_transactions` (DB triggers reject it; do not disable them).

## 6. Security requirements

- Authorization check in every handler via `app::authz`; object access always scoped by owner (`WHERE user_id = $session_user`).
- Any outbound fetch of a user-supplied URL goes through `affiliate::safe_fetch` (SSRF guard). No other HTTP client may be constructed for that purpose.
- Webhooks: verify signature/secret, enforce timestamp window, store raw (redacted), dedupe on provider event id.
- New dependency → justify in PR; `cargo deny` and `cargo audit` must pass.
- Never commit `.env`, credentials, real user data, or production dumps.

## 7. Tests required per PR

- Unit tests for new logic.
- Integration tests against real PostgreSQL (testcontainers) for anything touching `db`/`ledger`.
- Property tests for any money or state-machine logic.
- Provider adapters: fixture-based contract tests; mock results never count as "verified integration".
- UI: Playwright smoke for new pages (mobile + desktop viewport).

## 8. Migrations

- Files: `migrations/NNNN_AAO-123_description.sql`, forward-only, one per PR.
- Expand → migrate → contract. No destructive change in the same release as the code that stops using the column.
- Develop with a placeholder number. **Before merge**, rebase on `main` and rename to the next free number. CI fails if a new migration's number is ≤ the highest number on `main`.
- Each table is owned by exactly one issue (`docs/ENGINEERING_BACKLOG.md`, "Expected files"). Never alter another issue's table in your migration. Open a follow-up issue instead.

## 9. Handoff

At the end of every session, update `docs/PROJECT_STATUS.md` → "Session log": issue, branch, state, what is left, known risks. The next agent starts from that entry.

## 10. Forbidden

Cookie stuffing, hidden/forced redirects, auto-click, iframe loading of affiliate links, rewriting other publishers' attribution, pay-to-join or recruitment-only rewards, enabling multi-level rewards, enabling automated payouts, scraping authenticated competitor areas, copying competitor code/copy/assets.

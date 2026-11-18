# Migrations

Forward-only SQL for PostgreSQL 17, applied by the `db` crate's migration runner (AAO-004).
Rules come from `AGENTS.md` §8; this file is the short version.

- **File name:** `NNNN_AAO-123_description.sql` (four-digit number, issue ID, snake_case description).
- **One migration per PR**, numbered by the issue that owns the table. Each table has exactly one
  owning issue (see "Expected files" in `docs/ENGINEERING_BACKLOG.md`); never alter another issue's
  table. Open a follow-up issue instead.
- **Placeholder number while developing.** Before merge, rebase on `main` and rename to the next
  free number. Never edit or renumber a merged migration.
- **Expand → migrate → contract.** Never ship a destructive change in the same release as the code
  that stops using the column or table.
- **Owner approval** is required for anything touching `ledger_*` or `payout_*` tables (CODEOWNERS
  covers this whole directory).
- **No data in migrations** except reference/seed rows the application cannot run without; never
  personal data or secrets.

Enforcement that a new number is higher than the highest on `main` is added together with the first
migration in AAO-004.

Schema reference: `docs/DATABASE_DESIGN.md`. Ledger tables and triggers: `docs/FINANCIAL_LEDGER.md`.

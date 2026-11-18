# ADR-0010: Double-entry immutable ledger as financial source of truth

Status: Proposed · Date: 2026-11-17

## Decision
Every movement of value (estimated → pending → approved → available → reserved → paid, and every reversal, reward and fee) is a balanced `ledger_transaction` of ≥ 2 `ledger_entries`. Rows are append-only (DB triggers reject UPDATE/DELETE). `ledger_balances` is a derived, transactionally maintained projection that is verified nightly against a full recomputation. Corrections use compensating transactions only. See `FINANCIAL_LEDGER.md`.

## Consequences
- (+) Auditability, reproducibility, and invariants we can test.
- (−) More rows and more ceremony than a `balance` column. The ceremony is the point.

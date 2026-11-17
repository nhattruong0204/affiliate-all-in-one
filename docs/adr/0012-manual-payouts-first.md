# ADR-0012: Manual-review payouts first; automated disbursement disabled

Status: Proposed · Date: 2026-11-17

## Context
VietQR is a payment-information standard (QR payload), not a transfer-execution API. MoMo publishes disbursement APIs, but using them requires a commercial contract. Bank transfer APIs require bank partnerships.

## Decision
V1 payouts are executed by a finance operator from the company bank account. The system generates a VietQR payload (bank BIN, account, amount, unique transfer reference) and records evidence (bank reference, screenshot hash). Status is reconciled against bank statement import (CSV). `PayoutRail` trait with `ManualBankTransfer` (enabled), `MomoDisbursement` (disabled), `BankApi` (disabled). Flag `payouts.automated` defaults off, needs two-person approval, and has an emergency kill switch.

## Consequences
Payout throughput is limited by operator capacity. Batch views and a maker-checker split make roughly 200 payouts per day manageable.

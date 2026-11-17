# ADR-0014: Single-level referrals only; multi-level hard-disabled

Status: Proposed · Date: 2026-11-17

## Context
The reference product hints at a two-tier (F1/F2) scheme (INFERRED). In Vietnam, multi-level selling is a licensed, conditional business line (Decree 40/2018/ND-CP as amended), and affiliate programs commonly prohibit sub-affiliate arrangements. Legal advice is required.

## Decision
Ship single-level referral rewards only. They are funded from platform retained commission and paid only on the referee's **settled** orders. Never reward sign-ups alone, never require payment to join. The schema stores `depth` for future evaluation, but code rejects `depth > 1` unless the flag `referrals.multilevel` is on. That flag requires owner and legal approval recorded in `COMPLIANCE_CHECKLIST.md`.

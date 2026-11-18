# ADR-0004: PostgreSQL as the only stateful dependency in V1 (no Redis)

Status: Proposed · Date: 2026-11-17

## Context
Expected V1 load is well under 50 req/s sustained (`COST_ESTIMATION.md`). Every extra stateful service adds backup, monitoring and failure modes.

## Decision
PostgreSQL 17 holds the system of record, sessions, job queue, outbox, rate-limit buckets (token bucket rows updated with `UPDATE … RETURNING`), product metadata cache (TTL column) and idempotency keys. Hot read-only config is cached in process with `moka` (≤ 30s TTL). SQLx with compile-time checked queries.

## Consequences
- (+) One backup and restore procedure. Transactional enqueue.
- (−) Rate-limit writes add DB load. We revisit when Postgres CPU stays above 60% or rate-limit queries pass 30% of QPS.

## Revisit triggers
Introduce Redis via a new ADR if p95 DB latency > 20 ms is attributable to sessions or rate limits, or we need cross-instance pub/sub.

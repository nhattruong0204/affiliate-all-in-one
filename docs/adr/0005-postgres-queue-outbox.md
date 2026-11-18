# ADR-0005: Postgres-backed job queue with transactional outbox

Status: Proposed · Date: 2026-11-17

## Context
We need durable background work (provider sync, reconciliation, emails) that never loses or double-applies financial effects, even across worker crashes.

## Decision
- `jobs(id, kind, payload, dedupe_key, run_at, attempts, max_attempts, leased_until, leased_by, last_error, status)` with `UNIQUE(kind, dedupe_key) WHERE status IN ('queued','running')`.
- Workers lease with `SELECT … FOR UPDATE SKIP LOCKED LIMIT n`, set `leased_until`, and heartbeat. Expired leases are reclaimable.
- Backoff: `min(2^attempt × base, cap)` with jitter. At `max_attempts`, the job becomes `dead` and is shown in admin with a one-click requeue (audited).
- Business transactions write domain events to `outbox` in the same transaction. A worker publishes them and marks them `published_at`. Consumers are idempotent on `event_id` (inbox table `processed_events`).
- Job handlers must be idempotent. The financial effect is guarded by ledger idempotency keys, not by "exactly once" delivery.

## Consequences
- (+) No lost jobs and no dual-write between DB and broker.
- (−) Queue throughput is bounded by Postgres, which is ample for us (thousands of jobs per minute).

## Alternatives
Redis queues (extra dependency, non-transactional enqueue). Kafka (operationally heavy, unjustified).

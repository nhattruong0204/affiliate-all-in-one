# ADR-0002: Modular monolith with separate web and worker processes

Status: Proposed · Date: 2026-11-17

## Context
The money path (conversion → entitlement → ledger → withdrawal) needs atomic multi-table changes. The team is three AI agents plus an owner, and ops budget is a single VPS.

## Decision
One Cargo workspace, nine crates max (see `ARCHITECTURE.md` §4), two binaries: `web` (HTTP) and `worker` (jobs). Both share `app` and talk to one PostgreSQL.

## Consequences
- (+) Use-cases that touch several contexts run in a single DB transaction. No sagas or two-phase commit.
- (+) A crashing or slow sync job can't take down HTTP serving, and the two scale independently later.
- (−) Module discipline depends on review and the dependency check. Contexts must not reach into each other's tables.

## Alternatives
Microservices: rejected (distributed money consistency, higher cost). Single binary: rejected (fault isolation, independent restarts).

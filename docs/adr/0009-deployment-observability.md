# ADR-0009: Single-VPS Docker Compose deployment with Caddy and self-hosted observability

Status: Proposed · Date: 2026-11-17

## Decision
- Production runs on one Linux VPS (Singapore or Vietnam region for latency) with Docker Compose: caddy, web, worker, postgres, otel-collector, prometheus, loki, grafana, backup.
- Images are built in GitHub Actions (multi-stage, `cargo-chef`, distroless runtime, non-root UID 10001), signed (cosign keyless) and pushed to GHCR. Deploys pull by digest.
- Networks: `edge` (caddy↔web), `app` (web/worker↔postgres), `obs`. Postgres has no published port. Grafana is only reachable via Caddy with basic auth plus an IP allowlist, or through an SSH tunnel.
- Staging is the same compose file on a smaller VPS, with providers in sandbox/mock mode.

## Consequences
Cheap and simple, but a single host is a single point of failure: target 99.5% availability, RTO 4h. `INFRASTRUCTURE.md` §8 describes the documented path to split hosts.

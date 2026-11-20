# Observability, health and configuration (AAO-005)

What ships in V0.1: structured JSON logs with PII redaction, per-route RED metrics, liveness and
readiness endpoints, RFC 9457 error responses with trace ids, and typed startup configuration.
OTLP trace export is **not** included (issue scope allows JSON logs + Prometheus to suffice);
the trace id is already W3C-shaped so an exporter can be added without touching call sites.

Code: `crates/app/src/{config,telemetry,metrics,health,error}.rs`,
`crates/web/src/{health,problem}.rs`, `crates/web/src/middleware/observe.rs`.

## 1. Configuration

Read once at startup by `app::config::Config`, validated as a whole (all problems are reported
together) and fail-fast. Secret values are never echoed in errors or `Debug` output.

| Variable | Default | Notes |
|---|---|---|
| `APP_ENV` | `development` | `production` tightens validation (below). **Set it explicitly in deployments.** |
| `RUST_LOG` | `info` | `tracing` filter; an invalid filter fails startup |
| `LOG_FORMAT` | `json` in production, `text` otherwise | `json` \| `text` |
| `WEB_BIND` | `127.0.0.1:8080` | public listener |
| `METRICS_BIND` | `127.0.0.1:9100` | internal listener, `/metrics` only; must differ from `WEB_BIND` |
| `RATE_LIMIT_PAGES_PER_MINUTE` | `120` | per-client-IP page budget (1–10 000); `429` + `Retry-After` when exceeded; assets and probes exempt |
| `RATE_LIMIT_FORMS_PER_MINUTE` | `10` | budget for anonymous form endpoints (applied from AAO-100) |
| `TRUST_FORWARDED_FOR` | `false` | use the right-most `X-Forwarded-For` entry as the client IP; set `true` only behind our own proxy |
| `TURNSTILE_SITE_KEY`, `TURNSTILE_SECRET_KEY[_FILE]` | unset | both or neither; enables the CAPTCHA CSP origins (verification adapter lands with AAO-100) |
| `READINESS_TIMEOUT_MS` | `2000` | budget per readiness check, 100–10000 |
| `DATABASE_URL` / `DATABASE_URL_FILE` | unset | `postgres://…`; set one, not both |
| `WEB_UI_GALLERY`, `WEB_PUBLIC_BASE_URL` | off / unset | page-rendering switches (`web::ssr::SsrConfig`); the gallery is refused when `APP_ENV=production` |

Production rules: `DATABASE_URL_FILE` is **required**, and secrets must come from a `*_FILE` path
(compose `secrets:` → `/run/secrets/*`). An inline `DATABASE_URL` is rejected because environment
values leak through `/proc`, `docker inspect` and crash reports. Adding a new secret: read it with
`config::secret(...)` so it inherits these rules.

## 2. Logging

* One JSON object per line on stdout: `timestamp` (RFC 3339 UTC, ms), `level`, `target`,
  `message`, `trace_id`, `fields` (the event's fields), `span` (merged span fields).
* **Redaction** is by field name: any field whose lowercase name *contains* `email`, `phone`,
  `account_number`, `token` or `password` has its value replaced by `[REDACTED]`, for every value
  type, on events and spans (`access_token`, `phone_number`, `new_password_hash` are all caught;
  the price is the occasional false positive such as `token_count`, which is the safe direction).
  Extend `telemetry::REDACTED_FIELDS` to widen it.
* **Limit:** the layer cannot see PII inside a formatted message. Log values as fields
  (`tracing::info!(email = %e, "login")`), never interpolated (`tracing::info!("login {e}")`).
* The request log line records the **route pattern**, never the raw path or query string
  (queries may carry tokens).
* `/healthz` and `/readyz` request lines are logged at `debug` to keep probes out of the log.

## 3. Trace ids

Every request gets a 32-hex W3C trace id: continued from a valid incoming `traceparent`,
otherwise generated. It is placed in the request span (so every log line of that request carries
`trace_id`), in the `X-Trace-Id` response header and in the `trace_id` of problem+json bodies.
A user can quote it to support; operators grep the logs for it.

## 4. Errors (RFC 9457)

Handlers return `Result<_, web::problem::ApiError>`; `?` converts an `app::error::AppError`.
Body (`docs/api/openapi.yaml`, `Problem`): `type` (`https://docs.aaio.vn/errors/<code>`), `title`
(English), `status`, `code` (stable; clients switch on it), `detail` (localized from
`Accept-Language`, Vietnamese by default), `trace_id`, optional `errors[{field,code}]`.
`detail` is fixed text per code, so internal error messages cannot leak; the cause of an
`Internal` error is logged under the request's trace id.

New error: add an `AppError` variant, then its `status`, `code`, `title` and both `detail`
languages (the catalogue tests fail on a missing translation), then document the code in the
OpenAPI file.

## 5. Health and readiness

| Endpoint | Meaning | Touches dependencies |
|---|---|---|
| `GET /healthz` | process is alive (liveness) | no |
| `GET /readyz` | can serve traffic (readiness) | yes, each check bounded by `READINESS_TIMEOUT_MS` |

`/readyz` is `200 {"status":"ok","checks":[…]}` or `503` problem+json (`code: not_ready`,
`errors: [{field: "database", code: "unavailable"}]`). Failure reasons are logged at `warn`, not
returned. Alert on `/readyz` failing (INFRASTRUCTURE §8).

**Interim database check.** `crates/db` (AAO-004) owns sqlx and has not landed, so the only
check today is a TCP reachability probe of the `DATABASE_URL` host and port
(`app::health::TcpProbe`). It detects a stopped or unreachable database, **not** a database that
accepts connections but is unhealthy, and it cannot verify migrations. AAO-004 must register a
`ReadinessCheck` that runs `SELECT 1` and checks that migrations are current (both under the
same timeout) and drop `TcpProbe` from `Readiness::from_config`. Until then the "migrations
current" half of the AAO-005 acceptance criteria is not met; see the PR's owner decision.

## 6. Metrics

`GET /metrics` (Prometheus text format 0.0.4) is served on **`METRICS_BIND`, a separate
listener**, so it is not reachable through the public port or the reverse proxy by construction.

| Series | Labels | Use |
|---|---|---|
| `http_requests_total` | `method`, `route`, `status` | rate; errors = `status=~"5.."` |
| `http_request_duration_seconds` (histogram, 5 ms–10 s) | `method`, `route` | duration, p95 |
| `aao_build_info` | `version` | constant 1 |

`route` is the matched route pattern (`/assets/{*path}`); unmatched requests share
`route="unmatched"`, and unknown HTTP methods collapse to `OTHER`, so label cardinality is bounded
by the router, not by clients. Hand-rolled instead of the `prometheus` crate (three families,
no new dependencies, no floats); revisit if the metric set grows. DB-pool, job and business
gauges arrive with the issues that own those subsystems.

Example queries:

```promql
sum by (route) (rate(http_requests_total[5m]))                                   # Rate
sum(rate(http_requests_total{status=~"5.."}[5m])) / sum(rate(http_requests_total[5m]))  # Errors
histogram_quantile(0.95, sum by (le, route) (rate(http_request_duration_seconds_bucket[5m])))  # Duration
```

## 7. Operating notes for AAO-092 (deployment)

* Set `APP_ENV=production`, `DATABASE_URL_FILE=/run/secrets/database_url`.
* `METRICS_BIND=0.0.0.0:9100` inside the container network; **do not publish the port** and do not
  proxy `/metrics` in Caddy. Scrape it from Prometheus on the compose network.
* Container healthcheck → `/healthz`; Caddy active health check and the uptime monitor → `/readyz`.
* Ship container stdout (JSON) to Loki; query by `trace_id`.

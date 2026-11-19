//! Operational endpoints.
//!
//! * Public listener: `GET /healthz` (process is alive) and `GET /readyz` (dependencies are
//!   reachable). Both carry no data, so they need no authentication (`API_SPECIFICATION` §1).
//! * Internal listener: `GET /metrics` in Prometheus text format. It is a **separate socket**
//!   (`METRICS_BIND`) so that it cannot be reached through the public reverse proxy by accident.
//!   Deployments must not publish that port (AAO-092; `docs/OBSERVABILITY.md`).

use std::sync::Arc;

use app::error::AppError;
use app::health::Readiness;
use app::metrics::{self, Metrics};
use axum::Router;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;

use crate::problem::ApiError;

/// `/healthz` and `/readyz`, to be merged into the public router.
pub fn routes(readiness: Arc<Readiness>) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .with_state(readiness)
}

/// The internal router: `/metrics` only, everything else is a plain 404.
pub fn metrics_router(metrics: Arc<Metrics>) -> Router {
    Router::new()
        .route("/metrics", get(render_metrics))
        .with_state(metrics)
}

async fn healthz() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        "ok",
    )
        .into_response()
}

async fn readyz(State(readiness): State<Arc<Readiness>>) -> Response {
    let report = readiness.run().await;
    if report.is_ready() {
        (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "application/json"),
                (header::CACHE_CONTROL, "no-store"),
            ],
            report.to_json().to_string(),
        )
            .into_response()
    } else {
        ApiError(AppError::NotReady(report.failures())).into_response()
    }
}

async fn render_metrics(State(metrics): State<Arc<Metrics>>) -> Response {
    (
        [
            (header::CONTENT_TYPE, metrics::CONTENT_TYPE),
            (header::CACHE_CONTROL, "no-store"),
        ],
        metrics.render(),
    )
        .into_response()
}

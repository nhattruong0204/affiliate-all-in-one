//! Per-request observability: trace id, request span, access log line and RED metrics.
//!
//! This is the outermost layer, so it also sees responses produced by inner layers (security
//! headers, 404 fallback). It installs a [`RequestContext`] for the duration of the request;
//! `crate::problem::ApiError` reads it to fill `trace_id` and the localized `detail`.

use std::sync::Arc;
use std::time::Instant;

use app::error::{Lang, RequestContext};
use app::metrics::Metrics;
use app::telemetry::TraceId;
use axum::extract::{MatchedPath, Request, State};
use axum::http::header::ACCEPT_LANGUAGE;
use axum::http::{HeaderName, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;
use tracing::Instrument as _;

/// Response header carrying the trace id, so a user can quote it to support.
pub const TRACE_ID_HEADER: &str = "x-trace-id";

/// Route label for requests that matched no route (404s); one bounded value, never the raw path.
const UNMATCHED: &str = "unmatched";

/// Probes are polled every few seconds; keep them out of the normal log level.
fn is_probe(route: &str) -> bool {
    matches!(route, "/healthz" | "/readyz")
}

pub async fn observe(State(metrics): State<Arc<Metrics>>, req: Request, next: Next) -> Response {
    let method = req.method().as_str().to_owned();
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map_or(UNMATCHED, MatchedPath::as_str)
        .to_owned();
    // Never log the query string: it may carry tokens. The path is logged only as the route.
    let trace_id = req
        .headers()
        .get("traceparent")
        .and_then(|value| value.to_str().ok())
        .and_then(TraceId::from_traceparent)
        .unwrap_or_else(TraceId::generate);
    let lang = Lang::from_accept_language(
        req.headers()
            .get(ACCEPT_LANGUAGE)
            .and_then(|value| value.to_str().ok()),
    );

    let span = tracing::info_span!(
        "request",
        trace_id = %trace_id,
        method = %method,
        route = %route,
    );
    let context = RequestContext {
        trace_id: trace_id.clone(),
        lang,
    };

    let started = Instant::now();
    let mut response = context.scope(next.run(req).instrument(span.clone())).await;
    let elapsed = started.elapsed();
    let status = response.status().as_u16();

    span.in_scope(|| {
        let elapsed_ms = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
        if is_probe(&route) {
            tracing::debug!(status, elapsed_ms, "request");
        } else {
            tracing::info!(status, elapsed_ms, "request");
        }
    });
    metrics.observe_request(&method, &route, status, elapsed);

    if let Ok(value) = HeaderValue::from_str(trace_id.as_str()) {
        response
            .headers_mut()
            .insert(HeaderName::from_static(TRACE_ID_HEADER), value);
    }
    response
}

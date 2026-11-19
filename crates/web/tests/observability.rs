//! End-to-end behaviour of the operational surface: readiness against a real (local) TCP
//! dependency, problem+json on failure, trace-id propagation, metrics and log hygiene.

use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use app::health::{Readiness, TcpProbe};
use app::metrics::Metrics;
use app::telemetry::{JsonLayer, TraceId};
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Request, StatusCode};
use tokio::net::TcpListener;
use tower::ServiceExt;
use tracing_subscriber::layer::SubscriberExt;
use web::health::metrics_router;
use web::ssr::{SsrConfig, router};

struct Harness {
    public: Router,
    metrics: Arc<Metrics>,
}

impl Harness {
    fn new(readiness: Readiness) -> Self {
        let metrics = Arc::new(Metrics::new());
        Self {
            public: router(SsrConfig::default(), Arc::new(readiness), metrics.clone()),
            metrics,
        }
    }

    async fn get(&self, uri: &str, headers: &[(&str, &str)]) -> (StatusCode, HeaderMap, String) {
        send(self.public.clone(), uri, headers).await
    }
}

async fn send(app: Router, uri: &str, headers: &[(&str, &str)]) -> (StatusCode, HeaderMap, String) {
    let mut request = Request::builder().uri(uri);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = app
        .oneshot(request.body(Body::empty()).expect("request"))
        .await
        .expect("infallible");
    let (parts, body) = response.into_parts();
    let bytes = to_bytes(body, usize::MAX).await.expect("body");
    (
        parts.status,
        parts.headers,
        String::from_utf8_lossy(&bytes).into_owned(),
    )
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> &'a str {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
}

async fn database_probe() -> (TcpListener, Readiness) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let readiness = Readiness::new(Duration::from_secs(2)).with_check(Arc::new(TcpProbe::new(
        "database",
        "127.0.0.1",
        port,
    )));
    (listener, readiness)
}

#[tokio::test]
async fn readyz_is_200_while_the_database_listens_and_503_once_it_stops() {
    let (listener, readiness) = database_probe().await;
    let harness = Harness::new(readiness);

    let (status, headers, body) = harness.get("/readyz", &[]).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(header(&headers, "cache-control"), "no-store");
    let json: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(json["status"], "ok");
    assert_eq!(json["checks"][0]["name"], "database");

    drop(listener); // the database goes away

    let (status, headers, body) = harness.get("/readyz", &[]).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    assert_eq!(header(&headers, "content-type"), "application/problem+json");
    let json: serde_json::Value = serde_json::from_str(&body).expect("problem json");
    assert_eq!(json["code"], "not_ready");
    assert_eq!(json["status"], 503);
    assert_eq!(json["type"], "https://docs.aaio.vn/errors/not_ready");
    assert_eq!(json["errors"][0]["field"], "database");
    assert_eq!(json["errors"][0]["code"], "unavailable");
    assert!(
        !body.contains("127.0.0.1"),
        "infrastructure detail leaked: {body}"
    );
    assert_eq!(
        json["trace_id"].as_str(),
        Some(header(&headers, "x-trace-id")),
        "body and header carry the same trace id"
    );
}

#[tokio::test]
async fn healthz_ignores_dependencies() {
    let (listener, readiness) = database_probe().await;
    drop(listener);
    let harness = Harness::new(readiness);
    let (status, _, body) = harness.get("/healthz", &[]).await;
    assert_eq!((status, body.as_str()), (StatusCode::OK, "ok"));
}

#[tokio::test]
async fn problem_detail_follows_accept_language() {
    let (listener, readiness) = database_probe().await;
    drop(listener);
    let harness = Harness::new(readiness);
    let detail = |body: &str| {
        serde_json::from_str::<serde_json::Value>(body).expect("json")["detail"]
            .as_str()
            .expect("detail")
            .to_owned()
    };
    let (_, _, vi) = harness.get("/readyz", &[]).await;
    let (_, _, en) = harness
        .get("/readyz", &[("accept-language", "en-US,en;q=0.9")])
        .await;
    assert_eq!(detail(&vi), "Hệ thống chưa sẵn sàng. Vui lòng thử lại sau.");
    assert_eq!(
        detail(&en),
        "The service is not ready. Please try again shortly."
    );
}

#[tokio::test]
async fn an_incoming_traceparent_is_continued_and_garbage_is_replaced() {
    let harness = Harness::new(Readiness::new(Duration::from_secs(1)));
    let upstream = "4bf92f3577b34da6a3ce929d0e0e4736";
    let (_, headers, _) = harness
        .get(
            "/healthz",
            &[("traceparent", &format!("00-{upstream}-00f067aa0ba902b7-01"))],
        )
        .await;
    assert_eq!(header(&headers, "x-trace-id"), upstream);

    let (_, headers, _) = harness
        .get("/healthz", &[("traceparent", "nonsense")])
        .await;
    let generated = header(&headers, "x-trace-id");
    assert!(TraceId::parse(generated).is_some(), "{generated}");
    assert_ne!(generated, upstream);
}

#[tokio::test]
async fn metrics_are_served_only_on_the_internal_router() {
    let harness = Harness::new(Readiness::new(Duration::from_secs(1)));
    harness.get("/", &[]).await;
    harness.get("/", &[]).await;
    harness.get("/healthz", &[]).await;
    harness.get("/no/such/page", &[]).await;
    harness.get("/assets/app.css", &[]).await;

    // Not reachable through the public listener: it is just another 404 there.
    let (status, _, body) = harness.get("/metrics", &[]).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(!body.contains("http_requests_total"), "{body}");

    let internal = metrics_router(harness.metrics.clone());
    let (status, headers, text) = send(internal.clone(), "/metrics", &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert!(header(&headers, "content-type").starts_with("text/plain; version=0.0.4"));
    for expected in [
        r#"http_requests_total{method="GET",route="/",status="200"} 2"#,
        r#"http_requests_total{method="GET",route="/healthz",status="200"} 1"#,
        // the raw path is never a label: unknown URLs share one bounded series
        r#"http_requests_total{method="GET",route="unmatched",status="404"} 2"#,
        r#"http_requests_total{method="GET",route="/assets/{*path}",status="200"} 1"#,
        r#"http_request_duration_seconds_count{method="GET",route="/"} 2"#,
    ] {
        assert!(text.contains(expected), "missing {expected}\n{text}");
    }
    assert!(!text.contains("no/such/page"), "{text}");

    let (status, _, _) = send(internal, "/", &[]).await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "the internal router serves metrics only"
    );
}

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);

impl io::Write for Capture {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().expect("lock").extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn request_logs_carry_the_trace_id_and_never_the_query_string() {
    let sink = Capture::default();
    let writer = sink.clone();
    let subscriber = tracing_subscriber::registry().with(JsonLayer::new(move || writer.clone()));
    let _guard = tracing::subscriber::set_default(subscriber);

    let harness = Harness::new(Readiness::new(Duration::from_secs(1)));
    let (_, headers, _) = harness
        .get(
            "/en?token=super-secret&email=a@b.co",
            &[("cookie", "aao_theme=dark")],
        )
        .await;
    let trace_id = header(&headers, "x-trace-id").to_owned();

    let raw = String::from_utf8(sink.0.lock().expect("lock").clone()).expect("utf8");
    assert!(
        !raw.contains("super-secret") && !raw.contains("a@b.co"),
        "{raw}"
    );
    let lines: Vec<serde_json::Value> = raw
        .lines()
        .map(|l| serde_json::from_str(l).expect("JSON line"))
        .collect();
    let request = lines
        .iter()
        .find(|l| l["message"] == "request")
        .unwrap_or_else(|| panic!("no request log line in {raw}"));
    assert_eq!(request["trace_id"], trace_id.as_str());
    assert_eq!(request["span"]["route"], "/en");
    assert_eq!(request["span"]["method"], "GET");
    assert_eq!(request["fields"]["status"], 200);
}

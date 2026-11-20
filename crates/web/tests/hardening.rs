//! The public surface as a whole: security headers on every response and per-client rate limits
//! (AAO-091). The Mozilla Observatory A+ acceptance check needs the staging stack (AAO-092); these
//! tests pin the header set it grades.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use app::health::Readiness;
use app::metrics::Metrics;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, Request, StatusCode};
use tower::ServiceExt;
use web::middleware::Hardening;
use web::middleware::headers::HeaderPolicy;
use web::ssr::{SsrConfig, router};

fn app(hardening: Hardening) -> Router {
    router(
        SsrConfig::default(),
        Arc::new(Readiness::new(Duration::from_secs(1))),
        Arc::new(Metrics::new()),
        hardening,
    )
}

async fn get(
    app: &Router,
    uri: &str,
    peer: &str,
    forwarded: Option<&str>,
) -> (StatusCode, HeaderMap, String) {
    let mut request = Request::builder().uri(uri);
    if let Some(value) = forwarded {
        request = request.header("x-forwarded-for", value);
    }
    let mut request = request.body(Body::empty()).expect("request");
    let addr: SocketAddr = format!("{peer}:50000").parse().expect("peer");
    request.extensions_mut().insert(ConnectInfo(addr));
    let response = app.clone().oneshot(request).await.expect("infallible");
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
        .unwrap_or("")
}

const ROUTES: [&str; 7] = [
    "/",
    "/en",
    "/nope",
    "/healthz",
    "/readyz",
    "/assets/app.css",
    "/assets/missing",
];

#[tokio::test]
async fn development_sends_the_baseline_headers_everywhere_but_no_hsts() {
    let app = app(Hardening::default());
    for uri in ROUTES {
        let (_, h, _) = get(&app, uri, "192.0.2.1", None).await;
        assert!(
            header(&h, "content-security-policy").contains("default-src 'none'"),
            "{uri}"
        );
        assert_eq!(header(&h, "x-content-type-options"), "nosniff", "{uri}");
        assert_eq!(header(&h, "x-frame-options"), "DENY", "{uri}");
        assert_eq!(
            header(&h, "referrer-policy"),
            "strict-origin-when-cross-origin",
            "{uri}"
        );
        assert_eq!(
            header(&h, "cross-origin-resource-policy"),
            "same-origin",
            "{uri}"
        );
        assert!(!header(&h, "permissions-policy").is_empty(), "{uri}");
        assert!(h.get("strict-transport-security").is_none(), "{uri}");
    }
}

#[tokio::test]
async fn production_adds_hsts_and_turnstile_only_when_configured() {
    let prod = app(Hardening::new(
        HeaderPolicy {
            hsts: true,
            turnstile: false,
        },
        120,
        false,
    ));
    let (_, h, _) = get(&prod, "/", "192.0.2.1", None).await;
    assert!(header(&h, "strict-transport-security").starts_with("max-age="));
    assert!(!header(&h, "content-security-policy").contains("cloudflare"));

    let with_captcha = app(Hardening::new(
        HeaderPolicy {
            hsts: true,
            turnstile: true,
        },
        120,
        false,
    ));
    let (_, h, _) = get(&with_captcha, "/", "192.0.2.1", None).await;
    let csp = header(&h, "content-security-policy");
    assert!(csp.contains("https://challenges.cloudflare.com"), "{csp}");
    assert!(!csp.contains("unsafe-"), "{csp}");
}

#[tokio::test]
async fn pages_are_limited_per_client_with_a_problem_response_that_keeps_the_headers() {
    let app = app(Hardening::new(HeaderPolicy::default(), 2, false));
    for _ in 0..2 {
        assert_eq!(get(&app, "/", "192.0.2.1", None).await.0, StatusCode::OK);
    }
    let (status, h, body) = get(&app, "/en", "192.0.2.1", None).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(header(&h, "content-type"), "application/problem+json");
    // 2 per minute = one token per 30 s.
    assert_eq!(header(&h, "retry-after"), "30");
    assert_eq!(
        header(&h, "x-frame-options"),
        "DENY",
        "a 429 is still hardened"
    );
    assert!(
        !header(&h, "x-trace-id").is_empty(),
        "a 429 is still observed"
    );
    let json: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(json["status"], 429);
    assert_eq!(json["code"], "rate_limited");
    assert!(
        !body.contains("192.0.2.1"),
        "the client address never appears in a response"
    );

    // Another client is unaffected, and the 404 fallback shares the page budget.
    assert_eq!(get(&app, "/", "192.0.2.2", None).await.0, StatusCode::OK);
    assert_eq!(
        get(&app, "/nope", "192.0.2.1", None).await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
}

#[tokio::test]
async fn assets_and_probes_are_never_limited() {
    let app = app(Hardening::new(HeaderPolicy::default(), 1, false));
    assert_eq!(get(&app, "/", "192.0.2.1", None).await.0, StatusCode::OK);
    assert_eq!(
        get(&app, "/", "192.0.2.1", None).await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
    for _ in 0..5 {
        for uri in ["/assets/app.css", "/healthz", "/readyz"] {
            assert_ne!(
                get(&app, uri, "192.0.2.1", None).await.0,
                StatusCode::TOO_MANY_REQUESTS,
                "{uri}"
            );
        }
    }
}

#[tokio::test]
async fn forwarded_for_is_honoured_only_when_trusted() {
    let proxy = "10.0.0.2";
    let trusted = app(Hardening::new(HeaderPolicy::default(), 1, true));
    assert_eq!(
        get(&trusted, "/", proxy, Some("203.0.113.1")).await.0,
        StatusCode::OK
    );
    assert_eq!(
        get(&trusted, "/", proxy, Some("203.0.113.2")).await.0,
        StatusCode::OK,
        "distinct clients behind the proxy"
    );
    assert_eq!(
        get(&trusted, "/", proxy, Some("203.0.113.1")).await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
    // A spoofed left-most entry does not buy a fresh bucket: the proxy-appended one counts.
    assert_eq!(
        get(&trusted, "/", proxy, Some("1.1.1.1, 203.0.113.1"))
            .await
            .0,
        StatusCode::TOO_MANY_REQUESTS
    );

    let untrusted = app(Hardening::new(HeaderPolicy::default(), 1, false));
    assert_eq!(
        get(&untrusted, "/", proxy, Some("203.0.113.1")).await.0,
        StatusCode::OK
    );
    assert_eq!(
        get(&untrusted, "/", proxy, Some("203.0.113.2")).await.0,
        StatusCode::TOO_MANY_REQUESTS,
        "the header is ignored, so both share the peer's bucket"
    );
}

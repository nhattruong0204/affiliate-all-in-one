//! Server-side rendering of the `ui` pages: routing, locale/theme resolution, response headers
//! and the embedded static assets.
//!
//! V0.1 serves only the neutral shell preview, the (opt-in) component gallery, `/healthz`,
//! assets and a 404. Business routes arrive with their own issues and are added to [`router`].

// axum handlers must be `async fn` even when they never await, and extractors are taken by value.
#![allow(clippy::unused_async, clippy::needless_pass_by_value)]

use std::sync::Arc;
use std::time::Instant;

use axum::Router;
use axum::extract::{Path, RawQuery, Request, State};
use axum::http::header::{self, HeaderName};
use axum::http::{HeaderMap, HeaderValue, StatusCode, Uri};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use ui::i18n::neutral_path;
use ui::{Locale, PageContext, Theme, assets, pages};

/// Behaviour switches for page rendering.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SsrConfig {
    /// Serve the component gallery at `/_ui` and `/en/_ui` (development and CI only).
    pub ui_gallery: bool,
    /// Public origin used for canonical and `hreflang` links; `None` until the domain is decided.
    pub public_base_url: Option<String>,
}

impl SsrConfig {
    /// Reads `WEB_UI_GALLERY` and `WEB_PUBLIC_BASE_URL` through `lookup` (an environment reader
    /// in production, a map in tests).
    ///
    /// # Errors
    /// Returns an error when a value is present but malformed, so a typo fails at startup
    /// instead of silently disabling canonical links or exposing the gallery.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, String> {
        let ui_gallery = match lookup("WEB_UI_GALLERY").as_deref().map(str::trim) {
            None | Some("" | "0" | "false") => false,
            Some("1" | "true") => true,
            Some(other) => {
                return Err(format!(
                    "WEB_UI_GALLERY must be 1/true/0/false, got '{other}'"
                ));
            }
        };
        let public_base_url = match lookup("WEB_PUBLIC_BASE_URL").as_deref().map(str::trim) {
            None | Some("") => None,
            Some(url) => {
                let valid = (url.starts_with("https://") || url.starts_with("http://"))
                    && !url
                        .contains(|c: char| c.is_whitespace() || c == '"' || c == '<' || c == '>');
                if !valid {
                    return Err(
                        "WEB_PUBLIC_BASE_URL must be an http(s) origin without spaces".to_owned(),
                    );
                }
                Some(url.trim_end_matches('/').to_owned())
            }
        };
        Ok(Self {
            ui_gallery,
            public_base_url,
        })
    }
}

/// The router for every page and asset this crate serves.
#[must_use]
pub fn router(config: SsrConfig) -> Router {
    Router::new()
        .route("/", get(home))
        .route("/en", get(home))
        .route("/_ui", get(gallery))
        .route("/en/_ui", get(gallery))
        .route("/healthz", get(healthz))
        .route("/assets/{*path}", get(asset))
        .fallback(not_found)
        .layer(middleware::from_fn(security_headers))
        .layer(middleware::from_fn(log_requests))
        .with_state(Arc::new(config))
}

type Config = State<Arc<SsrConfig>>;

fn context(config: &SsrConfig, headers: &HeaderMap, uri: &Uri) -> PageContext {
    let path = uri.path();
    let cookie = headers.get(header::COOKIE).and_then(|v| v.to_str().ok());
    let mut ctx = PageContext::new(
        Locale::from_path(path),
        Theme::from_cookie_header(cookie),
        neutral_path(path),
    );
    ctx.base_url.clone_from(&config.public_base_url);
    ctx
}

/// HTML varies by the theme cookie, so shared caches must not reuse it across users.
fn html(status: StatusCode, body: String) -> Response {
    (
        status,
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
            (header::VARY, "Cookie"),
        ],
        body,
    )
        .into_response()
}

async fn home(State(config): Config, headers: HeaderMap, uri: Uri) -> Response {
    html(
        StatusCode::OK,
        pages::shell_preview(&context(&config, &headers, &uri)),
    )
}

async fn gallery(State(config): Config, headers: HeaderMap, uri: Uri) -> Response {
    let ctx = context(&config, &headers, &uri);
    if config.ui_gallery {
        html(StatusCode::OK, pages::gallery(&ctx))
    } else {
        html(StatusCode::NOT_FOUND, pages::not_found(&ctx))
    }
}

async fn not_found(State(config): Config, headers: HeaderMap, uri: Uri) -> Response {
    html(
        StatusCode::NOT_FOUND,
        pages::not_found(&context(&config, &headers, &uri)),
    )
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

/// `?v=<hash>` as emitted by `ui::assets::Asset::url`.
fn version_param(query: &str) -> Option<&str> {
    query.split('&').find_map(|pair| pair.strip_prefix("v="))
}

/// Embedded assets. A request whose `v` matches the current content hash is immutable for a
/// year; a stale or missing `v` gets a short lifetime so a deploy is picked up quickly.
async fn asset(Path(path): Path<String>, RawQuery(query): RawQuery) -> Response {
    let Some(asset) = assets::find(&path) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let current = query
        .as_deref()
        .and_then(version_param)
        .is_some_and(|v| v == asset.version());
    let cache = match (asset.versioned, current) {
        (true, true) => "public, max-age=31536000, immutable",
        (true, false) => "public, max-age=300",
        (false, _) => "public, max-age=604800",
    };
    (
        [
            (header::CONTENT_TYPE, asset.mime),
            (header::CACHE_CONTROL, cache),
        ],
        asset.bytes,
    )
        .into_response()
}

/// Everything is first-party: no third-party origins, no inline script or style, no `data:`
/// images, no framing. Loosening any of this needs a security review (docs/SECURITY_THREAT_MODEL.md).
const CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self'; \
font-src 'self'; connect-src 'self'; manifest-src 'self'; base-uri 'none'; \
form-action 'self'; frame-ancestors 'none'";

const SECURITY_HEADERS: [(&str, &str); 5] = [
    ("content-security-policy", CSP),
    ("x-content-type-options", "nosniff"),
    ("referrer-policy", "strict-origin-when-cross-origin"),
    ("x-frame-options", "DENY"),
    (
        "permissions-policy",
        "camera=(), microphone=(), geolocation=(), payment=()",
    ),
];

async fn security_headers(req: Request, next: Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    for (name, value) in SECURITY_HEADERS {
        headers.insert(
            HeaderName::from_static(name),
            HeaderValue::from_static(value),
        );
    }
    response
}

/// One line per request: method, path (never the query string, which may carry tokens), status
/// and latency. `/healthz` is logged at debug to keep probes out of the normal log.
async fn log_requests(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_owned();
    let started = Instant::now();
    let response = next.run(req).await;
    let status = response.status().as_u16();
    let elapsed_ms = started.elapsed().as_millis();
    if path == "/healthz" {
        tracing::debug!(%method, %path, status, elapsed_ms, "request");
    } else {
        tracing::info!(%method, %path, status, elapsed_ms, "request");
    }
    response
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use axum::body::{Body, to_bytes};
    use tower::ServiceExt;

    use super::*;

    async fn fetch(
        config: SsrConfig,
        uri: &str,
        cookie: Option<&str>,
    ) -> (StatusCode, HeaderMap, String) {
        let mut request = axum::http::Request::builder().uri(uri);
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        let response = router(config)
            .oneshot(request.body(Body::empty()).expect("valid request"))
            .await
            .expect("infallible");
        let (parts, body) = response.into_parts();
        let bytes = to_bytes(body, usize::MAX).await.expect("readable body");
        (
            parts.status,
            parts.headers,
            String::from_utf8_lossy(&bytes).into_owned(),
        )
    }

    fn header_value<'a>(headers: &'a HeaderMap, name: &str) -> &'a str {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
    }

    #[tokio::test]
    async fn home_renders_vietnamese_by_default_and_english_under_en() {
        let (status, headers, body) = fetch(SsrConfig::default(), "/", None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(header_value(&headers, "content-type").starts_with("text/html"));
        assert_eq!(header_value(&headers, "vary"), "Cookie");
        assert!(body.contains(r#"<html lang="vi">"#), "{body}");

        let (status, _, body) = fetch(SsrConfig::default(), "/en", None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            body.contains(r#"<html lang="en">"#) && body.contains("Under construction"),
            "{body}"
        );
    }

    #[tokio::test]
    async fn theme_cookie_is_rendered_on_the_html_element() {
        let (_, _, dark) = fetch(
            SsrConfig::default(),
            "/",
            Some("aao_theme=dark; aao_lang=vi"),
        )
        .await;
        assert!(
            dark.contains(r#"<html lang="vi" data-theme="dark">"#),
            "{dark}"
        );
        let (_, _, system) = fetch(SsrConfig::default(), "/", Some("aao_theme=bogus")).await;
        assert!(
            !system.contains("data-theme"),
            "an unknown value falls back to the OS preference: {system}"
        );
    }

    #[tokio::test]
    async fn gallery_is_hidden_unless_enabled() {
        let (status, _, body) = fetch(SsrConfig::default(), "/_ui", None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(
            !body.contains("btn-danger"),
            "must not leak the gallery: {body}"
        );

        let enabled = SsrConfig {
            ui_gallery: true,
            ..SsrConfig::default()
        };
        let (status, _, body) = fetch(enabled.clone(), "/_ui", None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains(r#"content="noindex, nofollow""#), "{body}");
        let (status, _, body) = fetch(enabled, "/en/_ui", None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains(r#"<html lang="en">"#), "{body}");
    }

    #[tokio::test]
    async fn unknown_paths_get_a_localized_404_page() {
        let (status, _, body) = fetch(SsrConfig::default(), "/en/nope", None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body.contains("Page not found"), "{body}");
        let (status, _, body) = fetch(SsrConfig::default(), "/khong-co", None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body.contains("Không tìm thấy trang"), "{body}");
    }

    #[tokio::test]
    async fn every_response_carries_security_headers() {
        for uri in [
            "/",
            "/en",
            "/nope",
            "/healthz",
            "/assets/app.css",
            "/assets/missing",
        ] {
            let (_, headers, _) = fetch(SsrConfig::default(), uri, None).await;
            let csp = header_value(&headers, "content-security-policy");
            assert!(
                csp.contains("default-src 'none'") && csp.contains("frame-ancestors 'none'"),
                "{uri}: {csp}"
            );
            assert!(
                !csp.contains("unsafe-inline")
                    && !csp.contains("unsafe-eval")
                    && !csp.contains("data:"),
                "{uri}: {csp}"
            );
            assert_eq!(
                header_value(&headers, "x-content-type-options"),
                "nosniff",
                "{uri}"
            );
            assert_eq!(header_value(&headers, "x-frame-options"), "DENY", "{uri}");
            assert!(
                !header_value(&headers, "referrer-policy").is_empty(),
                "{uri}"
            );
        }
    }

    #[tokio::test]
    async fn assets_are_served_with_cache_policy_by_version() {
        let css = ui::assets::APP_CSS;
        let (status, headers, body) = fetch(SsrConfig::default(), &css.url(), None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(header_value(&headers, "content-type").starts_with("text/css"));
        assert!(header_value(&headers, "cache-control").contains("immutable"));
        assert_eq!(body.len(), css.bytes.len());

        let (_, headers, _) = fetch(SsrConfig::default(), "/assets/app.css?v=stale", None).await;
        assert_eq!(
            header_value(&headers, "cache-control"),
            "public, max-age=300"
        );
        let (_, headers, _) = fetch(SsrConfig::default(), "/assets/app.css", None).await;
        assert_eq!(
            header_value(&headers, "cache-control"),
            "public, max-age=300"
        );

        let font = ui::assets::FONT_REGULAR;
        let (status, headers, _) = fetch(SsrConfig::default(), &font.url(), None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            header_value(&headers, "cache-control"),
            "public, max-age=604800"
        );
    }

    #[tokio::test]
    async fn only_embedded_assets_are_reachable() {
        for uri in [
            "/assets/nope.css",
            "/assets/fonts",
            "/assets/%2e%2e/Cargo.toml",
            "/assets/..%2fCargo.toml",
        ] {
            let (status, _, _) = fetch(SsrConfig::default(), uri, None).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
        }
    }

    #[tokio::test]
    async fn every_html_page_references_assets_that_exist() {
        let (_, _, body) = fetch(
            SsrConfig {
                ui_gallery: true,
                ..SsrConfig::default()
            },
            "/_ui",
            None,
        )
        .await;
        let urls: Vec<String> = body
            .split('"')
            .filter(|p| p.starts_with("/assets/"))
            .map(str::to_owned)
            .collect();
        assert!(
            urls.len() >= 4,
            "css, js, favicon and fonts expected: {urls:?}"
        );
        for url in urls {
            let (status, _, _) = fetch(SsrConfig::default(), &url, None).await;
            assert_eq!(
                status,
                StatusCode::OK,
                "{url} is referenced by the page but not served"
            );
        }
    }

    #[tokio::test]
    async fn healthz_is_plain_and_uncached() {
        let (status, headers, body) = fetch(SsrConfig::default(), "/healthz", None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "ok");
        assert_eq!(header_value(&headers, "cache-control"), "no-store");
    }

    #[tokio::test]
    async fn canonical_links_appear_only_with_a_public_base_url() {
        let (_, _, without) = fetch(SsrConfig::default(), "/en", None).await;
        assert!(!without.contains("rel=\"canonical\""));
        let config = SsrConfig {
            public_base_url: Some("https://example.test".into()),
            ..SsrConfig::default()
        };
        let (_, _, with) = fetch(config, "/en", None).await;
        assert!(
            with.contains(r#"rel="canonical" href="https://example.test/en""#),
            "{with}"
        );
    }

    fn lookup(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        move |name| map.get(name).cloned()
    }

    #[test]
    fn config_defaults_are_safe() {
        assert_eq!(
            SsrConfig::from_lookup(lookup(&[])),
            Ok(SsrConfig::default())
        );
        assert_eq!(
            SsrConfig::from_lookup(lookup(&[("WEB_UI_GALLERY", "0")])),
            Ok(SsrConfig::default())
        );
    }

    #[test]
    fn config_parses_and_normalizes_values() {
        let config = SsrConfig::from_lookup(lookup(&[
            ("WEB_UI_GALLERY", "true"),
            ("WEB_PUBLIC_BASE_URL", " https://example.test/ "),
        ]))
        .expect("valid configuration");
        assert!(config.ui_gallery);
        assert_eq!(
            config.public_base_url.as_deref(),
            Some("https://example.test")
        );
    }

    #[test]
    fn config_rejects_malformed_values() {
        assert!(SsrConfig::from_lookup(lookup(&[("WEB_UI_GALLERY", "yes")])).is_err());
        assert!(
            SsrConfig::from_lookup(lookup(&[("WEB_PUBLIC_BASE_URL", "example.test")])).is_err()
        );
        assert!(SsrConfig::from_lookup(lookup(&[("WEB_PUBLIC_BASE_URL", "https://a b")])).is_err());
    }
}

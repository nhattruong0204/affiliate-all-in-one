//! Security response headers (threat model §5, AAO-091).
//!
//! The policy is **first-party only** by default: no third-party origins, no inline script or
//! style, no `data:` images, no framing. Every relaxation is an explicit, configuration-derived
//! decision in [`content_security_policy`]; adding a merchant image host or another provider means
//! editing this file, extending its tests and the Playwright CSP check, and a security review.
//!
//! Deliberately absent: `Cross-Origin-Embedder-Policy` (`require-corp` would block the Turnstile
//! iframe and nothing here needs cross-origin isolation) and cookies (none are set server-side).

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{HeaderName, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;

/// Cloudflare's challenge origin, the only third-party origin ever allowed, and only when
/// Turnstile is configured.
///
/// UNVERIFIED against Cloudflare's official CSP page (not reachable when this was written): the
/// `script-src` and `frame-src` entries are the long-standing minimum, `connect-src`/`style-src`
/// requirements are not confirmed. Turnstile is off by default; AAO-100 must verify this against
/// the official documentation and the real widget in the Playwright CSP test before enabling it.
const TURNSTILE_ORIGIN: &str = "https://challenges.cloudflare.com";

/// `max-age` two years, as required for Mozilla Observatory A+ (≥ 6 months) and HSTS preload.
/// `includeSubDomains; preload` is only safe once the apex domain is decided and every subdomain
/// is HTTPS-only (owner decision D-4); the list submission itself is a separate, manual step.
const HSTS: &str = "max-age=63072000; includeSubDomains; preload";

/// Which optional parts of the policy are active.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HeaderPolicy {
    /// Send `Strict-Transport-Security` (production only: browsers ignore it over plain HTTP and
    /// a development server must not teach browsers to refuse `http://localhost`).
    pub hsts: bool,
    /// Allow the Turnstile script and frame origins.
    pub turnstile: bool,
}

/// The `Content-Security-Policy` for `policy`.
pub fn content_security_policy(policy: HeaderPolicy) -> String {
    let (script_extra, frame) = if policy.turnstile {
        (
            format!(" {TURNSTILE_ORIGIN}"),
            format!("; frame-src {TURNSTILE_ORIGIN}"),
        )
    } else {
        (String::new(), String::new())
    };
    format!(
        "default-src 'none'; script-src 'self'{script_extra}; style-src 'self'; img-src 'self'; \
font-src 'self'; connect-src 'self'; manifest-src 'self'; object-src 'none'; \
base-uri 'none'; form-action 'self'; frame-ancestors 'none'{frame}"
    )
}

/// The headers added to every response, computed once at startup.
#[derive(Clone, Debug)]
pub struct SecurityHeaders {
    headers: Vec<(HeaderName, HeaderValue)>,
}

impl SecurityHeaders {
    pub fn new(policy: HeaderPolicy) -> Self {
        let mut headers = vec![
            fixed("content-security-policy", &content_security_policy(policy)),
            fixed("x-content-type-options", "nosniff"),
            fixed("referrer-policy", "strict-origin-when-cross-origin"),
            fixed("x-frame-options", "DENY"),
            fixed(
                "permissions-policy",
                "camera=(), microphone=(), geolocation=(), payment=(), usb=(), browsing-topics=()",
            ),
            fixed("cross-origin-opener-policy", "same-origin"),
            fixed("cross-origin-resource-policy", "same-origin"),
            fixed("x-permitted-cross-domain-policies", "none"),
        ];
        if policy.hsts {
            headers.push(fixed("strict-transport-security", HSTS));
        }
        Self { headers }
    }
}

/// Header names and values here are compile-time constants or built from them, so they are always
/// valid; the fallback keeps this infallible without `expect`.
fn fixed(name: &'static str, value: &str) -> (HeaderName, HeaderValue) {
    (
        HeaderName::from_static(name),
        HeaderValue::from_str(value).unwrap_or_else(|_| HeaderValue::from_static("")),
    )
}

/// Sets the security headers on every response, overriding anything a handler set.
pub async fn security_headers(
    State(security): State<Arc<SecurityHeaders>>,
    req: Request,
    next: Next,
) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    for (name, value) in &security.headers {
        headers.insert(name.clone(), value.clone());
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    fn directives(csp: &str) -> Vec<&str> {
        csp.split(';').map(str::trim).collect()
    }

    #[test]
    fn the_default_policy_is_first_party_only() {
        let csp = content_security_policy(HeaderPolicy::default());
        for expected in [
            "default-src 'none'",
            "script-src 'self'",
            "style-src 'self'",
            "img-src 'self'",
            "font-src 'self'",
            "connect-src 'self'",
            "object-src 'none'",
            "base-uri 'none'",
            "form-action 'self'",
            "frame-ancestors 'none'",
        ] {
            assert!(
                directives(&csp).contains(&expected),
                "missing {expected}: {csp}"
            );
        }
        for forbidden in [
            "unsafe-inline",
            "unsafe-eval",
            "data:",
            "http:",
            "*",
            "cloudflare",
            "frame-src",
        ] {
            assert!(!csp.contains(forbidden), "{forbidden} in {csp}");
        }
    }

    #[test]
    fn turnstile_widens_only_script_and_frame_sources() {
        let on = content_security_policy(HeaderPolicy {
            hsts: false,
            turnstile: true,
        });
        let off = content_security_policy(HeaderPolicy::default());
        assert!(
            directives(&on).contains(&"script-src 'self' https://challenges.cloudflare.com"),
            "{on}"
        );
        assert!(
            directives(&on).contains(&"frame-src https://challenges.cloudflare.com"),
            "{on}"
        );
        // Everything else is identical, and no wildcard or inline escape hatch appears.
        let strip = |csp: &str| {
            directives(csp)
                .into_iter()
                .filter(|d| !d.starts_with("script-src") && !d.starts_with("frame-src"))
                .map(str::to_owned)
                .collect::<Vec<_>>()
        };
        assert_eq!(strip(&on), strip(&off));
        assert!(!on.contains("unsafe-") && !on.contains('*'), "{on}");
    }

    #[test]
    fn hsts_is_sent_only_when_asked_for() {
        let value = |policy| {
            SecurityHeaders::new(policy)
                .headers
                .into_iter()
                .find(|(name, _)| name == "strict-transport-security")
                .map(|(_, v)| v.to_str().expect("ascii").to_owned())
        };
        assert_eq!(value(HeaderPolicy::default()), None);
        let hsts = value(HeaderPolicy {
            hsts: true,
            turnstile: false,
        })
        .expect("sent");
        let max_age: u64 = hsts
            .strip_prefix("max-age=")
            .and_then(|rest| rest.split(';').next())
            .and_then(|n| n.parse().ok())
            .expect("max-age");
        assert!(
            max_age >= 15_768_000,
            "Observatory A+ needs six months: {hsts}"
        );
        assert!(
            hsts.contains("includeSubDomains") && hsts.contains("preload"),
            "{hsts}"
        );
    }

    #[test]
    fn every_header_value_is_valid_and_present() {
        for hsts in [false, true] {
            for turnstile in [false, true] {
                let set = SecurityHeaders::new(HeaderPolicy { hsts, turnstile });
                for (name, value) in &set.headers {
                    assert!(
                        !value.as_bytes().is_empty(),
                        "{name} fell back to the empty value"
                    );
                }
                let names: Vec<&str> = set.headers.iter().map(|(n, _)| n.as_str()).collect();
                for required in [
                    "content-security-policy",
                    "x-content-type-options",
                    "referrer-policy",
                    "x-frame-options",
                    "permissions-policy",
                    "cross-origin-opener-policy",
                    "cross-origin-resource-policy",
                ] {
                    assert!(names.contains(&required), "{required} missing: {names:?}");
                }
            }
        }
    }
}

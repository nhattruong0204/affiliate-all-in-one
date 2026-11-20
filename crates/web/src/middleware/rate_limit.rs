//! Per-client-IP rate limiting for public endpoints (AAO-091).
//!
//! Over-budget requests get `429` problem+json (`code: rate_limited`) with `Retry-After`. Client
//! addresses are never logged (they are personal data); the limiter key lives only in memory.
//!
//! **Which address is "the client".** Behind Caddy the TCP peer is always the proxy, so with
//! `TRUST_FORWARDED_FOR=true` the *right-most* `X-Forwarded-For` entry is used: that is the one our
//! own proxy appended, so a client cannot choose it by sending the header itself. With the flag
//! off (the default, for a directly reachable server) the header is ignored entirely. IPv6
//! clients are keyed by their /64, because a single subscriber controls a whole /64 and could
//! otherwise mint unlimited addresses.

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

use app::error::AppError;
use app::ratelimit::{Decision, RateLimiter};
use axum::extract::{ConnectInfo, Request, State};
use axum::http::HeaderMap;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::problem::ApiError;

/// Limiter plus the rule for finding the client address.
#[derive(Clone, Debug)]
pub struct Limit {
    pub limiter: Arc<RateLimiter>,
    pub trust_forwarded_for: bool,
}

/// The client address per the trust rule above, or `None` if it cannot be determined.
pub fn client_ip(
    trust_forwarded_for: bool,
    headers: &HeaderMap,
    peer: Option<IpAddr>,
) -> Option<IpAddr> {
    if trust_forwarded_for {
        let forwarded = headers
            .get_all("x-forwarded-for")
            .iter()
            .filter_map(|value| value.to_str().ok())
            .flat_map(|value| value.split(','))
            .map(str::trim)
            .rfind(|entry| !entry.is_empty())
            .and_then(|entry| entry.parse::<IpAddr>().ok());
        if forwarded.is_some() {
            return forwarded;
        }
    }
    peer
}

/// The bucket key: IPv4 as is, IPv4-mapped IPv6 as IPv4, other IPv6 by /64.
pub fn limit_key(ip: Option<IpAddr>) -> String {
    match ip {
        None => "unknown".to_owned(),
        Some(IpAddr::V4(v4)) => v4.to_string(),
        Some(IpAddr::V6(v6)) => v6.to_ipv4_mapped().map_or_else(
            || {
                let s = v6.segments();
                format!("{:x}:{:x}:{:x}:{:x}::/64", s[0], s[1], s[2], s[3])
            },
            |v4| v4.to_string(),
        ),
    }
}

/// Whole seconds for `Retry-After`: rounded up, never zero.
fn retry_after_secs(wait: Duration) -> u64 {
    wait.as_secs()
        .saturating_add(u64::from(wait.subsec_nanos() > 0))
        .max(1)
}

pub async fn rate_limit(State(limit): State<Limit>, req: Request, next: Next) -> Response {
    let peer = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|connect| connect.0.ip());
    let key = limit_key(client_ip(limit.trust_forwarded_for, req.headers(), peer));
    match limit.limiter.check(&key, Instant::now()) {
        Decision::Allowed => next.run(req).await,
        Decision::Limited { retry_after } => ApiError(AppError::RateLimited {
            retry_after_secs: retry_after_secs(retry_after),
        })
        .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    fn headers(values: &[&str]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for value in values {
            map.append(
                "x-forwarded-for",
                HeaderValue::from_str(value).expect("valid"),
            );
        }
        map
    }

    fn ip(s: &str) -> IpAddr {
        s.parse().expect("ip")
    }

    #[test]
    fn untrusted_forwarded_for_is_ignored() {
        let h = headers(&["6.6.6.6"]);
        assert_eq!(
            client_ip(false, &h, Some(ip("10.0.0.2"))),
            Some(ip("10.0.0.2"))
        );
        assert_eq!(client_ip(false, &h, None), None);
    }

    #[test]
    fn trusted_forwarded_for_uses_the_entry_our_proxy_appended() {
        // A client that sends its own header cannot pick the right-most entry.
        let h = headers(&["6.6.6.6, 198.51.100.7"]);
        assert_eq!(
            client_ip(true, &h, Some(ip("10.0.0.2"))),
            Some(ip("198.51.100.7"))
        );
        let h = headers(&["6.6.6.6", "198.51.100.7"]);
        assert_eq!(
            client_ip(true, &h, Some(ip("10.0.0.2"))),
            Some(ip("198.51.100.7"))
        );
        let h = headers(&["2001:db8::1"]);
        assert_eq!(client_ip(true, &h, None), Some(ip("2001:db8::1")));
    }

    #[test]
    fn missing_or_garbage_forwarded_for_falls_back_to_the_peer() {
        for h in [
            headers(&[]),
            headers(&["not-an-ip"]),
            headers(&[" , "]),
            headers(&["1.2.3.4:80"]),
        ] {
            assert_eq!(
                client_ip(true, &h, Some(ip("10.0.0.2"))),
                Some(ip("10.0.0.2"))
            );
        }
    }

    #[test]
    fn keys_collapse_ipv6_subscribers_and_mapped_addresses() {
        assert_eq!(limit_key(None), "unknown");
        assert_eq!(limit_key(Some(ip("203.0.113.9"))), "203.0.113.9");
        assert_eq!(limit_key(Some(ip("::ffff:203.0.113.9"))), "203.0.113.9");
        let a = limit_key(Some(ip("2001:db8:1:2:aaaa:bbbb:cccc:dddd")));
        let b = limit_key(Some(ip("2001:db8:1:2:1::1")));
        let other = limit_key(Some(ip("2001:db8:1:3::1")));
        assert_eq!(a, b, "same /64 shares a bucket");
        assert_ne!(a, other);
        assert_eq!(a, "2001:db8:1:2::/64");
    }

    #[test]
    fn retry_after_rounds_up_and_is_never_zero() {
        assert_eq!(retry_after_secs(Duration::ZERO), 1);
        assert_eq!(retry_after_secs(Duration::from_millis(1)), 1);
        assert_eq!(retry_after_secs(Duration::from_secs(6)), 6);
        assert_eq!(retry_after_secs(Duration::from_millis(8_572)), 9);
    }
}

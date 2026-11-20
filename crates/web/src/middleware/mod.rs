//! Tower/axum middleware shared by every route.

use std::sync::Arc;

use app::config::{AppEnv, Config};
use app::ratelimit::{Quota, RateLimiter};

pub mod headers;
pub mod observe;
pub mod rate_limit;

/// Everything `ssr::router` needs to harden the public surface (AAO-091).
#[derive(Clone, Debug)]
pub struct Hardening {
    pub headers: Arc<headers::SecurityHeaders>,
    /// Budget for page requests (assets and probes are exempt).
    pub pages: rate_limit::Limit,
}

impl Hardening {
    pub fn new(
        policy: headers::HeaderPolicy,
        pages_per_minute: u32,
        trust_forwarded_for: bool,
    ) -> Self {
        Self {
            headers: Arc::new(headers::SecurityHeaders::new(policy)),
            pages: rate_limit::Limit {
                limiter: Arc::new(RateLimiter::new(Quota::per_minute(pages_per_minute))),
                trust_forwarded_for,
            },
        }
    }

    /// The hardening implied by the validated configuration: HSTS in production, Turnstile origins
    /// only when Turnstile is configured, limits from `RATE_LIMIT_*`.
    pub fn from_config(config: &Config) -> Self {
        Self::new(
            headers::HeaderPolicy {
                hsts: config.env == AppEnv::Production,
                turnstile: config.turnstile.is_some(),
            },
            config.rate_limits.pages_per_minute,
            config.trust_forwarded_for,
        )
    }
}

impl Default for Hardening {
    /// Development defaults: no HSTS, no Turnstile, the documented page limit, header untrusted.
    fn default() -> Self {
        Self::new(headers::HeaderPolicy::default(), 120, false)
    }
}

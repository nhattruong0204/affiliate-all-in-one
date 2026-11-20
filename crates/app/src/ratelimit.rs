//! In-process token-bucket rate limiting for public endpoints.
//!
//! Why in-process: V0.1 runs as a single web instance, so one process sees all traffic; a
//! PostgreSQL-backed bucket (`rate_limit_buckets`, `DATABASE_DESIGN` §4) adds a round trip per
//! request and arrives with the first stateful consumer. Consequences, accepted for V0.1:
//! counters reset when the process restarts and are not shared between instances (revisit before
//! scaling out or before login/withdrawal limits, which must survive restarts: V0.5).
//!
//! Integer arithmetic only (`float_arithmetic` is denied in this crate). Tokens are tracked in
//! "token-milliseconds" so the refill of `requests` tokens per `per` needs no division until a
//! retry hint is computed.
//!
//! Memory is bounded: at most `max_keys` buckets. When full, buckets that have refilled
//! completely (idle for a full window) are swept; if the table is still full the request is
//! **allowed untracked** rather than evicting a real client's bucket. That fails open for a
//! flood of distinct keys, which Caddy and the OS connection limits must absorb; the alternative
//! would let an attacker with many addresses lock legitimate users out.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

const DEFAULT_MAX_KEYS: usize = 10_000;

/// `requests` allowed per `per`, as a steady rate with a burst of `requests`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Quota {
    requests: u32,
    per_ms: u64,
}

impl Quota {
    /// A quota of at least one request per at-least-one millisecond.
    pub fn new(requests: u32, per: Duration) -> Self {
        Self {
            requests: requests.max(1),
            per_ms: u64::try_from(per.as_millis()).unwrap_or(u64::MAX).max(1),
        }
    }

    pub fn per_minute(requests: u32) -> Self {
        Self::new(requests, Duration::from_mins(1))
    }

    fn capacity(self) -> u64 {
        u64::from(self.requests).saturating_mul(self.per_ms)
    }
}

/// Outcome of one [`RateLimiter::check`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Allowed,
    /// Over budget; the earliest moment a retry can succeed is `retry_after` from now.
    Limited {
        retry_after: Duration,
    },
}

#[derive(Debug)]
struct Bucket {
    /// Available tokens × `per_ms`.
    scaled: u64,
    /// Last instant whose refill has been accounted for (advanced by whole milliseconds only, so
    /// requests arriving faster than 1 ms apart cannot starve the refill).
    updated: Instant,
}

/// A keyed token-bucket limiter. Cheap to share behind an `Arc`.
#[derive(Debug)]
pub struct RateLimiter {
    quota: Quota,
    max_keys: usize,
    buckets: Mutex<HashMap<String, Bucket>>,
}

impl RateLimiter {
    pub fn new(quota: Quota) -> Self {
        Self::with_max_keys(quota, DEFAULT_MAX_KEYS)
    }

    pub fn with_max_keys(quota: Quota, max_keys: usize) -> Self {
        Self {
            quota,
            max_keys: max_keys.max(1),
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// Takes one token for `key` at time `now` (injected so tests need no sleeping).
    pub fn check(&self, key: &str, now: Instant) -> Decision {
        let requests = u64::from(self.quota.requests);
        let cost = self.quota.per_ms;
        let capacity = self.quota.capacity();
        let mut buckets = self.buckets.lock().unwrap_or_else(PoisonError::into_inner);

        if !buckets.contains_key(key) && buckets.len() >= self.max_keys {
            buckets.retain(|_, bucket| refilled(bucket, now, requests, capacity) < capacity);
            if buckets.len() >= self.max_keys {
                return Decision::Allowed;
            }
        }

        let bucket = buckets.entry(key.to_owned()).or_insert(Bucket {
            scaled: capacity,
            updated: now,
        });
        let elapsed_ms = elapsed_ms(bucket.updated, now);
        bucket.scaled = refilled(bucket, now, requests, capacity);
        if elapsed_ms > 0 {
            bucket.updated += Duration::from_millis(elapsed_ms);
        }

        if bucket.scaled >= cost {
            bucket.scaled -= cost;
            Decision::Allowed
        } else {
            let missing = cost - bucket.scaled;
            Decision::Limited {
                retry_after: Duration::from_millis(missing.div_ceil(requests)),
            }
        }
    }

    /// Number of tracked keys (for tests and diagnostics).
    pub fn tracked_keys(&self) -> usize {
        self.buckets
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }
}

fn elapsed_ms(from: Instant, to: Instant) -> u64 {
    u64::try_from(to.saturating_duration_since(from).as_millis()).unwrap_or(u64::MAX)
}

fn refilled(bucket: &Bucket, now: Instant, requests: u64, capacity: u64) -> u64 {
    bucket
        .scaled
        .saturating_add(elapsed_ms(bucket.updated, now).saturating_mul(requests))
        .min(capacity)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    #[test]
    fn allows_a_burst_up_to_the_limit_then_limits() {
        let limiter = RateLimiter::new(Quota::per_minute(10));
        let t0 = Instant::now();
        for i in 0..10 {
            assert_eq!(limiter.check("ip", t0), Decision::Allowed, "request {i}");
        }
        // 10 per minute = one token per 6 s.
        assert_eq!(
            limiter.check("ip", t0),
            Decision::Limited {
                retry_after: secs(6)
            }
        );
    }

    #[test]
    fn tokens_refill_at_the_steady_rate() {
        let limiter = RateLimiter::new(Quota::per_minute(10));
        let t0 = Instant::now();
        for _ in 0..10 {
            limiter.check("ip", t0);
        }
        assert!(matches!(
            limiter.check("ip", t0 + secs(5)),
            Decision::Limited { .. }
        ));
        assert_eq!(limiter.check("ip", t0 + secs(6)), Decision::Allowed);
        // The retry hint shrinks as time passes.
        assert_eq!(
            limiter.check("ip", t0 + secs(8)),
            Decision::Limited {
                retry_after: secs(4)
            }
        );
    }

    #[test]
    fn idle_time_never_banks_more_than_the_burst() {
        let limiter = RateLimiter::new(Quota::per_minute(3));
        let t0 = Instant::now();
        let later = t0 + secs(3_600);
        for _ in 0..3 {
            assert_eq!(limiter.check("ip", later), Decision::Allowed);
        }
        assert!(matches!(
            limiter.check("ip", later),
            Decision::Limited { .. }
        ));
    }

    #[test]
    fn keys_are_independent() {
        let limiter = RateLimiter::new(Quota::per_minute(1));
        let t0 = Instant::now();
        assert_eq!(limiter.check("a", t0), Decision::Allowed);
        assert!(matches!(limiter.check("a", t0), Decision::Limited { .. }));
        assert_eq!(limiter.check("b", t0), Decision::Allowed);
    }

    #[test]
    fn requests_faster_than_a_millisecond_cannot_starve_the_refill() {
        let limiter = RateLimiter::new(Quota::per_minute(60)); // one token per second
        let t0 = Instant::now();
        for _ in 0..60 {
            limiter.check("ip", t0);
        }
        // Hammer every 0.5 ms for 1.2 s: the fractional elapsed time must still accumulate.
        let mut allowed = 0;
        for step in 1..=2_400_u64 {
            if limiter.check("ip", t0 + Duration::from_micros(step * 500)) == Decision::Allowed {
                allowed += 1;
            }
        }
        assert_eq!(allowed, 1, "exactly the one token that refilled in 1.2 s");
    }

    #[test]
    fn retry_hint_rounds_up_so_an_obedient_client_succeeds() {
        // 7 per minute: one token per 8571.43 ms → hint 8572 ms.
        let limiter = RateLimiter::new(Quota::per_minute(7));
        let t0 = Instant::now();
        for _ in 0..7 {
            limiter.check("ip", t0);
        }
        let Decision::Limited { retry_after } = limiter.check("ip", t0) else {
            panic!("must be limited");
        };
        assert_eq!(retry_after, Duration::from_millis(8_572));
        assert_eq!(limiter.check("ip", t0 + retry_after), Decision::Allowed);
    }

    #[test]
    fn the_table_is_bounded_and_fails_open_when_flooded() {
        let limiter = RateLimiter::with_max_keys(Quota::per_minute(1), 3);
        let t0 = Instant::now();
        for key in ["a", "b", "c"] {
            assert_eq!(limiter.check(key, t0), Decision::Allowed);
        }
        assert_eq!(limiter.tracked_keys(), 3);
        // Table full of active buckets: a new key is allowed but not tracked.
        assert_eq!(limiter.check("d", t0), Decision::Allowed);
        assert_eq!(limiter.check("d", t0), Decision::Allowed);
        assert_eq!(limiter.tracked_keys(), 3);
        // Existing clients keep being limited.
        assert!(matches!(limiter.check("a", t0), Decision::Limited { .. }));
    }

    #[test]
    fn idle_buckets_are_swept_to_make_room() {
        let limiter = RateLimiter::with_max_keys(Quota::per_minute(1), 2);
        let t0 = Instant::now();
        limiter.check("a", t0);
        limiter.check("b", t0);
        let later = t0 + secs(61);
        assert_eq!(limiter.check("c", later), Decision::Allowed);
        assert_eq!(
            limiter.tracked_keys(),
            1,
            "a and b had fully refilled and were swept"
        );
        assert!(matches!(
            limiter.check("c", later),
            Decision::Limited { .. }
        ));
    }

    #[test]
    fn degenerate_quotas_are_clamped() {
        let limiter = RateLimiter::new(Quota::new(0, Duration::ZERO));
        let t0 = Instant::now();
        assert_eq!(limiter.check("ip", t0), Decision::Allowed);
        assert!(matches!(limiter.check("ip", t0), Decision::Limited { .. }));
    }
}

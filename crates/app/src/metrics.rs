//! Prometheus metrics for the RED method (rate, errors, duration) per route.
//!
//! Hand-rolled instead of the `prometheus` crate: three series families are all V0.1 needs, it
//! keeps the dependency tree (and supply-chain surface) unchanged, and integer microseconds avoid
//! floating point (`float_arithmetic` is denied in this crate). Revisit if the metric set grows
//! (AAO-092 adds infrastructure dashboards, not new series).
//!
//! Series, all labelled with the *matched route pattern* (`/assets/{*path}`), never the raw
//! path, so label cardinality is bounded by the router:
//! * `http_requests_total{method,route,status}`: rate; errors are `status=~"5.."`.
//! * `http_request_duration_seconds{method,route}`: duration histogram.
//! * `aao_build_info{version}`: constant 1.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

/// Content type of the text exposition format.
pub const CONTENT_TYPE: &str = "text/plain; version=0.0.4; charset=utf-8";

/// Upper bounds (µs) with their `le` labels: 5 ms … 10 s.
const BUCKETS: [(u64, &str); 11] = [
    (5_000, "0.005"),
    (10_000, "0.01"),
    (25_000, "0.025"),
    (50_000, "0.05"),
    (100_000, "0.1"),
    (250_000, "0.25"),
    (500_000, "0.5"),
    (1_000_000, "1"),
    (2_500_000, "2.5"),
    (5_000_000, "5"),
    (10_000_000, "10"),
];

const KNOWN_METHODS: [&str; 9] = [
    "GET", "HEAD", "POST", "PUT", "PATCH", "DELETE", "OPTIONS", "CONNECT", "TRACE",
];

#[derive(Debug, Default)]
struct Histogram {
    /// Observations per bucket, non-cumulative; values above the last bound are only in `count`.
    buckets: [u64; BUCKETS.len()],
    count: u64,
    sum_micros: u64,
}

#[derive(Debug, Default)]
struct Inner {
    requests: BTreeMap<(String, String, u16), u64>,
    durations: BTreeMap<(String, String), Histogram>,
}

/// Thread-safe registry. Cheap to share behind an `Arc`.
#[derive(Debug, Default)]
pub struct Metrics {
    inner: Mutex<Inner>,
}

impl Metrics {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records one finished request. Unknown HTTP methods collapse into `OTHER` so a client
    /// cannot create unbounded label values.
    pub fn observe_request(&self, method: &str, route: &str, status: u16, elapsed: Duration) {
        let method = if KNOWN_METHODS.contains(&method) {
            method
        } else {
            "OTHER"
        };
        let micros = u64::try_from(elapsed.as_micros()).unwrap_or(u64::MAX);
        let mut inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let count = inner
            .requests
            .entry((method.to_owned(), route.to_owned(), status))
            .or_insert(0);
        *count = count.saturating_add(1);
        let histogram = inner
            .durations
            .entry((method.to_owned(), route.to_owned()))
            .or_default();
        if let Some(index) = BUCKETS.iter().position(|(bound, _)| micros <= *bound) {
            histogram.buckets[index] = histogram.buckets[index].saturating_add(1);
        }
        histogram.count = histogram.count.saturating_add(1);
        histogram.sum_micros = histogram.sum_micros.saturating_add(micros);
    }

    /// The Prometheus text exposition of every series.
    pub fn render(&self) -> String {
        let inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let mut out = String::new();

        out.push_str("# HELP http_requests_total HTTP requests handled.\n");
        out.push_str("# TYPE http_requests_total counter\n");
        for ((method, route, status), count) in &inner.requests {
            let _ = writeln!(
                out,
                "http_requests_total{{method=\"{}\",route=\"{}\",status=\"{status}\"}} {count}",
                escape(method),
                escape(route)
            );
        }

        out.push_str("# HELP http_request_duration_seconds HTTP request latency.\n");
        out.push_str("# TYPE http_request_duration_seconds histogram\n");
        for ((method, route), histogram) in &inner.durations {
            let labels = format!("method=\"{}\",route=\"{}\"", escape(method), escape(route));
            let mut cumulative = 0_u64;
            for ((_, le), bucket) in BUCKETS.iter().zip(histogram.buckets) {
                cumulative += bucket;
                let _ = writeln!(
                    out,
                    "http_request_duration_seconds_bucket{{{labels},le=\"{le}\"}} {cumulative}"
                );
            }
            let _ = writeln!(
                out,
                "http_request_duration_seconds_bucket{{{labels},le=\"+Inf\"}} {}",
                histogram.count
            );
            let _ = writeln!(
                out,
                "http_request_duration_seconds_sum{{{labels}}} {}.{:06}",
                histogram.sum_micros / 1_000_000,
                histogram.sum_micros % 1_000_000
            );
            let _ = writeln!(
                out,
                "http_request_duration_seconds_count{{{labels}}} {}",
                histogram.count
            );
        }

        out.push_str("# HELP aao_build_info Build metadata; the value is always 1.\n");
        out.push_str("# TYPE aao_build_info gauge\n");
        let _ = writeln!(
            out,
            "aao_build_info{{version=\"{}\"}} 1",
            env!("CARGO_PKG_VERSION")
        );
        out
    }
}

/// Escapes a label value per the exposition format.
fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line<'a>(text: &'a str, prefix: &str) -> &'a str {
        text.lines()
            .find(|l| l.starts_with(prefix))
            .unwrap_or_else(|| panic!("no line starting with {prefix} in:\n{text}"))
    }

    #[test]
    fn counts_requests_per_method_route_and_status() {
        let metrics = Metrics::new();
        for _ in 0..3 {
            metrics.observe_request("GET", "/", 200, Duration::from_millis(1));
        }
        metrics.observe_request("GET", "/", 500, Duration::from_millis(1));
        metrics.observe_request("POST", "/api/v1/links", 201, Duration::from_millis(1));
        let text = metrics.render();
        assert!(
            text.contains(r#"http_requests_total{method="GET",route="/",status="200"} 3"#),
            "{text}"
        );
        assert!(
            text.contains(r#"http_requests_total{method="GET",route="/",status="500"} 1"#),
            "{text}"
        );
        assert!(
            text.contains(
                r#"http_requests_total{method="POST",route="/api/v1/links",status="201"} 1"#
            ),
            "{text}"
        );
    }

    #[test]
    fn histogram_buckets_are_cumulative_and_sums_are_exact() {
        let metrics = Metrics::new();
        metrics.observe_request("GET", "/", 200, Duration::from_micros(4_000)); // ≤ 5 ms
        metrics.observe_request("GET", "/", 200, Duration::from_micros(80_000)); // ≤ 100 ms
        metrics.observe_request("GET", "/", 200, Duration::from_secs(30)); // beyond 10 s
        let text = metrics.render();
        let labels = r#"method="GET",route="/""#;
        let bucket = |le: &str| {
            line(
                &text,
                &format!("http_request_duration_seconds_bucket{{{labels},le=\"{le}\"}}"),
            )
            .rsplit(' ')
            .next()
            .expect("value")
            .to_owned()
        };
        assert_eq!(bucket("0.005"), "1");
        assert_eq!(bucket("0.05"), "1");
        assert_eq!(bucket("0.1"), "2");
        assert_eq!(bucket("10"), "2");
        assert_eq!(bucket("+Inf"), "3");
        assert!(
            text.contains(&format!(
                "http_request_duration_seconds_sum{{{labels}}} 30.084000"
            )),
            "{text}"
        );
        assert!(
            text.contains(&format!(
                "http_request_duration_seconds_count{{{labels}}} 3"
            )),
            "{text}"
        );
    }

    #[test]
    fn unknown_methods_and_hostile_labels_cannot_break_the_format() {
        let metrics = Metrics::new();
        metrics.observe_request("BREW", "/", 418, Duration::ZERO);
        metrics.observe_request("GET", "a\"b\\c\nd", 200, Duration::ZERO);
        let text = metrics.render();
        assert!(
            text.contains(r#"method="OTHER""#) && !text.contains("BREW"),
            "{text}"
        );
        assert!(text.contains(r#"route="a\"b\\c\nd""#), "{text}");
        assert!(
            text.lines().all(|l| !l.is_empty()),
            "no blank/broken lines: {text}"
        );
    }

    #[test]
    fn empty_registry_still_exposes_build_info() {
        let text = Metrics::new().render();
        assert!(text.contains("# TYPE http_requests_total counter"));
        assert!(text.contains(r#"aao_build_info{version=""#), "{text}");
    }
}

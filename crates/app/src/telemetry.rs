//! Structured logging: JSON lines with a redaction layer, trace ids, and subscriber setup.
//!
//! Redaction works on **field names**: any field whose lowercase name contains one of
//! [`REDACTED_FIELDS`] has its value replaced by [`REDACTED`], whatever its type, both on events
//! and on span fields. It cannot see PII interpolated into a message string, so log values as
//! fields (`tracing::info!(email = %e, "…")` is safe, `tracing::info!("user {e}")` is not).
//!
//! OTLP trace export is deliberately not part of V0.1 (issue scope: JSON logs and Prometheus
//! metrics suffice). The `trace_id` is already W3C-compatible so an exporter can be added later
//! without touching call sites.

use std::fmt::{self, Write as _};
use std::io::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Record};
use tracing::{Event, Id, Subscriber};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Layer;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;

use crate::config::{Config, LogFormat};

/// Field-name fragments whose values are never logged (AAO-005 deny-list).
pub const REDACTED_FIELDS: [&str; 5] = ["email", "phone", "account_number", "token", "password"];

/// Replacement for a redacted value.
pub const REDACTED: &str = "[REDACTED]";

fn is_sensitive(field_name: &str) -> bool {
    let lower = field_name.to_ascii_lowercase();
    REDACTED_FIELDS.iter().any(|denied| lower.contains(denied))
}

/// A 128-bit W3C trace id: 32 lowercase hex characters, never all zeros.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceId(String);

static FALLBACK_COUNTER: AtomicU64 = AtomicU64::new(1);

impl TraceId {
    /// A new random id. If the OS random source fails (it should not) the id degrades to a
    /// time-and-counter value: still unique per process, only less unpredictable.
    pub fn generate() -> Self {
        let mut bytes = [0_u8; 16];
        if getrandom::fill(&mut bytes).is_err() || bytes == [0_u8; 16] {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let counter = FALLBACK_COUNTER.fetch_add(1, Ordering::Relaxed);
            bytes[..8].copy_from_slice(&u64::try_from(nanos).unwrap_or(u64::MAX).to_be_bytes());
            bytes[8..].copy_from_slice(&counter.to_be_bytes());
        }
        let mut hex = String::with_capacity(32);
        for byte in bytes {
            let _ = write!(hex, "{byte:02x}");
        }
        Self(hex)
    }

    /// Accepts exactly 32 lowercase hex digits that are not all zero.
    pub fn parse(value: &str) -> Option<Self> {
        let valid = value.len() == 32
            && value
                .bytes()
                .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
            && value.bytes().any(|b| b != b'0');
        valid.then(|| Self(value.to_owned()))
    }

    /// Extracts the trace id from a W3C `traceparent` header (`00-<trace>-<parent>-<flags>`).
    pub fn from_traceparent(header: &str) -> Option<Self> {
        let mut parts = header.trim().split('-');
        let (version, trace, parent, flags) =
            (parts.next()?, parts.next()?, parts.next()?, parts.next()?);
        let is_hex = |s: &str, len: usize| {
            s.len() == len && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        };
        if !is_hex(version, 2) || version == "ff" || !is_hex(parent, 16) || !is_hex(flags, 2) {
            return None;
        }
        Self::parse(trace)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TraceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Why logging could not be initialised.
#[derive(Debug, thiserror::Error)]
pub enum TelemetryError {
    #[error("invalid RUST_LOG filter: {0}")]
    Filter(String),
    #[error("a global tracing subscriber is already installed")]
    AlreadyInitialised,
}

/// Installs the global subscriber for `config`: JSON-with-redaction in production, readable text
/// in development. Call once, before serving.
///
/// # Errors
/// Fails on an invalid `RUST_LOG` filter or if a subscriber is already installed.
pub fn init(config: &Config) -> Result<(), TelemetryError> {
    let filter = EnvFilter::try_new(&config.log_filter)
        .map_err(|error| TelemetryError::Filter(error.to_string()))?;
    let json = (config.log_format == LogFormat::Json)
        .then(|| JsonLayer::new(std::io::stdout as fn() -> std::io::Stdout));
    let text = (config.log_format == LogFormat::Text).then(tracing_subscriber::fmt::layer);
    tracing_subscriber::registry()
        .with(filter)
        .with(json)
        .with(text)
        .try_init()
        .map_err(|_| TelemetryError::AlreadyInitialised)
}

/// Writes one JSON object per event: `timestamp`, `level`, `target`, `message`, `trace_id` (from
/// the enclosing span), `fields` (event) and `span` (merged span fields), all redacted.
#[derive(Debug)]
pub struct JsonLayer<W> {
    make_writer: W,
}

impl<W> JsonLayer<W> {
    pub fn new(make_writer: W) -> Self {
        Self { make_writer }
    }
}

/// Span fields stored in the span's extensions.
#[derive(Debug, Default)]
struct SpanFields(Map<String, Value>);

struct FieldVisitor<'a>(&'a mut Map<String, Value>);

impl FieldVisitor<'_> {
    fn put(&mut self, field: &Field, value: Value) {
        let value = if is_sensitive(field.name()) {
            Value::String(REDACTED.to_owned())
        } else {
            value
        };
        self.0.insert(field.name().to_owned(), value);
    }
}

impl Visit for FieldVisitor<'_> {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.put(field, Value::String(value.to_owned()));
    }
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.put(field, Value::from(value));
    }
    fn record_u64(&mut self, field: &Field, value: u64) {
        self.put(field, Value::from(value));
    }
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.put(field, Value::Bool(value));
    }
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.put(field, Value::String(format!("{value:?}")));
    }
}

impl<S, W> Layer<S> for JsonLayer<W>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    W: for<'w> MakeWriter<'w> + 'static,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let mut fields = SpanFields::default();
        attrs.record(&mut FieldVisitor(&mut fields.0));
        if let Some(span) = ctx.span(id) {
            span.extensions_mut().insert(fields);
        }
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id) {
            let mut extensions = span.extensions_mut();
            if let Some(fields) = extensions.get_mut::<SpanFields>() {
                values.record(&mut FieldVisitor(&mut fields.0));
            }
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let mut fields = Map::new();
        event.record(&mut FieldVisitor(&mut fields));
        let message = fields.remove("message");

        let mut span_fields = Map::new();
        if let Some(scope) = ctx.event_scope(event) {
            for span in scope.from_root() {
                if let Some(stored) = span.extensions().get::<SpanFields>() {
                    for (key, value) in &stored.0 {
                        span_fields.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        let trace_id = span_fields
            .remove("trace_id")
            .or_else(|| fields.remove("trace_id"));

        let metadata = event.metadata();
        let mut line = Map::new();
        line.insert("timestamp".to_owned(), Value::String(now_rfc3339()));
        line.insert(
            "level".to_owned(),
            Value::String(metadata.level().to_string()),
        );
        line.insert(
            "target".to_owned(),
            Value::String(metadata.target().to_owned()),
        );
        if let Some(message) = message {
            line.insert("message".to_owned(), message);
        }
        if let Some(trace_id) = trace_id {
            line.insert("trace_id".to_owned(), trace_id);
        }
        if !fields.is_empty() {
            line.insert("fields".to_owned(), Value::Object(fields));
        }
        if !span_fields.is_empty() {
            line.insert("span".to_owned(), Value::Object(span_fields));
        }

        if let Ok(mut text) = serde_json::to_string(&Value::Object(line)) {
            text.push('\n');
            // A failing log sink must never take the request down with it.
            let _ = self.make_writer.make_writer().write_all(text.as_bytes());
        }
    }
}

fn now_rfc3339() -> String {
    rfc3339_utc(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default(),
    )
}

/// `1970-01-01T00:00:00.000Z` style timestamp for a duration since the Unix epoch.
fn rfc3339_utc(since_epoch: Duration) -> String {
    let secs = i64::try_from(since_epoch.as_secs()).unwrap_or(0);
    let (year, month, day) = civil_from_days(secs.div_euclid(86_400));
    let of_day = secs.rem_euclid(86_400);
    let (hour, minute, second) = (of_day / 3_600, of_day % 3_600 / 60, of_day % 60);
    let millis = since_epoch.subsec_millis();
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
}

/// Days since 1970-01-01 → proleptic Gregorian (year, month, day). Hinnant's algorithm.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::sync::{Arc, Mutex};

    use super::*;

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

    impl Capture {
        fn lines(&self) -> Vec<Value> {
            let bytes = self.0.lock().expect("lock").clone();
            String::from_utf8(bytes)
                .expect("utf8")
                .lines()
                .map(|l| serde_json::from_str(l).expect("every line is JSON"))
                .collect()
        }
        fn raw(&self) -> String {
            String::from_utf8(self.0.lock().expect("lock").clone()).expect("utf8")
        }
    }

    fn capture(run: impl FnOnce()) -> Capture {
        let sink = Capture::default();
        let writer = sink.clone();
        let subscriber =
            tracing_subscriber::registry().with(JsonLayer::new(move || writer.clone()));
        tracing::subscriber::with_default(subscriber, run);
        sink
    }

    #[test]
    fn denied_fields_are_redacted_and_nothing_else_is() {
        let sink = capture(|| {
            tracing::info!(
                email = "alice@example.com",
                phone = "0901234567",
                account_number = "0123456789",
                token = "tok_live_abc",
                password = "hunter2",
                order_id = 42_u64,
                status = "ok",
                "user action"
            );
        });
        let raw = sink.raw();
        for secret in [
            "alice@example.com",
            "0901234567",
            "0123456789",
            "tok_live_abc",
            "hunter2",
        ] {
            assert!(!raw.contains(secret), "{secret} leaked: {raw}");
        }
        let line = &sink.lines()[0];
        assert_eq!(line["message"], "user action");
        assert_eq!(line["level"], "INFO");
        let fields = &line["fields"];
        for denied in REDACTED_FIELDS {
            assert_eq!(fields[denied], REDACTED, "{denied}");
        }
        assert_eq!(fields["order_id"], 42);
        assert_eq!(fields["status"], "ok");
    }

    #[test]
    fn matching_is_case_insensitive_and_covers_compound_names_and_numbers() {
        let sink = capture(|| {
            tracing::warn!(
                User_Email = "a@b.co",
                access_token = "t0k3n",
                new_password_hash = "$argon2id$x",
                phone_number = 84_901_234_567_u64,
                "compound names"
            );
        });
        let raw = sink.raw();
        for secret in ["a@b.co", "t0k3n", "argon2id", "84901234567"] {
            assert!(!raw.contains(secret), "{secret} leaked: {raw}");
        }
        assert_eq!(sink.lines()[0]["fields"]["phone_number"], REDACTED);
    }

    #[test]
    fn debug_and_display_values_are_redacted_too() {
        #[derive(Debug)]
        struct Credentials {
            #[allow(dead_code)]
            password: &'static str,
        }
        let sink = capture(|| {
            let credentials = Credentials {
                password: "hunter2",
            };
            tracing::error!(password = ?credentials, email = %"bob@example.com", "login failed");
        });
        let raw = sink.raw();
        assert!(
            !raw.contains("hunter2") && !raw.contains("bob@example.com"),
            "{raw}"
        );
    }

    #[test]
    fn span_fields_are_redacted_and_trace_id_is_promoted() {
        let sink = capture(|| {
            let span = tracing::info_span!(
                "request",
                trace_id = "4bf92f3577b34da6a3ce929d0e0e4736",
                route = "/api/v1/links",
                password = "hunter2"
            );
            let _guard = span.enter();
            tracing::info!(status = 201_u64, "request");
        });
        assert!(!sink.raw().contains("hunter2"), "{}", sink.raw());
        let line = &sink.lines()[0];
        assert_eq!(line["trace_id"], "4bf92f3577b34da6a3ce929d0e0e4736");
        assert_eq!(line["span"]["route"], "/api/v1/links");
        assert_eq!(line["span"]["password"], REDACTED);
        assert_eq!(line["fields"]["status"], 201);
        assert!(
            line["span"].get("trace_id").is_none(),
            "promoted, not duplicated"
        );
    }

    #[test]
    fn late_recorded_span_fields_are_redacted() {
        let sink = capture(|| {
            let span = tracing::info_span!(
                "job",
                token = tracing::field::Empty,
                attempt = tracing::field::Empty
            );
            span.record("token", "secret-token-value");
            span.record("attempt", 2_u64);
            let _guard = span.enter();
            tracing::info!("running");
        });
        assert!(!sink.raw().contains("secret-token-value"), "{}", sink.raw());
        assert_eq!(sink.lines()[0]["span"]["attempt"], 2);
    }

    #[test]
    fn every_line_is_one_json_object_with_a_timestamp() {
        let sink = capture(|| {
            tracing::info!("first");
            tracing::debug!("second");
        });
        let lines = sink.lines();
        assert_eq!(lines.len(), 2);
        for line in lines {
            let timestamp = line["timestamp"].as_str().expect("timestamp");
            assert!(
                timestamp.ends_with('Z') && timestamp.len() == 24,
                "{timestamp}"
            );
            assert!(line["target"].as_str().is_some());
        }
    }

    #[test]
    fn timestamps_are_rfc3339_utc() {
        let cases = [
            (0, "1970-01-01T00:00:00.000Z"),
            (951_782_400, "2000-02-29T00:00:00.000Z"),
            (1_700_000_000, "2023-11-14T22:13:20.000Z"),
            (4_102_444_800, "2100-01-01T00:00:00.000Z"),
        ];
        for (secs, expected) in cases {
            assert_eq!(rfc3339_utc(Duration::from_secs(secs)), expected);
        }
        assert_eq!(
            rfc3339_utc(Duration::from_millis(1_700_000_000_123)),
            "2023-11-14T22:13:20.123Z"
        );
    }

    #[test]
    fn generated_trace_ids_are_valid_and_distinct() {
        let (a, b) = (TraceId::generate(), TraceId::generate());
        assert_ne!(a, b);
        for id in [&a, &b] {
            assert_eq!(TraceId::parse(id.as_str()).as_ref(), Some(id));
        }
    }

    #[test]
    fn trace_ids_parse_strictly() {
        assert!(TraceId::parse("4bf92f3577b34da6a3ce929d0e0e4736").is_some());
        for bad in [
            "",
            "4BF92F3577B34DA6A3CE929D0E0E4736",
            "4bf92f3577b34da6a3ce929d0e0e473",
            "4bf92f3577b34da6a3ce929d0e0e47366",
            "00000000000000000000000000000000",
            "4bf92f3577b34da6a3ce929d0e0e473g",
        ] {
            assert!(TraceId::parse(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn traceparent_headers_are_validated() {
        let good = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
        assert_eq!(
            TraceId::from_traceparent(good)
                .map(|t| t.to_string())
                .as_deref(),
            Some("4bf92f3577b34da6a3ce929d0e0e4736")
        );
        for bad in [
            "",
            "garbage",
            "ff-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
            "00-00000000000000000000000000000000-00f067aa0ba902b7-01",
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b-01",
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7",
        ] {
            assert!(TraceId::from_traceparent(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn invalid_filters_fail_at_startup() {
        let mut config = Config::load(|_| None, |_| Err(std::io::ErrorKind::NotFound.into()))
            .expect("defaults are valid");
        config.log_filter = "info,=,[[".to_owned();
        assert!(matches!(init(&config), Err(TelemetryError::Filter(_))));
    }
}

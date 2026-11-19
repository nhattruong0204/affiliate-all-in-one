//! The API error type and its RFC 9457 `application/problem+json` representation.
//!
//! `code` is the stable, machine-readable contract (clients switch on it); `detail` is localized
//! per `Accept-Language`; `trace_id` ties the response to the server log line. Adding an error
//! means adding a variant here **and** its entry in [`AppError::detail`], then listing the code
//! in `docs/api/openapi.yaml`; the catalogue tests fail if a language is missing.
//!
//! This crate cannot depend on `axum`, so the HTTP rendering lives in `web::problem::ApiError`.

use std::future::Future;

use serde::Serialize;

use crate::telemetry::TraceId;

const ERROR_TYPE_BASE: &str = "https://docs.aaio.vn/errors/";

/// Fallback body if serialization ever fails; a response must always be producible.
const FALLBACK_BODY: &str = r#"{"type":"https://docs.aaio.vn/errors/internal_error","title":"Internal error","status":500,"code":"internal_error"}"#;

/// Languages for API error messages. Vietnamese is the product default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lang {
    #[default]
    Vi,
    En,
}

impl Lang {
    /// Picks the best supported language from an `Accept-Language` header value: highest `q`
    /// wins, ties go to the earlier entry, anything unsupported or malformed is skipped.
    pub fn from_accept_language(header: Option<&str>) -> Self {
        let Some(header) = header else {
            return Self::default();
        };
        let mut best: Option<(u16, Self)> = None;
        for part in header.split(',') {
            let mut pieces = part.split(';');
            let tag = pieces
                .next()
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase();
            let lang = match tag.split('-').next().unwrap_or_default() {
                "vi" => Self::Vi,
                "en" => Self::En,
                _ => continue,
            };
            let quality = match pieces.find_map(|p| p.trim().strip_prefix("q=")) {
                None => Some(1000),
                Some(raw) => parse_quality(raw.trim()),
            };
            let Some(quality) = quality.filter(|q| *q > 0) else {
                continue;
            };
            if best.is_none_or(|(current, _)| quality > current) {
                best = Some((quality, lang));
            }
        }
        best.map_or_else(Self::default, |(_, lang)| lang)
    }
}

/// `q` value in thousandths: `"0.8"` → 800, `"1"` → 1000. Integer math only.
fn parse_quality(raw: &str) -> Option<u16> {
    let (whole, fraction) = raw.split_once('.').unwrap_or((raw, ""));
    let whole: u16 = whole.parse().ok()?;
    if whole > 1 || fraction.len() > 3 || !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let fraction: u16 = format!("{fraction:0<3}").parse().ok()?;
    Some((whole * 1000 + fraction).min(1000))
}

/// What a request carries for the duration of its handling: set by the observability middleware,
/// read when an error is rendered.
#[derive(Clone, Debug)]
pub struct RequestContext {
    pub trace_id: TraceId,
    pub lang: Lang,
}

tokio::task_local! {
    static REQUEST_CONTEXT: RequestContext;
}

impl RequestContext {
    /// Runs `future` with this context installed.
    pub async fn scope<F: Future>(self, future: F) -> F::Output {
        REQUEST_CONTEXT.scope(self, future).await
    }

    /// The context of the request being handled, if called from inside [`RequestContext::scope`].
    pub fn current() -> Option<Self> {
        REQUEST_CONTEXT.try_with(Clone::clone).ok()
    }

    /// A fresh context for code running outside a request (jobs, tests).
    pub fn detached() -> Self {
        Self {
            trace_id: TraceId::generate(),
            lang: Lang::default(),
        }
    }
}

/// One invalid field in a `422` or one failing dependency in a `503`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FieldError {
    pub field: String,
    pub code: String,
}

impl FieldError {
    pub fn new(field: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            code: code.into(),
        }
    }
}

/// Errors a handler may return. The internal source is logged, never sent to the client.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("validation failed")]
    Validation(Vec<FieldError>),
    #[error("authentication required")]
    Unauthenticated,
    #[error("forbidden")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error("conflict")]
    Conflict,
    #[error("rate limited")]
    RateLimited { retry_after_secs: u64 },
    /// A readiness dependency is failing; `errors` names it (`{field: "database", code: "unavailable"}`).
    #[error("service not ready")]
    NotReady(Vec<FieldError>),
    #[error("internal error: {0}")]
    Internal(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// The RFC 9457 body. Field names are the wire contract (`docs/api/openapi.yaml`, `Problem`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Problem {
    #[serde(rename = "type")]
    pub kind: String,
    pub title: String,
    pub status: u16,
    pub code: String,
    pub detail: String,
    pub trace_id: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<FieldError>,
}

impl Problem {
    /// The JSON text of the body.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| FALLBACK_BODY.to_owned())
    }
}

impl AppError {
    /// Wraps an unexpected failure; its message is logged but never exposed.
    pub fn internal(source: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self::Internal(source.into())
    }

    pub fn status(&self) -> u16 {
        match self {
            Self::Validation(_) => 422,
            Self::Unauthenticated => 401,
            Self::Forbidden => 403,
            Self::NotFound => 404,
            Self::Conflict => 409,
            Self::RateLimited { .. } => 429,
            Self::NotReady(_) => 503,
            Self::Internal(_) => 500,
        }
    }

    /// Stable machine-readable code; never change an existing value.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Validation(_) => "validation_failed",
            Self::Unauthenticated => "unauthenticated",
            Self::Forbidden => "forbidden",
            Self::NotFound => "not_found",
            Self::Conflict => "conflict",
            Self::RateLimited { .. } => "rate_limited",
            Self::NotReady(_) => "not_ready",
            Self::Internal(_) => "internal_error",
        }
    }

    /// Short English summary; identical for every client.
    pub fn title(&self) -> &'static str {
        match self {
            Self::Validation(_) => "Validation failed",
            Self::Unauthenticated => "Authentication required",
            Self::Forbidden => "Forbidden",
            Self::NotFound => "Not found",
            Self::Conflict => "Conflict",
            Self::RateLimited { .. } => "Too many requests",
            Self::NotReady(_) => "Service not ready",
            Self::Internal(_) => "Internal error",
        }
    }

    /// Human-readable explanation in `lang`. Fixed text per code: never built from internal
    /// error messages, so nothing sensitive can leak through it.
    pub fn detail(&self, lang: Lang) -> &'static str {
        match (self, lang) {
            (Self::Validation(_), Lang::Vi) => "Một hoặc nhiều trường không hợp lệ.",
            (Self::Validation(_), Lang::En) => "One or more fields are invalid.",
            (Self::Unauthenticated, Lang::Vi) => "Vui lòng đăng nhập để tiếp tục.",
            (Self::Unauthenticated, Lang::En) => "Please sign in to continue.",
            (Self::Forbidden, Lang::Vi) => "Bạn không có quyền thực hiện thao tác này.",
            (Self::Forbidden, Lang::En) => "You are not allowed to perform this action.",
            (Self::NotFound, Lang::Vi) => "Không tìm thấy nội dung được yêu cầu.",
            (Self::NotFound, Lang::En) => "The requested resource was not found.",
            (Self::Conflict, Lang::Vi) => "Yêu cầu xung đột với trạng thái hiện tại.",
            (Self::Conflict, Lang::En) => "The request conflicts with the current state.",
            (Self::RateLimited { .. }, Lang::Vi) => "Bạn thao tác quá nhanh. Vui lòng thử lại sau.",
            (Self::RateLimited { .. }, Lang::En) => {
                "You are going too fast. Please try again later."
            }
            (Self::NotReady(_), Lang::Vi) => "Hệ thống chưa sẵn sàng. Vui lòng thử lại sau.",
            (Self::NotReady(_), Lang::En) => "The service is not ready. Please try again shortly.",
            (Self::Internal(_), Lang::Vi) => {
                "Đã có lỗi xảy ra. Vui lòng thử lại hoặc gửi mã truy vết cho bộ phận hỗ trợ."
            }
            (Self::Internal(_), Lang::En) => {
                "Something went wrong. Please try again, or quote the trace ID to support."
            }
        }
    }

    /// `Retry-After` seconds for the response header, where the error defines one.
    pub fn retry_after_secs(&self) -> Option<u64> {
        match self {
            Self::RateLimited { retry_after_secs } => Some(*retry_after_secs),
            _ => None,
        }
    }

    /// Builds the response body for the given request context.
    pub fn problem(&self, context: &RequestContext) -> Problem {
        let errors = match self {
            Self::Validation(errors) | Self::NotReady(errors) => errors.clone(),
            _ => Vec::new(),
        };
        Problem {
            kind: format!("{ERROR_TYPE_BASE}{}", self.code()),
            title: self.title().to_owned(),
            status: self.status(),
            code: self.code().to_owned(),
            detail: self.detail(context.lang).to_owned(),
            trace_id: context.trace_id.to_string(),
            errors,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn every_variant() -> Vec<AppError> {
        vec![
            AppError::Validation(vec![FieldError::new("amount", "min")]),
            AppError::Unauthenticated,
            AppError::Forbidden,
            AppError::NotFound,
            AppError::Conflict,
            AppError::RateLimited {
                retry_after_secs: 30,
            },
            AppError::NotReady(vec![FieldError::new("database", "unavailable")]),
            AppError::internal("connection string postgres://u:hunter2@db/x refused"),
        ]
    }

    fn context(lang: Lang) -> RequestContext {
        RequestContext {
            trace_id: TraceId::parse("4bf92f3577b34da6a3ce929d0e0e4736").expect("valid"),
            lang,
        }
    }

    #[test]
    fn body_matches_the_openapi_problem_schema() {
        for error in every_variant() {
            let problem = error.problem(&context(Lang::En));
            let json: serde_json::Value = serde_json::from_str(&problem.to_json()).expect("json");
            for required in ["type", "title", "status", "code"] {
                assert!(
                    json.get(required).is_some(),
                    "{} lacks {required}",
                    error.code()
                );
            }
            assert_eq!(
                json["type"],
                format!("https://docs.aaio.vn/errors/{}", error.code())
            );
            assert_eq!(json["status"], error.status());
            assert_eq!(json["trace_id"], "4bf92f3577b34da6a3ce929d0e0e4736");
            assert!(json["detail"].as_str().is_some_and(|d| !d.is_empty()));
            let code = json["code"].as_str().expect("code");
            assert!(
                code.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'),
                "codes are snake_case: {code}"
            );
            assert!((400..600).contains(&error.status()));
        }
    }

    #[test]
    fn codes_and_statuses_are_stable() {
        let table: Vec<(u16, &str)> = every_variant()
            .iter()
            .map(|e| (e.status(), e.code()))
            .collect();
        assert_eq!(
            table,
            [
                (422, "validation_failed"),
                (401, "unauthenticated"),
                (403, "forbidden"),
                (404, "not_found"),
                (409, "conflict"),
                (429, "rate_limited"),
                (503, "not_ready"),
                (500, "internal_error"),
            ]
        );
    }

    #[test]
    fn every_code_is_localized_in_both_languages() {
        for error in every_variant() {
            let vi = error.detail(Lang::Vi);
            let en = error.detail(Lang::En);
            assert!(!vi.is_empty() && !en.is_empty(), "{}", error.code());
            assert_ne!(vi, en, "{} is not translated", error.code());
        }
        let vi = AppError::NotFound.problem(&context(Lang::Vi));
        assert_eq!(vi.detail, "Không tìm thấy nội dung được yêu cầu.");
    }

    #[test]
    fn field_errors_are_listed_only_when_present() {
        let json = AppError::Validation(vec![FieldError::new("amount", "min")])
            .problem(&context(Lang::En))
            .to_json();
        assert!(
            json.contains(r#""errors":[{"field":"amount","code":"min"}]"#),
            "{json}"
        );
        let json = AppError::Forbidden.problem(&context(Lang::En)).to_json();
        assert!(!json.contains("errors"), "{json}");
    }

    #[test]
    fn internal_errors_never_leak_their_cause() {
        let error = AppError::internal("connection string postgres://u:hunter2@db/x refused");
        assert!(
            error.to_string().contains("hunter2"),
            "kept for the server log"
        );
        for lang in [Lang::Vi, Lang::En] {
            let body = error.problem(&context(lang)).to_json();
            assert!(
                !body.contains("hunter2") && !body.contains("postgres"),
                "{body}"
            );
        }
    }

    #[test]
    fn retry_after_is_defined_only_for_rate_limits() {
        assert_eq!(
            AppError::RateLimited {
                retry_after_secs: 30
            }
            .retry_after_secs(),
            Some(30)
        );
        assert_eq!(AppError::NotFound.retry_after_secs(), None);
    }

    #[test]
    fn accept_language_selection() {
        let cases = [
            (None, Lang::Vi),
            (Some(""), Lang::Vi),
            (Some("en"), Lang::En),
            (Some("en-US,en;q=0.9,vi;q=0.8"), Lang::En),
            (Some("vi-VN,vi;q=0.9,en;q=0.8"), Lang::Vi),
            (Some("fr-FR,fr;q=0.9,en;q=0.5"), Lang::En),
            (Some("en;q=0.4, vi;q=0.7"), Lang::Vi),
            (Some("en;q=0, vi;q=0.1"), Lang::Vi),
            (Some("en;q=0"), Lang::Vi),
            (Some("de, ja"), Lang::Vi),
            (Some("*"), Lang::Vi),
            (Some("en;q=abc"), Lang::Vi),
            (Some("EN-gb"), Lang::En),
            (Some("vi;q=0.5, en;q=0.5"), Lang::Vi),
        ];
        for (header, expected) in cases {
            assert_eq!(Lang::from_accept_language(header), expected, "{header:?}");
        }
    }

    #[tokio::test]
    async fn request_context_is_visible_only_inside_its_scope() {
        assert!(RequestContext::current().is_none());
        let ctx = context(Lang::En);
        let seen = ctx
            .clone()
            .scope(async { RequestContext::current().map(|c| (c.trace_id, c.lang)) })
            .await;
        assert_eq!(seen, Some((ctx.trace_id, Lang::En)));
        assert!(RequestContext::current().is_none());
        assert_eq!(RequestContext::detached().lang, Lang::Vi);
    }
}

//! HTTP rendering of [`AppError`] as RFC 9457 `application/problem+json`.
//!
//! `AppError` lives in `app`, which cannot depend on axum (and the orphan rule forbids
//! implementing `IntoResponse` for it here), so handlers return `Result<_, ApiError>`; `?` converts
//! an `AppError` automatically.

use app::error::{AppError, RequestContext};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};

pub const PROBLEM_JSON: &str = "application/problem+json";

#[derive(Debug)]
pub struct ApiError(pub AppError);

impl From<AppError> for ApiError {
    fn from(error: AppError) -> Self {
        Self(error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let error = self.0;
        // Outside a request (should not happen) the error still renders, with a fresh trace id.
        let context = RequestContext::current().unwrap_or_else(RequestContext::detached);
        if let AppError::Internal(source) = &error {
            // The cause goes to the log (correlated by the request span), never to the client.
            tracing::error!(error = %source, code = error.code(), "internal error");
        }
        let problem = error.problem(&context);
        let status =
            StatusCode::from_u16(problem.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let mut response = (
            status,
            [
                (header::CONTENT_TYPE, PROBLEM_JSON),
                (header::CACHE_CONTROL, "no-store"),
            ],
            problem.to_json(),
        )
            .into_response();
        if let Some(seconds) = error.retry_after_secs()
            && let Ok(value) = HeaderValue::from_str(&seconds.to_string())
        {
            response.headers_mut().insert(header::RETRY_AFTER, value);
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use app::error::{FieldError, Lang};
    use app::telemetry::TraceId;
    use axum::body::to_bytes;

    use super::*;

    async fn render(
        error: AppError,
        context: Option<RequestContext>,
    ) -> (Response, serde_json::Value) {
        let response = match context {
            Some(context) => {
                context
                    .scope(async { ApiError(error).into_response() })
                    .await
            }
            None => ApiError(error).into_response(),
        };
        let (parts, body) = response.into_parts();
        let bytes = to_bytes(body, usize::MAX).await.expect("body");
        let json = serde_json::from_slice(&bytes).expect("json body");
        (Response::from_parts(parts, axum::body::Body::empty()), json)
    }

    fn context(lang: Lang) -> RequestContext {
        RequestContext {
            trace_id: TraceId::parse("4bf92f3577b34da6a3ce929d0e0e4736").expect("valid"),
            lang,
        }
    }

    #[tokio::test]
    async fn renders_problem_json_with_trace_id_and_localized_detail() {
        let error = AppError::Validation(vec![FieldError::new("amount", "min")]);
        let (response, json) = render(error, Some(context(Lang::Vi))).await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(response.headers()["content-type"], PROBLEM_JSON);
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert_eq!(json["code"], "validation_failed");
        assert_eq!(
            json["type"],
            "https://docs.aaio.vn/errors/validation_failed"
        );
        assert_eq!(json["status"], 422);
        assert_eq!(json["detail"], "Một hoặc nhiều trường không hợp lệ.");
        assert_eq!(json["trace_id"], "4bf92f3577b34da6a3ce929d0e0e4736");
        assert_eq!(json["errors"][0]["field"], "amount");
    }

    #[tokio::test]
    async fn english_detail_follows_the_request_language() {
        let (_, json) = render(AppError::NotFound, Some(context(Lang::En))).await;
        assert_eq!(json["detail"], "The requested resource was not found.");
    }

    #[tokio::test]
    async fn rate_limits_set_retry_after() {
        let (response, _) = render(
            AppError::RateLimited {
                retry_after_secs: 30,
            },
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers()["retry-after"], "30");
    }

    #[tokio::test]
    async fn internal_errors_hide_their_cause_and_still_carry_a_trace_id() {
        let error = AppError::internal("password=hunter2 rejected by db.internal");
        let (response, json) = render(error, None).await;
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(json["code"], "internal_error");
        let text = json.to_string();
        assert!(
            !text.contains("hunter2") && !text.contains("db.internal"),
            "{text}"
        );
        let trace = json["trace_id"].as_str().expect("trace id");
        assert!(TraceId::parse(trace).is_some(), "{trace}");
    }

    #[test]
    fn question_mark_converts_app_errors() {
        fn handler() -> Result<(), ApiError> {
            Err(AppError::Forbidden)?
        }
        assert!(matches!(handler(), Err(ApiError(AppError::Forbidden))));
    }
}

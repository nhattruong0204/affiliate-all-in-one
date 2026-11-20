//! The CAPTCHA hook that anonymous endpoints (analyze, waitlist; AAO-100) call before doing work.
//!
//! Only the seam lives here. The Cloudflare Turnstile verification call is an outbound HTTPS
//! request, and `reqwest` is confined to the `affiliate` and `testkit` crates
//! (`scripts/check-deps.sh`), so the network adapter is added together with its first consumer
//! once the owner decides where outbound HTTP clients live. Until then [`DisabledCaptcha`] is the
//! only implementation; it is explicit about being a no-op so a forgotten configuration cannot
//! look like protection (`enabled()` is `false`, and `web` logs it at startup in production).

use std::future::Future;
use std::net::IpAddr;
use std::pin::Pin;

use crate::error::{AppError, FieldError};

pub type VerifyFuture<'a> = Pin<Box<dyn Future<Output = Result<(), CaptchaError>> + Send + 'a>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CaptchaError {
    /// The request carried no token although verification is enabled.
    #[error("captcha token missing")]
    Missing,
    /// The provider said the token is invalid, expired or already used.
    #[error("captcha rejected")]
    Rejected,
    /// The provider could not be reached: fail closed, the caller may retry.
    #[error("captcha service unavailable")]
    Unavailable,
}

impl From<CaptchaError> for AppError {
    fn from(error: CaptchaError) -> Self {
        match error {
            CaptchaError::Missing => Self::Validation(vec![FieldError::new("captcha", "required")]),
            CaptchaError::Rejected => Self::Validation(vec![FieldError::new("captcha", "invalid")]),
            CaptchaError::Unavailable => {
                Self::NotReady(vec![FieldError::new("captcha", "unavailable")])
            }
        }
    }
}

/// Verifies a CAPTCHA response token.
pub trait CaptchaVerifier: Send + Sync {
    /// `false` for the no-op implementation, so callers and startup checks can tell.
    fn enabled(&self) -> bool;

    /// `token` is the provider's response field from the submitted form, if any.
    fn verify<'a>(&'a self, token: Option<&'a str>, client_ip: Option<IpAddr>) -> VerifyFuture<'a>;
}

/// Accepts everything. Used when no CAPTCHA provider is configured.
#[derive(Clone, Copy, Debug, Default)]
pub struct DisabledCaptcha;

impl CaptchaVerifier for DisabledCaptcha {
    fn enabled(&self) -> bool {
        false
    }

    fn verify<'a>(&'a self, _: Option<&'a str>, _: Option<IpAddr>) -> VerifyFuture<'a> {
        Box::pin(async { Ok(()) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_disabled_verifier_accepts_anything_and_says_so() {
        let verifier = DisabledCaptcha;
        assert!(!verifier.enabled());
        assert_eq!(verifier.verify(None, None).await, Ok(()));
        assert_eq!(
            verifier
                .verify(Some("junk"), Some("127.0.0.1".parse().unwrap()))
                .await,
            Ok(())
        );
    }

    #[test]
    fn failures_map_to_stable_api_errors() {
        let cases = [
            (CaptchaError::Missing, 422, "validation_failed", "required"),
            (CaptchaError::Rejected, 422, "validation_failed", "invalid"),
            (CaptchaError::Unavailable, 503, "not_ready", "unavailable"),
        ];
        for (error, status, code, field_code) in cases {
            let app_error = AppError::from(error);
            assert_eq!(
                (app_error.status(), app_error.code()),
                (status, code),
                "{error}"
            );
            let body = app_error
                .problem(&crate::error::RequestContext::detached())
                .to_json();
            assert!(
                body.contains(&format!(r#""field":"captcha","code":"{field_code}""#)),
                "{body}"
            );
        }
    }
}

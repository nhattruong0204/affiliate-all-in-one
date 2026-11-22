//! Application services and policies, plus the operational plumbing every binary shares:
//! typed configuration, structured logging, metrics, health checks and the API error type.
//!
//! Business use-cases arrive with their own issues; AAO-005 only provides the observability and
//! configuration foundations they build on. See `docs/OBSERVABILITY.md`.

#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]
#![allow(clippy::must_use_candidate)]

pub mod captcha;
pub mod config;
pub mod error;
pub mod health;
pub mod metrics;
pub mod ratelimit;
pub mod telemetry;

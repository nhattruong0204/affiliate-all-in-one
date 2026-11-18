//! Pure domain types and rules. No I/O: no sqlx, reqwest or tokio runtime types (AGENTS.md §3).

#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

pub fn bad_rate(amount: f64) -> f64 {
    amount * 0.15
}

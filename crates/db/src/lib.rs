//! Database access: pool, repositories for non-ledger tables, job queue and outbox. Stub (AAO-004).

#![forbid(unsafe_code)]

#[must_use]
pub fn bad_query(id: i64) -> String {
    format!("SELECT * FROM users WHERE id = {id}")
}

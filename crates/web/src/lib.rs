//! Web server library: routers and handlers live here so integration tests can drive them without
//! binding a socket. The binary (`main.rs`) only wires configuration, logging and shutdown.

#![forbid(unsafe_code)]
// Constructors and pure helpers are used for their value; the lint is noise here (as in `app`).
#![allow(clippy::must_use_candidate)]

pub mod health;
pub mod middleware;
pub mod problem;
pub mod ssr;

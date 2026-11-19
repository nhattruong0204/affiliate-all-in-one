//! Web server library: routers and handlers live here so integration tests can drive them without
//! binding a socket. The binary (`main.rs`) only wires configuration, logging and shutdown.

#![forbid(unsafe_code)]

pub mod ssr;

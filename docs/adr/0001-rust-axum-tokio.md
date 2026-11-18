# ADR-0001: Rust + Axum + Tokio for all production logic

Status: Proposed · Date: 2026-11-17

## Context
The owner requires Rust. The product handles financial obligations, untrusted URLs and third-party APIs, so type safety, memory safety and predictable performance all matter.

## Decision
All production business logic is written in Rust stable. The HTTP stack is Axum on Tokio with Tower middleware (timeouts, body limits, concurrency limits, tracing). `#![forbid(unsafe_code)]` applies in every workspace crate.

## Consequences
- (+) One language across UI (Leptos), API, worker and tests.
- (+) Newtypes and exhaustive `match` on state enums catch whole classes of money and state bugs at compile time.
- (−) Slower iteration than dynamic stacks, and a smaller hiring pool. We mitigate this with clear crate boundaries and fast incremental builds (`cargo-chef`, `mold` linker in dev).
- (−) Some provider SDKs only exist for JS/Python. We implement HTTP clients ourselves from official docs.

## Alternatives
Actix-web (comparable; Tower ecosystem decided it). Node/Python backends (excluded by the brief).

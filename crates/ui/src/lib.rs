//! Design system and page rendering for Affiliate All-in-One.
//!
//! V0.1 scope (AAO-014): tokens, AppShell, navigation, theme, vi/en, base components, loading /
//! empty / error states and toasts. Business pages (landing, analyzer, wallet …) build on these in
//! their own issues. See `docs/UI_FOUNDATIONS.md` for usage and conventions.
//!
//! Everything renders on the server (Leptos SSR) to an HTML string. No WASM is shipped yet:
//! the first issue that needs a client-side island adds hydration (ADR-0003).

#![forbid(unsafe_code)]
// Leptos `#[component]` functions take their props by value; that is the framework's calling
// convention, not an accidental clone.
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::must_use_candidate)]

pub mod assets;
pub mod components;
pub mod cookie;
pub mod document;
pub mod i18n;
pub mod icons;
pub mod pages;
pub mod theme;

#[cfg(test)]
mod tokens;

pub use document::PageContext;
pub use i18n::Locale;
pub use theme::Theme;

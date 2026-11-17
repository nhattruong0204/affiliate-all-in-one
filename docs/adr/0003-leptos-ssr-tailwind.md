# ADR-0003: Leptos SSR with hydration and Tailwind standalone CLI

Status: Proposed · Date: 2026-11-17

## Context
Vietnamese-first, mobile-first. SEO matters for the landing page and blog, and mid-range Android over 4G is the target device. Rust is required for production logic.

## Decision
Leptos in SSR mode with selective hydration (islands) for interactive widgets (link analyzer, wallet, admin tables). Styling uses Tailwind v4 compiled by the standalone CLI binary in the Docker build, so no Node at runtime. Design tokens live as CSS custom properties (`UI_UX_SPECIFICATION.md` §2).

## Consequences
- (+) Pages render without JS. WASM ships only for islands, and we enforce a budget of ≤ 250 KB compressed per page.
- (+) Server functions share types with the API.
- (−) Leptos is younger than React. Mitigation: keep components simple, keep the business API in plain JSON (`/api/v1`) so a different frontend could replace `ui` later.
- (−) WASM debugging is harder; we rely on Playwright coverage.

## Alternatives
HTMX + Askama: simpler, viable fallback if Leptos hydration proves costly. SPA (React/Vue): second language plus a Node toolchain.

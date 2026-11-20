# UI Foundations (AAO-014)

How the web UI is built, what exists in V0.1, and how to add to it. Visual rules and page designs live in `UI_UX_SPECIFICATION.md`; this document is the implementation guide for the `ui` crate and its host in `web`.

## 1. Decisions

| Decision | Why |
|---|---|
| **SSR only for V0.1** (Leptos 0.8 renders HTML strings; no WASM, no hydration) | Nothing in V0.1 needs client-side state. Shipping no WASM gives the best mobile performance (Lighthouse mobile 100/100 on the shell), a strict CSP without `wasm-unsafe-eval`, and a faster path to launch. ADR-0003 already says hydration is *selective*; the first issue that needs an island (analyzer in AAO-100 can be a plain form + redirect) adds `cargo-leptos` and hydration for that island only. |
| **~3 KB first-party JS** (`assets/app.js`) | Theme toggle, language memory, toasts, double-submit guard. Everything has a server-rendered baseline, so the site works without JavaScript. |
| **Tailwind v4 standalone CLI**, generated CSS committed | No Node in the Rust build. `just css-check` in CI fails when `assets/app.css` is stale. |
| **Semantic classes over utility soup** | `btn`, `input`, `card`, … are defined once in `style/tailwind.css` with `@apply`; components stay small and visually consistent. Utilities are still used for layout in pages. |
| **No inline script or style, no `data:` URLs** | Lets the server send `default-src 'none'; script-src 'self'; style-src 'self'; …` (see §7). Enforced by Rust unit tests and a Playwright test. |
| **Cookies, not local storage, for preferences** | The server needs the theme before first paint (no flash) and the language to redirect `/`. Both are functional cookies (`aao_theme`, `aao_lang`, 1 year, `SameSite=Lax`, no tracking). |

## 2. Layout of the code

```
crates/ui/
  assets/            app.css (generated, committed) · app.js · favicon.svg · fonts/ (Be Vietnam Pro, OFL)
  locales/{vi,en}/   common.ftl · gallery.ftl      (Fluent catalogs)
  style/tailwind.css tokens + base + component classes
  src/
    components/      AppShell, Button, Card, TextField, Alert, Toast, Skeleton, EmptyState, ErrorState…
    pages.rs         shell_preview (/), gallery (/_ui), not_found
    document.rs      <head>, lang/theme attributes, canonical/hreflang
    i18n.rs          Locale, tr/tr_args, path localisation
    theme.rs cookie.rs icons.rs assets.rs tokens.rs (tests only)
crates/web/src/ssr.rs  routes, locale/theme from the request, security headers, asset serving
e2e/                   Playwright + axe + Lighthouse (not shipped)
```

`ui` renders; it never does I/O. `web` turns a request into a `PageContext { locale, theme, path, base_url }` and calls a function in `ui::pages`.

## 3. Design tokens

Defined once in `style/tailwind.css` (`@theme static`, `--color-*: initial` removes Tailwind's default palette, so `text-slate-500` cannot compile). Values follow `UI_UX_SPECIFICATION.md` §2; additions are marked.

| Group | Tokens |
|---|---|
| Surfaces / text | `bg`, `surface`, `surface-2`\*, `border`\*, `input-border`\*, `text`, `text-muted` |
| Brand | `brand-50/100/600/700`, `on-brand`; accent `accent-50/500`, `accent-text`, `on-accent`; `focus`\* |
| Feedback | `danger`, `success`, `warning`, `info` (+ `-bg` tints) |
| Order/money state (for `StatePill`, AAO-036) | `state-estimated/pending/approved/available/reversed` |
| Shape / elevation | `--radius-card` 16px, `--radius-control` 12px, `--radius-pill`, `--shadow-card`, `--shadow-pop` |
| Type | `text-display/h1/h2/h3/lead/body/small/caption`, fluid, Vietnamese-safe line heights; Be Vietnam Pro 400/600 |

Brand direction: deep teal for trust and action, amber for emphasis, neutral slate surfaces, 16 px cards, 44 px touch targets, calm motion (disabled under `prefers-reduced-motion`).

**Dark mode.** Dark values are declared twice, in `:root[data-theme="dark"]` (explicit choice) and in `@media (prefers-color-scheme: dark) :root:not([data-theme="light"])` (no choice yet). `src/tokens.rs` tests fail if:
- the two blocks differ,
- a colour has no dark value (or a dark value has no light token),
- any text/background pair used by components drops below WCAG AA 4.5:1, or a control border / focus ring below 3:1, in either theme.

**Adding a token:** add it to `@theme static`, add the dark value to *both* dark blocks, add the pair it is used with to `TEXT_PAIRS`/`UI_PAIRS` in `tokens.rs`, run `just css`, commit `assets/app.css`.

## 4. Theme and language

- **Theme:** cookie `aao_theme` = `light` | `dark`; absent = follow the OS. The server writes `<html data-theme="…">` from the cookie, so the first byte is already correct (no flash, no inline script). The toggle flips the attribute, stores the cookie and updates `aria-pressed`; the sun/moon icon is chosen in CSS. Hidden when scripting is off.
- **Language:** Vietnamese is canonical at `/…`, English at `/en/…` (UI/UX spec §4). `LanguageSwitch` renders plain links to the same page in the other language (works without JS, `hreflang` set). Clicking one stores `aao_lang`; the server then sends a bare `/` to `/en` for returning English users. Crawlers send no cookie, so both versions stay indexable. `canonical` and `hreflang` links appear when `WEB_PUBLIC_BASE_URL` is set (domain decision D-4).
- **Catalogs:** Fluent, `locales/{vi,en}/*.ftl`, one message per line. Tests enforce identical keys and variables in both languages, no empty values, valid syntax and that every `tr(locale, "key")` call site exists. A missing key shows the key (visible), never a blank. Keys are kebab-case, grouped by prefix (`nav-`, `error-`, `toast-`, `g-` for the gallery).
- **Add a string:** add the key to both languages, call `tr(locale, "key")`. **Add a language:** new `Locale` variant, catalogs, path prefix; the parity tests then cover it.

## 5. Components (V0.1)

All are server-rendered Leptos components. Props are plain values; optional strings default to "omit the attribute".

| Component | Notes |
|---|---|
| `AppShell` | Skip link, sticky header (logo, desktop nav, `LanguageSwitch`, `ThemeToggle`, optional CTA), `<main id="main" tabindex="-1">`, footer, mobile bottom nav (< 1024 px) / top nav (≥ 1024 px), toast region. Navigation is data (`Vec<NavItem>`); `aria-current="page"` is derived from the path. Later releases add Orders/Wallet/Account items without touching the shell. |
| `Button`, `ButtonLink` | `Primary/Secondary/Ghost/Danger`, `Sm/Md/Lg`, `loading` (spinner + `aria-busy`), `disabled`, `block`. `Button` defaults to `type="button"`. Use `ButtonLink` for navigation. |
| `TextField`, `TextAreaField`, `CheckboxField` | Label, hint, error wired with `for`/`id`/`aria-describedby`/`aria-invalid`; error = icon + text (never colour alone); required asterisk is decorative; 48 px inputs; `autocomplete`/`inputmode` supported. |
| `Card` | Surface with border and `padding` None/Md/Lg. |
| `Alert` | Info/Success/Warning/Error; errors use `role="alert"`, the rest `role="status"`. |
| `Toast`, `ToastRegion`, `window.AAO.toast()` | Polite live region; success/info/warning auto-dismiss (6 s, pause on hover/focus), **errors never auto-dismiss** (WCAG 2.2.1), Escape and close button, text inserted with `textContent`. Server-rendered toasts (flash messages) use `Toast` with `timeout_ms`. |
| `Skeleton`, `SkeletonGroup`, `SkeletonText` | Decorative shapes + one polite "loading" announcement; shimmer stops under reduced motion. |
| `EmptyState`, `ErrorState` | Icon, title, explanation, optional action. `ErrorState` adds retry link and a reference id for support (`error-trace-id`). |
| `ThemeToggle`, `LanguageSwitch`, `LogoMark`, `IconView` | Inline SVG icons (no icon font, no extra requests). |

**Deferred (not needed by V0.1 pages):** `Modal` (add with its first user, AAO-092/100 follow-ups), `MoneyText` and `StatePill` (AAO-036, they depend on `domain::Money` and the state machine). The tokens for them already exist.

`/_ui` (only when `WEB_UI_GALLERY=1`; `noindex`; 404 otherwise) shows every component in every state in both languages and both themes. It is the review surface for designers and the target of the axe scans.

## 6. JavaScript contract

`app.js` binds by delegated events to `data-*` attributes rendered by the components: `data-theme-toggle`, `data-lang-link`, `data-toast-region|template|close|demo`, `data-submit-guard` (forms), `data-requires-js` (hidden by `@media (scripting: none)`). No framework, no `eval`, no inline handlers, no globals except the frozen `window.AAO`. When adding behaviour: render a working baseline on the server first, then enhance.

## 7. Security headers (served by `web::ssr`)

Implemented in `crates/web/src/middleware/headers.rs` (AAO-091); the set is built once at startup from configuration and applied to **every** response, including `404`, `429` and the probes. Base CSP: `default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self'; font-src 'self'; connect-src 'self'; manifest-src 'self'; object-src 'none'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'`. Also `X-Content-Type-Options: nosniff`, `Referrer-Policy: strict-origin-when-cross-origin`, `X-Frame-Options: DENY`, `Permissions-Policy` (camera, microphone, geolocation, payment, usb, browsing-topics off), `Cross-Origin-Resource-Policy: same-origin`, `X-Permitted-Cross-Domain-Policies: none`. `Cross-Origin-Opener-Policy` is deliberately not sent: `same-origin` made the Lighthouse CI gate fail with `NO_NAVSTART`; revisit with the first auth popup. `Strict-Transport-Security: max-age=63072000; includeSubDomains; preload` is sent **only when `APP_ENV=production`** (the app is the single source; Caddy must not add a second value). `includeSubDomains`/`preload` are only safe once the domain is decided (D-4); submitting to the preload list is a separate manual step. When `TURNSTILE_SITE_KEY` is set, `https://challenges.cloudflare.com` is added to `script-src` and a `frame-src` is added; nothing else widens. **UNVERIFIED** against Cloudflare's CSP documentation (not reachable when written) — AAO-100 must confirm with the real widget in the Playwright CSP test before enabling it. HTML is `Cache-Control: no-cache` + `Vary: Cookie`; `app.css`/`app.js` are immutable when requested with the current `?v=<hash>`. Adding any other third-party origin (merchant images) means editing `content_security_policy`, its tests and the Playwright CSP test, with a security review.

## 8. Assets

`just css` runs the checksum-pinned Tailwind binary (`scripts/install-tailwind.sh`, downloaded to `.tools/`) and writes `crates/ui/assets/app.css`. Source detection is off (`source(none)`) and only `crates/ui/src` + `app.js` are scanned, so output is identical from any directory. Assets are embedded with `include_bytes!` and served from `/assets/…`; there is no runtime file path. Fonts are a Vietnamese-capable Be Vietnam Pro subset (WOFF, two weights, ~68 KB, SIL OFL; licence text served at `/assets/fonts/OFL.txt`), preloaded and `font-display: swap`.

## 9. Quality gates

| Gate | Where |
|---|---|
| fmt, clippy `-D warnings`, nextest, doctests, cargo-deny, cargo-audit, gitleaks, dependency-direction checks | existing CI jobs |
| `just css-check` (generated CSS fresh) | `lint` job |
| Rust: component markup, escaping, i18n parity, token contrast, headers, assets, redirects, config parsing | `ui`/`web` unit tests |
| Playwright (desktop 1280 and mobile 390): theme + language persistence, no-JS behaviour, toasts, layout/breakpoint, skip link, focus ring, touch targets, strict-CSP (no violations, no external requests, no inline code) | `e2e` job |
| axe-core WCAG 2.2 A/AA on every page × light/dark × desktop/mobile (zero violations) | `e2e` job |
| Lighthouse mobile, median of 3: Performance ≥ 90, Accessibility ≥ 95 (release build) | `lighthouse` job |

Locally: `just css-check`, `npm ci --prefix e2e && just e2e` (needs Chrome), `just lighthouse`. `SCREENSHOTS=1 just e2e` writes `e2e/screenshots/`.

## 10. Building a page

1. Add a function to `ui::pages` that returns `String` via `render_document`, wrapping content in `AppShell`.
2. Use exactly one `<h1>`; take every user-visible string from the catalogs.
3. Route it in `web::ssr::router` (Vietnamese slug canonical, `/en` alias) and add it to `PAGES` in `e2e/tests/helpers.ts` so axe, CSP and layout tests cover it.
4. Reuse components; if a new one is needed, add it to the gallery in both languages.
5. Forms POST with CSRF (AAO-010) and redirect; show errors with `TextField.error` and `Alert`.

## 11. Known limits

- Navigation entries for *Cách dùng* (`/#how-it-works`), *Hỏi đáp* (`/hoi-dap`) and *Đăng ký sớm* (`/#waitlist`) point at pages delivered by AAO-100; until then the first resolves to the preview page and the others to the 404 page. The footer's Terms/Privacy links behave the same way.
- Brand name and logo mark are **placeholders** (D-4). They are isolated in `brand-name` and `LogoMark`, so replacing them is a one-line + one-asset change.
- Hydration/islands are not set up yet (§1).

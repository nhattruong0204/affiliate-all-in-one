# Competitor Research

Access date: 2026-11-17. Method: unauthenticated fetch of public pages only. No account was created; authenticated areas are **UNKNOWN** unless a public page describes them.

## 1. hoantien.com (primary reference)

### 1.1 Verified public surface

| Area | Finding | Source | Status |
|---|---|---|---|
| Navigation | Home, Blog, Login, Register, "Ví" (→ `/dashboard`), Leaderboard (`/ranking`), dark-mode toggle, language switch (`/change-language/vi|en`) | `/` | VERIFIED |
| Merchants | Shopee and TikTok Shop only | `/` | VERIFIED |
| Link submission | Single paste field + CTA "paste link to get cashback" on the landing page | `/` | VERIFIED |
| Estimate preview | Marketing demo shows rate (14%), price (250k), expected cashback (+35,000đ) | `/` | VERIFIED (demo); live preview behaviour INFERRED |
| Share claim | "Up to 70%" of the commission Shopee pays | `/` | VERIFIED (claim) |
| Order sync | Claims Shopee orders sync near real time via API into wallet history | `/` | VERIFIED (claim); mechanism UNKNOWN |
| Timing | Credited 3–7 days after delivery confirmation; withdrawable ≈7 days after delivery. A blog post instead says "a few weeks to a few dozen days" | `/`, blog | VERIFIED (inconsistent on their side) |
| Purchase rules | Empty cart first; no other affiliate links; pay within 20–30 min; disable ad blockers; regenerate link after cancel/reorder | `/` | VERIFIED |
| Withdrawal | Min 50,000đ; bank (VietQR) or MoMo; own-name account; 1–24 working hours after review; no fee mentioned | `/`, ToS | VERIFIED |
| Auth | Email + password, "remember me", Google ("Continue with Google" → `/auth/google`), forgot password (`/forgot-password`); CSRF token present; no CAPTCHA visible | `/login` | VERIFIED |
| Registration | Full name, email, optional phone, password + confirm, ToS/privacy checkbox, Google. No referral-code field on the form | `/register` | VERIFIED |
| Leaderboard | Page exists but currently shows "no leaderboard available — disabled by admin". Meta description: ranks by order count, cashback, check-in streak, downline referrals | `/ranking` | VERIFIED (disabled); design INFERRED from meta |
| Daily check-in | Only evidence is the leaderboard meta description mentioning check-in streaks | `/ranking` | INFERRED exists behind login; rules UNKNOWN |
| Referral | Bonus when referee's order is reconciled; "percentage of the system budget"; referee cashback not reduced; fake accounts void bonus | ToS | VERIFIED |
| Multi-level referral | A blog post title/slug mentions "2 tầng F1, F2"; the body describes one level only; ToS states no multi-level structure | blog, ToS | INFERRED a 2-level scheme exists or existed; rates UNKNOWN |
| Blog | `/blog`, `/blog/{slug}`, `/blog/category/{slug}`, `/blog/tag/{slug}`; featured, latest, search, most-viewed, tags; view counts; 4 posts | `/blog` | VERIFIED |
| PWA | `apple-mobile-web-app-capable` meta; theme colour set; no native app | `/` | VERIFIED (meta only); manifest/service worker UNKNOWN |
| Support | Email, hotline 8:00–18:00, Zalo, Facebook page, Messenger; floating contact bar | `/` | VERIFIED |
| Missing cashback claim | No public claim flow; guidance is "check later / contact support" | blog | VERIFIED absent publicly; in-app flow UNKNOWN |
| Legal | ToS: 18+ or guardian; one account per person. Privacy: collects name, email, phone, bank/e-wallet number; bcrypt; no retention period, no data-subject rights described; no legal entity, registration number, or address | `/page/*` | VERIFIED |
| Disclaimer | States independence from Shopee/TikTok Shop | footer | VERIFIED |
| Notifications | Privacy policy mentions transaction emails and system notices | privacy | VERIFIED (email); in-app UNKNOWN |

### 1.2 Unknown (not observable without an account)
Dashboard layout, order status labels, wallet transaction types, withdrawal form fields and limits, bank account management, referral dashboard and rates, check-in rewards, notification centre, user tiers, mobile deep-link behaviour, which affiliate provider(s) they use, whether they advance funds before provider settlement.

**Rule:** nothing in this list may be specified as "parity". We design these as PROPOSED features and the owner may later supply screenshots from a real account to refine them.

### 1.3 Observations that shape our design
1. **They appear to pay before provider settlement.** "≈7 days after delivery" is faster than typical marketplace affiliate payout cycles (INFERRED). Matching that speed means advancing our own cash. Our ledger makes this an explicit, capped policy (`FINANCIAL_LEDGER.md` §6), default off.
2. **Compliance gap is a differentiation opportunity.** No legal entity, retention, or rights disclosure. We ship these from day one (`COMPLIANCE_CHECKLIST.md`).
3. **Their own timing copy is inconsistent.** We show per-order timelines derived from actual state, labelled as estimates.
4. **Leaderboard is switched off.** Low parity priority; ship behind a flag in a later milestone.

## 2. hoantien365.io.vn (secondary reference)

| Finding | Status |
|---|---|
| Bot-first product: Telegram bot and Zalo; login by Telegram or Zalo ID, no password | VERIFIED |
| Merchants: Shopee, Lazada, TikTok | VERIFIED |
| Public order lookup page (`/orders`), cashback policy page, privacy page | VERIFIED |
| Claims: up to 80% share; category rate table (5–15%); "commission credited next day, paid when order completes"; bank transfer; no stated minimum | VERIFIED (claims, self-reported) |
| WordPress site (asset paths under `wp-content`) | VERIFIED |
| No referral programme on the page | VERIFIED |

Takeaway: a Telegram/Zalo bot channel is a cheap acquisition surface. PROPOSED as a post-V1 epic reusing the same link engine; not in V1 scope.

## 3. What we will not copy
Brand name, logo, theme colour (`#ee4d2d` is also Shopee's brand orange — we avoid it deliberately), copy, images, blog content, or page markup. We reproduce workflows only.

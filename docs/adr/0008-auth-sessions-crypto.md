# ADR-0008: Server-side sessions, Argon2id, Google OIDC, envelope encryption

Status: Proposed · Date: 2026-11-17

## Decision
- Browser sessions: opaque 256-bit token in a `__Host-` cookie (`Secure; HttpOnly; SameSite=Lax; Path=/`). Only the token's SHA-256 is stored server-side. Idle timeout 30 days for users ("remember me") or 1 day otherwise. Admin sessions use 8h absolute and 30 min idle.
- Passwords: Argon2id with parameters `m=19456 KiB, t=2, p=1` (OWASP 2024 baseline), tuned so hashing takes ≤ 300 ms on the VPS. Parameters are stored in the PHC string for rehash-on-login.
- Google: OIDC authorization code flow with PKCE, `state` + `nonce`, `email_verified=true` required. Linking to an existing password account requires the user to be logged in or to confirm by email.
- MFA: TOTP (RFC 6238) is mandatory for every staff role and optional for users. Step-up (fresh password or TOTP within 5 min) is required for withdrawals, bank-account changes, email change and admin money actions. WebAuthn is post-V1.
- CSRF: SameSite=Lax plus a synchronizer token on every state-changing form or `fetch`, checked by middleware.
- Secrets at rest (bank account numbers, provider credentials): AES-256-GCM envelope encryption. The DEK is encrypted by a KEK from a Docker secret file. Rows store `key_id`. Rotation is a background re-encrypt job.

## Consequences
Sessions can be revoked instantly (logout everywhere, password reset, compromise). Session reads cost one indexed DB lookup per request, which is acceptable.

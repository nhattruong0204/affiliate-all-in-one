# Compliance Checklist (Vietnam)

**This is not legal advice.** It lists architectural measures and the questions that need a qualified Vietnamese lawyer or tax advisor. Items marked ⚖️ need professional sign-off before launch. Status as of 2026-11-17.

## 1. Applicable framework (VERIFIED existence; applicability to us ⚖️)
| Instrument | Relevance | Status |
|---|---|---|
| Law on Personal Data Protection No. 91/2025/QH15, effective 1 Jan 2026 | Consent, data-subject rights, security, breach handling | VERIFIED in force |
| Decree 356/2025/ND-CP (replaces Decree 13/2023), effective 1 Jan 2026 | Detailed PDPL rules: basic vs sensitive data, response deadlines for data-subject requests, 5-year grace for small and start-up businesses (Art. 41, per VCCI summary) | VERIFIED via secondary sources; read full text ⚖️ |
| Law on E-commerce No. 122/2025/QH15, effective 1 Jul 2026 | First regulation of affiliate marketing services ("dịch vụ tiếp thị liên kết"): definitions in Art. 3, affiliate obligations in Art. 26 (identity, takedown on request, disclosure) | VERIFIED in force; obligations on a cashback platform ⚖️ |
| Multi-level selling regulation (Decree 40/2018/ND-CP as amended) | Any multi-tier referral compensation may need an MLM licence | ⚖️ (ADR-0014 keeps multi-level off) |
| PIT law and Decree 253/2026/ND-CP (in force 1 Jul 2026) | 10% withholding threshold on payments to individuals without a labour contract raised from 2M to 5M VND per payment. Whether cashback counts as taxable income or as a price rebate is unclear | VERIFIED threshold change via news; characterization ⚖️ |
| Consumer protection law (2023) | Clear terms, no misleading claims ("up to 70%"), complaint handling | ⚖️ |
| Advertising law (amended 2025) | Claims in marketing copy, disclosure of affiliate relationship | ⚖️ |
| Shopee / TikTok / ACCESSTRADE program terms | Contractual: incentive traffic, brand use, prohibited behaviours | OB-4 |

## 2. Checklist

### Corporate & commercial
- [ ] C-CORP-1 ⚖️ Legal entity, business lines registered (incl. e-commerce service if required). Website discloses entity name, tax code, address and contact (the reference product does not; we will).
- [ ] C-CORP-2 ⚖️ Determine whether a cashback site is an "e-commerce platform" or "affiliate marketing service provider" under Law 122/2025, and which notifications or registrations apply.
- [ ] C-CORP-3 Written cashback permission per campaign (OB-4). Routing enforces `cashback_allowed=yes`.
- [ ] C-CORP-4 Trademark clearance for the brand name. No use of Shopee/TikTok marks beyond nominative use plus a disclaimer of independence.

### Personal data (PDPL)
- [ ] C-PD-1 Privacy notice (vi/en): purposes, legal bases, data categories, retention, recipients (providers, email vendor, hosting), cross-border transfer if hosting is outside VN, rights and how to exercise them, contact. Versioned in `legal_documents`.
- [ ] C-PD-2 Granular consents recorded with document version (`consents`). Marketing email opt-in is separate.
- [ ] C-PD-3 Data-subject requests (access/export, correction, deletion, withdraw consent) via `/me/data-export`, `/me/close`, plus a support queue with SLA timers set to the Decree 356 deadlines ⚖️.
- [ ] C-PD-4 ⚖️ Impact assessment dossier (processing and cross-border transfer) if required for our size, or confirm the small-business grace period applies.
- [ ] C-PD-5 Breach response runbook with a notification timeline ⚖️ (`INFRASTRUCTURE.md` §9).
- [ ] C-PD-6 Minimization: no national ID numbers collected in V1. Bank numbers encrypted. Click IPs truncated.
- [ ] C-PD-7 ⚖️ Hosting location: a VPS outside Vietnam may trigger cross-border transfer obligations. Owner decision D-6.
- [ ] C-PD-8 Age: ToS requires 18+ or guardian consent (matches reference). No targeting of minors.

### Consumer & marketing
- [ ] C-CON-1 All cashback figures shown as "ước tính" (estimate) with conditions. "Up to X%" claims must be substantiated by rule configuration.
- [ ] C-CON-2 Clear statement of when cashback becomes withdrawable (settlement-based), minimums, fees, and reversal policy.
- [ ] C-CON-3 Complaint channel and response SLA published.
- [ ] C-CON-4 Affiliate relationship disclosure on landing page and blog posts.

### Tax ⚖️
- [ ] C-TAX-1 Commission revenue: VAT/CIT treatment and invoicing to networks.
- [ ] C-TAX-2 Cashback paid to users: is it a rebate (not income) or taxable income requiring PIT withholding above 5M VND/payment and annual reporting? The design supports withholding (`WITHDRAWAL_DESIGN.md` §8), off by default.
- [ ] C-TAX-3 Referral and check-in rewards: likely taxable income to the recipient. Confirm treatment.
- [ ] C-TAX-4 Record retention period for accounting documents. Our design assumes 10 years.

### Referral / MLM ⚖️
- [ ] C-MLM-1 Confirm single-level, purchase-based referral rewards don't constitute multi-level selling.
- [ ] C-MLM-2 Multi-level stays disabled (`referrals.multilevel`). Any change needs a legal opinion attached to the approval record.

### Payments ⚖️
- [ ] C-PAY-1 Confirm that paying users from the company bank account (no stored-value wallet, no top-up, no P2P) does not require a payment intermediary licence. Our design deliberately avoids custodial-wallet features.
- [ ] C-PAY-2 If MoMo disbursement is pursued: merchant agreement and its KYC obligations.

## 3. Reference-product gaps we close (from COMPETITOR_RESEARCH §1.3)
Legal entity disclosure, data retention, rights procedure, consistent payout timing copy, explicit reversal policy.

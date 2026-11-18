//! Full pages built from the components. V0.1 ships only what the design system needs to be
//! demonstrated and tested: a neutral shell preview (`/`), a component gallery (`/_ui`, served
//! only when enabled) and a 404. The real landing, analyzer and waitlist arrive with AAO-100.

use leptos::prelude::*;

use crate::components::{
    Alert, AlertKind, AppShell, Button, ButtonLink, ButtonSize, ButtonVariant, Card, CardPadding,
    CheckboxField, EmptyState, ErrorState, InputKind, NavItem, Skeleton, SkeletonGroup,
    SkeletonText, TextAreaField, TextField,
};
use crate::document::{PageContext, render_document};
use crate::i18n::{Locale, localize_path, tr};
use crate::icons::{Icon, IconView};

/// Public-site navigation (UI/UX spec §4/§7). Destinations land with AAO-100 (landing sections)
/// and AAO-092 (FAQ); until then they resolve to the 404 page.
#[must_use]
pub fn public_nav(locale: Locale) -> Vec<NavItem> {
    vec![
        NavItem::new(Icon::Home, tr(locale, "nav-home"), "/"),
        NavItem::new(
            Icon::Steps,
            tr(locale, "nav-how-it-works"),
            "/#how-it-works",
        ),
        NavItem::new(Icon::Help, tr(locale, "nav-faq"), "/hoi-dap"),
        NavItem::new(Icon::Mail, tr(locale, "nav-waitlist"), "/#waitlist"),
    ]
}

fn waitlist_cta(locale: Locale) -> NavItem {
    NavItem::new(Icon::Mail, tr(locale, "nav-waitlist"), "/#waitlist")
}

fn finish(ctx: &PageContext, title: &str, description: &str, body: impl IntoView) -> String {
    render_document(ctx, title, description, &body.to_html())
}

/// Neutral "preview" page inside the shell. Makes no cashback claim: no accounts, links or
/// amounts exist yet.
#[must_use]
pub fn shell_preview(ctx: &PageContext) -> String {
    let l = ctx.locale;
    let body = view! {
        <AppShell locale=l theme=ctx.theme path=ctx.path.clone() nav=public_nav(l) cta=waitlist_cta(l)>
            <section class="max-w-2xl">
                <span class="badge">{tr(l, "preview-badge")}</span>
                <h1 class="mt-4 text-display font-semibold">{tr(l, "preview-title")}</h1>
                <p class="mt-4 text-lead text-text-muted">{tr(l, "preview-lead")}</p>
            </section>
        </AppShell>
    };
    finish(
        ctx,
        &tr(l, "preview-meta-title"),
        &tr(l, "preview-meta-description"),
        body,
    )
}

/// 404 inside the shell, with a way home.
#[must_use]
pub fn not_found(ctx: &PageContext) -> String {
    let l = ctx.locale;
    let home = localize_path(l, "/");
    let body = view! {
        <AppShell locale=l theme=ctx.theme path=ctx.path.clone() nav=public_nav(l) cta=waitlist_cta(l)>
            <div class="card">
                <ErrorState
                    locale=l
                    level=1
                    title=tr(l, "not-found-title")
                    description=tr(l, "not-found-body")
                >
                    <ButtonLink href=home variant=ButtonVariant::Secondary>{tr(l, "not-found-home")}</ButtonLink>
                </ErrorState>
            </div>
        </AppShell>
    };
    finish(
        ctx,
        &tr(l, "not-found-title"),
        &tr(l, "not-found-body"),
        body,
    )
}

const SWATCHES: [(&str, &str); 19] = [
    ("bg-bg", "bg"),
    ("bg-surface", "surface"),
    ("bg-surface-2", "surface-2"),
    ("bg-border", "border"),
    ("bg-input-border", "input-border"),
    ("bg-text", "text"),
    ("bg-text-muted", "text-muted"),
    ("bg-brand-50", "brand-50"),
    ("bg-brand-600", "brand-600"),
    ("bg-brand-700", "brand-700"),
    ("bg-accent-500", "accent-500"),
    ("bg-danger", "danger"),
    ("bg-success", "success"),
    ("bg-warning", "warning"),
    ("bg-info", "info"),
    ("bg-state-estimated", "state-estimated"),
    ("bg-state-pending", "state-pending"),
    ("bg-state-approved", "state-approved"),
    ("bg-state-available", "state-available"),
];

/// Every component in every state, for design review, accessibility scans and screenshots.
/// Not part of the public site: the web server serves it only when `WEB_UI_GALLERY` is set.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn gallery(ctx: &PageContext) -> String {
    let l = ctx.locale;
    let swatches = SWATCHES
        .into_iter()
        .map(|(class, label)| {
            view! {
                <div>
                    <div class=format!("swatch {class}")></div>
                    <p class="mt-1.5 text-caption text-text-muted">{label}</p>
                </div>
            }
        })
        .collect_view();
    let cards = [
        (Icon::Steps, "g-card-1-title", "g-card-1-body"),
        (Icon::CheckCircle, "g-card-2-title", "g-card-2-body"),
        (Icon::Mail, "g-card-3-title", "g-card-3-body"),
    ]
    .into_iter()
    .map(|(icon, title, body)| {
        view! {
            <Card>
                <span class="chip-icon"><IconView icon=icon /></span>
                <h3 class="mt-3 text-h3 font-semibold">{tr(l, title)}</h3>
                <p class="mt-1 text-small text-text-muted">{tr(l, body)}</p>
            </Card>
        }
    })
    .collect_view();

    let body = view! {
        <AppShell locale=l theme=ctx.theme path=ctx.path.clone() nav=public_nav(l) cta=waitlist_cta(l)>
            <h1 class="text-h1 font-semibold">{tr(l, "g-title")}</h1>
            <p class="mt-2 max-w-2xl text-text-muted">{tr(l, "g-intro")}</p>

            <section class="mt-10" aria-labelledby="g-type">
                <h2 id="g-type" class="text-h2 font-semibold">{tr(l, "g-typography")}</h2>
                <div class="mt-4 grid gap-3">
                    <p class="text-display font-semibold">{tr(l, "g-sample")}</p>
                    <p class="text-h1 font-semibold">{tr(l, "g-sample")}</p>
                    <p class="text-h2 font-semibold">{tr(l, "g-sample")}</p>
                    <p class="text-h3 font-semibold">{tr(l, "g-sample")}</p>
                    <p class="text-lead text-text-muted">{tr(l, "g-sample")}</p>
                    <p class="text-body">{tr(l, "g-sample")}</p>
                    <p class="text-small text-text-muted">{tr(l, "g-sample")}</p>
                    <p class="text-caption text-text-muted">{tr(l, "g-sample")}</p>
                </div>
            </section>

            <section class="mt-10" aria-labelledby="g-buttons">
                <h2 id="g-buttons" class="text-h2 font-semibold">{tr(l, "g-buttons")}</h2>
                <div class="mt-4 flex flex-wrap items-center gap-3">
                    <Button>{tr(l, "g-btn-primary")}</Button>
                    <Button variant=ButtonVariant::Secondary>{tr(l, "g-btn-secondary")}</Button>
                    <Button variant=ButtonVariant::Ghost>{tr(l, "g-btn-ghost")}</Button>
                    <Button variant=ButtonVariant::Danger>{tr(l, "g-btn-danger")}</Button>
                    <Button disabled=true>{tr(l, "g-btn-disabled")}</Button>
                    <Button loading=true>{tr(l, "g-btn-loading")}</Button>
                    <Button variant=ButtonVariant::Secondary loading=true>{tr(l, "g-btn-loading")}</Button>
                </div>
                <div class="mt-3 flex flex-wrap items-center gap-3">
                    <Button size=ButtonSize::Sm>{tr(l, "g-btn-small")}</Button>
                    <Button size=ButtonSize::Lg>{tr(l, "g-btn-large")}</Button>
                    <ButtonLink href="#g-buttons" variant=ButtonVariant::Secondary>{tr(l, "g-btn-link")}</ButtonLink>
                </div>
            </section>

            <section class="mt-10" aria-labelledby="g-forms">
                <h2 id="g-forms" class="text-h2 font-semibold">{tr(l, "g-forms")}</h2>
                <form class="card card-pad-lg mt-4 grid max-w-xl gap-5" data-submit-guard="" action="#g-forms" method="get" novalidate>
                    <TextField
                        id="g-email"
                        name="email"
                        label=tr(l, "g-email")
                        kind=InputKind::Email
                        placeholder="name@example.com"
                        hint=tr(l, "g-email-hint")
                        autocomplete="email"
                        inputmode="email"
                        required=true
                    />
                    <TextField
                        id="g-email-bad"
                        name="email_bad"
                        label=tr(l, "g-email")
                        kind=InputKind::Email
                        value="name@"
                        error=tr(l, "g-email-error")
                        required=true
                    />
                    <TextField id="g-disabled" name="disabled" label=tr(l, "g-field-disabled") value="—" disabled=true />
                    <TextAreaField id="g-note" name="note" label=tr(l, "g-note") placeholder=tr(l, "g-note-placeholder") rows=3 />
                    <CheckboxField id="g-consent" name="consent" required=true>
                        {tr(l, "g-consent")}" "
                        <a href=localize_path(l, "/chinh-sach-bao-mat")>{tr(l, "footer-privacy")}</a>
                    </CheckboxField>
                    <Button submit=true size=ButtonSize::Lg block=true>{tr(l, "g-submit")}</Button>
                </form>
            </section>

            <section class="mt-10" aria-labelledby="g-cards">
                <h2 id="g-cards" class="text-h2 font-semibold">{tr(l, "g-cards")}</h2>
                <div class="mt-4 grid gap-4 sm:grid-cols-3">{cards}</div>
            </section>

            <section class="mt-10" aria-labelledby="g-alerts">
                <h2 id="g-alerts" class="text-h2 font-semibold">{tr(l, "g-alerts")}</h2>
                <div class="mt-4 grid max-w-2xl gap-3">
                    <Alert kind=AlertKind::Info title=tr(l, "g-alert-info-title")>{tr(l, "g-alert-info-body")}</Alert>
                    <Alert kind=AlertKind::Success title=tr(l, "g-alert-success-title")>{tr(l, "g-alert-success-body")}</Alert>
                    <Alert kind=AlertKind::Warning title=tr(l, "g-alert-warning-title")>{tr(l, "g-alert-warning-body")}</Alert>
                    <Alert kind=AlertKind::Error title=tr(l, "g-alert-error-title")>{tr(l, "g-alert-error-body")}</Alert>
                </div>
            </section>

            <section class="mt-10" aria-labelledby="g-loading">
                <h2 id="g-loading" class="text-h2 font-semibold">{tr(l, "g-loading")}</h2>
                <Card class="mt-4 max-w-xl">
                    <SkeletonGroup label=tr(l, "loading")>
                        <div class="flex gap-4">
                            <Skeleton class="size-20 shrink-0" />
                            <div class="grid flex-1 content-center gap-2.5">
                                <Skeleton class="h-4 w-4/5" />
                                <Skeleton class="h-4 w-3/5" />
                                <Skeleton class="h-9 w-2/5" />
                            </div>
                        </div>
                        <div class="mt-5"><SkeletonText lines=3 /></div>
                    </SkeletonGroup>
                </Card>
            </section>

            <section class="mt-10" aria-labelledby="g-states">
                <h2 id="g-states" class="text-h2 font-semibold">{tr(l, "g-states")}</h2>
                <div class="mt-4 grid gap-4 lg:grid-cols-2">
                    <Card padding=CardPadding::None>
                        <EmptyState icon=Icon::Inbox level=3 title=tr(l, "g-empty-title") description=tr(l, "g-empty-body")>
                            <ButtonLink href="#g-forms">{tr(l, "g-empty-action")}</ButtonLink>
                        </EmptyState>
                    </Card>
                    <Card padding=CardPadding::None>
                        <ErrorState locale=l level=3 retry_href="#g-states" trace_id="01JB7X4K9Q2M" />
                    </Card>
                </div>
            </section>

            <section class="mt-10" aria-labelledby="g-toasts">
                <h2 id="g-toasts" class="text-h2 font-semibold">{tr(l, "g-toasts")}</h2>
                <p class="mt-2 max-w-2xl text-small text-text-muted">{tr(l, "g-toasts-note")}</p>
                <div class="mt-4 flex flex-wrap gap-3" data-requires-js="">
                    <button type="button" class="btn btn-secondary btn-md" data-toast-demo="success" data-demo-message=tr(l, "g-toast-success")>{tr(l, "g-toast-btn-success")}</button>
                    <button type="button" class="btn btn-secondary btn-md" data-toast-demo="info" data-demo-message=tr(l, "g-toast-info")>{tr(l, "g-toast-btn-info")}</button>
                    <button type="button" class="btn btn-secondary btn-md" data-toast-demo="warning" data-demo-message=tr(l, "g-toast-warning")>{tr(l, "g-toast-btn-warning")}</button>
                    <button type="button" class="btn btn-secondary btn-md" data-toast-demo="error" data-demo-message=tr(l, "g-toast-error")>{tr(l, "g-toast-btn-error")}</button>
                </div>
            </section>

            <section class="mt-10" aria-labelledby="g-colors">
                <h2 id="g-colors" class="text-h2 font-semibold">{tr(l, "g-colors")}</h2>
                <div class="mt-4 grid grid-cols-2 gap-3 sm:grid-cols-4 lg:grid-cols-6">{swatches}</div>
            </section>
        </AppShell>
    };
    let mut ctx = ctx.clone();
    ctx.noindex = true; // never index the gallery, even if exposed by mistake
    finish(&ctx, &tr(l, "g-title"), &tr(l, "g-intro"), body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;

    fn ctx(locale: Locale, path: &str) -> PageContext {
        PageContext::new(locale, Theme::System, path)
    }

    #[test]
    fn shell_preview_is_a_complete_localized_document_without_cashback_claims() {
        for (locale, path) in [(Locale::Vi, "/"), (Locale::En, "/en")] {
            let html = shell_preview(&ctx(locale, path));
            assert!(html.starts_with("<!doctype html>"), "{html}");
            assert!(
                html.contains(&format!(r#"<html lang="{}""#, locale.code())),
                "{html}"
            );
            assert_eq!(html.matches("<h1").count(), 1, "exactly one h1: {html}");
            assert!(
                !html.contains('₫') && !html.contains("%"),
                "no amounts or rates on the preview: {html}"
            );
        }
        assert!(shell_preview(&ctx(Locale::En, "/en")).contains("Under construction"));
    }

    #[test]
    fn gallery_is_noindex_and_covers_every_component() {
        let html = gallery(&ctx(Locale::En, "/en/_ui"));
        assert!(
            html.contains(r#"name="robots" content="noindex, nofollow""#),
            "{html}"
        );
        for needle in [
            "btn-primary",
            "btn-secondary",
            "btn-ghost",
            "btn-danger",
            "aria-busy=\"true\"",
            "field-error",
            "check-row",
            "skeleton",
            "state-block",
            "alert-error",
            "data-toast-demo=\"success\"",
            "swatch",
            "data-submit-guard",
        ] {
            assert!(html.contains(needle), "gallery is missing {needle}");
        }
        assert_eq!(html.matches("<h1").count(), 1);
        let ids: Vec<_> = html
            .match_indices(" id=\"")
            .map(|(i, _)| &html[i..i + 40])
            .collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            ids.len(),
            sorted.len(),
            "duplicate element ids on the gallery page"
        );
    }

    #[test]
    fn not_found_has_a_single_h1_and_a_way_home() {
        let html = not_found(&ctx(Locale::En, "/en/nope"));
        assert_eq!(html.matches("<h1").count(), 1, "{html}");
        assert!(
            html.contains("Page not found") && html.contains(r#"href="/en""#),
            "{html}"
        );
    }

    #[test]
    fn public_nav_has_four_destinations() {
        let nav = public_nav(Locale::Vi);
        assert_eq!(nav.len(), 4);
        assert_eq!(nav[0].path, "/");
    }
}

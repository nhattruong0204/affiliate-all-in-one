use leptos::prelude::*;

use super::ButtonLink;
use super::non_empty;
use crate::i18n::{Locale, tr, tr_args};
use crate::icons::{Icon, IconView};

/// Placeholder block shown while content loads. Decorative: wrap groups in [`SkeletonGroup`]
/// so assistive tech hears one "loading" message instead of nothing. Animation stops under
/// `prefers-reduced-motion`. Size it with width/height utilities via `class`.
#[component]
pub fn Skeleton(#[prop(optional, into)] class: String) -> impl IntoView {
    let class = if class.is_empty() {
        "skeleton".to_owned()
    } else {
        format!("skeleton {class}")
    };
    view! { <span class=class aria-hidden="true"></span> }
}

/// Announces loading politely and marks the region busy; children are the skeleton shapes.
#[component]
pub fn SkeletonGroup(
    #[prop(into)] label: String,
    #[prop(optional, into)] class: String,
    children: Children,
) -> impl IntoView {
    view! {
        <div class=non_empty(class) role="status" aria-busy="true">
            <span class="sr-only">{label}</span>
            <div aria-hidden="true">{children()}</div>
        </div>
    }
}

/// A few lines of text-shaped skeleton with a ragged right edge.
#[component]
pub fn SkeletonText(#[prop(default = 3)] lines: usize) -> impl IntoView {
    const WIDTHS: [&str; 3] = ["h-4 w-full", "h-4 w-11/12", "h-4 w-3/5"];
    let rows = (0..lines)
        .map(|i| {
            let width = if i + 1 == lines && lines > 1 {
                WIDTHS[2]
            } else {
                WIDTHS[i % 2]
            };
            view! { <Skeleton class=width /> }
        })
        .collect_view();
    view! { <div class="grid gap-2.5">{rows}</div> }
}

fn heading(level: u8, class: &'static str, text: String) -> AnyView {
    match level {
        1 => view! { <h1 class=class>{text}</h1> }.into_any(),
        3 => view! { <h3 class=class>{text}</h3> }.into_any(),
        _ => view! { <h2 class=class>{text}</h2> }.into_any(),
    }
}

/// "Nothing here yet" with an explanation and (optionally) the next action as children.
#[component]
pub fn EmptyState(
    icon: Icon,
    #[prop(into)] title: String,
    #[prop(into)] description: String,
    #[prop(default = 2)] level: u8,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    let actions = children.map(|c| view! { <div class="state-actions">{c()}</div> });
    view! {
        <div class="state-block">
            <span class="state-icon"><IconView icon=icon /></span>
            {heading(level, "state-title", title)}
            <p class="state-body">{description}</p>
            {actions}
        </div>
    }
}

/// A failure the user can act on: what happened, what to do, an optional retry link and a
/// reference id for support. Title/description default to the generic localized copy.
/// `announce` adds `role="alert"` for errors that appear after the page has loaded.
#[component]
pub fn ErrorState(
    locale: Locale,
    #[prop(optional, into)] title: String,
    #[prop(optional, into)] description: String,
    #[prop(optional, into)] retry_href: String,
    #[prop(optional, into)] trace_id: String,
    #[prop(default = 2)] level: u8,
    #[prop(optional)] announce: bool,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    let title = non_empty(title).unwrap_or_else(|| tr(locale, "error-generic-title"));
    let description = non_empty(description).unwrap_or_else(|| tr(locale, "error-generic-body"));
    let retry = non_empty(retry_href).map(|href| {
        view! {
            <ButtonLink href=href>
                <IconView icon=Icon::Refresh />
                {tr(locale, "action-retry")}
            </ButtonLink>
        }
    });
    let extra = children.map(|c| c());
    let has_actions = retry.is_some() || extra.is_some();
    let actions = has_actions.then(|| view! { <div class="state-actions">{retry}{extra}</div> });
    let trace = non_empty(trace_id).map(
        |id| view! { <p class="trace-id">{tr_args(locale, "error-trace-id", &[("id", &id)])}</p> },
    );
    let role = announce.then_some("alert");
    view! {
        <div class="state-block" role=role>
            <span class="state-icon state-icon-error"><IconView icon=Icon::Alert /></span>
            {heading(level, "state-title", title)}
            <p class="state-body">{description}</p>
            {actions}
            {trace}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skeletons_are_hidden_from_assistive_tech_but_group_announces() {
        let html =
            view! { <SkeletonGroup label="Loading…"><SkeletonText lines=3 /></SkeletonGroup> }
                .to_html();
        assert!(
            html.contains(r#"role="status""#) && html.contains(r#"aria-busy="true""#),
            "{html}"
        );
        assert!(
            html.contains("sr-only") && html.contains("Loading…"),
            "{html}"
        );
        assert_eq!(
            html.matches(r#"aria-hidden="true""#).count(),
            4,
            "3 lines + wrapper: {html}"
        );
        assert!(html.contains("w-3/5"), "last line is shorter: {html}");
    }

    #[test]
    fn empty_state_renders_title_description_and_action() {
        let html = view! {
            <EmptyState icon=Icon::Inbox title="No orders" description="Orders appear here." level=3>
                <ButtonLink href="/x">"Start"</ButtonLink>
            </EmptyState>
        }
        .to_html();
        assert!(
            html.contains("<h3")
                && html.contains("No orders")
                && html.contains("Orders appear here."),
            "{html}"
        );
        assert!(
            html.contains("state-actions") && html.contains("Start"),
            "{html}"
        );
    }

    #[test]
    fn error_state_defaults_retry_and_trace() {
        let html = view! { <ErrorState locale=Locale::En retry_href="/again" trace_id="01ABC" announce=true /> }.to_html();
        assert!(html.contains("Something went wrong"), "{html}");
        assert!(
            html.contains(r#"href="/again""#) && html.contains("Try again"),
            "{html}"
        );
        assert!(html.contains("Reference: 01ABC"), "{html}");
        assert!(html.contains(r#"role="alert""#), "{html}");
        let quiet = view! { <ErrorState locale=Locale::Vi /> }.to_html();
        assert!(
            !quiet.contains("role=")
                && !quiet.contains("state-actions")
                && !quiet.contains("trace-id"),
            "{quiet}"
        );
    }
}

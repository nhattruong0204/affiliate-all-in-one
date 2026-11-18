use leptos::prelude::*;

use crate::i18n::{Locale, tr};
use crate::icons::{Icon, IconView};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AlertKind {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

/// Inline message. Errors use `role="alert"` (announced immediately); the rest use `role="status"`.
/// Meaning is carried by icon + text, not colour alone.
#[component]
pub fn Alert(
    #[prop(optional)] kind: AlertKind,
    #[prop(optional, into)] title: String,
    children: Children,
) -> impl IntoView {
    let (class, icon, role) = match kind {
        AlertKind::Info => ("alert alert-info", Icon::Info, "status"),
        AlertKind::Success => ("alert alert-success", Icon::CheckCircle, "status"),
        AlertKind::Warning => ("alert alert-warning", Icon::Alert, "status"),
        AlertKind::Error => ("alert alert-error", Icon::Alert, "alert"),
    };
    let title = super::non_empty(title).map(|t| view! { <p class="alert-title">{t}</p> });
    view! {
        <div class=class role=role>
            <IconView icon=icon />
            <div>
                {title}
                <div>{children()}</div>
            </div>
        </div>
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToastKind {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

impl ToastKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }

    fn title_key(self) -> &'static str {
        match self {
            Self::Info => "toast-title-info",
            Self::Success => "toast-title-success",
            Self::Warning => "toast-title-warning",
            Self::Error => "toast-title-error",
        }
    }
}

/// Shared toast markup: used for server-rendered toasts and as the template that
/// `window.AAO.toast()` clones. All four kind icons are present; CSS shows the one matching
/// `data-kind`.
#[component]
fn ToastBody(
    locale: Locale,
    kind: ToastKind,
    #[prop(into)] title: String,
    #[prop(into)] message: String,
    #[prop(optional, into)] timeout: String,
) -> impl IntoView {
    let role = if kind == ToastKind::Error {
        "alert"
    } else {
        "status"
    };
    view! {
        <div
            class="toast"
            data-toast=""
            data-kind=kind.as_str()
            data-timeout=super::non_empty(timeout)
            role=role
        >
            <span class="toast-icon" data-icon="info"><IconView icon=Icon::Info /></span>
            <span class="toast-icon" data-icon="success"><IconView icon=Icon::CheckCircle /></span>
            <span class="toast-icon" data-icon="warning"><IconView icon=Icon::Alert /></span>
            <span class="toast-icon" data-icon="error"><IconView icon=Icon::Alert /></span>
            <div class="min-w-0">
                <p class="toast-title" data-toast-title="">{title}</p>
                <p class="toast-message" data-toast-message="">{message}</p>
            </div>
            <button type="button" class="toast-close" data-toast-close="" aria-label=tr(locale, "toast-close")>
                <IconView icon=Icon::Close />
            </button>
        </div>
    }
}

/// A server-rendered toast (e.g. a flash message after a redirect). Inside a [`ToastRegion`]
/// it is announced politely; `timeout_ms` > 0 auto-dismisses it (never use for errors).
#[component]
pub fn Toast(
    locale: Locale,
    #[prop(optional)] kind: ToastKind,
    #[prop(optional, into)] title: String,
    #[prop(into)] message: String,
    #[prop(optional)] timeout_ms: u32,
) -> impl IntoView {
    let title = if title.is_empty() {
        tr(locale, kind.title_key())
    } else {
        title
    };
    let timeout = if timeout_ms > 0 {
        timeout_ms.to_string()
    } else {
        String::new()
    };
    view! { <ToastBody locale=locale kind=kind title=title message=message timeout=timeout /> }
}

/// The live region every page includes once (the app shell does this). It carries localized
/// titles and the toast template for `window.AAO.toast()`; server-rendered [`Toast`]s may be
/// passed as children.
#[component]
pub fn ToastRegion(locale: Locale, #[prop(optional)] children: Option<Children>) -> impl IntoView {
    view! {
        <div
            class="toast-region"
            data-toast-region=""
            aria-live="polite"
            data-title-info=tr(locale, "toast-title-info")
            data-title-success=tr(locale, "toast-title-success")
            data-title-warning=tr(locale, "toast-title-warning")
            data-title-error=tr(locale, "toast-title-error")
        >
            <template data-toast-template="">
                <ToastBody locale=locale kind=ToastKind::Info title="" message="" />
            </template>
            {children.map(|c| c())}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alert_roles_follow_severity() {
        let err = view! { <Alert kind=AlertKind::Error title="Oops">"Bad"</Alert> }.to_html();
        assert!(
            err.contains(r#"role="alert""#) && err.contains("alert-error") && err.contains("Oops"),
            "{err}"
        );
        let info = view! { <Alert>"Fyi"</Alert> }.to_html();
        assert!(
            info.contains(r#"role="status""#) && !info.contains("alert-title"),
            "{info}"
        );
    }

    #[test]
    fn region_is_a_polite_live_region_with_template_and_titles() {
        let html = view! { <ToastRegion locale=Locale::En /> }.to_html();
        assert!(html.contains(r#"aria-live="polite""#), "{html}");
        assert!(
            html.contains("data-toast-template") && html.contains("<template"),
            "{html}"
        );
        assert!(html.contains(r#"data-title-success="Success""#), "{html}");
        assert!(
            html.contains(r#"data-title-error="Something went wrong""#),
            "{html}"
        );
    }

    #[test]
    fn server_toast_has_localized_title_close_label_and_optional_timeout() {
        let html = view! { <Toast locale=Locale::Vi kind=ToastKind::Success message="Đã lưu" timeout_ms=5000 /> }.to_html();
        assert!(
            html.contains("Thành công") && html.contains("Đã lưu"),
            "{html}"
        );
        assert!(
            html.contains(r#"data-kind="success""#) && html.contains(r#"data-timeout="5000""#),
            "{html}"
        );
        assert!(html.contains(r#"aria-label="Đóng thông báo""#), "{html}");
        let err = view! { <Toast locale=Locale::En kind=ToastKind::Error message="x" /> }.to_html();
        assert!(
            err.contains(r#"role="alert""#) && !err.contains("data-timeout"),
            "errors never auto-dismiss: {err}"
        );
    }
}

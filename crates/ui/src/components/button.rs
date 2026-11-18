use leptos::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Ghost,
    Danger,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonSize {
    Sm,
    #[default]
    Md,
    Lg,
}

fn classes(variant: ButtonVariant, size: ButtonSize, block: bool, extra: &str) -> String {
    let variant = match variant {
        ButtonVariant::Primary => "btn-primary",
        ButtonVariant::Secondary => "btn-secondary",
        ButtonVariant::Ghost => "btn-ghost",
        ButtonVariant::Danger => "btn-danger",
    };
    let size = match size {
        ButtonSize::Sm => "btn-sm",
        ButtonSize::Md => "btn-md",
        ButtonSize::Lg => "btn-lg",
    };
    let mut out = format!("btn {variant} {size}");
    if block {
        out.push_str(" btn-block");
    }
    if !extra.is_empty() {
        out.push(' ');
        out.push_str(extra);
    }
    out
}

/// A `<button>`. Defaults to `type="button"` so it never submits a form by accident.
///
/// `loading` shows a spinner and sets `aria-busy`; pair forms with `data-submit-guard` so a second
/// click cannot resubmit.
// Four independent state flags mirror the HTML attributes; an enum would only obscure call sites.
#[allow(clippy::fn_params_excessive_bools)]
#[component]
pub fn Button(
    #[prop(optional)] variant: ButtonVariant,
    #[prop(optional)] size: ButtonSize,
    #[prop(optional)] submit: bool,
    #[prop(optional)] loading: bool,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] block: bool,
    #[prop(optional, into)] class: String,
    children: Children,
) -> impl IntoView {
    let class = classes(variant, size, block, &class);
    let kind = if submit { "submit" } else { "button" };
    let busy = loading.then_some("true");
    view! {
        <button type=kind class=class disabled=disabled aria-busy=busy>
            {children()}
        </button>
    }
}

/// An `<a>` styled as a button, for navigation (use [`Button`] for actions).
#[component]
pub fn ButtonLink(
    #[prop(into)] href: String,
    #[prop(optional)] variant: ButtonVariant,
    #[prop(optional)] size: ButtonSize,
    #[prop(optional)] block: bool,
    #[prop(optional, into)] class: String,
    children: Children,
) -> impl IntoView {
    let class = classes(variant, size, block, &class);
    view! {
        <a href=href class=class>
            {children()}
        </a>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(view: impl IntoView) -> String {
        view.to_html()
    }

    #[test]
    fn button_defaults_to_non_submitting() {
        let html = render(view! { <Button>"Go"</Button> });
        assert!(html.contains(r#"type="button""#), "{html}");
        assert!(html.contains("btn btn-primary btn-md"), "{html}");
        assert!(!html.contains("aria-busy"), "{html}");
        assert!(!html.contains("disabled"), "{html}");
    }

    #[test]
    fn button_states_map_to_attributes() {
        let html = render(view! {
            <Button variant=ButtonVariant::Danger size=ButtonSize::Lg submit=true loading=true block=true class="mt-2">"X"</Button>
        });
        assert!(html.contains(r#"type="submit""#), "{html}");
        assert!(html.contains(r#"aria-busy="true""#), "{html}");
        assert!(
            html.contains("btn btn-danger btn-lg btn-block mt-2"),
            "{html}"
        );
        let disabled = render(view! { <Button disabled=true>"X"</Button> });
        assert!(disabled.contains("disabled"), "{disabled}");
    }

    #[test]
    fn button_link_renders_anchor() {
        let html = render(
            view! { <ButtonLink href="/en" variant=ButtonVariant::Secondary>"Home"</ButtonLink> },
        );
        assert!(html.starts_with("<a "), "{html}");
        assert!(
            html.contains(r#"href="/en""#) && html.contains("btn-secondary"),
            "{html}"
        );
    }
}

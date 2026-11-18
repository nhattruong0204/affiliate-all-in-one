use leptos::prelude::*;

use super::non_empty;
use crate::icons::{Icon, IconView};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InputKind {
    #[default]
    Text,
    Email,
    Password,
    Tel,
    Url,
    Search,
}

impl InputKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Email => "email",
            Self::Password => "password",
            Self::Tel => "tel",
            Self::Url => "url",
            Self::Search => "search",
        }
    }
}

/// `aria-describedby` value pointing at whichever of hint/error are rendered.
fn described_by(id: &str, has_hint: bool, has_error: bool) -> Option<String> {
    let mut ids = vec![];
    if has_hint {
        ids.push(format!("{id}-hint"));
    }
    if has_error {
        ids.push(format!("{id}-error"));
    }
    (!ids.is_empty()).then(|| ids.join(" "))
}

/// Hint and error paragraphs shared by all field types.
fn messages(id: &str, hint: String, error: String) -> impl IntoView + use<> {
    let hint_view = non_empty(hint).map(|text| {
        view! { <p class="field-hint" id=format!("{id}-hint")>{text}</p> }
    });
    let error_view = non_empty(error).map(|text| {
        view! {
            <p class="field-error" id=format!("{id}-error")>
                <IconView icon=Icon::Alert />
                <span>{text}</span>
            </p>
        }
    });
    (hint_view, error_view)
}

/// Label + `<input>` + hint + error, wired with `for`/`id`/`aria-describedby`/`aria-invalid`.
/// Errors are shown with an icon and text (never colour alone).
#[component]
pub fn TextField(
    #[prop(into)] id: String,
    #[prop(into)] name: String,
    #[prop(into)] label: String,
    #[prop(optional)] kind: InputKind,
    #[prop(optional, into)] value: String,
    #[prop(optional, into)] placeholder: String,
    #[prop(optional, into)] hint: String,
    #[prop(optional, into)] error: String,
    #[prop(optional, into)] autocomplete: String,
    #[prop(optional, into)] inputmode: String,
    #[prop(optional)] required: bool,
    #[prop(optional)] disabled: bool,
) -> impl IntoView {
    let described = described_by(&id, !hint.is_empty(), !error.is_empty());
    let invalid = (!error.is_empty()).then_some("true");
    let star =
        required.then(|| view! { <span class="field-required" aria-hidden="true">"*"</span> });
    let notes = messages(&id, hint, error);
    let label_for = id.clone();
    view! {
        <div class="field">
            <label class="field-label" for=label_for>
                {label}
                {star}
            </label>
            <input
                class="input"
                id=id
                name=name
                type=kind.as_str()
                value=value
                placeholder=non_empty(placeholder)
                autocomplete=non_empty(autocomplete)
                inputmode=non_empty(inputmode)
                required=required
                disabled=disabled
                aria-invalid=invalid
                aria-describedby=described
            />
            {notes}
        </div>
    }
}

/// Multi-line variant of [`TextField`].
#[component]
pub fn TextAreaField(
    #[prop(into)] id: String,
    #[prop(into)] name: String,
    #[prop(into)] label: String,
    #[prop(optional, into)] value: String,
    #[prop(optional, into)] placeholder: String,
    #[prop(optional, into)] hint: String,
    #[prop(optional, into)] error: String,
    #[prop(default = 4)] rows: u32,
    #[prop(optional)] required: bool,
    #[prop(optional)] disabled: bool,
) -> impl IntoView {
    let described = described_by(&id, !hint.is_empty(), !error.is_empty());
    let invalid = (!error.is_empty()).then_some("true");
    let star =
        required.then(|| view! { <span class="field-required" aria-hidden="true">"*"</span> });
    let notes = messages(&id, hint, error);
    let label_for = id.clone();
    view! {
        <div class="field">
            <label class="field-label" for=label_for>
                {label}
                {star}
            </label>
            <textarea
                class="input"
                id=id
                name=name
                rows=rows.to_string()
                placeholder=non_empty(placeholder)
                required=required
                disabled=disabled
                aria-invalid=invalid
                aria-describedby=described
            >
                {value}
            </textarea>
            {notes}
        </div>
    }
}

/// Checkbox with rich label content (e.g. consent text containing links). The whole row is a
/// 44 px tap target.
#[component]
pub fn CheckboxField(
    #[prop(into)] id: String,
    #[prop(into)] name: String,
    #[prop(optional)] checked: bool,
    #[prop(optional)] required: bool,
    #[prop(optional, into)] error: String,
    children: Children,
) -> impl IntoView {
    let described = described_by(&id, false, !error.is_empty());
    let invalid = (!error.is_empty()).then_some("true");
    let notes = messages(&id, String::new(), error);
    let label_for = id.clone();
    view! {
        <div class="field">
            <div class="check-row">
                <input
                    type="checkbox"
                    id=id
                    name=name
                    value="on"
                    checked=checked
                    required=required
                    aria-invalid=invalid
                    aria-describedby=described
                />
                <label for=label_for>{children()}</label>
            </div>
            {notes}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_field_wires_label_hint_and_error() {
        let html = view! {
            <TextField id="email" name="email" label="Email" kind=InputKind::Email required=true
                hint="We only use it once." error="Invalid email" autocomplete="email" />
        }
        .to_html();
        assert!(html.contains(r#"for="email""#), "{html}");
        assert!(
            html.contains(r#"id="email""#) && html.contains(r#"type="email""#),
            "{html}"
        );
        assert!(
            html.contains(r#"aria-describedby="email-hint email-error""#),
            "{html}"
        );
        assert!(html.contains(r#"aria-invalid="true""#), "{html}");
        assert!(
            html.contains(r#"id="email-hint""#) && html.contains(r#"id="email-error""#),
            "{html}"
        );
        assert!(
            html.contains("required") && html.contains(r#"autocomplete="email""#),
            "{html}"
        );
        assert!(
            html.contains(r#"aria-hidden="true">*"#),
            "asterisk must be decorative: {html}"
        );
    }

    #[test]
    fn text_field_without_messages_has_no_aria_noise() {
        let html = view! { <TextField id="n" name="n" label="Name" /> }.to_html();
        assert!(
            !html.contains("aria-describedby") && !html.contains("aria-invalid"),
            "{html}"
        );
        assert!(
            !html.contains("autocomplete") && !html.contains("inputmode"),
            "{html}"
        );
        assert!(!html.contains("required"), "{html}");
    }

    #[test]
    fn textarea_and_checkbox_are_labelled() {
        let t = view! { <TextAreaField id="msg" name="msg" label="Note" rows=3 hint="Optional" /> }
            .to_html();
        assert!(
            t.contains(r#"rows="3""#) && t.contains(r#"aria-describedby="msg-hint""#),
            "{t}"
        );
        let c = view! { <CheckboxField id="ok" name="ok" required=true error="Required">"I agree"</CheckboxField> }.to_html();
        assert!(
            c.contains(r#"type="checkbox""#) && c.contains(r#"for="ok""#),
            "{c}"
        );
        assert!(
            c.contains(r#"aria-describedby="ok-error""#) && c.contains("I agree"),
            "{c}"
        );
    }

    #[test]
    fn user_values_are_escaped() {
        let html = view! { <TextField id="x" name="x" label="L" value=r#""><script>alert(1)</script>"# /> }.to_html();
        assert!(!html.contains("<script>"), "value must be escaped: {html}");
    }
}

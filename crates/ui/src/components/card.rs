use leptos::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CardPadding {
    None,
    #[default]
    Md,
    Lg,
}

/// A bordered surface (`--color-surface`, `--radius-card`). Compose headings and content inside.
#[component]
pub fn Card(
    #[prop(optional)] padding: CardPadding,
    #[prop(optional, into)] class: String,
    children: Children,
) -> impl IntoView {
    let pad = match padding {
        CardPadding::None => "",
        CardPadding::Md => " card-pad-md",
        CardPadding::Lg => " card-pad-lg",
    };
    let extra = if class.is_empty() {
        String::new()
    } else {
        format!(" {class}")
    };
    let class = format!("card{pad}{extra}");
    view! { <div class=class>{children()}</div> }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_padding_and_extra_classes() {
        let html = view! { <Card padding=CardPadding::Lg class="mt-4">"x"</Card> }.to_html();
        assert!(html.contains(r#"class="card card-pad-lg mt-4""#), "{html}");
        let none = view! { <Card padding=CardPadding::None>"x"</Card> }.to_html();
        assert!(none.contains(r#"class="card""#), "{none}");
    }
}

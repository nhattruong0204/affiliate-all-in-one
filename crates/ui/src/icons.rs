//! Original stroke icons (24×24, `currentColor`), inlined so there is no icon font, no sprite
//! request and no third-party asset. Decorative by default (`aria-hidden`); give the surrounding
//! control an accessible name instead.

use leptos::prelude::*;

/// Icons available to components. Add new ones here, not ad hoc in markup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Home,
    Steps,
    Help,
    Mail,
    Sun,
    Moon,
    CheckCircle,
    Info,
    Alert,
    Close,
    Inbox,
    Refresh,
}

const SVG_OPEN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">"#;

impl Icon {
    fn body(self) -> &'static str {
        match self {
            Self::Home => {
                r#"<path d="M3.5 11 12 4l8.5 7"/><path d="M5.5 9.7V19a1 1 0 0 0 1 1h11a1 1 0 0 0 1-1V9.7"/><path d="M10 20v-5.5h4V20"/>"#
            }
            Self::Steps => {
                r#"<path d="M9.5 6.5H20M9.5 12H20M9.5 17.5H20"/><path d="m3.8 6.6 1.1 1.1 2-2.2M3.8 12.1l1.1 1.1 2-2.2M3.8 17.6l1.1 1.1 2-2.2"/>"#
            }
            Self::Help => {
                r#"<circle cx="12" cy="12" r="8.75"/><path d="M9.6 9.4a2.5 2.5 0 1 1 3.5 2.3c-.7.35-1.1.9-1.1 1.8"/><path d="M12 16.9h.01"/>"#
            }
            Self::Mail => {
                r#"<rect x="3" y="5" width="18" height="14" rx="2.5"/><path d="m4 7.5 8 5.5 8-5.5"/>"#
            }
            Self::Sun => {
                r#"<circle cx="12" cy="12" r="3.8"/><path d="M12 3v2M12 19v2M3 12h2M19 12h2M5.6 5.6 7 7M17 17l1.4 1.4M18.4 5.6 17 7M7 17l-1.4 1.4"/>"#
            }
            Self::Moon => r#"<path d="M20 14.2A8 8 0 0 1 9.8 4 8 8 0 1 0 20 14.2Z"/>"#,
            Self::CheckCircle => {
                r#"<circle cx="12" cy="12" r="8.75"/><path d="m8.4 12.4 2.5 2.5 4.7-5.2"/>"#
            }
            Self::Info => r#"<circle cx="12" cy="12" r="8.75"/><path d="M12 11v5.2M12 7.9h.01"/>"#,
            Self::Alert => {
                r#"<path d="M12 4 2.9 19.5h18.2L12 4Z"/><path d="M12 10v4.4M12 17.2h.01"/>"#
            }
            Self::Close => r#"<path d="M6 6l12 12M18 6 6 18"/>"#,
            Self::Inbox => {
                r#"<path d="M3.5 13.5H8l1.4 2.8h5.2l1.4-2.8h4.5"/><path d="m3.5 13.5 2.6-7a1 1 0 0 1 .94-.65h9.9a1 1 0 0 1 .94.65l2.6 7V18a1.5 1.5 0 0 1-1.5 1.5H5A1.5 1.5 0 0 1 3.5 18v-4.5Z"/>"#
            }
            Self::Refresh => {
                r#"<path d="M19.5 10.5A7.8 7.8 0 0 0 5.9 7.3M4.5 4.5v3.4h3.4"/><path d="M4.5 13.5a7.8 7.8 0 0 0 13.6 3.2M19.5 19.5v-3.4h-3.4"/>"#
            }
        }
    }

    /// Complete `<svg>` markup.
    #[must_use]
    pub fn svg(self) -> String {
        format!("{SVG_OPEN}{}</svg>", self.body())
    }
}

/// Renders an [`Icon`]. The wrapper uses `display: contents`, so it never affects layout.
#[component]
pub fn IconView(icon: Icon) -> impl IntoView {
    view! { <span class="contents" inner_html=icon.svg()></span> }
}

/// Brand mark (original). Colours come from tokens so it follows the theme.
#[component]
pub fn LogoMark() -> impl IntoView {
    view! { <span class="contents" inner_html=LOGO_SVG></span> }
}

const LOGO_SVG: &str = r#"<svg class="logo-mark" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32" aria-hidden="true" focusable="false"><rect width="32" height="32" rx="9" fill="var(--color-brand-600)"/><path d="M9.6 15.4a6.4 6.4 0 0 1 11-3.9M22.4 16.6a6.4 6.4 0 0 1-11 3.9" fill="none" stroke="var(--color-on-brand)" stroke-width="2.3" stroke-linecap="round"/><path d="m20.9 8.1.3 3.9-3.9-.1M11.1 23.9l-.3-3.9 3.9.1" fill="none" stroke="var(--color-on-brand)" stroke-width="2.3" stroke-linecap="round" stroke-linejoin="round"/><circle cx="16" cy="16" r="2.3" fill="var(--color-accent-500)"/></svg>"#;

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Icon; 12] = [
        Icon::Home,
        Icon::Steps,
        Icon::Help,
        Icon::Mail,
        Icon::Sun,
        Icon::Moon,
        Icon::CheckCircle,
        Icon::Info,
        Icon::Alert,
        Icon::Close,
        Icon::Inbox,
        Icon::Refresh,
    ];

    #[test]
    fn every_icon_is_well_formed_and_decorative() {
        for icon in ALL {
            let svg = icon.svg();
            assert!(
                svg.starts_with("<svg ") && svg.ends_with("</svg>"),
                "{icon:?}"
            );
            assert!(
                svg.contains(r#"aria-hidden="true""#),
                "{icon:?} must be decorative"
            );
            assert!(svg.contains("currentColor"), "{icon:?} must inherit colour");
            assert_eq!(svg.matches("<svg").count(), 1, "{icon:?}");
        }
    }
}

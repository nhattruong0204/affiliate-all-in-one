use leptos::prelude::*;

use crate::i18n::{Locale, localize_path, neutral_path, tr};
use crate::icons::{Icon, IconView, LogoMark};
use crate::theme::Theme;

use super::ToastRegion;

/// One navigation destination. `path` is locale-neutral (`/hoi-dap`) or a fragment (`#faq`);
/// the shell adds the `/en` prefix where needed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavItem {
    pub icon: Icon,
    pub label: String,
    pub path: String,
}

impl NavItem {
    #[must_use]
    pub fn new(icon: Icon, label: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            icon,
            label: label.into(),
            path: path.into(),
        }
    }
}

/// Sun/moon button. The server renders the right icon from the theme cookie / OS preference
/// (CSS only), so there is no flash; `app.js` flips the theme and stores the cookie. Hidden when
/// scripting is off, because it could not work.
#[component]
pub fn ThemeToggle(locale: Locale, theme: Theme) -> impl IntoView {
    let pressed = if theme == Theme::Dark {
        "true"
    } else {
        "false"
    };
    view! {
        <button
            type="button"
            class="icon-button"
            data-theme-toggle=""
            data-requires-js=""
            aria-label=tr(locale, "theme-toggle")
            aria-pressed=pressed
        >
            <span class="theme-icon-sun"><IconView icon=Icon::Sun /></span>
            <span class="theme-icon-moon"><IconView icon=Icon::Moon /></span>
        </button>
    }
}

/// VI | EN links to the same page in the other language. Plain links, so it works without JS
/// and crawlers see both versions; `app.js` additionally remembers the choice.
#[component]
pub fn LanguageSwitch(locale: Locale, #[prop(into)] path: String) -> impl IntoView {
    let links = Locale::ALL
        .into_iter()
        .map(|target| {
            let current = (target == locale).then_some("true");
            view! {
                <a
                    href=localize_path(target, neutral_path(&path))
                    lang=target.code()
                    hreflang=target.code()
                    aria-current=current
                    data-lang-link=target.code()
                >
                    {target.code().to_uppercase()}
                </a>
            }
        })
        .collect_view();
    view! {
        <div class="lang-switch" role="group" aria-label=tr(locale, "language-switch")>
            {links}
        </div>
    }
}

/// Page chrome: skip link, sticky header (logo, desktop nav, language, theme, optional CTA),
/// main landmark, footer, mobile bottom navigation and the toast region.
///
/// Navigation is data (`nav`), so later releases add Orders/Wallet/Account without touching the
/// shell. `path` is the current locale-neutral path, used for `aria-current` and the language
/// switch.
#[component]
pub fn AppShell(
    locale: Locale,
    theme: Theme,
    #[prop(into)] path: String,
    nav: Vec<NavItem>,
    #[prop(optional)] cta: Option<NavItem>,
    children: Children,
) -> impl IntoView {
    let current_path = neutral_path(&path).to_owned();
    let is_current = |item: &NavItem| (item.path == current_path).then_some("page");

    let desktop_links = nav
        .iter()
        .map(|item| {
            view! {
                <a class="nav-link" href=localize_path(locale, &item.path) aria-current=is_current(item)>
                    {item.label.clone()}
                </a>
            }
        })
        .collect_view();
    let bottom_links = nav
        .iter()
        .map(|item| {
            view! {
                <li>
                    <a class="bottom-nav-link" href=localize_path(locale, &item.path) aria-current=is_current(item)>
                        <span class="nav-icon"><IconView icon=item.icon /></span>
                        <span>{item.label.clone()}</span>
                    </a>
                </li>
            }
        })
        .collect_view();
    let cta = cta.map(|item| {
        view! {
            <a class="btn btn-primary btn-sm hidden lg:inline-flex" href=localize_path(locale, &item.path)>
                {item.label}
            </a>
        }
    });

    view! {
        <a class="skip-link" href="#main">{tr(locale, "skip-to-content")}</a>
        <header class="site-header">
            <div class="container-page header-inner">
                <a class="logo" href=localize_path(locale, "/")>
                    <LogoMark />
                    <span class="logo-word">{tr(locale, "brand-name")}</span>
                </a>
                <nav class="desktop-nav" aria-label=tr(locale, "nav-primary")>{desktop_links}</nav>
                <div class="header-actions">
                    <LanguageSwitch locale=locale path=path.clone() />
                    <ThemeToggle locale=locale theme=theme />
                    {cta}
                </div>
            </div>
        </header>
        <main id="main" tabindex="-1" class="page-main">
            <div class="container-page">{children()}</div>
        </main>
        <footer class="site-footer">
            <div class="container-page footer-inner">
                <p>{tr(locale, "footer-tagline")}</p>
                <nav class="footer-links" aria-label=tr(locale, "nav-legal")>
                    <a href=localize_path(locale, "/dieu-khoan")>{tr(locale, "footer-terms")}</a>
                    <a href=localize_path(locale, "/chinh-sach-bao-mat")>{tr(locale, "footer-privacy")}</a>
                </nav>
            </div>
        </footer>
        <nav class="bottom-nav" aria-label=tr(locale, "nav-primary")>
            <ul>{bottom_links}</ul>
        </nav>
        <ToastRegion locale=locale />
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nav() -> Vec<NavItem> {
        vec![
            NavItem::new(Icon::Home, "Home", "/"),
            NavItem::new(Icon::Help, "FAQ", "/hoi-dap"),
        ]
    }

    fn shell(locale: Locale, theme: Theme, path: &str) -> String {
        view! {
            <AppShell locale=locale theme=theme path=path.to_owned() nav=nav()
                cta=NavItem::new(Icon::Mail, "Join", "/#waitlist")>
                <h1>"Content"</h1>
            </AppShell>
        }
        .to_html()
    }

    #[test]
    fn has_landmarks_skip_link_and_single_main() {
        let html = shell(Locale::En, Theme::System, "/");
        assert!(
            html.contains(r##"href="#main""##) && html.contains("Skip to main content"),
            "{html}"
        );
        assert_eq!(html.matches("<main").count(), 1, "{html}");
        assert!(
            html.contains(r#"id="main""#) && html.contains(r#"tabindex="-1""#),
            "{html}"
        );
        assert!(
            html.contains("<header") && html.contains("<footer"),
            "{html}"
        );
        assert!(html.contains("<h1>Content</h1>"), "{html}");
        assert_eq!(
            html.matches(r#"aria-label="Main navigation""#).count(),
            2,
            "desktop + mobile: {html}"
        );
    }

    #[test]
    fn marks_current_page_in_both_navs() {
        let html = shell(Locale::Vi, Theme::System, "/hoi-dap");
        assert_eq!(html.matches(r#"aria-current="page""#).count(), 2, "{html}");
        let en = shell(Locale::En, Theme::System, "/en/hoi-dap");
        assert_eq!(
            en.matches(r#"aria-current="page""#).count(),
            2,
            "english path is neutralised: {en}"
        );
        assert!(
            en.contains(r#"href="/en/hoi-dap""#) && en.contains(r#"href="/en""#),
            "{en}"
        );
    }

    #[test]
    fn language_switch_links_to_the_same_page() {
        let html = shell(Locale::Vi, Theme::System, "/hoi-dap");
        assert!(
            html.contains(r#"hreflang="en""#) && html.contains(r#"href="/en/hoi-dap""#),
            "{html}"
        );
        assert!(
            html.contains(r#"aria-current="true""#),
            "current language is marked: {html}"
        );
        assert_eq!(html.matches("data-lang-link").count(), 2, "{html}");
    }

    #[test]
    fn theme_toggle_reflects_dark_and_needs_js() {
        let dark = shell(Locale::En, Theme::Dark, "/");
        assert!(
            dark.contains(r#"aria-pressed="true""#) && dark.contains("data-requires-js"),
            "{dark}"
        );
        let light = shell(Locale::En, Theme::Light, "/");
        assert!(light.contains(r#"aria-pressed="false""#), "{light}");
    }

    #[test]
    fn cta_is_desktop_only_and_footer_links_are_localized() {
        let html = shell(Locale::En, Theme::System, "/");
        assert!(
            html.contains("hidden lg:inline-flex") && html.contains("Join"),
            "{html}"
        );
        assert!(
            html.contains(r#"href="/en/dieu-khoan""#)
                && html.contains(r#"href="/en/chinh-sach-bao-mat""#),
            "{html}"
        );
        assert!(html.contains("data-toast-region"), "{html}");
    }
}

//! The full HTML document around a rendered body: `<head>` (meta, preload, CSS, JS), language and
//! theme attributes. Built as a string with explicit escaping: the head is small, fixed in shape
//! and must stay free of inline scripts/styles (strict CSP).

use crate::assets::{APP_CSS, APP_JS, FAVICON, FONT_REGULAR, FONT_SEMIBOLD};
use crate::i18n::{Locale, localize_path};
use crate::theme::Theme;

/// What a page needs to know about the request.
#[derive(Clone, Debug)]
pub struct PageContext {
    pub locale: Locale,
    pub theme: Theme,
    /// Locale-neutral path (`/`, `/hoi-dap`).
    pub path: String,
    /// Public origin (`https://example.com`, no trailing slash). `None` until the domain is
    /// decided (D-4): canonical and `hreflang` alternates are then omitted.
    pub base_url: Option<String>,
    pub noindex: bool,
}

impl PageContext {
    #[must_use]
    pub fn new(locale: Locale, theme: Theme, path: impl Into<String>) -> Self {
        Self {
            locale,
            theme,
            path: path.into(),
            base_url: None,
            noindex: false,
        }
    }
}

/// Escapes text for use in HTML text and double-quoted attribute values.
#[must_use]
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Wraps `body_html` (already-rendered, trusted markup from this crate) in a complete document.
#[must_use]
pub fn render_document(
    ctx: &PageContext,
    title: &str,
    description: &str,
    body_html: &str,
) -> String {
    let lang = ctx.locale.code();
    let theme_attr = ctx
        .theme
        .attr()
        .map(|t| format!(r#" data-theme="{t}""#))
        .unwrap_or_default();

    let mut head = String::with_capacity(1024);
    head.push_str(r#"<meta charset="utf-8">"#);
    head.push_str(r#"<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">"#);
    head.push_str(&format!("<title>{}</title>", escape(title)));
    head.push_str(&format!(
        r#"<meta name="description" content="{}">"#,
        escape(description)
    ));
    head.push_str(r#"<meta name="color-scheme" content="light dark">"#);
    head.push_str(
        r##"<meta name="theme-color" content="#ffffff" media="(prefers-color-scheme: light)">"##,
    );
    head.push_str(
        r##"<meta name="theme-color" content="#0b1220" media="(prefers-color-scheme: dark)">"##,
    );
    if ctx.noindex {
        head.push_str(r#"<meta name="robots" content="noindex, nofollow">"#);
    }
    if let Some(base) = &ctx.base_url {
        let base = base.trim_end_matches('/');
        let canonical = format!("{base}{}", localize_path(ctx.locale, &ctx.path));
        head.push_str(&format!(
            r#"<link rel="canonical" href="{}">"#,
            escape(&canonical)
        ));
        for l in Locale::ALL {
            let href = format!("{base}{}", localize_path(l, &ctx.path));
            head.push_str(&format!(
                r#"<link rel="alternate" hreflang="{}" href="{}">"#,
                l.code(),
                escape(&href)
            ));
        }
        let default = format!("{base}{}", localize_path(Locale::Vi, &ctx.path));
        head.push_str(&format!(
            r#"<link rel="alternate" hreflang="x-default" href="{}">"#,
            escape(&default)
        ));
    }
    head.push_str(&format!(
        r#"<link rel="icon" href="{}" type="image/svg+xml">"#,
        FAVICON.url()
    ));
    for font in [FONT_REGULAR, FONT_SEMIBOLD] {
        head.push_str(&format!(
            r#"<link rel="preload" href="{}" as="font" type="font/woff" crossorigin>"#,
            font.url()
        ));
    }
    head.push_str(&format!(
        r#"<link rel="stylesheet" href="{}">"#,
        APP_CSS.url()
    ));
    head.push_str(&format!(
        r#"<script src="{}" defer></script>"#,
        APP_JS.url()
    ));

    format!(
        r#"<!doctype html><html lang="{lang}"{theme_attr}><head>{head}</head><body>{body_html}</body></html>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(ctx: &PageContext) -> String {
        render_document(ctx, "T <b>", "D \"q\"", "<p>body</p>")
    }

    #[test]
    fn sets_language_and_theme_attributes_before_paint() {
        let vi = doc(&PageContext::new(Locale::Vi, Theme::System, "/"));
        assert!(
            vi.starts_with("<!doctype html><html lang=\"vi\"><head>"),
            "{vi}"
        );
        let en = doc(&PageContext::new(Locale::En, Theme::Dark, "/"));
        assert!(en.contains(r#"<html lang="en" data-theme="dark">"#), "{en}");
    }

    #[test]
    fn escapes_title_and_description() {
        let html = doc(&PageContext::new(Locale::Vi, Theme::System, "/"));
        assert!(
            html.contains("<title>T &lt;b&gt;</title>")
                && html.contains(r#"content="D &quot;q&quot;""#),
            "{html}"
        );
    }

    #[test]
    fn head_is_csp_clean() {
        let html = doc(&PageContext::new(Locale::Vi, Theme::Light, "/"));
        assert!(
            !html.contains("style=") && !html.contains("<style"),
            "no inline styles: {html}"
        );
        assert!(
            !html.contains("onclick=") && !html.contains("javascript:"),
            "{html}"
        );
        // Exactly one script, external and deferred.
        assert_eq!(html.matches("<script").count(), 1, "{html}");
        assert!(
            html.contains("defer></script>") && html.contains(r#"<script src="/assets/app.js?v="#),
            "{html}"
        );
        assert!(
            html.contains(r#"<link rel="stylesheet" href="/assets/app.css?v="#),
            "{html}"
        );
        assert!(
            html.contains(r#"as="font""#) && html.contains("crossorigin"),
            "fonts preloaded: {html}"
        );
    }

    #[test]
    fn canonical_and_alternates_only_with_a_base_url() {
        let mut ctx = PageContext::new(Locale::En, Theme::System, "/hoi-dap");
        assert!(!doc(&ctx).contains("hreflang"));
        ctx.base_url = Some("https://example.test/".into());
        let html = doc(&ctx);
        assert!(
            html.contains(r#"rel="canonical" href="https://example.test/en/hoi-dap""#),
            "{html}"
        );
        assert!(
            html.contains(r#"hreflang="vi" href="https://example.test/hoi-dap""#),
            "{html}"
        );
        assert!(
            html.contains(r#"hreflang="x-default" href="https://example.test/hoi-dap""#),
            "{html}"
        );
    }

    #[test]
    fn noindex_flag_adds_robots_meta() {
        let mut ctx = PageContext::new(Locale::Vi, Theme::System, "/_ui");
        ctx.noindex = true;
        assert!(doc(&ctx).contains(r#"name="robots" content="noindex, nofollow""#));
    }
}

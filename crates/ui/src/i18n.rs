//! Locales and Fluent catalogs (`vi` default, `en` complete translation, no silent fallbacks in
//! tests). Vietnamese slugs are canonical; English lives under `/en` (UI/UX spec §4, §9).

use std::sync::LazyLock;

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource};
use unic_langid::LanguageIdentifier;

use crate::cookie;

/// Name of the (functional, non-tracking) language preference cookie.
pub const COOKIE: &str = "aao_lang";

/// Supported UI languages.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Locale {
    #[default]
    Vi,
    En,
}

impl Locale {
    pub const ALL: [Self; 2] = [Self::Vi, Self::En];

    /// BCP 47 code, also used for `lang`/`hreflang` and the cookie value.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Vi => "vi",
            Self::En => "en",
        }
    }

    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "vi" => Some(Self::Vi),
            "en" => Some(Self::En),
            _ => None,
        }
    }

    /// Language preference from a `Cookie` header value.
    #[must_use]
    pub fn from_cookie_header(header: Option<&str>) -> Option<Self> {
        header
            .and_then(|h| cookie::value(h, COOKIE))
            .and_then(Self::from_code)
    }

    /// Detects the locale a request path belongs to (`/en` and `/en/...` are English).
    #[must_use]
    pub fn from_path(path: &str) -> Self {
        if path == "/en" || path.starts_with("/en/") {
            Self::En
        } else {
            Self::Vi
        }
    }

    #[must_use]
    pub fn other(self) -> Self {
        match self {
            Self::Vi => Self::En,
            Self::En => Self::Vi,
        }
    }
}

/// Maps a locale-neutral path (`/`, `/hoi-dap`) to the public URL path for `locale`.
/// Fragment-only (`#x`) and absolute (`http…`) targets are returned unchanged.
#[must_use]
pub fn localize_path(locale: Locale, path: &str) -> String {
    if !path.starts_with('/') {
        return path.to_owned();
    }
    match locale {
        Locale::Vi => path.to_owned(),
        Locale::En if path == "/" => "/en".to_owned(),
        Locale::En => format!("/en{path}"),
    }
}

/// Removes the `/en` prefix, giving the locale-neutral path.
#[must_use]
pub fn neutral_path(path: &str) -> &str {
    if path == "/en" {
        "/"
    } else {
        path.strip_prefix("/en")
            .filter(|rest| rest.starts_with('/'))
            .unwrap_or(path)
    }
}

pub(crate) const VI_SOURCES: [&str; 2] = [
    include_str!("../locales/vi/common.ftl"),
    include_str!("../locales/vi/gallery.ftl"),
];
pub(crate) const EN_SOURCES: [&str; 2] = [
    include_str!("../locales/en/common.ftl"),
    include_str!("../locales/en/gallery.ftl"),
];

type Bundle = FluentBundle<FluentResource>;

struct Catalogs {
    vi: Bundle,
    en: Bundle,
}

impl Catalogs {
    fn get(&self, locale: Locale) -> &Bundle {
        match locale {
            Locale::Vi => &self.vi,
            Locale::En => &self.en,
        }
    }
}

// The catalogs are compile-time embedded; a parse error is a programmer error that the
// `catalogs_parse_without_errors` test catches before it can reach production.
#[allow(clippy::expect_used)]
fn build(locale: Locale, sources: &[&str]) -> Bundle {
    let id: LanguageIdentifier = locale.code().parse().expect("valid language identifier");
    let mut bundle = FluentBundle::new_concurrent(vec![id]);
    // Isolation marks (U+2068/2069) would leak into attribute values and text; we never mix RTL.
    bundle.set_use_isolating(false);
    for src in sources {
        let resource = match FluentResource::try_new((*src).to_owned()) {
            Ok(r) => r,
            Err((r, _errors)) => r,
        };
        bundle
            .add_resource(resource)
            .expect("no duplicate message ids across catalog files");
    }
    bundle
}

static CATALOGS: LazyLock<Catalogs> = LazyLock::new(|| Catalogs {
    vi: build(Locale::Vi, &VI_SOURCES),
    en: build(Locale::En, &EN_SOURCES),
});

fn lookup(locale: Locale, key: &str, args: Option<&FluentArgs>) -> String {
    // Fall back to English, then to the key itself, so a gap is visible rather than blank.
    // Tests fail on any missing key, so this only matters if a deploy ships a bad catalog.
    for l in [locale, Locale::En] {
        let bundle = CATALOGS.get(l);
        if let Some(pattern) = bundle.get_message(key).and_then(|m| m.value()) {
            let mut errors = vec![];
            return bundle
                .format_pattern(pattern, args, &mut errors)
                .into_owned();
        }
    }
    key.to_owned()
}

/// Translates `key` for `locale`.
#[must_use]
pub fn tr(locale: Locale, key: &str) -> String {
    lookup(locale, key, None)
}

/// Translates `key` with Fluent variables, e.g. `tr_args(l, "error-trace-id", &[("id", "abc")])`.
#[must_use]
pub fn tr_args(locale: Locale, key: &str, vars: &[(&str, &str)]) -> String {
    let mut args = FluentArgs::new();
    for (k, v) in vars {
        args.set(*k, *v);
    }
    lookup(locale, key, Some(&args))
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;

    /// Message ids with their `$variables`, from a catalog's text (one-line scanner: enough for
    /// the subset of Fluent we use, and independent of the parser under test).
    fn messages(src: &str) -> BTreeMap<String, BTreeSet<String>> {
        let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut current: Option<String> = None;
        for line in src.lines() {
            let is_id_line =
                line.chars().next().is_some_and(|c| c.is_ascii_lowercase()) && line.contains('=');
            if is_id_line {
                let id = line.split('=').next().unwrap_or_default().trim().to_owned();
                out.entry(id.clone()).or_default();
                current = Some(id);
            } else if !line.starts_with(' ') && !line.trim().is_empty() {
                current = None;
            }
            if let Some(id) = &current {
                for part in line.split('$').skip(1) {
                    let var: String = part
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
                        .collect();
                    out.entry(id.clone()).or_default().insert(var);
                }
            }
        }
        out
    }

    fn all(sources: &[&str]) -> BTreeMap<String, BTreeSet<String>> {
        sources.iter().flat_map(|s| messages(s)).collect()
    }

    #[test]
    fn catalogs_parse_without_errors() {
        for (locale, sources) in [(Locale::Vi, &VI_SOURCES), (Locale::En, &EN_SOURCES)] {
            for src in sources {
                let parsed = FluentResource::try_new((*src).to_owned());
                assert!(
                    parsed.is_ok(),
                    "{} catalog has Fluent syntax errors",
                    locale.code()
                );
            }
        }
    }

    #[test]
    fn vi_and_en_have_identical_keys_and_variables() {
        let vi = all(&VI_SOURCES);
        let en = all(&EN_SOURCES);
        let only_vi: Vec<_> = vi.keys().filter(|k| !en.contains_key(*k)).collect();
        let only_en: Vec<_> = en.keys().filter(|k| !vi.contains_key(*k)).collect();
        assert!(only_vi.is_empty(), "keys missing from en: {only_vi:?}");
        assert!(only_en.is_empty(), "keys missing from vi: {only_en:?}");
        for (key, vars) in &vi {
            assert_eq!(
                vars, &en[key],
                "variables differ between vi and en for '{key}'"
            );
        }
        assert!(
            vi.len() > 30,
            "scanner found suspiciously few keys: {}",
            vi.len()
        );
    }

    #[test]
    fn no_empty_values() {
        for (locale, sources) in [(Locale::Vi, &VI_SOURCES), (Locale::En, &EN_SOURCES)] {
            for key in all(sources).keys() {
                assert!(
                    !tr(locale, key).trim().is_empty(),
                    "empty '{key}' in {}",
                    locale.code()
                );
            }
        }
    }

    /// Every key referenced from `tr(…, "key")` / `tr_args(…, "key", …)` in this workspace's UI
    /// and web sources exists in both catalogs.
    #[test]
    fn every_referenced_key_exists() {
        let vi = all(&VI_SOURCES);
        let mut seen = 0;
        for dir in ["src", "src/components"] {
            let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
            for entry in std::fs::read_dir(dir).expect("readable source dir") {
                let path = entry.expect("dir entry").path();
                if path.extension().is_some_and(|e| e == "rs") && !path.ends_with("i18n.rs") {
                    let text = std::fs::read_to_string(&path).expect("readable source file");
                    for key in referenced_keys(&text) {
                        seen += 1;
                        assert!(
                            vi.contains_key(&key),
                            "{} uses unknown key '{key}'",
                            path.display()
                        );
                    }
                }
            }
        }
        assert!(seen > 20, "key scanner found only {seen} references");
    }

    /// Finds `tr(l, "key"` and `tr_args(l, "key"` call sites.
    pub(crate) fn referenced_keys(src: &str) -> Vec<String> {
        let mut keys = vec![];
        for needle in ["tr(", "tr_args("] {
            let mut rest = src;
            while let Some(pos) = rest.find(needle) {
                let before = rest[..pos].chars().next_back();
                rest = &rest[pos + needle.len()..];
                if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                    continue; // e.g. `str(`
                }
                let Some((_, after_comma)) = rest.split_once(',') else {
                    break;
                };
                let after_comma = after_comma.trim_start();
                if let Some(lit) = after_comma.strip_prefix('"')
                    && let Some((key, _)) = lit.split_once('"')
                {
                    keys.push(key.to_owned());
                }
            }
        }
        keys
    }

    #[test]
    fn key_scanner_finds_call_sites() {
        let src =
            r#"let a = tr(locale, "nav-home"); x.str(1, "no"); tr_args(l, "error-trace-id", &[])"#;
        assert_eq!(referenced_keys(src), vec!["nav-home", "error-trace-id"]);
    }

    #[test]
    fn translates_and_interpolates() {
        assert_eq!(tr(Locale::Vi, "nav-home"), "Trang chủ");
        assert_eq!(tr(Locale::En, "nav-home"), "Home");
        assert_eq!(
            tr_args(Locale::En, "error-trace-id", &[("id", "ABC")]),
            "Reference: ABC"
        );
        assert_eq!(tr(Locale::En, "no-such-key"), "no-such-key");
    }

    #[test]
    fn locale_and_path_helpers() {
        assert_eq!(Locale::from_path("/en"), Locale::En);
        assert_eq!(Locale::from_path("/en/hoi-dap"), Locale::En);
        assert_eq!(Locale::from_path("/english"), Locale::Vi);
        assert_eq!(localize_path(Locale::En, "/"), "/en");
        assert_eq!(localize_path(Locale::En, "/hoi-dap"), "/en/hoi-dap");
        assert_eq!(localize_path(Locale::Vi, "/hoi-dap"), "/hoi-dap");
        assert_eq!(localize_path(Locale::En, "#top"), "#top");
        assert_eq!(neutral_path("/en"), "/");
        assert_eq!(neutral_path("/en/hoi-dap"), "/hoi-dap");
        assert_eq!(neutral_path("/english"), "/english");
        assert_eq!(
            Locale::from_cookie_header(Some("aao_lang=en")),
            Some(Locale::En)
        );
        assert_eq!(Locale::from_cookie_header(Some("aao_lang=fr")), None);
    }
}

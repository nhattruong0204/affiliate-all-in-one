//! Colour theme preference.
//!
//! The server renders `data-theme` on `<html>` from the `aao_theme` cookie, so the first paint is
//! already correct and no inline script is needed (CSP-friendly, no flash). With no cookie the
//! attribute is omitted and CSS follows `prefers-color-scheme`.

use crate::cookie;

/// Name of the (functional, non-tracking) preference cookie.
pub const COOKIE: &str = "aao_theme";

/// The user's explicit theme choice; `System` means "follow the OS".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

impl Theme {
    /// Reads the preference from a `Cookie` header value. Unknown values mean `System`.
    #[must_use]
    pub fn from_cookie_header(header: Option<&str>) -> Self {
        match header.and_then(|h| cookie::value(h, COOKIE)) {
            Some("light") => Self::Light,
            Some("dark") => Self::Dark,
            _ => Self::System,
        }
    }

    /// Value for the `data-theme` attribute; `None` for `System`.
    #[must_use]
    pub fn attr(self) -> Option<&'static str> {
        match self {
            Self::System => None,
            Self::Light => Some("light"),
            Self::Dark => Some("dark"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Theme;

    #[test]
    fn parses_known_values_and_defaults_to_system() {
        assert_eq!(
            Theme::from_cookie_header(Some("aao_theme=dark")),
            Theme::Dark
        );
        assert_eq!(
            Theme::from_cookie_header(Some("x=1; aao_theme=light")),
            Theme::Light
        );
        assert_eq!(
            Theme::from_cookie_header(Some("aao_theme=purple")),
            Theme::System
        );
        assert_eq!(Theme::from_cookie_header(None), Theme::System);
    }

    #[test]
    fn system_has_no_attribute() {
        assert_eq!(Theme::System.attr(), None);
        assert_eq!(Theme::Dark.attr(), Some("dark"));
    }
}

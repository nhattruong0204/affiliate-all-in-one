//! Minimal `Cookie` request-header parsing for the two preference cookies.

/// Returns the value of cookie `name` from a `Cookie` header value, if present.
///
/// Only exact name matches count; a malformed pair is skipped. Values are not decoded: the
/// preference cookies we read hold short ASCII tokens.
#[must_use]
pub fn value<'a>(header: &'a str, name: &str) -> Option<&'a str> {
    header.split(';').find_map(|pair| {
        let (k, v) = pair.trim().split_once('=')?;
        (k == name).then_some(v.trim())
    })
}

#[cfg(test)]
mod tests {
    use super::value;

    #[test]
    fn finds_exact_name_only() {
        let h = "a=1; aao_theme=dark; xaao_theme=light; aao_lang=en";
        assert_eq!(value(h, "aao_theme"), Some("dark"));
        assert_eq!(value(h, "aao_lang"), Some("en"));
        assert_eq!(value(h, "missing"), None);
    }

    #[test]
    fn tolerates_malformed_pairs_and_spacing() {
        assert_eq!(value("junk;  aao_theme = light ;x", "aao_theme"), None);
        assert_eq!(
            value("junk;  aao_theme=light ;x", "aao_theme"),
            Some("light")
        );
        assert_eq!(value("", "aao_theme"), None);
    }
}

//! Guards for the design tokens in `style/tailwind.css`: the two dark-theme blocks cannot drift,
//! every colour has a dark value, and text/background pairs meet WCAG 2.2 AA.
//!
//! The CSS is parsed with plain string handling on purpose: the file's token section has a fixed,
//! one-declaration-per-line shape (documented at the top of the file).

use std::collections::BTreeMap;

const CSS: &str = include_str!("../style/tailwind.css");

type Colors = BTreeMap<String, [u8; 3]>;

fn between<'a>(text: &'a str, start: &str, end: &str) -> &'a str {
    let after = text
        .split_once(start)
        .unwrap_or_else(|| panic!("marker not found: {start}"))
        .1;
    after
        .split_once(end)
        .unwrap_or_else(|| panic!("marker not found: {end}"))
        .0
}

fn hex(value: &str) -> Option<[u8; 3]> {
    let digits = value.strip_prefix('#').filter(|d| d.len() == 6)?;
    let channel = |i: usize| u8::from_str_radix(&digits[i..i + 2], 16).ok();
    Some([channel(0)?, channel(2)?, channel(4)?])
}

/// `--color-name: #rrggbb;` declarations in `block`.
fn colors(block: &str) -> Colors {
    block
        .lines()
        .filter_map(|line| {
            let (name, value) = line.trim().strip_prefix("--color-")?.split_once(':')?;
            Some((name.to_owned(), hex(value.trim().trim_end_matches(';'))?))
        })
        .collect()
}

fn declarations(block: &str) -> Vec<String> {
    block
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("--"))
        .map(str::to_owned)
        .collect()
}

fn light() -> Colors {
    colors(between(CSS, "@theme static {", "\n}"))
}

fn dark_block() -> &'static str {
    between(CSS, "/* BEGIN dark-tokens (", "/* END dark-tokens */")
}

fn dark_media_block() -> &'static str {
    between(
        CSS,
        "/* BEGIN dark-tokens-media */",
        "/* END dark-tokens-media */",
    )
}

/// Light colours with the dark overrides applied.
fn dark() -> Colors {
    let mut merged = light();
    merged.extend(colors(dark_block()));
    merged
}

fn channel_luminance(c: u8) -> f64 {
    let s = f64::from(c) / 255.0;
    if s <= 0.03928 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

fn luminance([r, g, b]: [u8; 3]) -> f64 {
    0.2126 * channel_luminance(r) + 0.7152 * channel_luminance(g) + 0.0722 * channel_luminance(b)
}

fn contrast(a: [u8; 3], b: [u8; 3]) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

#[test]
fn colour_parser_reads_the_token_section() {
    let light = light();
    assert_eq!(light.get("bg"), Some(&[255, 255, 255]));
    assert!(light.len() >= 25, "parsed only {} colour tokens", light.len());
}

#[test]
fn dark_blocks_are_identical() {
    // The same declarations must appear for an explicit `data-theme="dark"` and for the OS
    // preference, otherwise the toggle and "system" would show different dark themes.
    let explicit = declarations(dark_block());
    let media = declarations(dark_media_block());
    assert!(
        explicit.len() > 30,
        "found only {} declarations",
        explicit.len()
    );
    assert_eq!(
        explicit, media,
        "dark theme blocks drifted apart; edit both"
    );
}

#[test]
fn every_colour_token_has_a_dark_value_and_nothing_extra() {
    let light = light();
    let overrides = colors(dark_block());
    let missing: Vec<_> = light
        .keys()
        .filter(|k| !overrides.contains_key(*k))
        .collect();
    let extra: Vec<_> = overrides
        .keys()
        .filter(|k| !light.contains_key(*k))
        .collect();
    assert!(
        missing.is_empty(),
        "colours without a dark value: {missing:?}"
    );
    assert!(
        extra.is_empty(),
        "dark values without a light token: {extra:?}"
    );
}

/// (foreground, background) token pairs that appear together in components.
const TEXT_PAIRS: &[(&str, &str)] = &[
    ("text", "bg"),
    ("text", "surface"),
    ("text", "surface-2"),
    ("text-muted", "bg"),
    ("text-muted", "surface"),
    ("text-muted", "surface-2"),
    ("on-brand", "brand-600"),
    ("on-brand", "brand-700"),
    ("on-accent", "accent-500"),
    ("accent-text", "accent-50"),
    ("brand-700", "brand-100"),
    ("brand-600", "brand-50"),
    ("brand-600", "bg"),
    ("brand-700", "bg"),
    ("danger", "bg"),
    ("danger", "danger-bg"),
    ("success", "bg"),
    ("success", "success-bg"),
    ("warning", "bg"),
    ("warning", "warning-bg"),
    ("info", "bg"),
    ("info", "info-bg"),
    ("state-estimated", "bg"),
    ("state-estimated", "surface"),
    ("state-pending", "bg"),
    ("state-pending", "surface"),
    ("state-approved", "bg"),
    ("state-approved", "surface"),
    ("state-available", "bg"),
    ("state-available", "surface"),
    ("state-reversed", "bg"),
    ("state-reversed", "surface"),
];

/// Non-text indicators (control borders, focus ring) need 3:1 (WCAG 1.4.11).
const UI_PAIRS: &[(&str, &str)] = &[
    ("input-border", "bg"),
    ("focus", "bg"),
    ("focus", "surface"),
    ("brand-600", "bg"),
];

fn check(theme: &str, colors: &Colors, pairs: &[(&str, &str)], minimum: f64) {
    for (fg, bg) in pairs {
        let (fg_c, bg_c) = (colors[*fg], colors[*bg]);
        let ratio = contrast(fg_c, bg_c);
        assert!(
            ratio >= minimum,
            "{theme}: {fg} on {bg} has contrast {ratio:.2}, needs {minimum}"
        );
    }
}

#[test]
fn text_pairs_meet_wcag_aa_in_both_themes() {
    check("light", &light(), TEXT_PAIRS, 4.5);
    check("dark", &dark(), TEXT_PAIRS, 4.5);
}

#[test]
fn control_borders_and_focus_ring_meet_3_to_1_in_both_themes() {
    check("light", &light(), UI_PAIRS, 3.0);
    check("dark", &dark(), UI_PAIRS, 3.0);
}

#[test]
fn contrast_formula_matches_known_values() {
    let ratio = contrast([0, 0, 0], [255, 255, 255]);
    assert!((ratio - 21.0).abs() < 0.01, "{ratio}");
    let same = contrast([10, 20, 30], [10, 20, 30]);
    assert!((same - 1.0).abs() < 1e-9, "{same}");
}

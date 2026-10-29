//! Text helpers for case-insensitive matching and ordering of names.
//! SQLite `NOCASE` and `LIKE` fold ASCII only, so we store folded keys.

/// Case-folded key that orders Cyrillic like dictionaries do ("ё" as "е").
pub fn fold(text: &str) -> String {
    text.to_lowercase().replace('ё', "е")
}

/// `LIKE` pattern matching `query` anywhere in a folded key, with `%`, `_`
/// and `\` escaped. Use with `ESCAPE '\'`.
pub fn contains_pattern(query: &str) -> String {
    let mut pattern = String::from("%");
    for ch in fold(query).chars() {
        if matches!(ch, '%' | '_' | '\\') {
            pattern.push('\\');
        }
        pattern.push(ch);
    }
    pattern.push('%');
    pattern
}

/// Trimmed value, or `None` if it is blank.
pub fn clean(value: Option<String>) -> Option<String> {
    value.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty())
}

/// "Last First Middle", skipping missing parts.
pub fn full_name(last: &Option<String>, first: &Option<String>, middle: &Option<String>) -> String {
    [last, first, middle]
        .into_iter()
        .flatten()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fold_lowercases_and_replaces_yo() {
        assert_eq!(fold("Ёлкин"), "елкин");
        assert_eq!(fold("ACME"), "acme");
    }

    #[test]
    fn clean_trims_and_drops_blank() {
        assert_eq!(clean(Some("  а ".into())), Some("а".into()));
        assert_eq!(clean(Some(" \t".into())), None);
        assert_eq!(clean(None), None);
    }

    #[test]
    fn full_name_skips_missing_parts() {
        let s = |v: &str| Some(v.to_owned());
        assert_eq!(
            full_name(&s("Иванов"), &s("Иван"), &s("Иванович")),
            "Иванов Иван Иванович"
        );
        assert_eq!(full_name(&None, &s("Иван"), &None), "Иван");
    }

    #[test]
    fn contains_pattern_escapes_wildcards() {
        assert_eq!(contains_pattern("Ёж"), "%еж%");
        assert_eq!(contains_pattern(r"5%_\"), r"%5\%\_\\%");
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fold_lowercases_and_replaces_yo() {
        assert_eq!(fold("Ёлкин"), "елкин");
        assert_eq!(fold("ACME"), "acme");
    }

    #[test]
    fn contains_pattern_escapes_wildcards() {
        assert_eq!(contains_pattern("Ёж"), "%еж%");
        assert_eq!(contains_pattern(r"5%_\"), r"%5\%\_\\%");
    }
}

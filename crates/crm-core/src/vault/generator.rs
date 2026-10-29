//! Password and passphrase generator (FR-VA-7). Uses the OS random number
//! generator with rejection sampling, so every symbol is equally likely.

use serde::{Deserialize, Serialize};

use crate::{CoreError, CoreResult};

pub const LOWERCASE: &str = "abcdefghijklmnopqrstuvwxyz";
pub const UPPERCASE: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
pub const DIGITS: &str = "0123456789";
pub const SYMBOLS: &str = "!@#$%^&*()-_=+[]{};:,.?/";
/// Characters that are easy to confuse when read or typed.
pub const SIMILAR: &str = "Il1O0o";

pub const MIN_LENGTH: u32 = 4;
pub const MAX_LENGTH: u32 = 128;
pub const MIN_WORDS: u32 = 3;
pub const MAX_WORDS: u32 = 16;

/// EFF Short Wordlist 1 (CC BY 3.0 US): 1296 short, distinct words.
const WORDLIST: &str = include_str!("../../wordlists/eff_short_1.txt");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PasswordOptions {
    pub length: u32,
    pub lowercase: bool,
    pub uppercase: bool,
    pub digits: bool,
    pub symbols: bool,
    pub exclude_similar: bool,
}

impl Default for PasswordOptions {
    fn default() -> Self {
        Self {
            length: 20,
            lowercase: true,
            uppercase: true,
            digits: true,
            symbols: true,
            exclude_similar: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PassphraseOptions {
    pub words: u32,
    pub separator: String,
    /// Capitalize the first letter of every word.
    pub capitalize: bool,
}

impl Default for PassphraseOptions {
    fn default() -> Self {
        Self {
            words: 5,
            separator: "-".into(),
            capitalize: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Generated {
    pub value: String,
    /// Estimated strength against an attacker who knows the method.
    pub entropy_bits: f64,
}

/// Bits of entropy of `count` independent picks from `choices` options.
pub fn entropy_bits(count: u32, choices: usize) -> f64 {
    f64::from(count) * (choices as f64).log2()
}

/// Uniform random index below `n` (rejection sampling avoids modulo bias).
fn random_below(n: usize) -> usize {
    assert!(n > 0 && n <= u32::MAX as usize);
    let n = n as u32;
    let zone = u32::MAX - (u32::MAX % n);
    loop {
        let mut bytes = [0u8; 4];
        getrandom::fill(&mut bytes).expect("the OS random number generator is available");
        let value = u32::from_le_bytes(bytes);
        if value < zone {
            return (value % n) as usize;
        }
    }
}

fn pick<T: Copy>(items: &[T]) -> T {
    items[random_below(items.len())]
}

pub fn words() -> impl Iterator<Item = &'static str> {
    WORDLIST
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
}

pub fn generate_password(options: &PasswordOptions) -> CoreResult<Generated> {
    if !(MIN_LENGTH..=MAX_LENGTH).contains(&options.length) {
        return Err(CoreError::validation("length", "range"));
    }
    let sets: Vec<Vec<char>> = [
        (options.lowercase, LOWERCASE),
        (options.uppercase, UPPERCASE),
        (options.digits, DIGITS),
        (options.symbols, SYMBOLS),
    ]
    .into_iter()
    .filter(|(on, _)| *on)
    .map(|(_, set)| {
        set.chars()
            .filter(|c| !(options.exclude_similar && SIMILAR.contains(*c)))
            .collect()
    })
    .collect();
    if sets.is_empty() {
        return Err(CoreError::validation("charsets", "required"));
    }
    let alphabet: Vec<char> = sets.concat();
    // One character from every chosen set, the rest from all of them, then
    // a Fisher-Yates shuffle so the guaranteed ones have no fixed place.
    let mut chars: Vec<char> = sets.iter().map(|set| pick(set)).collect();
    while chars.len() < options.length as usize {
        chars.push(pick(&alphabet));
    }
    for i in (1..chars.len()).rev() {
        chars.swap(i, random_below(i + 1));
    }
    Ok(Generated {
        value: chars.into_iter().collect(),
        entropy_bits: entropy_bits(options.length, alphabet.len()),
    })
}

pub fn generate_passphrase(options: &PassphraseOptions) -> CoreResult<Generated> {
    if !(MIN_WORDS..=MAX_WORDS).contains(&options.words) {
        return Err(CoreError::validation("words", "range"));
    }
    let list: Vec<&str> = words().collect();
    let chosen: Vec<String> = (0..options.words)
        .map(|_| {
            let word = pick(&list);
            if options.capitalize {
                let mut chars = word.chars();
                chars
                    .next()
                    .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
                    .unwrap_or_default()
            } else {
                word.to_owned()
            }
        })
        .collect();
    Ok(Generated {
        value: chosen.join(&options.separator),
        entropy_bits: entropy_bits(options.words, list.len()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn opts(length: u32) -> PasswordOptions {
        PasswordOptions {
            length,
            ..PasswordOptions::default()
        }
    }

    #[test]
    fn the_wordlist_has_1296_distinct_words() {
        let mut all: Vec<_> = words().collect();
        assert_eq!(all.len(), 1296);
        assert!(
            all.iter()
                .all(|w| w.chars().all(|c| c.is_ascii_lowercase() || c == '-'))
        );
        all.sort();
        all.dedup();
        assert_eq!(all.len(), 1296);
    }

    #[test]
    fn entropy() {
        assert_eq!(entropy_bits(0, 62), 0.0);
        assert!((entropy_bits(10, 2) - 10.0).abs() < 1e-9);
        assert!((entropy_bits(20, 94) - 131.092).abs() < 0.001);
        assert!((entropy_bits(5, 1296) - 51.699).abs() < 0.001);
    }

    #[test]
    fn password_entropy_counts_the_whole_alphabet() {
        let g = generate_password(&opts(16)).unwrap();
        let alphabet = LOWERCASE.len() + UPPERCASE.len() + DIGITS.len() + SYMBOLS.len();
        assert!((g.entropy_bits - entropy_bits(16, alphabet)).abs() < 1e-9);
    }

    #[test]
    fn invalid_options_are_rejected() {
        assert_eq!(
            generate_password(&opts(3)).unwrap_err(),
            CoreError::validation("length", "range")
        );
        assert_eq!(
            generate_password(&opts(129)).unwrap_err(),
            CoreError::validation("length", "range")
        );
        let none = PasswordOptions {
            lowercase: false,
            uppercase: false,
            digits: false,
            symbols: false,
            ..opts(10)
        };
        assert_eq!(
            generate_password(&none).unwrap_err(),
            CoreError::validation("charsets", "required")
        );
        assert_eq!(
            generate_passphrase(&PassphraseOptions {
                words: 2,
                ..Default::default()
            })
            .unwrap_err(),
            CoreError::validation("words", "range")
        );
    }

    #[test]
    fn consecutive_passwords_differ() {
        let a = generate_password(&opts(20)).unwrap().value;
        let b = generate_password(&opts(20)).unwrap().value;
        assert_ne!(a, b);
    }

    #[test]
    fn passphrases_use_the_wordlist_count_and_separator() {
        let g = generate_passphrase(&PassphraseOptions {
            words: 6,
            separator: " · ".into(),
            capitalize: false,
        })
        .unwrap();
        let parts: Vec<_> = g.value.split(" · ").collect();
        assert_eq!(parts.len(), 6);
        assert!(parts.iter().all(|w| words().any(|known| known == *w)));
        assert!((g.entropy_bits - entropy_bits(6, 1296)).abs() < 1e-9);

        let capitalized = generate_passphrase(&PassphraseOptions {
            capitalize: true,
            ..Default::default()
        })
        .unwrap();
        for word in capitalized.value.split('-').filter(|w| !w.is_empty()) {
            assert!(word.chars().next().unwrap().is_ascii_uppercase(), "{word}");
        }
    }

    proptest! {
        #[test]
        fn passwords_follow_the_options(
            length in 4u32..64,
            lowercase: bool,
            uppercase: bool,
            digits: bool,
            symbols: bool,
            exclude_similar: bool,
        ) {
            prop_assume!(lowercase || uppercase || digits || symbols);
            let options = PasswordOptions { length, lowercase, uppercase, digits, symbols, exclude_similar };
            let value = generate_password(&options).unwrap().value;
            prop_assert_eq!(value.chars().count(), length as usize);
            for (on, set) in [(lowercase, LOWERCASE), (uppercase, UPPERCASE), (digits, DIGITS), (symbols, SYMBOLS)] {
                let present = value.chars().any(|c| set.contains(c));
                prop_assert_eq!(present, on, "set {:?} in {:?}", set, value);
            }
            if exclude_similar {
                prop_assert!(!value.chars().any(|c| SIMILAR.contains(c)), "{}", value);
            }
        }
    }
}

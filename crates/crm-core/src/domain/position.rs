//! Fractional indexing: string keys that sort between any two neighbours,
//! so moving a card rewrites only that card (FR-TS-5). Keys use base-62
//! digits in ASCII order and never end with the zero digit, which keeps
//! room for a key before every key.

const DIGITS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// A key strictly between `before` and `after` (`None` means the start or
/// the end). Panics if `before >= after`, which is a caller bug.
pub fn key_between(before: Option<&str>, after: Option<&str>) -> String {
    if let (Some(a), Some(b)) = (before, after) {
        assert!(a < b, "{a:?} must be before {b:?}");
    }
    let key = midpoint(before.unwrap_or("").as_bytes(), after.map(str::as_bytes));
    String::from_utf8(key).expect("keys are ASCII")
}

fn digit(c: u8) -> usize {
    DIGITS
        .iter()
        .position(|&d| d == c)
        .expect("key uses base-62 digits")
}

/// Midpoint of two fractions written as digit strings (`a` may be empty,
/// meaning 0; `b` absent means 1).
fn midpoint(a: &[u8], b: Option<&[u8]>) -> Vec<u8> {
    if let Some(b) = b {
        // Skip the common prefix, reading missing digits of `a` as zeros.
        let n = b
            .iter()
            .enumerate()
            .take_while(|&(i, &d)| a.get(i).copied().unwrap_or(DIGITS[0]) == d)
            .count();
        if n > 0 {
            let mut key = b[..n].to_vec();
            key.extend(midpoint(a.get(n..).unwrap_or_default(), Some(&b[n..])));
            return key;
        }
    }
    let low = a.first().map_or(0, |&c| digit(c));
    let high = b.map_or(DIGITS.len(), |b| digit(b[0]));
    if high - low > 1 {
        vec![DIGITS[(low + high) / 2]]
    } else if let Some(b) = b.filter(|b| b.len() > 1) {
        // `b` continues after its first digit, so that digit alone fits.
        vec![b[0]]
    } else {
        let mut key = vec![DIGITS[low]];
        key.extend(midpoint(a.get(1..).unwrap_or_default(), None));
        key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid(key: &str) -> bool {
        !key.is_empty() && !key.ends_with('0') && key.bytes().all(|b| DIGITS.contains(&b))
    }

    #[test]
    fn first_key_is_in_the_middle() {
        assert_eq!(key_between(None, None), "V");
    }

    #[test]
    fn inserts_between_two_keys() {
        for (a, b) in [
            ("V", "W"),
            ("A", "z"),
            ("V", "V1"),
            ("0V", "1"),
            ("abc", "abd"),
        ] {
            let k = key_between(Some(a), Some(b));
            assert!(a < k.as_str() && k.as_str() < b, "{a} < {k} < {b}");
            assert!(valid(&k), "{k}");
        }
    }

    #[test]
    fn inserts_at_the_start_and_end() {
        let mut first = key_between(None, None);
        let mut last = first.clone();
        for _ in 0..200 {
            let before = key_between(None, Some(&first));
            assert!(before < first && valid(&before), "{before} < {first}");
            first = before;
            let after = key_between(Some(&last), None);
            assert!(after > last && valid(&after), "{after} > {last}");
            last = after;
        }
    }

    #[test]
    #[should_panic(expected = "must be before")]
    fn rejects_unordered_bounds() {
        key_between(Some("b"), Some("a"));
    }
}

//! Recovery key (FR-VA-11, PRD §6.4 p. 8): a random 256-bit key shown once
//! as base32 groups (`XXXX-XXXX-…`) with a checksum against typos. It
//! wraps the same DEK a second time, so it can set a new master password.
//! The app never stores it.

use zeroize::Zeroizing;

use super::crypto::{KEY_LEN, Key};
use crate::{CoreError, CoreResult};

/// RFC 4648 base32 alphabet.
const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
/// Checksum bytes appended to the key: 32 + 3 bytes = 280 bits = 56 symbols.
const CHECKSUM_LEN: usize = 3;
pub const GROUP_LEN: usize = 4;
pub const GROUPS: usize = 14;

/// `XXXX-XXXX-…` for the user to save.
pub fn encode(key: &Key) -> Zeroizing<String> {
    let mut data = Zeroizing::new(key.as_bytes().to_vec());
    data.extend_from_slice(&checksum(key));
    let symbols = Zeroizing::new(to_base32(&data));
    let mut text = Zeroizing::new(String::with_capacity(GROUPS * (GROUP_LEN + 1)));
    for (i, chunk) in symbols.as_bytes().chunks(GROUP_LEN).enumerate() {
        if i > 0 {
            text.push('-');
        }
        text.push_str(std::str::from_utf8(chunk).expect("ASCII"));
    }
    text
}

/// Accepts any case, spaces and hyphens. Fails with
/// `recovery.invalid_format` or `recovery.checksum`.
pub fn decode(text: &str) -> CoreResult<Key> {
    let invalid = || CoreError::new("recovery.invalid_format");
    let symbols: Zeroizing<String> = Zeroizing::new(
        text.chars()
            .filter(|c| !c.is_whitespace() && *c != '-')
            .map(|c| c.to_ascii_uppercase())
            .collect(),
    );
    if symbols.len() != GROUPS * GROUP_LEN {
        return Err(invalid());
    }
    let data = Zeroizing::new(from_base32(&symbols).ok_or_else(invalid)?);
    let (bytes, sum) = data.split_at(KEY_LEN);
    let key = Key::from_bytes(bytes.try_into().expect("length checked"));
    if checksum(&key) != sum {
        return Err(CoreError::new("recovery.checksum"));
    }
    Ok(key)
}

/// The key that wraps the DEK, derived from the recovery key with HKDF.
pub fn wrapping_key(recovery: &Key) -> Key {
    let hkdf = hkdf::Hkdf::<sha2::Sha256>::new(None, recovery.as_bytes());
    let mut out = [0u8; KEY_LEN];
    hkdf.expand(b"freelance-crm/recovery/v1", &mut out)
        .expect("32 bytes is a valid HKDF output length");
    Key::from_bytes(out)
}

/// First bytes of SHA-256 over the key: catches typos, reveals nothing
/// useful (the key is 256 random bits).
fn checksum(key: &Key) -> [u8; CHECKSUM_LEN] {
    use sha2::Digest;
    let digest = sha2::Sha256::digest(key.as_bytes());
    [digest[0], digest[1], digest[2]]
}

fn to_base32(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len() * 8 / 5 + 1);
    let (mut buffer, mut bits) = (0u32, 0u32);
    for &byte in data {
        buffer = (buffer << 8) | u32::from(byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(ALPHABET[((buffer >> bits) & 31) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(ALPHABET[((buffer << (5 - bits)) & 31) as usize] as char);
    }
    out
}

fn from_base32(symbols: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(symbols.len() * 5 / 8);
    let (mut buffer, mut bits) = (0u32, 0u32);
    for c in symbols.bytes() {
        let value = ALPHABET.iter().position(|&a| a == c)? as u32;
        buffer = (buffer << 5) | value;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn looks_like_fourteen_groups_of_four() {
        let text = encode(&Key::random());
        let groups: Vec<_> = text.split('-').collect();
        assert_eq!(groups.len(), GROUPS);
        assert!(groups.iter().all(|g| g.len() == GROUP_LEN));
        assert!(
            text.chars()
                .all(|c| c == '-' || ALPHABET.contains(&(c as u8)))
        );
    }

    #[test]
    fn input_is_forgiving_about_case_spaces_and_hyphens() {
        let key = Key::random();
        let text = encode(&key);
        let messy = format!("  {}  ", text.to_lowercase().replace('-', " - "));
        assert_eq!(decode(&messy).unwrap(), key);
        assert_eq!(decode(&text.replace('-', "")).unwrap(), key);
    }

    #[test]
    fn a_single_typo_fails_the_checksum() {
        let text = encode(&Key::random()).to_string();
        for position in [0, 7, 30, text.len() - 1] {
            let mut chars: Vec<char> = text.chars().collect();
            if chars[position] == '-' {
                continue;
            }
            chars[position] = if chars[position] == 'A' { 'B' } else { 'A' };
            let typo: String = chars.into_iter().collect();
            assert_eq!(
                decode(&typo).unwrap_err(),
                CoreError::new("recovery.checksum"),
                "typo at {position}"
            );
        }
    }

    #[test]
    fn malformed_input_is_rejected() {
        let invalid = CoreError::new("recovery.invalid_format");
        assert_eq!(decode("").unwrap_err(), invalid);
        assert_eq!(decode("ABCD-EFGH").unwrap_err(), invalid);
        let text = encode(&Key::random()).to_string();
        // Base32 has no 0, 1, 8 or 9.
        assert_eq!(
            decode(&text.replacen(|c: char| c != '-', "0", 1)).unwrap_err(),
            invalid
        );
        assert_eq!(decode(&format!("{text}A")).unwrap_err(), invalid);
    }

    #[test]
    fn wrapping_key_differs_from_the_recovery_key() {
        let rk = Key::random();
        assert_eq!(wrapping_key(&rk), wrapping_key(&rk));
        assert_ne!(wrapping_key(&rk), rk);
    }

    proptest! {
        #[test]
        fn encode_decode_round_trip(bytes in proptest::array::uniform32(any::<u8>())) {
            let key = Key::from_bytes(bytes);
            prop_assert_eq!(decode(&encode(&key)).unwrap(), key);
        }
    }

    #[test]
    fn key_length_matches_the_layout() {
        assert_eq!((KEY_LEN + CHECKSUM_LEN) * 8, GROUPS * GROUP_LEN * 5);
    }
}

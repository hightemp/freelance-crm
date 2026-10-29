//! Streaming file encryption (PRD §6.4 p. 5): the STREAM construction over
//! XChaCha20-Poly1305 with 64 KiB chunks. Each chunk's nonce is a random
//! per-file prefix, a chunk counter and a "last chunk" flag, so reordering,
//! dropping or truncating chunks fails authentication.
//!
//! Layout: `MAGIC` (8 bytes) · nonce prefix (19 bytes) · chunks, each
//! ciphertext of up to 64 KiB + 16-byte tag. The final chunk is flagged as
//! last and may be shorter (or empty for an empty file).

use std::io::{Read, Write};

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use zeroize::Zeroizing;

use super::crypto::{KEY_LEN, Key};
use crate::{CoreError, CoreResult};

pub const CHUNK_LEN: usize = 64 * 1024;
const TAG_LEN: usize = 16;
const PREFIX_LEN: usize = 19;
const MAGIC: &[u8; 8] = b"FCRMENC1";

/// The key for files, derived from the DEK (HKDF, `info = "file"`).
pub fn file_key(dek: &Key) -> Key {
    let hkdf = hkdf::Hkdf::<sha2::Sha256>::new(None, dek.as_bytes());
    let mut out = [0u8; KEY_LEN];
    hkdf.expand(b"file", &mut out)
        .expect("32 bytes is a valid HKDF output length");
    Key::from_bytes(out)
}

fn cipher(key: &Key) -> XChaCha20Poly1305 {
    XChaCha20Poly1305::new(&(*key.as_bytes()).into())
}

fn nonce(prefix: &[u8; PREFIX_LEN], counter: u32, last: bool) -> XNonce {
    let mut bytes = [0u8; 24];
    bytes[..PREFIX_LEN].copy_from_slice(prefix);
    bytes[PREFIX_LEN..PREFIX_LEN + 4].copy_from_slice(&counter.to_be_bytes());
    bytes[23] = u8::from(last);
    XNonce::from(bytes)
}

/// Reads until `buf` is full or the input ends; returns the bytes read.
fn read_full(reader: &mut impl Read, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        match reader.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(filled)
}

fn read_failed(_: std::io::Error) -> CoreError {
    CoreError::new("file.read_failed")
}

fn write_failed(_: std::io::Error) -> CoreError {
    CoreError::new("file.write_failed")
}

/// Splits the input into chunks, telling which one is the last; reads one
/// chunk ahead so the last flag is known before sealing.
fn for_each_chunk(
    mut reader: impl Read,
    size: usize,
    mut f: impl FnMut(&[u8], bool) -> CoreResult<()>,
) -> CoreResult<()> {
    let mut current = Zeroizing::new(vec![0u8; size]);
    let mut next = Zeroizing::new(vec![0u8; size]);
    let mut current_len = read_full(&mut reader, &mut current).map_err(read_failed)?;
    loop {
        let next_len = if current_len == size {
            read_full(&mut reader, &mut next).map_err(read_failed)?
        } else {
            0
        };
        let last = next_len == 0;
        f(&current[..current_len], last)?;
        if last {
            return Ok(());
        }
        std::mem::swap(&mut current, &mut next);
        current_len = next_len;
    }
}

pub fn encrypt(key: &Key, reader: impl Read, mut writer: impl Write) -> CoreResult<()> {
    let mut prefix = [0u8; PREFIX_LEN];
    getrandom::fill(&mut prefix).expect("the OS random number generator is available");
    writer.write_all(MAGIC).map_err(write_failed)?;
    writer.write_all(&prefix).map_err(write_failed)?;
    let cipher = cipher(key);
    let mut counter: u32 = 0;
    for_each_chunk(reader, CHUNK_LEN, |chunk, last| {
        let sealed = cipher
            .encrypt(&nonce(&prefix, counter, last), chunk)
            .expect("encryption only fails for oversized messages");
        writer.write_all(&sealed).map_err(write_failed)?;
        counter = counter
            .checked_add(1)
            .ok_or_else(|| CoreError::new("file.too_large"))?;
        Ok(())
    })?;
    writer.flush().map_err(write_failed)
}

/// Fails with `vault.decrypt` on any tampering, truncation or wrong key.
/// Chunks are written as they are verified, so on failure the output must
/// be discarded (callers decrypt into a temporary file).
pub fn decrypt(key: &Key, mut reader: impl Read, mut writer: impl Write) -> CoreResult<()> {
    let invalid = || CoreError::new("vault.decrypt");
    let mut header = [0u8; 8 + PREFIX_LEN];
    if read_full(&mut reader, &mut header).map_err(read_failed)? != header.len()
        || &header[..8] != MAGIC
    {
        return Err(invalid());
    }
    let prefix: [u8; PREFIX_LEN] = header[8..].try_into().expect("length checked");
    let cipher = cipher(key);
    let mut counter: u32 = 0;
    for_each_chunk(reader, CHUNK_LEN + TAG_LEN, |chunk, last| {
        let plain = Zeroizing::new(
            cipher
                .decrypt(&nonce(&prefix, counter, last), chunk)
                .map_err(|_| invalid())?,
        );
        writer.write_all(&plain).map_err(write_failed)?;
        counter = counter.checked_add(1).ok_or_else(invalid)?;
        Ok(())
    })?;
    writer.flush().map_err(write_failed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn encrypt_vec(key: &Key, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        encrypt(key, data, &mut out).unwrap();
        out
    }

    fn decrypt_vec(key: &Key, data: &[u8]) -> CoreResult<Vec<u8>> {
        let mut out = Vec::new();
        decrypt(key, data, &mut out).map(|()| out)
    }

    const HEADER: usize = 8 + PREFIX_LEN;
    const SEALED: usize = CHUNK_LEN + TAG_LEN;

    #[rstest]
    #[case(0)]
    #[case(1)]
    #[case(CHUNK_LEN)]
    #[case(CHUNK_LEN + 1)]
    #[case(10 * 1024 * 1024)]
    fn round_trip(#[case] len: usize) {
        let key = Key::random();
        let data: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
        let sealed = encrypt_vec(&key, &data);
        assert_eq!(decrypt_vec(&key, &sealed).unwrap(), data);
        assert!(sealed.starts_with(MAGIC));
    }

    fn two_full_chunks_and_a_tail() -> (Key, Vec<u8>) {
        let key = Key::random();
        let data = vec![9u8; 2 * CHUNK_LEN + 100];
        let sealed = encrypt_vec(&key, &data);
        assert_eq!(sealed.len(), HEADER + 2 * SEALED + 100 + TAG_LEN);
        (key, sealed)
    }

    fn decrypt_error() -> CoreError {
        CoreError::new("vault.decrypt")
    }

    #[test]
    fn swapped_chunks_are_detected() {
        let (key, sealed) = two_full_chunks_and_a_tail();
        let mut swapped = sealed[..HEADER].to_vec();
        swapped.extend_from_slice(&sealed[HEADER + SEALED..HEADER + 2 * SEALED]);
        swapped.extend_from_slice(&sealed[HEADER..HEADER + SEALED]);
        swapped.extend_from_slice(&sealed[HEADER + 2 * SEALED..]);
        assert_eq!(decrypt_vec(&key, &swapped).unwrap_err(), decrypt_error());
    }

    #[test]
    fn dropped_chunks_and_truncation_are_detected() {
        let (key, sealed) = two_full_chunks_and_a_tail();
        let mut dropped = sealed[..HEADER + SEALED].to_vec();
        dropped.extend_from_slice(&sealed[HEADER + 2 * SEALED..]);
        assert_eq!(decrypt_vec(&key, &dropped).unwrap_err(), decrypt_error());
        // Cut exactly at a chunk boundary: no chunk is flagged as last.
        let truncated = &sealed[..HEADER + 2 * SEALED];
        assert_eq!(decrypt_vec(&key, truncated).unwrap_err(), decrypt_error());
        assert_eq!(
            decrypt_vec(&key, &sealed[..HEADER]).unwrap_err(),
            decrypt_error()
        );
    }

    #[test]
    fn flipped_bits_wrong_keys_and_garbage_fail() {
        let (key, mut sealed) = two_full_chunks_and_a_tail();
        assert_eq!(
            decrypt_vec(&Key::random(), &sealed).unwrap_err(),
            decrypt_error()
        );
        let last = sealed.len() - 1;
        sealed[last] ^= 1;
        assert_eq!(decrypt_vec(&key, &sealed).unwrap_err(), decrypt_error());
        assert_eq!(
            decrypt_vec(&key, b"not encrypted at all").unwrap_err(),
            decrypt_error()
        );
    }

    #[test]
    fn no_plaintext_in_the_output() {
        let key = Key::random();
        let marker = b"PLAINTEXT_MARKER_42";
        let sealed = encrypt_vec(&key, &marker.repeat(10_000));
        assert!(!sealed.windows(marker.len()).any(|w| w == marker));
    }

    #[test]
    fn file_key_is_derived_from_the_dek() {
        let dek = Key::random();
        assert_eq!(file_key(&dek), file_key(&dek));
        assert_ne!(file_key(&dek), dek);
        assert_eq!(file_key(&dek).as_bytes().len(), KEY_LEN);
    }
}

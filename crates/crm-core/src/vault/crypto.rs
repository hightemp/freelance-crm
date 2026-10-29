//! Vault cryptography (PRD §6.4): a random data key (DEK) encrypts every
//! secret with XChaCha20-Poly1305; the DEK itself is wrapped by a key
//! derived from the master password with Argon2id (KEK). Key material is
//! zeroized when dropped and never leaves the backend.

use argon2::{Algorithm, Argon2, AssociatedData, ParamsBuilder, Version};
use chacha20poly1305::aead::{Aead, Generate, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use crate::{CoreError, CoreResult};

pub const KEY_LEN: usize = 32;
pub const SALT_LEN: usize = 16;
pub const NONCE_LEN: usize = 24;

/// Argon2id cost parameters, stored with the vault so they can be raised
/// later without breaking existing vaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KdfParams {
    /// Memory in KiB.
    pub m_kib: u32,
    pub t: u32,
    pub p: u32,
}

impl Default for KdfParams {
    /// m = 64 MiB, t = 3, p = 1 (PRD §6.4).
    fn default() -> Self {
        Self {
            m_kib: 64 * 1024,
            t: 3,
            p: 1,
        }
    }
}

/// A 256-bit symmetric key (DEK, KEK or recovery key).
#[derive(Clone, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct Key([u8; KEY_LEN]);

impl std::fmt::Debug for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Key(<redacted>)")
    }
}

impl Key {
    pub fn random() -> Self {
        let mut bytes = [0u8; KEY_LEN];
        fill_random(&mut bytes);
        Self(bytes)
    }

    pub fn from_bytes(bytes: [u8; KEY_LEN]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; KEY_LEN] {
        &self.0
    }
}

/// Ciphertext with its nonce; the Poly1305 tag is part of `ciphertext`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sealed {
    pub nonce: [u8; NONCE_LEN],
    pub ciphertext: Vec<u8>,
}

/// Fills the buffer from the OS random number generator.
fn fill_random(buf: &mut [u8]) {
    getrandom::fill(buf).expect("the OS random number generator is available");
}

pub fn random_salt() -> [u8; SALT_LEN] {
    let mut salt = [0u8; SALT_LEN];
    fill_random(&mut salt);
    salt
}

/// KEK = Argon2id(password, salt, params).
pub fn derive_kek(password: &[u8], salt: &[u8; SALT_LEN], params: KdfParams) -> CoreResult<Key> {
    argon2id_with_secret(password, salt, &[], &[], params).map(Key)
}

fn cipher(key: &Key) -> XChaCha20Poly1305 {
    XChaCha20Poly1305::new(&key.0.into())
}

/// Encrypts with a fresh random nonce; `aad` is authenticated, not encrypted.
pub fn seal(key: &Key, plaintext: &[u8], aad: &[u8]) -> Sealed {
    let nonce = XNonce::generate();
    let ciphertext = cipher(key)
        .encrypt(
            &nonce,
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .expect("encryption only fails for oversized messages");
    Sealed {
        nonce: nonce.into(),
        ciphertext,
    }
}

/// Fails with `vault.decrypt` if the key, nonce, ciphertext or AAD differ.
pub fn open(key: &Key, sealed: &Sealed, aad: &[u8]) -> CoreResult<Zeroizing<Vec<u8>>> {
    cipher(key)
        .decrypt(
            &XNonce::from(sealed.nonce),
            Payload {
                msg: &sealed.ciphertext,
                aad,
            },
        )
        .map(Zeroizing::new)
        .map_err(|_| CoreError::new("vault.decrypt"))
}

/// AAD binding a wrapped DEK to its purpose.
const WRAP_AAD: &[u8] = b"freelance-crm/dek/v1";

pub fn wrap(kek: &Key, dek: &Key) -> Sealed {
    seal(kek, dek.as_bytes(), WRAP_AAD)
}

/// A wrong KEK (wrong password) fails AEAD authentication, so no separate
/// verifier is stored.
pub fn unwrap(kek: &Key, wrapped: &Sealed) -> CoreResult<Key> {
    let bytes = open(kek, wrapped, WRAP_AAD).map_err(|_| CoreError::new("vault.wrong_password"))?;
    let bytes: [u8; KEY_LEN] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| CoreError::new("vault.corrupt"))?;
    Ok(Key(bytes))
}

/// Argon2id with an optional secret and associated data (only used to
/// check the implementation against the RFC test vector).
fn argon2id_with_secret(
    password: &[u8],
    salt: &[u8],
    secret: &[u8],
    ad: &[u8],
    params: KdfParams,
) -> CoreResult<[u8; KEY_LEN]> {
    let kdf = |_| CoreError::new("internal.kdf");
    let mut builder = ParamsBuilder::new();
    builder
        .m_cost(params.m_kib)
        .t_cost(params.t)
        .p_cost(params.p);
    if !ad.is_empty() {
        builder.data(AssociatedData::new(ad).map_err(kdf)?);
    }
    let params = builder.build().map_err(kdf)?;
    let argon = if secret.is_empty() {
        Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
    } else {
        Argon2::new_with_secret(secret, Algorithm::Argon2id, Version::V0x13, params).map_err(kdf)?
    };
    let mut out = [0u8; KEY_LEN];
    argon
        .hash_password_into(password, salt, &mut out)
        .map_err(kdf)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Cheap parameters keep tests fast; production uses the defaults.
    const FAST: KdfParams = KdfParams {
        m_kib: 64,
        t: 1,
        p: 1,
    };

    #[test]
    fn key_types_zeroize_on_drop() {
        fn assert_zeroize<T: Zeroize + ZeroizeOnDrop>() {}
        assert_zeroize::<Key>();
        assert_eq!(format!("{:?}", Key::random()), "Key(<redacted>)");
    }

    #[test]
    fn default_kdf_params_follow_the_prd() {
        assert_eq!(
            KdfParams::default(),
            KdfParams {
                m_kib: 65_536,
                t: 3,
                p: 1
            }
        );
    }

    #[test]
    fn argon2id_matches_rfc_9106() {
        // RFC 9106 §5.3: Argon2id, v=0x13, m=32 KiB, t=3, p=4, with secret and AD.
        let tag = argon2id_with_secret(
            &[0x01; 32],
            &[0x02; 16],
            &[0x03; 8],
            &[0x04; 12],
            KdfParams {
                m_kib: 32,
                t: 3,
                p: 4,
            },
        )
        .unwrap();
        assert_eq!(
            tag,
            [
                0x0d, 0x64, 0x0d, 0xf5, 0x8d, 0x78, 0x76, 0x6c, 0x08, 0xc0, 0x37, 0xa3, 0x4a, 0x8b,
                0x53, 0xc9, 0xd0, 0x1e, 0xf0, 0x45, 0x2d, 0x75, 0xb6, 0x5e, 0xb5, 0x25, 0x20, 0xe9,
                0x6b, 0x01, 0xe6, 0x59
            ]
        );
    }

    #[test]
    fn kek_is_deterministic_per_password_and_salt() {
        let salt = random_salt();
        let a = derive_kek(b"correct horse", &salt, FAST).unwrap();
        assert_eq!(a, derive_kek(b"correct horse", &salt, FAST).unwrap());
        assert_ne!(a, derive_kek(b"correct horsE", &salt, FAST).unwrap());
        assert_ne!(
            a,
            derive_kek(b"correct horse", &random_salt(), FAST).unwrap()
        );
    }

    #[test]
    fn invalid_kdf_params_are_internal_errors() {
        let err = derive_kek(
            b"x",
            &random_salt(),
            KdfParams {
                m_kib: 1,
                t: 1,
                p: 1,
            },
        );
        assert_eq!(err.unwrap_err().code, "internal.kdf");
    }

    #[test]
    fn seal_and_open_round_trip() {
        let key = Key::random();
        let sealed = seal(&key, b"hunter2", b"secret:1");
        assert!(!sealed.ciphertext.windows(7).any(|w| w == b"hunter2"));
        assert_eq!(&*open(&key, &sealed, b"secret:1").unwrap(), b"hunter2");
    }

    #[test]
    fn tampering_is_detected() {
        let key = Key::random();
        let sealed = seal(&key, b"hunter2", b"secret:1");
        let decrypt = CoreError::new("vault.decrypt");

        let mut bad = sealed.clone();
        bad.ciphertext[0] ^= 1;
        assert_eq!(open(&key, &bad, b"secret:1").unwrap_err(), decrypt);

        let mut bad = sealed.clone();
        bad.nonce[0] ^= 1;
        assert_eq!(open(&key, &bad, b"secret:1").unwrap_err(), decrypt);

        assert_eq!(open(&key, &sealed, b"secret:2").unwrap_err(), decrypt);
        assert_eq!(
            open(&Key::random(), &sealed, b"secret:1").unwrap_err(),
            decrypt
        );
    }

    #[test]
    fn nonces_are_unique() {
        let key = Key::random();
        let nonces: HashSet<_> = (0..10_000).map(|_| seal(&key, b"x", b"").nonce).collect();
        assert_eq!(nonces.len(), 10_000);
    }

    #[test]
    fn wrong_password_cannot_unwrap_the_dek() {
        let salt = random_salt();
        let dek = Key::random();
        let wrapped = wrap(&derive_kek(b"right", &salt, FAST).unwrap(), &dek);
        let right = derive_kek(b"right", &salt, FAST).unwrap();
        assert_eq!(unwrap(&right, &wrapped).unwrap(), dek);
        let wrong = derive_kek(b"wrong", &salt, FAST).unwrap();
        assert_eq!(
            unwrap(&wrong, &wrapped).unwrap_err(),
            CoreError::new("vault.wrong_password")
        );
        // A wrapped DEK cannot be opened as a regular secret and vice versa.
        assert!(open(&right, &wrapped, b"").is_err());
    }

    proptest::proptest! {
        #[test]
        fn round_trip_random_data(data in proptest::collection::vec(proptest::num::u8::ANY, 0..2048),
                                  aad in proptest::collection::vec(proptest::num::u8::ANY, 0..64)) {
            let key = Key::random();
            let sealed = seal(&key, &data, &aad);
            proptest::prop_assert_eq!(&*open(&key, &sealed, &aad).unwrap(), &data);
        }
    }
}

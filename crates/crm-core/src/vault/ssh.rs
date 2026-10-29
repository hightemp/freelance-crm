//! SSH keys (FR-VA-8, 9): generation (Ed25519 by default, RSA 4096) and
//! import of OpenSSH private keys, optionally protected by a passphrase.
//! Keys are kept in OpenSSH format inside `ssh_key` secrets.

use std::path::Path;

use serde::{Deserialize, Serialize};
use ssh_key::private::{Ed25519Keypair, KeypairData, RsaKeypair};
use ssh_key::rand_core::OsRng;
use ssh_key::{HashAlg, LineEnding, PrivateKey};
use zeroize::Zeroizing;

use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum SshKeyType {
    Ed25519,
    Rsa4096,
}

/// A key pair ready to be stored as an `ssh_key` secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshKeyPair {
    /// OpenSSH private key, encrypted when a passphrase was given.
    pub private_key: Zeroizing<String>,
    /// `<algorithm> <base64> <comment>` line for `authorized_keys`.
    pub public_key: String,
    /// `ssh-ed25519`, `ssh-rsa`, …
    pub algorithm: String,
    pub comment: String,
    /// `SHA256:…`
    pub fingerprint: String,
    pub encrypted: bool,
}

const RSA_BITS: usize = 4096;

fn invalid(_: ssh_key::Error) -> CoreError {
    CoreError::new("ssh.invalid_key")
}

/// Describes `stored` (as written) using `readable`, its decrypted form
/// when available, for the comment.
fn describe(stored: &PrivateKey, readable: &PrivateKey) -> CoreResult<SshKeyPair> {
    let mut public = readable.public_key().clone();
    public.set_comment(readable.comment());
    Ok(SshKeyPair {
        private_key: stored.to_openssh(LineEnding::LF).map_err(invalid)?,
        public_key: public.to_openssh().map_err(invalid)?,
        algorithm: public.algorithm().as_str().to_owned(),
        comment: readable.comment().to_owned(),
        fingerprint: public.fingerprint(HashAlg::Sha256).to_string(),
        encrypted: stored.is_encrypted(),
    })
}

pub fn generate(
    key_type: SshKeyType,
    comment: &str,
    passphrase: Option<&str>,
) -> CoreResult<SshKeyPair> {
    let keypair = match key_type {
        SshKeyType::Ed25519 => KeypairData::from(Ed25519Keypair::random(&mut OsRng)),
        SshKeyType::Rsa4096 => {
            KeypairData::from(RsaKeypair::random(&mut OsRng, RSA_BITS).map_err(invalid)?)
        }
    };
    let key = PrivateKey::new(keypair, comment.trim()).map_err(invalid)?;
    let stored = match passphrase.filter(|p| !p.is_empty()) {
        Some(p) => key.encrypt(&mut OsRng, p).map_err(invalid)?,
        None => key.clone(),
    };
    describe(&stored, &key)
}

/// Reads an OpenSSH private key. An encrypted key is checked against the
/// passphrase when one is given; it stays encrypted as stored.
pub fn import(pem: &str, passphrase: Option<&str>) -> CoreResult<SshKeyPair> {
    let stored = PrivateKey::from_openssh(pem.trim()).map_err(invalid)?;
    let readable = match passphrase.filter(|p| !p.is_empty()) {
        Some(p) if stored.is_encrypted() => stored
            .decrypt(p)
            .map_err(|_| CoreError::new("ssh.wrong_passphrase"))?,
        _ => stored.clone(),
    };
    describe(&stored, &readable)
}

/// Writes a private key file readable only by the owner (0600 on Unix).
pub fn export_private_key(private_key: &str, path: &Path) -> CoreResult<()> {
    let io = |_| CoreError::new("ssh.export_failed");
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        options.mode(0o600);
        // `mode` only applies to new files; tighten an existing one first.
        if path.exists() {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).map_err(io)?;
        }
    }
    let mut file = options.open(path).map_err(io)?;
    std::io::Write::write_all(&mut file, private_key.as_bytes()).map_err(io)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn public_of(private_key: &str, passphrase: Option<&str>) -> String {
        let key = ssh_key::PrivateKey::from_openssh(private_key).unwrap();
        let key = match passphrase {
            Some(p) => key.decrypt(p).unwrap(),
            None => key,
        };
        key.public_key().to_openssh().unwrap()
    }

    #[test]
    fn generates_ed25519_keys() {
        let pair = generate(SshKeyType::Ed25519, "me@laptop", None).unwrap();
        assert_eq!(pair.algorithm, "ssh-ed25519");
        assert_eq!(pair.comment, "me@laptop");
        assert!(!pair.encrypted);
        assert!(
            pair.private_key
                .starts_with("-----BEGIN OPENSSH PRIVATE KEY-----")
        );
        assert!(pair.public_key.starts_with("ssh-ed25519 "));
        assert!(pair.public_key.ends_with(" me@laptop"));
        assert!(pair.fingerprint.starts_with("SHA256:"));
        // The public key belongs to the private one.
        assert_eq!(public_of(&pair.private_key, None), pair.public_key);
    }

    #[test]
    fn generates_rsa_4096_keys() {
        let pair = generate(SshKeyType::Rsa4096, "deploy", None).unwrap();
        assert_eq!(pair.algorithm, "ssh-rsa");
        let key = ssh_key::PrivateKey::from_openssh(&*pair.private_key).unwrap();
        assert_eq!(
            key.key_data()
                .rsa()
                .unwrap()
                .public
                .n
                .as_positive_bytes()
                .unwrap()
                .len(),
            512
        );
        assert_eq!(public_of(&pair.private_key, None), pair.public_key);
    }

    #[test]
    fn a_passphrase_protects_the_key() {
        let pair = generate(SshKeyType::Ed25519, "x", Some("open sesame")).unwrap();
        assert!(pair.encrypted);
        assert_eq!(
            public_of(&pair.private_key, Some("open sesame")),
            pair.public_key
        );
        assert_eq!(
            import(&pair.private_key, Some("wrong")).unwrap_err(),
            CoreError::new("ssh.wrong_passphrase")
        );
        let imported = import(&pair.private_key, Some("open sesame")).unwrap();
        assert_eq!(imported.public_key, pair.public_key);
        assert!(imported.encrypted, "stored as given, still encrypted");
        // Without a passphrase the public part is still readable (the
        // comment is in the encrypted section, so compare fingerprints).
        let locked = import(&pair.private_key, None).unwrap();
        assert_eq!(locked.fingerprint, pair.fingerprint);
        assert!(locked.encrypted);
    }

    #[test]
    fn import_detects_type_and_comment() {
        let original = generate(SshKeyType::Ed25519, "work key", None).unwrap();
        let imported = import(&format!("\n  {}  \n", *original.private_key), None).unwrap();
        assert_eq!(imported.algorithm, "ssh-ed25519");
        assert_eq!(imported.comment, "work key");
        assert_eq!(imported.public_key, original.public_key);
        assert_eq!(imported.fingerprint, original.fingerprint);
    }

    #[test]
    fn broken_keys_are_rejected() {
        let invalid = CoreError::new("ssh.invalid_key");
        assert_eq!(import("not a key", None).unwrap_err(), invalid);
        let pair = generate(SshKeyType::Ed25519, "x", None).unwrap();
        let truncated = &pair.private_key[..pair.private_key.len() / 2];
        assert_eq!(import(truncated, None).unwrap_err(), invalid);
    }

    #[test]
    fn exports_a_private_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("id_ed25519");
        let pair = generate(SshKeyType::Ed25519, "x", None).unwrap();
        export_private_key(&pair.private_key, &path).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), *pair.private_key);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        // Overwriting an existing file also leaves it private.
        export_private_key(&pair.private_key, &path).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }
}

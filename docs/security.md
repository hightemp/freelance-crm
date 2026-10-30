# Security

How freelance-crm protects data: what is encrypted, with which keys, and
what is not. The code is in `crates/crm-core/src/vault` (keys and secrets),
`crates/crm-core/src/files` (encrypted files) and
`crates/crm-core/src/backup` (backups).

## Threat model

The app is local-first: there is no server and no account, and nothing is
sent over the network. The database (`crm.db`, SQLite) and the stored files
live in the app's data folder.

- **Protected by encryption:** the values of secrets in the vault and the
  files marked as encrypted. Someone who gets the data folder or a backup
  file, but not the master password, cannot read them.
- **Not encrypted at rest:** everything else (clients, projects, tasks,
  invoices, notes, plain files, secret titles). Protect the computer with
  full-disk encryption and an OS account password. Backups encrypt
  everything, see [Backups](#backups).
- **Out of scope:** malware running as your user while the vault is unlocked,
  and a compromised operating system.

## Keys

```
master password ──Argon2id──▶ KEK ──wraps──▶ DEK ◀──wraps── recovery key (HKDF)
                                              │
                         ┌────────────────────┼─────────────────────┐
                         ▼                    ▼                     ▼
                  secret payloads   file key (HKDF "file")   backup passphrase
                 (XChaCha20-Poly1305)  (STREAM, 64 KiB chunks)   (vault mode)
```

- **DEK** (data encryption key): 256 random bits, created with the vault. All
  vault data is encrypted with it or with keys derived from it. It never
  changes, so changing the master password does not re-encrypt anything.
- **KEK** (key encryption key): derived from the master password with
  **Argon2id** (64 MiB of memory, 3 passes, 1 lane by default; the parameters
  and a random 16-byte salt are stored with the vault so that they can be
  raised later). The KEK wraps the DEK with XChaCha20-Poly1305 and the AAD
  `freelance-crm/dek/v1`. A wrong password fails authentication; no password
  hash or verifier is stored.
- **Recovery key**: 256 random bits shown once as 14 groups of base32 with a
  checksum. A key derived from it with HKDF-SHA256 wraps the same DEK a second
  time, so it can set a new master password if the old one is forgotten. The
  app never stores the recovery key.
- Keys in memory are zeroed when dropped (`zeroize`). The DEK is held only
  while the vault is unlocked; the vault locks itself after a period of
  inactivity (15 minutes by default, configurable), and on demand.

## Secrets

A secret's title, kind, links and expiry date are stored in plain text, so
they can be listed and searched. All its values (passwords, keys, tokens,
custom fields) form one JSON payload that is encrypted with the DEK using
**XChaCha20-Poly1305** with a random 192-bit nonce. The AAD
`secret:<id>:<kind>:<key version>` binds the ciphertext to its record, so a
payload copied into another secret does not decrypt.

Decrypted values exist only in memory, are not written to logs (debug output
redacts them) and are never added to the search index.

## Files

Files attached to records are stored by content hash (SHA-256) and
deduplicated. Files marked as encrypted (and every file of a secret) are
encrypted with a **file key** derived from the DEK with HKDF-SHA256
(`info = "file"`), using the STREAM construction over XChaCha20-Poly1305 in
64 KiB chunks, so large files are processed without loading them whole and
truncation or reordering is detected. Encrypted files get random names, so
equal contents cannot be recognized.

Opening an encrypted file decrypts it into a private temporary folder that
is wiped when the vault locks, when the app quits and at the next start.

## Backups

A backup file (`.fcrm-backup`) starts with a short plain header saying how
it is opened, followed by an [age](https://age-encryption.org) file
encrypted with a passphrase (scrypt). Inside is a zip with a consistent
snapshot of the database (`VACUUM INTO`), every stored file and a manifest
with the SHA-256 of each. Restoring checks the hashes and the schema version
before anything is replaced, and backs up the current data first.

- **Manual backups** use a passphrase you choose (at least 8 characters).
- **Automatic backups** use either a passphrase kept in the OS keychain
  (macOS Keychain, Windows Credential Manager, Secret Service on Linux), or
  the vault: a random passphrase sealed with the DEK, with the vault's
  wrapped DEK, salt and Argon2id parameters in the header. Such a backup
  opens with the master password on any computer; the header alone gives
  nothing without it.

Secrets and encrypted files stay encrypted inside a backup as well: after a
restore they still need the master password.

The JSON and CSV export is not encrypted and leaves out secrets entirely.

## The app

- The interface runs under a strict content security policy: scripts only
  from the app, no `eval`, no remote content.
- The webview has no file system, shell, HTTP or process permissions; files
  are chosen in native dialogs and read by the Rust side. Attachment images
  are served by a custom protocol by id only.
- Markdown is sanitized (DOMPurify) before it is shown.
- There is no telemetry and no network access besides links you open.

## Reporting a problem

Please report security issues privately to the maintainer rather than in
public issues.

// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! # vex-crypto
//!
//! Cryptographic primitives for the Vex browser engine.
//!
//! - Symmetric encryption: ChaCha20-Poly1305
//! - Key derivation: Argon2id
//! - Signatures: Ed25519
//! - Key exchange: X25519
//! - All secrets `Zeroize` on drop

use chacha20poly1305::aead::{Aead, KeyInit, OsRng};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use rand::RngCore;
use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey};
use zeroize::Zeroize;

/// Errors from cryptographic operations.
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    /// Encryption failed.
    #[error("encryption failed")]
    EncryptionFailed,

    /// Decryption failed (wrong key or corrupted data).
    #[error("decryption failed")]
    DecryptionFailed,

    /// Key derivation failed.
    #[error("key derivation failed: {0}")]
    DerivationFailed(String),

    /// Invalid signature.
    #[error("invalid signature")]
    InvalidSignature,

    /// Invalid key material.
    #[error("invalid key: {0}")]
    InvalidKey(String),
}

/// ChaCha20-Poly1305 symmetric key (32 bytes), zeroized on drop.
#[derive(Clone, Zeroize)]
#[zeroize(drop)]
pub struct SymmetricKey {
    bytes: [u8; 32],
}

impl SymmetricKey {
    /// Generate a random symmetric key.
    pub fn generate() -> Self {
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        Self { bytes }
    }

    /// Create from raw bytes.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }

    /// View the raw key bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }
}

impl std::fmt::Debug for SymmetricKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SymmetricKey([REDACTED])")
    }
}

/// Nonce size for ChaCha20-Poly1305 (12 bytes).
const NONCE_SIZE: usize = 12;

/// Encrypt plaintext with ChaCha20-Poly1305.
///
/// Returns nonce (12 bytes) prepended to ciphertext.
pub fn encrypt(key: &SymmetricKey, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let cipher = ChaCha20Poly1305::new(key.bytes.as_ref().into());

    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| CryptoError::EncryptionFailed)?;

    let mut out = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Decrypt ciphertext produced by [`encrypt`].
///
/// Expects nonce (12 bytes) prepended to ciphertext.
pub fn decrypt(key: &SymmetricKey, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if data.len() < NONCE_SIZE {
        return Err(CryptoError::DecryptionFailed);
    }

    let (nonce_bytes, ciphertext) = data.split_at(NONCE_SIZE);
    let cipher = ChaCha20Poly1305::new(key.bytes.as_ref().into());
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| CryptoError::DecryptionFailed)
}

/// Derive a symmetric key from a password using Argon2id.
///
/// `salt` should be at least 16 bytes and unique per user.
pub fn derive_key(password: &[u8], salt: &[u8]) -> Result<SymmetricKey, CryptoError> {
    let params = argon2::Params::new(65536, 3, 1, Some(32))
        .map_err(|e| CryptoError::DerivationFailed(e.to_string()))?;
    let argon = argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    let mut output = [0u8; 32];
    argon
        .hash_password_into(password, salt, &mut output)
        .map_err(|e| CryptoError::DerivationFailed(e.to_string()))?;

    Ok(SymmetricKey::from_bytes(output))
}

/// Ed25519 signing key pair.
pub struct SigningKeyPair {
    inner: SigningKey,
}

impl SigningKeyPair {
    /// Generate a new random signing key pair.
    pub fn generate() -> Self {
        let inner = SigningKey::generate(&mut OsRng);
        Self { inner }
    }

    /// Create from raw secret key bytes (32 bytes).
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        let inner = SigningKey::from_bytes(bytes);
        Self { inner }
    }

    /// Get the public verifying key (32 bytes).
    pub fn public_key(&self) -> [u8; 32] {
        self.inner.verifying_key().to_bytes()
    }

    /// Sign a message.
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.inner.sign(message).to_bytes()
    }
}

impl std::fmt::Debug for SigningKeyPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SigningKeyPair([REDACTED])")
    }
}

/// Verify an Ed25519 signature.
pub fn verify_signature(
    public_key: &[u8; 32],
    message: &[u8],
    signature: &[u8; 64],
) -> Result<(), CryptoError> {
    let verifying_key = VerifyingKey::from_bytes(public_key)
        .map_err(|_| CryptoError::InvalidKey("invalid Ed25519 public key".into()))?;
    let sig = ed25519_dalek::Signature::from_bytes(signature);
    verifying_key
        .verify(message, &sig)
        .map_err(|_| CryptoError::InvalidSignature)
}

/// Perform X25519 key exchange.
///
/// Returns `(our_public_key, shared_secret)`.
pub fn key_exchange(their_public: &[u8; 32]) -> ([u8; 32], [u8; 32]) {
    let secret = EphemeralSecret::random_from_rng(OsRng);
    let our_public = X25519PublicKey::from(&secret);
    let their_key = X25519PublicKey::from(*their_public);
    let shared = secret.diffie_hellman(&their_key);
    (our_public.to_bytes(), *shared.as_bytes())
}

/// Generate random bytes.
pub fn random_bytes(len: usize) -> Vec<u8> {
    let mut buf = vec![0u8; len];
    rand::thread_rng().fill_bytes(&mut buf);
    buf
}

/// Generate a random 16-byte salt for key derivation.
pub fn generate_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);
    salt
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let key = SymmetricKey::generate();
        let plaintext = b"Hello, Vigo Browser!";
        let encrypted = encrypt(&key, plaintext).unwrap();
        let decrypted = decrypt(&key, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn decrypt_wrong_key_fails() {
        let key1 = SymmetricKey::generate();
        let key2 = SymmetricKey::generate();
        let encrypted = encrypt(&key1, b"secret").unwrap();
        assert!(decrypt(&key2, &encrypted).is_err());
    }

    #[test]
    fn decrypt_truncated_data_fails() {
        let key = SymmetricKey::generate();
        assert!(decrypt(&key, &[0u8; 5]).is_err());
    }

    #[test]
    fn derive_key_produces_consistent_output() {
        let password = b"hunter2";
        let salt = b"0123456789abcdef";
        let k1 = derive_key(password, salt).unwrap();
        let k2 = derive_key(password, salt).unwrap();
        assert_eq!(k1.as_bytes(), k2.as_bytes());
    }

    #[test]
    fn derive_key_different_salt_different_output() {
        let password = b"hunter2";
        let k1 = derive_key(password, b"salt_aaaaaabbbbbb").unwrap();
        let k2 = derive_key(password, b"salt_ccccccdddddd").unwrap();
        assert_ne!(k1.as_bytes(), k2.as_bytes());
    }

    #[test]
    fn sign_verify_roundtrip() {
        let keypair = SigningKeyPair::generate();
        let message = b"Vex engine signed message";
        let signature = keypair.sign(message);
        let pubkey = keypair.public_key();
        assert!(verify_signature(&pubkey, message, &signature).is_ok());
    }

    #[test]
    fn verify_wrong_message_fails() {
        let keypair = SigningKeyPair::generate();
        let signature = keypair.sign(b"original message");
        let pubkey = keypair.public_key();
        assert!(verify_signature(&pubkey, b"tampered message", &signature).is_err());
    }

    #[test]
    fn key_exchange_produces_shared_secret() {
        // Simulate two parties.
        let secret_a = EphemeralSecret::random_from_rng(OsRng);
        let public_a = X25519PublicKey::from(&secret_a);

        let secret_b = EphemeralSecret::random_from_rng(OsRng);
        let public_b = X25519PublicKey::from(&secret_b);

        let shared_a = secret_a.diffie_hellman(&public_b);
        let shared_b = secret_b.diffie_hellman(&public_a);
        assert_eq!(shared_a.as_bytes(), shared_b.as_bytes());
    }

    #[test]
    fn random_bytes_length() {
        assert_eq!(random_bytes(32).len(), 32);
        assert_eq!(random_bytes(0).len(), 0);
    }

    #[test]
    fn symmetric_key_debug_redacted() {
        let key = SymmetricKey::generate();
        let debug = format!("{:?}", key);
        assert!(debug.contains("REDACTED"));
        assert!(!debug.contains("bytes"));
    }

    #[test]
    fn generate_salt_uniqueness() {
        let s1 = generate_salt();
        let s2 = generate_salt();
        assert_ne!(s1, s2);
    }
}

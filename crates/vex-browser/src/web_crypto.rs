// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Web Crypto API — browser-facing SubtleCrypto interface.
//!
//! Provides key generation, encrypt/decrypt, sign/verify, digest,
//! import/export for web-compatible cryptographic operations.
//!
//! Operations without a standards-compliant provider fail closed. In
//! particular, this module must never substitute demo cryptography for the
//! Web Crypto API's AES or asymmetric algorithms.

use std::collections::HashMap;

use sha1::Sha1;
use sha2::{Digest, Sha256, Sha384, Sha512};

// ── Types ────────────────────────────────────────────────────────────────────

/// Supported cryptographic algorithms.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CryptoAlgorithm {
    /// AES-GCM (128/256 bit)
    AesGcm { length: u16 },
    /// AES-CBC (128/256 bit)
    AesCbc { length: u16 },
    /// RSA-OAEP
    RsaOaep {
        modulus_length: u32,
        hash: HashAlgorithm,
    },
    /// RSASSA-PKCS1-v1_5
    RsassaPkcs1V15 {
        modulus_length: u32,
        hash: HashAlgorithm,
    },
    /// ECDSA
    Ecdsa { named_curve: EcCurve },
    /// ECDH
    Ecdh { named_curve: EcCurve },
    /// HMAC
    Hmac {
        hash: HashAlgorithm,
        length: Option<u32>,
    },
    /// PBKDF2
    Pbkdf2,
    /// HKDF
    Hkdf,
}

/// Hash algorithms for crypto operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HashAlgorithm {
    Sha1,
    Sha256,
    Sha384,
    Sha512,
}

impl HashAlgorithm {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sha1 => "SHA-1",
            Self::Sha256 => "SHA-256",
            Self::Sha384 => "SHA-384",
            Self::Sha512 => "SHA-512",
        }
    }

    pub fn from_label(s: &str) -> Option<Self> {
        match s.to_ascii_uppercase().as_str() {
            "SHA-1" => Some(Self::Sha1),
            "SHA-256" => Some(Self::Sha256),
            "SHA-384" => Some(Self::Sha384),
            "SHA-512" => Some(Self::Sha512),
            _ => None,
        }
    }

    /// Output length in bytes.
    pub fn output_length(&self) -> usize {
        match self {
            Self::Sha1 => 20,
            Self::Sha256 => 32,
            Self::Sha384 => 48,
            Self::Sha512 => 64,
        }
    }
}

/// Elliptic curve names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EcCurve {
    P256,
    P384,
    P521,
}

impl EcCurve {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::P256 => "P-256",
            Self::P384 => "P-384",
            Self::P521 => "P-521",
        }
    }
}

/// Key usages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyUsage {
    Encrypt,
    Decrypt,
    Sign,
    Verify,
    DeriveKey,
    DeriveBits,
    WrapKey,
    UnwrapKey,
}

/// Key type classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyType {
    Public,
    Private,
    Secret,
}

/// A CryptoKey handle.
#[derive(Debug, Clone)]
pub struct CryptoKey {
    pub id: u64,
    pub key_type: KeyType,
    pub extractable: bool,
    pub algorithm: CryptoAlgorithm,
    pub usages: Vec<KeyUsage>,
    /// Raw key material (opaque).
    pub(crate) raw: Vec<u8>,
}

/// A key pair.
#[derive(Debug, Clone)]
pub struct CryptoKeyPair {
    pub public_key: CryptoKey,
    pub private_key: CryptoKey,
}

/// Key format for import/export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyFormat {
    Raw,
    Pkcs8,
    Spki,
    Jwk,
}

/// Crypto error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CryptoError {
    NotSupported(String),
    InvalidKey(String),
    InvalidData(String),
    OperationError(String),
    DataError(String),
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotSupported(m) => write!(f, "NotSupportedError: {m}"),
            Self::InvalidKey(m) => write!(f, "InvalidAccessError: {m}"),
            Self::InvalidData(m) => write!(f, "InvalidStateError: {m}"),
            Self::OperationError(m) => write!(f, "OperationError: {m}"),
            Self::DataError(m) => write!(f, "DataError: {m}"),
        }
    }
}

// ── SubtleCrypto ─────────────────────────────────────────────────────────────

/// SubtleCrypto — the low-level crypto operations interface.
pub struct SubtleCrypto {
    next_key_id: u64,
    key_store: HashMap<u64, CryptoKey>,
}

impl Default for SubtleCrypto {
    fn default() -> Self {
        Self::new()
    }
}

impl SubtleCrypto {
    pub fn new() -> Self {
        Self {
            next_key_id: 1,
            key_store: HashMap::new(),
        }
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_key_id;
        self.next_key_id += 1;
        id
    }

    /// Generate a symmetric key.
    pub fn generate_key(
        &mut self,
        algorithm: CryptoAlgorithm,
        extractable: bool,
        usages: Vec<KeyUsage>,
    ) -> Result<CryptoKey, CryptoError> {
        let key_len = match &algorithm {
            CryptoAlgorithm::AesGcm { length } | CryptoAlgorithm::AesCbc { length } => {
                if *length != 128 && *length != 256 {
                    return Err(CryptoError::NotSupported(format!(
                        "AES key length {length} not supported"
                    )));
                }
                (*length / 8) as usize
            }
            CryptoAlgorithm::Hmac { hash, .. } => hash.output_length(),
            _ => {
                return Err(CryptoError::NotSupported(
                    "Use generate_key_pair for asymmetric algorithms".to_string(),
                ))
            }
        };

        // Generate random key material
        let raw = generate_random_bytes(key_len);
        let id = self.next_id();
        let key = CryptoKey {
            id,
            key_type: KeyType::Secret,
            extractable,
            algorithm,
            usages,
            raw,
        };
        self.key_store.insert(id, key.clone());
        Ok(key)
    }

    /// Generate an asymmetric key pair.
    ///
    /// This fails until a standards-compliant asymmetric provider is wired.
    pub fn generate_key_pair(
        &mut self,
        algorithm: CryptoAlgorithm,
        extractable: bool,
        usages: Vec<KeyUsage>,
    ) -> Result<CryptoKeyPair, CryptoError> {
        let _ = (algorithm, extractable, usages);
        Err(CryptoError::NotSupported(
            "asymmetric WebCrypto key generation is not available yet".to_string(),
        ))
    }

    /// Import a key from raw bytes.
    pub fn import_key(
        &mut self,
        format: KeyFormat,
        key_data: &[u8],
        algorithm: CryptoAlgorithm,
        extractable: bool,
        usages: Vec<KeyUsage>,
    ) -> Result<CryptoKey, CryptoError> {
        if format != KeyFormat::Raw {
            return Err(CryptoError::NotSupported(format!(
                "Import format {format:?} not yet supported"
            )));
        }

        let id = self.next_id();
        let key = CryptoKey {
            id,
            key_type: KeyType::Secret,
            extractable,
            algorithm,
            usages,
            raw: key_data.to_vec(),
        };
        self.key_store.insert(id, key.clone());
        Ok(key)
    }

    /// Export a key.
    pub fn export_key(&self, format: KeyFormat, key: &CryptoKey) -> Result<Vec<u8>, CryptoError> {
        if !key.extractable {
            return Err(CryptoError::InvalidKey(
                "Key is not extractable".to_string(),
            ));
        }
        if format != KeyFormat::Raw {
            return Err(CryptoError::NotSupported(format!(
                "Export format {format:?} not yet supported"
            )));
        }
        Ok(key.raw.clone())
    }

    /// Compute a digest (hash).
    pub fn digest(&self, algorithm: HashAlgorithm, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        Ok(match algorithm {
            HashAlgorithm::Sha1 => Sha1::digest(data).to_vec(),
            HashAlgorithm::Sha256 => Sha256::digest(data).to_vec(),
            HashAlgorithm::Sha384 => Sha384::digest(data).to_vec(),
            HashAlgorithm::Sha512 => Sha512::digest(data).to_vec(),
        })
    }

    /// Encrypt data.
    ///
    /// This fails until AES-GCM/CBC is backed by a standards-compliant
    /// implementation with the required Web Crypto parameters.
    pub fn encrypt(&self, key: &CryptoKey, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if !key.usages.contains(&KeyUsage::Encrypt) {
            return Err(CryptoError::InvalidKey(
                "Key does not support encrypt".to_string(),
            ));
        }

        let _ = data;
        Err(CryptoError::NotSupported(
            "AES encryption is unavailable until standards-compliant AES-GCM/CBC support is wired"
                .to_string(),
        ))
    }

    /// Decrypt data.
    pub fn decrypt(&self, key: &CryptoKey, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if !key.usages.contains(&KeyUsage::Decrypt) {
            return Err(CryptoError::InvalidKey(
                "Key does not support decrypt".to_string(),
            ));
        }

        let _ = data;
        Err(CryptoError::NotSupported(
            "AES decryption is unavailable until standards-compliant AES-GCM/CBC support is wired"
                .to_string(),
        ))
    }

    /// Get random values (fills buffer with random bytes).
    pub fn get_random_values(&self, buffer: &mut [u8]) -> Result<(), CryptoError> {
        if buffer.len() > 65536 {
            return Err(CryptoError::DataError(
                "Buffer exceeds 65536 bytes".to_string(),
            ));
        }
        let random = generate_random_bytes(buffer.len());
        buffer.copy_from_slice(&random);
        Ok(())
    }

    /// Generate a random UUID v4.
    pub fn random_uuid(&self) -> String {
        let mut bytes = generate_random_bytes(16);
        // Set version 4: bits 4-7 of byte 6
        bytes[6] = (bytes[6] & 0x0F) | 0x40;
        // Set variant 1: bits 6-7 of byte 8
        bytes[8] = (bytes[8] & 0x3F) | 0x80;

        format!(
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            bytes[0], bytes[1], bytes[2], bytes[3],
            bytes[4], bytes[5],
            bytes[6], bytes[7],
            bytes[8], bytes[9],
            bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        )
    }
}

fn generate_random_bytes(len: usize) -> Vec<u8> {
    vex_crypto::random_bytes(len)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_aes_key() {
        let mut crypto = SubtleCrypto::new();
        let key = crypto
            .generate_key(
                CryptoAlgorithm::AesGcm { length: 256 },
                true,
                vec![KeyUsage::Encrypt, KeyUsage::Decrypt],
            )
            .unwrap();
        assert_eq!(key.key_type, KeyType::Secret);
        assert_eq!(key.raw.len(), 32);
        assert!(key.extractable);
    }

    #[test]
    fn generate_invalid_aes_length() {
        let mut crypto = SubtleCrypto::new();
        let err = crypto
            .generate_key(
                CryptoAlgorithm::AesGcm { length: 192 },
                true,
                vec![KeyUsage::Encrypt],
            )
            .unwrap_err();
        assert!(matches!(err, CryptoError::NotSupported(_)));
    }

    #[test]
    fn asymmetric_key_generation_is_not_advertised_without_real_key_material() {
        let mut crypto = SubtleCrypto::new();
        let pair = crypto
            .generate_key_pair(
                CryptoAlgorithm::Ecdsa {
                    named_curve: EcCurve::P256,
                },
                true,
                vec![KeyUsage::Sign, KeyUsage::Verify],
            )
            .unwrap_err();
        assert!(matches!(pair, CryptoError::NotSupported(_)));
    }

    #[test]
    fn aes_operations_fail_closed_until_a_real_provider_is_wired() {
        let mut crypto = SubtleCrypto::new();
        let key = crypto
            .generate_key(
                CryptoAlgorithm::AesGcm { length: 128 },
                false,
                vec![KeyUsage::Encrypt, KeyUsage::Decrypt],
            )
            .unwrap();

        assert!(matches!(
            crypto.encrypt(&key, b"hello world"),
            Err(CryptoError::NotSupported(_))
        ));
        assert!(matches!(
            crypto.decrypt(&key, b"ciphertext"),
            Err(CryptoError::NotSupported(_))
        ));
    }

    #[test]
    fn encrypt_wrong_usage() {
        let mut crypto = SubtleCrypto::new();
        let key = crypto
            .generate_key(
                CryptoAlgorithm::AesGcm { length: 128 },
                false,
                vec![KeyUsage::Decrypt], // No encrypt usage
            )
            .unwrap();
        assert!(crypto.encrypt(&key, b"test").is_err());
    }

    #[test]
    fn digest_produces_correct_length() {
        let crypto = SubtleCrypto::new();
        let hash = crypto.digest(HashAlgorithm::Sha256, b"test").unwrap();
        assert_eq!(hash.len(), 32);

        let hash512 = crypto.digest(HashAlgorithm::Sha512, b"test").unwrap();
        assert_eq!(hash512.len(), 64);
    }

    #[test]
    fn digest_deterministic() {
        let crypto = SubtleCrypto::new();
        let h1 = crypto.digest(HashAlgorithm::Sha256, b"test").unwrap();
        let h2 = crypto.digest(HashAlgorithm::Sha256, b"test").unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn digest_matches_sha256_test_vector() {
        let crypto = SubtleCrypto::new();
        let hash = crypto.digest(HashAlgorithm::Sha256, b"abc").unwrap();
        assert_eq!(
            hash,
            vec![
                0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
                0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
                0xf2, 0x00, 0x15, 0xad,
            ]
        );
    }

    #[test]
    fn import_export_key() {
        let mut crypto = SubtleCrypto::new();
        let raw_key = vec![0x42; 16];
        let key = crypto
            .import_key(
                KeyFormat::Raw,
                &raw_key,
                CryptoAlgorithm::AesGcm { length: 128 },
                true,
                vec![KeyUsage::Encrypt],
            )
            .unwrap();

        let exported = crypto.export_key(KeyFormat::Raw, &key).unwrap();
        assert_eq!(exported, raw_key);
    }

    #[test]
    fn export_non_extractable() {
        let mut crypto = SubtleCrypto::new();
        let key = crypto
            .generate_key(
                CryptoAlgorithm::AesGcm { length: 128 },
                false, // not extractable
                vec![KeyUsage::Encrypt],
            )
            .unwrap();
        assert!(crypto.export_key(KeyFormat::Raw, &key).is_err());
    }

    #[test]
    fn get_random_values() {
        let crypto = SubtleCrypto::new();
        let mut buf = [0u8; 32];
        crypto.get_random_values(&mut buf).unwrap();
        // At least some bytes should be non-zero
        assert!(buf.iter().any(|&b| b != 0));
    }

    #[test]
    fn get_random_values_too_large() {
        let crypto = SubtleCrypto::new();
        let mut buf = vec![0u8; 70000];
        assert!(crypto.get_random_values(&mut buf).is_err());
    }

    #[test]
    fn random_uuid_format() {
        let crypto = SubtleCrypto::new();
        let uuid = crypto.random_uuid();
        // UUID v4 format: xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx
        assert_eq!(uuid.len(), 36);
        assert_eq!(uuid.chars().nth(8), Some('-'));
        assert_eq!(uuid.chars().nth(13), Some('-'));
        // Byte 6 high nibble = 4 (version)
        assert_eq!(uuid.chars().nth(14), Some('4'));
        assert_eq!(uuid.chars().nth(18), Some('-'));
        // Byte 8 high nibble should be 8, 9, a, or b (variant 1)
        let variant_char = uuid.chars().nth(19).unwrap();
        assert!(
            "89ab".contains(variant_char),
            "variant nibble should be 8/9/a/b, got {variant_char}"
        );
        assert_eq!(uuid.chars().nth(23), Some('-'));
    }

    #[test]
    fn hash_algorithm_from_label() {
        assert_eq!(
            HashAlgorithm::from_label("SHA-256"),
            Some(HashAlgorithm::Sha256)
        );
        assert_eq!(
            HashAlgorithm::from_label("sha-512"),
            Some(HashAlgorithm::Sha512)
        );
        assert_eq!(HashAlgorithm::from_label("md5"), None);
    }

    #[test]
    fn hmac_key_generation() {
        let mut crypto = SubtleCrypto::new();
        let key = crypto
            .generate_key(
                CryptoAlgorithm::Hmac {
                    hash: HashAlgorithm::Sha256,
                    length: None,
                },
                true,
                vec![KeyUsage::Sign, KeyUsage::Verify],
            )
            .unwrap();
        assert_eq!(key.raw.len(), 32); // SHA-256 output length
    }
}

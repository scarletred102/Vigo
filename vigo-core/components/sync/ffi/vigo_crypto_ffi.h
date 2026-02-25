// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

// C FFI header for the Rust crypto module (vigo_crypto crate).
// This header must stay synchronised with the Rust FFI exports in
// vigo-core/rust/vigo_crypto/src/ffi.rs.
//
// All cryptographic operations in Vigo MUST go through these functions.
// No direct calls to BoringSSL, OpenSSL, or OS crypto APIs for auth crypto.

#ifndef VIGO_COMPONENTS_SYNC_FFI_VIGO_CRYPTO_FFI_H_
#define VIGO_COMPONENTS_SYNC_FFI_VIGO_CRYPTO_FFI_H_

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// ─── Constants ──────────────────────────────────────────────────────────────

#define VIGO_CRYPTO_AEAD_KEY_SIZE 32
#define VIGO_CRYPTO_AEAD_NONCE_SIZE 24
#define VIGO_CRYPTO_AEAD_TAG_SIZE 16
#define VIGO_CRYPTO_KDF_SALT_SIZE 16
#define VIGO_CRYPTO_X25519_KEY_SIZE 32
#define VIGO_CRYPTO_ED25519_KEY_SIZE 32
#define VIGO_CRYPTO_ED25519_SIG_SIZE 64

// ─── Lifecycle ──────────────────────────────────────────────────────────────

// Initialise the crypto subsystem. Must be called once at startup.
// Returns true on success.
bool vigo_crypto_init(void);

// ─── Buffer Management ─────────────────────────────────────────────────────

// A buffer returned by Rust crypto functions.
// Caller must free with vigo_crypto_free_buffer().
typedef struct {
  uint8_t* data;
  size_t len;
} VigoCryptoBuffer;

// Free a buffer previously returned by a vigo_crypto_* function.
void vigo_crypto_free_buffer(VigoCryptoBuffer buf);

// ─── AEAD: XChaCha20-Poly1305 ──────────────────────────────────────────────

// Encrypt using XChaCha20-Poly1305.
// Returns a sealed buffer: nonce (24 bytes) || ciphertext || tag (16 bytes).
// Caller must free the returned buffer with vigo_crypto_free_buffer().
// Returns {NULL, 0} on failure.
VigoCryptoBuffer vigo_crypto_aead_seal(
    const uint8_t* key,          // 32 bytes
    const uint8_t* plaintext,
    size_t plaintext_len,
    const uint8_t* aad,          // may be NULL if aad_len is 0
    size_t aad_len);

// Decrypt a sealed AEAD buffer (nonce || ciphertext || tag).
// Returns decrypted plaintext. Caller must free with vigo_crypto_free_buffer().
// Returns {NULL, 0} if authentication fails.
VigoCryptoBuffer vigo_crypto_aead_open(
    const uint8_t* key,          // 32 bytes
    const uint8_t* sealed_data,
    size_t sealed_len,
    const uint8_t* aad,          // may be NULL if aad_len is 0
    size_t aad_len);

// ─── KDF: Argon2id ─────────────────────────────────────────────────────────

// Derive a key from a passphrase using Argon2id.
// |salt| must be exactly 16 bytes.
// Returns derived key. Caller must free with vigo_crypto_free_buffer().
VigoCryptoBuffer vigo_crypto_kdf_argon2id(
    const uint8_t* passphrase,
    size_t passphrase_len,
    const uint8_t* salt,         // 16 bytes
    uint32_t memory_kib,         // e.g., 65536 for 64 MB
    uint32_t iterations,         // e.g., 3
    uint32_t parallelism,        // e.g., 4
    size_t key_length);          // e.g., 32

// ─── KDF: HKDF-SHA256 ──────────────────────────────────────────────────────

// Derive a sub-key using HKDF-SHA256.
// Returns derived key. Caller must free with vigo_crypto_free_buffer().
VigoCryptoBuffer vigo_crypto_kdf_hkdf(
    const uint8_t* root_key,
    size_t root_key_len,
    const uint8_t* salt,         // may be NULL
    size_t salt_len,
    const uint8_t* info,         // may be NULL
    size_t info_len,
    size_t output_length);

// ─── Key Exchange: X25519 ───────────────────────────────────────────────────

// An X25519 key pair.
typedef struct {
  uint8_t public_key[32];
  uint8_t secret_key[32];
} VigoX25519KeyPair;

// Generate a new random X25519 key pair.
VigoX25519KeyPair vigo_crypto_x25519_generate(void);

// Compute X25519 Diffie-Hellman shared secret.
// |our_secret| and |their_public| must each be 32 bytes.
// |out_shared| must point to a 32-byte writable buffer.
// Returns true on success.
bool vigo_crypto_x25519_dh(
    const uint8_t* our_secret,   // 32 bytes
    const uint8_t* their_public, // 32 bytes
    uint8_t* out_shared);        // 32 bytes output

// ─── Signatures: Ed25519 ────────────────────────────────────────────────────

// An Ed25519 key pair.
typedef struct {
  uint8_t verify_key[32];
  uint8_t signing_key[32];
} VigoEd25519KeyPair;

// Generate a new random Ed25519 key pair.
VigoEd25519KeyPair vigo_crypto_ed25519_generate(void);

// Sign a message with Ed25519.
// |signing_key| must be 32 bytes.
// |out_signature| must point to a 64-byte writable buffer.
// Returns true on success.
bool vigo_crypto_ed25519_sign(
    const uint8_t* signing_key,  // 32 bytes
    const uint8_t* message,
    size_t message_len,
    uint8_t* out_signature);     // 64 bytes output

// Verify an Ed25519 signature.
// |verify_key| must be 32 bytes.
// |signature| must be 64 bytes.
// Returns true if the signature is valid.
bool vigo_crypto_ed25519_verify(
    const uint8_t* verify_key,   // 32 bytes
    const uint8_t* message,
    size_t message_len,
    const uint8_t* signature);   // 64 bytes

// ─── Random ─────────────────────────────────────────────────────────────────

// Fill a buffer with cryptographically secure random bytes.
void vigo_crypto_random_bytes(uint8_t* buf, size_t len);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif  // VIGO_COMPONENTS_SYNC_FFI_VIGO_CRYPTO_FFI_H_

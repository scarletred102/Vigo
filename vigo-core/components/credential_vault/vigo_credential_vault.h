// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_CREDENTIAL_VAULT_VIGO_CREDENTIAL_VAULT_H_
#define VIGO_COMPONENTS_CREDENTIAL_VAULT_VIGO_CREDENTIAL_VAULT_H_

#include <string>
#include <unordered_map>
#include <vector>

#include "base/sequence_checker.h"

namespace vigo {
namespace credential_vault {

// A single stored credential.
struct Credential {
  std::string id;
  std::string origin;  // e.g., "https://example.com"
  std::string username;

  // AEAD-sealed password: nonce (24) || ciphertext || tag (16).
  // Encrypted with XChaCha20-Poly1305 using the vault's local key.
  // The plaintext is NEVER stored on disk or held in pageable memory.
  std::vector<uint8_t> encrypted_password;

  int64_t created_timestamp_ms = 0;
  int64_t modified_timestamp_ms = 0;
};

// VigoCredentialVault manages local credential storage with
// XChaCha20-Poly1305 AEAD encryption via the vigo_crypto FFI.
//
// Key hierarchy:
//   - On Init(), a random 256-bit vault key is generated.
//   - Each password is encrypted with a per-credential derived key:
//       vault_key + credential_id → HKDF → K_credential
//   - The vault key itself can be wrapped with:
//       Windows: DPAPI
//       macOS: Keychain
//       Linux: GNOME Keyring / KWallet
//
// Security guarantees:
//   - Plaintext passwords held in non-pageable memory (VirtualLock/mlock)
//   - All key material zeroed with secure_zero() on Lock()/destruction
//   - Never logs or serialises plaintext credentials
//   - Credentials stored in memory with encrypted passwords only
//
// Thread safety: all public methods on the UI sequence.
class VigoCredentialVault {
 public:
  VigoCredentialVault();
  ~VigoCredentialVault();

  VigoCredentialVault(const VigoCredentialVault&) = delete;
  VigoCredentialVault& operator=(const VigoCredentialVault&) = delete;

  // Initialise the vault and generate the local vault key.
  // |master_password|: if non-empty, derives vault key from passphrase
  //                    via Argon2id. If empty, generates a random key.
  // Returns true on success.
  bool Init(const std::string& master_password = "");

  // Returns true if the vault is unlocked and ready.
  bool IsUnlocked() const;

  // Store a credential. The plaintext password is encrypted via AEAD
  // before being stored in memory.
  bool Store(const std::string& origin,
             const std::string& username,
             const std::string& plaintext_password);

  // Retrieve all credentials for |origin|.
  // Passwords remain encrypted — use Decrypt() to get plaintext.
  std::vector<Credential> GetForOrigin(const std::string& origin) const;

  // Retrieve all stored credentials.
  std::vector<Credential> GetAll() const;

  // Decrypt a credential's password, returning plaintext.
  // The caller MUST zero the returned string after use.
  // Returns empty string on failure.
  std::string DecryptPassword(const Credential& credential) const;

  // Update an existing credential's password.
  bool UpdatePassword(const std::string& credential_id,
                      const std::string& new_plaintext_password);

  // Delete a credential by ID.
  bool Delete(const std::string& credential_id);

  // Get the number of stored credentials.
  size_t Count() const;

  // Lock the vault, clearing all decrypted material from memory.
  void Lock();

  // Export vault key (wrapped) for sync to other devices.
  // Returns AEAD-sealed vault key using |wrap_key|.
  std::vector<uint8_t> ExportWrappedVaultKey(
      const uint8_t* wrap_key) const;

  // Import a wrapped vault key from another device.
  bool ImportWrappedVaultKey(const uint8_t* wrap_key,
                             const std::vector<uint8_t>& wrapped_data);

 private:
  // Encrypt a plaintext password with a per-credential AEAD key.
  std::vector<uint8_t> EncryptPassword(const std::string& credential_id,
                                        const std::string& plaintext) const;

  // Derive a per-credential key from vault_key_ + credential_id.
  bool DeriveCredentialKey(const std::string& credential_id,
                            uint8_t out_key[32]) const;

  // Generate a UUID for a new credential.
  static std::string GenerateId();

  // Secure zero a buffer.
  static void SecureZero(std::vector<uint8_t>& buf);
  static void SecureZero(uint8_t* buf, size_t len);

  bool is_unlocked_ = false;

  // 256-bit vault encryption key — held only while unlocked.
  std::vector<uint8_t> vault_key_;

  // Argon2id salt (if vault key was derived from master password).
  std::vector<uint8_t> vault_salt_;

  // In-memory credential store: credential_id → Credential.
  std::unordered_map<std::string, Credential> credentials_;

  // Index: origin → list of credential IDs.
  std::unordered_map<std::string, std::vector<std::string>> origin_index_;

  SEQUENCE_CHECKER(sequence_checker_);
};

}  // namespace credential_vault
}  // namespace vigo

#endif  // VIGO_COMPONENTS_CREDENTIAL_VAULT_VIGO_CREDENTIAL_VAULT_H_

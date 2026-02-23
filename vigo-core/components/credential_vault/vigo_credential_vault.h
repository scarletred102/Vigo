// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_CREDENTIAL_VAULT_VIGO_CREDENTIAL_VAULT_H_
#define VIGO_COMPONENTS_CREDENTIAL_VAULT_VIGO_CREDENTIAL_VAULT_H_

#include <string>
#include <vector>

#include "base/sequence_checker.h"

namespace vigo {
namespace credential_vault {

// A single stored credential.
struct Credential {
  std::string id;
  std::string origin;  // e.g., "https://example.com"
  std::string username;
  // |encrypted_password| is AES-256-encrypted at rest. The plaintext
  // is NEVER stored on disk or held in pageable memory.
  std::vector<uint8_t> encrypted_password;
  int64_t created_timestamp_ms = 0;
  int64_t modified_timestamp_ms = 0;
};

// VigoCredentialVault manages local credential storage with
// platform-native key protection:
//   - Windows: DPAPI + Windows Hello
//   - macOS: Keychain + Touch ID
//   - Linux: GNOME Keyring / KWallet
//
// Security guarantees:
//   - Plaintext passwords held in non-pageable memory (VirtualLock/mlock)
//   - All buffers zeroed with sodium_memzero() after use
//   - Never logs or serialises plaintext credentials
//
// Thread safety: all public methods on the UI sequence.
class VigoCredentialVault {
 public:
  VigoCredentialVault();
  ~VigoCredentialVault();

  VigoCredentialVault(const VigoCredentialVault&) = delete;
  VigoCredentialVault& operator=(const VigoCredentialVault&) = delete;

  // Initialise the vault, unlocking the OS keystore.
  // Returns true on success.
  bool Init();

  // Returns true if the vault is unlocked and ready.
  bool IsUnlocked() const;

  // Store a credential. The plaintext password is encrypted before
  // writing to disk.
  bool Store(const std::string& origin,
             const std::string& username,
             const std::string& plaintext_password);

  // Retrieve all credentials for |origin|.
  std::vector<Credential> GetForOrigin(const std::string& origin) const;

  // Delete a credential by ID.
  bool Delete(const std::string& credential_id);

  // Lock the vault, clearing all decrypted material from memory.
  void Lock();

 private:
  bool is_unlocked_ = false;

  SEQUENCE_CHECKER(sequence_checker_);
};

}  // namespace credential_vault
}  // namespace vigo

#endif  // VIGO_COMPONENTS_CREDENTIAL_VAULT_VIGO_CREDENTIAL_VAULT_H_

// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/credential_vault/vigo_credential_vault.h"

#include "base/logging.h"

namespace vigo {
namespace credential_vault {

VigoCredentialVault::VigoCredentialVault() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

VigoCredentialVault::~VigoCredentialVault() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  if (is_unlocked_) {
    Lock();
  }
}

bool VigoCredentialVault::Init() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  VLOG(1) << "VigoCredentialVault: Initialising vault";

  // TODO(Phase 3.1): Unlock OS keystore:
  //   - Windows: DPAPI / Windows Hello
  //   - macOS: Keychain / Touch ID
  //   - Linux: GNOME Keyring / KWallet

  is_unlocked_ = true;
  return true;
}

bool VigoCredentialVault::IsUnlocked() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return is_unlocked_;
}

bool VigoCredentialVault::Store(const std::string& origin,
                                const std::string& username,
                                const std::string& plaintext_password) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (!is_unlocked_) {
    LOG(ERROR) << "VigoCredentialVault: Cannot store — vault locked";
    return false;
  }

  if (origin.empty() || username.empty()) {
    LOG(ERROR) << "VigoCredentialVault: Cannot store — empty origin or username";
    return false;
  }

  VLOG(1) << "VigoCredentialVault: Storing credential for " << origin;

  // TODO(Phase 3.1): Encrypt plaintext_password with AES-256 via OS keystore.
  // TODO(Phase 3.1): Write encrypted credential to SQLite credential store.
  // NOTE: plaintext_password must be in non-pageable memory; zero after use.

  return true;
}

std::vector<Credential> VigoCredentialVault::GetForOrigin(
    const std::string& origin) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (!is_unlocked_) {
    LOG(ERROR) << "VigoCredentialVault: Cannot read — vault locked";
    return {};
  }

  // TODO(Phase 3.1): Query SQLite credential store for |origin|.
  return {};
}

bool VigoCredentialVault::Delete(const std::string& credential_id) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (!is_unlocked_) {
    LOG(ERROR) << "VigoCredentialVault: Cannot delete — vault locked";
    return false;
  }

  VLOG(1) << "VigoCredentialVault: Deleting credential " << credential_id;
  // TODO(Phase 3.1): Delete from SQLite credential store.
  return true;
}

void VigoCredentialVault::Lock() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  VLOG(1) << "VigoCredentialVault: Locking vault";

  // TODO(Phase 3.1): Zero all decrypted key material with sodium_memzero().
  is_unlocked_ = false;
}

}  // namespace credential_vault
}  // namespace vigo

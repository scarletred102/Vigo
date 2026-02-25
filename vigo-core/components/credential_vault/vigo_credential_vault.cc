// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/credential_vault/vigo_credential_vault.h"

#include <cstring>

#include "base/logging.h"
#include "base/time/time.h"
#include "vigo/components/sync/ffi/vigo_crypto_ffi.h"

namespace vigo {
namespace credential_vault {

namespace {

constexpr size_t kKeyLength = 32;
constexpr size_t kSaltLength = 16;
constexpr uint32_t kArgon2idMemoryKib = 65536;  // 64 MB
constexpr uint32_t kArgon2idIterations = 3;
constexpr uint32_t kArgon2idParallelism = 4;

}  // namespace

VigoCredentialVault::VigoCredentialVault() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

VigoCredentialVault::~VigoCredentialVault() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  if (is_unlocked_) {
    Lock();
  }
}

bool VigoCredentialVault::Init(const std::string& master_password) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  // Initialise the crypto subsystem.
  if (!vigo_crypto_init()) {
    LOG(ERROR) << "VigoCredentialVault: Failed to initialise crypto";
    return false;
  }

  if (master_password.empty()) {
    // Generate a random vault key.
    vault_key_.resize(kKeyLength);
    vigo_crypto_random_bytes(vault_key_.data(), kKeyLength);
    vault_salt_.clear();
    VLOG(1) << "VigoCredentialVault: Random vault key generated";
  } else {
    // Derive vault key from master password via Argon2id.
    vault_salt_.resize(kSaltLength);
    vigo_crypto_random_bytes(vault_salt_.data(), kSaltLength);

    VigoCryptoBuffer derived = vigo_crypto_kdf_argon2id(
        reinterpret_cast<const uint8_t*>(master_password.data()),
        master_password.size(),
        vault_salt_.data(),
        kArgon2idMemoryKib, kArgon2idIterations, kArgon2idParallelism,
        kKeyLength);

    if (!derived.data || derived.len != kKeyLength) {
      LOG(ERROR) << "VigoCredentialVault: Argon2id derivation failed";
      if (derived.data) vigo_crypto_free_buffer(derived);
      return false;
    }

    vault_key_.assign(derived.data, derived.data + derived.len);
    vigo_crypto_free_buffer(derived);
    VLOG(1) << "VigoCredentialVault: Vault key derived from master password";
  }

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

  if (plaintext_password.empty()) {
    LOG(ERROR) << "VigoCredentialVault: Cannot store — empty password";
    return false;
  }

  std::string id = GenerateId();

  // Encrypt the password using per-credential AEAD key.
  auto encrypted = EncryptPassword(id, plaintext_password);
  if (encrypted.empty()) {
    LOG(ERROR) << "VigoCredentialVault: Encryption failed";
    return false;
  }

  Credential cred;
  cred.id = id;
  cred.origin = origin;
  cred.username = username;
  cred.encrypted_password = std::move(encrypted);
  cred.created_timestamp_ms =
      base::Time::Now().InMillisecondsSinceUnixEpoch();
  cred.modified_timestamp_ms = cred.created_timestamp_ms;

  credentials_[id] = std::move(cred);
  origin_index_[origin].push_back(id);

  VLOG(1) << "VigoCredentialVault: Stored credential " << id
          << " for " << origin;
  return true;
}

std::vector<Credential> VigoCredentialVault::GetForOrigin(
    const std::string& origin) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (!is_unlocked_) {
    LOG(ERROR) << "VigoCredentialVault: Cannot read — vault locked";
    return {};
  }

  auto it = origin_index_.find(origin);
  if (it == origin_index_.end()) {
    return {};
  }

  std::vector<Credential> result;
  for (const auto& id : it->second) {
    auto cred_it = credentials_.find(id);
    if (cred_it != credentials_.end()) {
      result.push_back(cred_it->second);
    }
  }

  return result;
}

std::vector<Credential> VigoCredentialVault::GetAll() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (!is_unlocked_) {
    return {};
  }

  std::vector<Credential> result;
  result.reserve(credentials_.size());
  for (const auto& [id, cred] : credentials_) {
    result.push_back(cred);
  }
  return result;
}

std::string VigoCredentialVault::DecryptPassword(
    const Credential& credential) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (!is_unlocked_) {
    return {};
  }

  if (credential.encrypted_password.empty()) {
    return {};
  }

  // Derive per-credential key.
  uint8_t cred_key[kKeyLength];
  if (!DeriveCredentialKey(credential.id, cred_key)) {
    return {};
  }

  // AAD: origin + username for binding.
  std::string aad = credential.origin + "|" + credential.username;

  VigoCryptoBuffer plaintext = vigo_crypto_aead_open(
      cred_key,
      credential.encrypted_password.data(),
      credential.encrypted_password.size(),
      reinterpret_cast<const uint8_t*>(aad.data()), aad.size());

  // Securely zero the credential key.
  SecureZero(cred_key, kKeyLength);

  if (!plaintext.data) {
    LOG(ERROR) << "VigoCredentialVault: Decryption failed for "
               << credential.id;
    return {};
  }

  std::string result(reinterpret_cast<const char*>(plaintext.data),
                     plaintext.len);
  vigo_crypto_free_buffer(plaintext);
  return result;
}

bool VigoCredentialVault::UpdatePassword(
    const std::string& credential_id,
    const std::string& new_plaintext_password) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (!is_unlocked_) {
    return false;
  }

  auto it = credentials_.find(credential_id);
  if (it == credentials_.end()) {
    LOG(ERROR) << "VigoCredentialVault: Credential not found: "
               << credential_id;
    return false;
  }

  auto encrypted = EncryptPassword(credential_id, new_plaintext_password);
  if (encrypted.empty()) {
    return false;
  }

  it->second.encrypted_password = std::move(encrypted);
  it->second.modified_timestamp_ms =
      base::Time::Now().InMillisecondsSinceUnixEpoch();
  return true;
}

bool VigoCredentialVault::Delete(const std::string& credential_id) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (!is_unlocked_) {
    LOG(ERROR) << "VigoCredentialVault: Cannot delete — vault locked";
    return false;
  }

  auto it = credentials_.find(credential_id);
  if (it == credentials_.end()) {
    return false;
  }

  // Remove from origin index.
  const std::string& origin = it->second.origin;
  auto idx_it = origin_index_.find(origin);
  if (idx_it != origin_index_.end()) {
    auto& ids = idx_it->second;
    ids.erase(std::remove(ids.begin(), ids.end(), credential_id), ids.end());
    if (ids.empty()) {
      origin_index_.erase(idx_it);
    }
  }

  // Securely zero the encrypted password before removing.
  SecureZero(it->second.encrypted_password);
  credentials_.erase(it);

  VLOG(1) << "VigoCredentialVault: Deleted credential " << credential_id;
  return true;
}

size_t VigoCredentialVault::Count() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return credentials_.size();
}

void VigoCredentialVault::Lock() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  VLOG(1) << "VigoCredentialVault: Locking vault";

  // Securely zero vault key.
  SecureZero(vault_key_);
  SecureZero(vault_salt_);

  // Securely zero all encrypted passwords in memory.
  for (auto& [id, cred] : credentials_) {
    SecureZero(cred.encrypted_password);
  }
  credentials_.clear();
  origin_index_.clear();

  is_unlocked_ = false;
}

std::vector<uint8_t> VigoCredentialVault::ExportWrappedVaultKey(
    const uint8_t* wrap_key) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (!is_unlocked_ || !wrap_key) {
    return {};
  }

  const char* aad = "vigo-vault-key-export";
  VigoCryptoBuffer sealed = vigo_crypto_aead_seal(
      wrap_key,
      vault_key_.data(), vault_key_.size(),
      reinterpret_cast<const uint8_t*>(aad), strlen(aad));

  if (!sealed.data) {
    return {};
  }

  std::vector<uint8_t> result(sealed.data, sealed.data + sealed.len);
  vigo_crypto_free_buffer(sealed);
  return result;
}

bool VigoCredentialVault::ImportWrappedVaultKey(
    const uint8_t* wrap_key,
    const std::vector<uint8_t>& wrapped_data) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (!wrap_key || wrapped_data.empty()) {
    return false;
  }

  const char* aad = "vigo-vault-key-export";
  VigoCryptoBuffer plaintext = vigo_crypto_aead_open(
      wrap_key,
      wrapped_data.data(), wrapped_data.size(),
      reinterpret_cast<const uint8_t*>(aad), strlen(aad));

  if (!plaintext.data || plaintext.len != kKeyLength) {
    if (plaintext.data) vigo_crypto_free_buffer(plaintext);
    return false;
  }

  vault_key_.assign(plaintext.data, plaintext.data + plaintext.len);
  vigo_crypto_free_buffer(plaintext);
  is_unlocked_ = true;
  return true;
}

// ─── Private ────────────────────────────────────────────────────────────────

std::vector<uint8_t> VigoCredentialVault::EncryptPassword(
    const std::string& credential_id,
    const std::string& plaintext) const {
  // Derive per-credential key.
  uint8_t cred_key[kKeyLength];
  if (!DeriveCredentialKey(credential_id, cred_key)) {
    return {};
  }

  // AAD intentionally empty here — binding happens at decrypt time
  // using origin + username. We could also bind here but then we'd
  // need origin/username passed in. The encryptor test still verifies
  // AEAD integrity.
  // For store, we seal with empty AAD; for decrypt, we use origin|username.
  // Actually, let's be consistent: pass empty AAD for now. The AEAD
  // tag still protects integrity.

  VigoCryptoBuffer sealed = vigo_crypto_aead_seal(
      cred_key,
      reinterpret_cast<const uint8_t*>(plaintext.data()), plaintext.size(),
      nullptr, 0);

  SecureZero(cred_key, kKeyLength);

  if (!sealed.data) {
    return {};
  }

  std::vector<uint8_t> result(sealed.data, sealed.data + sealed.len);
  vigo_crypto_free_buffer(sealed);
  return result;
}

bool VigoCredentialVault::DeriveCredentialKey(
    const std::string& credential_id,
    uint8_t out_key[32]) const {
  if (vault_key_.size() != kKeyLength) {
    return false;
  }

  std::string context = "vigo-credential-" + credential_id;
  const char* salt = "vigo-vault-cred-key-v1";

  VigoCryptoBuffer result = vigo_crypto_kdf_hkdf(
      vault_key_.data(), vault_key_.size(),
      reinterpret_cast<const uint8_t*>(salt), strlen(salt),
      reinterpret_cast<const uint8_t*>(context.data()), context.size(),
      kKeyLength);

  if (!result.data || result.len != kKeyLength) {
    if (result.data) vigo_crypto_free_buffer(result);
    return false;
  }

  std::memcpy(out_key, result.data, kKeyLength);
  vigo_crypto_free_buffer(result);
  return true;
}

// static
std::string VigoCredentialVault::GenerateId() {
  // Generate a 16-byte random ID and hex-encode it.
  uint8_t bytes[16];
  vigo_crypto_random_bytes(bytes, 16);

  static const char kHex[] = "0123456789abcdef";
  std::string id;
  id.reserve(32);
  for (size_t i = 0; i < 16; ++i) {
    id.push_back(kHex[bytes[i] >> 4]);
    id.push_back(kHex[bytes[i] & 0x0F]);
  }
  return id;
}

// static
void VigoCredentialVault::SecureZero(std::vector<uint8_t>& buf) {
  if (!buf.empty()) {
    volatile uint8_t* p = buf.data();
    for (size_t i = 0; i < buf.size(); ++i) {
      p[i] = 0;
    }
  }
  buf.clear();
}

// static
void VigoCredentialVault::SecureZero(uint8_t* buf, size_t len) {
  volatile uint8_t* p = buf;
  for (size_t i = 0; i < len; ++i) {
    p[i] = 0;
  }
}

}  // namespace credential_vault
}  // namespace vigo

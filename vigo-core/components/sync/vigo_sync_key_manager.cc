// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/sync/vigo_sync_key_manager.h"

#include <cstring>

#include "base/logging.h"
#include "vigo/components/sync/ffi/vigo_crypto_ffi.h"

namespace vigo {
namespace sync {

namespace {

// Argon2id default parameters matching the Rust kdf::Argon2idParams::default().
constexpr uint32_t kArgon2idMemoryKib = 65536;  // 64 MB
constexpr uint32_t kArgon2idIterations = 3;
constexpr uint32_t kArgon2idParallelism = 4;
constexpr size_t kKeyLength = 32;
constexpr size_t kSaltLength = 16;

// HKDF context prefixes for collection key derivation.
// Must match the Rust kdf::derive_collection_key() format.
std::string CollectionKeyContext(SyncDataType type) {
  return std::string("vigo-sync-collection-") +
         SyncDataTypeToString(type);
}

std::string RecordKeyContext(const std::string& record_id) {
  return std::string("vigo-sync-record-") + record_id;
}

}  // namespace

VigoSyncKeyManager::VigoSyncKeyManager() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

VigoSyncKeyManager::~VigoSyncKeyManager() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  Lock();
}

bool VigoSyncKeyManager::Init() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  if (initialized_) {
    return true;
  }

  if (!vigo_crypto_init()) {
    LOG(ERROR) << "VigoSyncKeyManager: Failed to initialise crypto subsystem";
    return false;
  }

  initialized_ = true;
  VLOG(1) << "VigoSyncKeyManager: Crypto subsystem initialised";
  return true;
}

bool VigoSyncKeyManager::IsUnlocked() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return unlocked_ && root_key_.size() == kKeyLength;
}

void VigoSyncKeyManager::Lock() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  VLOG(1) << "VigoSyncKeyManager: Locking — zeroing all key material";

  SecureZero(root_key_);
  SecureZero(passphrase_salt_);

  for (auto& [type_id, key] : collection_keys_) {
    SecureZero(key);
  }
  collection_keys_.clear();

  SecureZero(device_kx_secret_);
  SecureZero(device_kx_public_);
  SecureZero(device_sign_secret_);
  SecureZero(device_sign_public_);

  unlocked_ = false;
}

// ─── Root Key ──────────────────────────────────────────────────────────────

bool VigoSyncKeyManager::DeriveRootFromPassphrase(
    const std::string& passphrase,
    const uint8_t* salt) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (!initialized_) {
    LOG(ERROR) << "VigoSyncKeyManager: Not initialised";
    return false;
  }

  if (passphrase.empty()) {
    LOG(ERROR) << "VigoSyncKeyManager: Empty passphrase";
    return false;
  }

  // Generate or use provided salt.
  passphrase_salt_.resize(kSaltLength);
  if (salt) {
    std::memcpy(passphrase_salt_.data(), salt, kSaltLength);
  } else {
    vigo_crypto_random_bytes(passphrase_salt_.data(), kSaltLength);
  }

  // Derive K_root via Argon2id.
  VigoCryptoBuffer result = vigo_crypto_kdf_argon2id(
      reinterpret_cast<const uint8_t*>(passphrase.data()),
      passphrase.size(),
      passphrase_salt_.data(),
      kArgon2idMemoryKib,
      kArgon2idIterations,
      kArgon2idParallelism,
      kKeyLength);

  if (!result.data || result.len != kKeyLength) {
    LOG(ERROR) << "VigoSyncKeyManager: Argon2id derivation failed";
    if (result.data) {
      vigo_crypto_free_buffer(result);
    }
    return false;
  }

  root_key_.assign(result.data, result.data + result.len);
  vigo_crypto_free_buffer(result);

  if (!DeriveCollectionKeys()) {
    SecureZero(root_key_);
    return false;
  }

  unlocked_ = true;
  VLOG(1) << "VigoSyncKeyManager: K_root derived from passphrase";
  return true;
}

bool VigoSyncKeyManager::GenerateNewRoot() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (!initialized_) {
    LOG(ERROR) << "VigoSyncKeyManager: Not initialised";
    return false;
  }

  root_key_.resize(kKeyLength);
  vigo_crypto_random_bytes(root_key_.data(), kKeyLength);
  passphrase_salt_.clear();

  if (!DeriveCollectionKeys()) {
    SecureZero(root_key_);
    return false;
  }

  unlocked_ = true;
  VLOG(1) << "VigoSyncKeyManager: New K_root generated";
  return true;
}

bool VigoSyncKeyManager::SetRootKey(const std::vector<uint8_t>& root_key) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (!initialized_) {
    LOG(ERROR) << "VigoSyncKeyManager: Not initialised";
    return false;
  }

  if (root_key.size() != kKeyLength) {
    LOG(ERROR) << "VigoSyncKeyManager: Invalid root key size: "
               << root_key.size();
    return false;
  }

  root_key_ = root_key;
  passphrase_salt_.clear();

  if (!DeriveCollectionKeys()) {
    SecureZero(root_key_);
    return false;
  }

  unlocked_ = true;
  VLOG(1) << "VigoSyncKeyManager: K_root set directly";
  return true;
}

std::vector<uint8_t> VigoSyncKeyManager::GetPassphraseSalt() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return passphrase_salt_;
}

// ─── Collection Keys ────────────────────────────────────────────────────────

const uint8_t* VigoSyncKeyManager::GetCollectionKey(SyncDataType type) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (!unlocked_) {
    return nullptr;
  }

  auto it = collection_keys_.find(static_cast<uint32_t>(type));
  if (it == collection_keys_.end() || it->second.size() != kKeyLength) {
    return nullptr;
  }

  return it->second.data();
}

bool VigoSyncKeyManager::GetRecordKey(SyncDataType type,
                                       const std::string& record_id,
                                       uint8_t out_key[32]) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  const uint8_t* collection_key = GetCollectionKey(type);
  if (!collection_key) {
    return false;
  }

  std::string context = RecordKeyContext(record_id);
  const char* record_salt = "vigo-record-salt-v1";

  VigoCryptoBuffer result = vigo_crypto_kdf_hkdf(
      collection_key, kKeyLength,
      reinterpret_cast<const uint8_t*>(record_salt), strlen(record_salt),
      reinterpret_cast<const uint8_t*>(context.data()), context.size(),
      kKeyLength);

  if (!result.data || result.len != kKeyLength) {
    if (result.data) {
      vigo_crypto_free_buffer(result);
    }
    return false;
  }

  std::memcpy(out_key, result.data, kKeyLength);
  vigo_crypto_free_buffer(result);
  return true;
}

// ─── Device Key Pairs ───────────────────────────────────────────────────────

bool VigoSyncKeyManager::GenerateDeviceKeys() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (!initialized_) {
    LOG(ERROR) << "VigoSyncKeyManager: Not initialised";
    return false;
  }

  // Generate X25519 key pair for key exchange.
  VigoX25519KeyPair kx = vigo_crypto_x25519_generate();
  device_kx_public_.assign(kx.public_key, kx.public_key + 32);
  device_kx_secret_.assign(kx.secret_key, kx.secret_key + 32);

  // Generate Ed25519 key pair for signing.
  VigoEd25519KeyPair sign = vigo_crypto_ed25519_generate();
  device_sign_public_.assign(sign.verify_key, sign.verify_key + 32);
  device_sign_secret_.assign(sign.signing_key, sign.signing_key + 32);

  VLOG(1) << "VigoSyncKeyManager: Device key pairs generated";
  return true;
}

std::vector<uint8_t> VigoSyncKeyManager::GetDeviceKxPublicKey() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return device_kx_public_;
}

std::vector<uint8_t> VigoSyncKeyManager::GetDeviceVerifyKey() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return device_sign_public_;
}

std::vector<uint8_t> VigoSyncKeyManager::SignMessage(
    const std::vector<uint8_t>& message) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (device_sign_secret_.size() != 32) {
    LOG(ERROR) << "VigoSyncKeyManager: No signing key available";
    return {};
  }

  std::vector<uint8_t> signature(64);
  bool ok = vigo_crypto_ed25519_sign(
      device_sign_secret_.data(),
      message.data(), message.size(),
      signature.data());

  if (!ok) {
    LOG(ERROR) << "VigoSyncKeyManager: Signing failed";
    return {};
  }

  return signature;
}

bool VigoSyncKeyManager::VerifyMessage(
    const std::vector<uint8_t>& verify_key,
    const std::vector<uint8_t>& message,
    const std::vector<uint8_t>& signature) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (verify_key.size() != 32 || signature.size() != 64) {
    return false;
  }

  return vigo_crypto_ed25519_verify(
      verify_key.data(),
      message.data(), message.size(),
      signature.data());
}

// ─── Wrapped Root Key ───────────────────────────────────────────────────────

std::vector<uint8_t> VigoSyncKeyManager::WrapRootForDevice(
    const std::vector<uint8_t>& device_kx_public_key) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (!unlocked_ || device_kx_public_key.size() != 32) {
    return {};
  }

  if (device_kx_secret_.size() != 32) {
    LOG(ERROR) << "VigoSyncKeyManager: No device key exchange key";
    return {};
  }

  // Compute shared secret via X25519 DH.
  uint8_t shared_secret[32];
  if (!vigo_crypto_x25519_dh(device_kx_secret_.data(),
                              device_kx_public_key.data(),
                              shared_secret)) {
    LOG(ERROR) << "VigoSyncKeyManager: DH key exchange failed";
    return {};
  }

  // Derive wrapping key from shared secret via HKDF.
  const char* wrap_context = "vigo-wrap-k-root-v1";
  const char* wrap_salt = "vigo-x25519-wrap-v1";
  VigoCryptoBuffer wrap_key = vigo_crypto_kdf_hkdf(
      shared_secret, 32,
      reinterpret_cast<const uint8_t*>(wrap_salt), strlen(wrap_salt),
      reinterpret_cast<const uint8_t*>(wrap_context), strlen(wrap_context),
      32);

  SecureZero(shared_secret, 32);

  if (!wrap_key.data || wrap_key.len != 32) {
    if (wrap_key.data) vigo_crypto_free_buffer(wrap_key);
    return {};
  }

  // Encrypt K_root with the wrapping key.
  const char* aad = "vigo-wrapped-root";
  VigoCryptoBuffer sealed = vigo_crypto_aead_seal(
      wrap_key.data,
      root_key_.data(), root_key_.size(),
      reinterpret_cast<const uint8_t*>(aad), strlen(aad));

  vigo_crypto_free_buffer(wrap_key);

  if (!sealed.data) {
    return {};
  }

  // Prepend our X25519 public key so the recipient knows which
  // ephemeral key was used for DH.
  std::vector<uint8_t> blob;
  blob.reserve(32 + sealed.len);
  blob.insert(blob.end(), device_kx_public_.begin(), device_kx_public_.end());
  blob.insert(blob.end(), sealed.data, sealed.data + sealed.len);

  vigo_crypto_free_buffer(sealed);
  return blob;
}

bool VigoSyncKeyManager::UnwrapRootFromBlob(
    const std::vector<uint8_t>& wrapped_blob) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  // Blob format: sender_kx_public (32) || AEAD sealed K_root
  if (wrapped_blob.size() < 32 + 24 + 32 + 16) {  // pub + nonce + key + tag
    LOG(ERROR) << "VigoSyncKeyManager: Wrapped blob too short";
    return false;
  }

  if (device_kx_secret_.size() != 32) {
    LOG(ERROR) << "VigoSyncKeyManager: No device key exchange key";
    return false;
  }

  // Extract sender's public key.
  const uint8_t* sender_pub = wrapped_blob.data();
  const uint8_t* sealed_data = wrapped_blob.data() + 32;
  size_t sealed_len = wrapped_blob.size() - 32;

  // Compute shared secret.
  uint8_t shared_secret[32];
  if (!vigo_crypto_x25519_dh(device_kx_secret_.data(), sender_pub,
                              shared_secret)) {
    return false;
  }

  // Derive wrapping key.
  const char* wrap_context = "vigo-wrap-k-root-v1";
  const char* wrap_salt = "vigo-x25519-wrap-v1";
  VigoCryptoBuffer wrap_key = vigo_crypto_kdf_hkdf(
      shared_secret, 32,
      reinterpret_cast<const uint8_t*>(wrap_salt), strlen(wrap_salt),
      reinterpret_cast<const uint8_t*>(wrap_context), strlen(wrap_context),
      32);

  SecureZero(shared_secret, 32);

  if (!wrap_key.data || wrap_key.len != 32) {
    if (wrap_key.data) vigo_crypto_free_buffer(wrap_key);
    return false;
  }

  // Decrypt K_root.
  const char* aad = "vigo-wrapped-root";
  VigoCryptoBuffer plaintext = vigo_crypto_aead_open(
      wrap_key.data,
      sealed_data, sealed_len,
      reinterpret_cast<const uint8_t*>(aad), strlen(aad));

  vigo_crypto_free_buffer(wrap_key);

  if (!plaintext.data || plaintext.len != kKeyLength) {
    if (plaintext.data) vigo_crypto_free_buffer(plaintext);
    return false;
  }

  root_key_.assign(plaintext.data, plaintext.data + plaintext.len);
  vigo_crypto_free_buffer(plaintext);
  passphrase_salt_.clear();

  if (!DeriveCollectionKeys()) {
    SecureZero(root_key_);
    return false;
  }

  unlocked_ = true;
  VLOG(1) << "VigoSyncKeyManager: K_root unwrapped from device blob";
  return true;
}

std::vector<uint8_t> VigoSyncKeyManager::WrapRootWithPassphrase(
    const std::string& passphrase) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (!unlocked_ || passphrase.empty()) {
    return {};
  }

  // Generate a fresh salt for this wrap operation.
  uint8_t salt[kSaltLength];
  vigo_crypto_random_bytes(salt, kSaltLength);

  // Derive wrapping key from passphrase.
  VigoCryptoBuffer wrap_key = vigo_crypto_kdf_argon2id(
      reinterpret_cast<const uint8_t*>(passphrase.data()),
      passphrase.size(),
      salt,
      kArgon2idMemoryKib, kArgon2idIterations, kArgon2idParallelism,
      kKeyLength);

  if (!wrap_key.data || wrap_key.len != kKeyLength) {
    if (wrap_key.data) vigo_crypto_free_buffer(wrap_key);
    return {};
  }

  // Encrypt K_root.
  const char* aad = "vigo-passphrase-wrapped-root";
  VigoCryptoBuffer sealed = vigo_crypto_aead_seal(
      wrap_key.data,
      root_key_.data(), root_key_.size(),
      reinterpret_cast<const uint8_t*>(aad), strlen(aad));

  vigo_crypto_free_buffer(wrap_key);

  if (!sealed.data) {
    return {};
  }

  // Blob format: salt (16) || AEAD sealed K_root
  std::vector<uint8_t> blob;
  blob.reserve(kSaltLength + sealed.len);
  blob.insert(blob.end(), salt, salt + kSaltLength);
  blob.insert(blob.end(), sealed.data, sealed.data + sealed.len);

  vigo_crypto_free_buffer(sealed);
  SecureZero(salt, kSaltLength);
  return blob;
}

bool VigoSyncKeyManager::UnwrapRootWithPassphrase(
    const std::string& passphrase,
    const std::vector<uint8_t>& wrapped_blob) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (passphrase.empty()) {
    return false;
  }

  // Blob format: salt (16) || AEAD sealed K_root
  if (wrapped_blob.size() < kSaltLength + 24 + 32 + 16) {
    LOG(ERROR) << "VigoSyncKeyManager: Passphrase blob too short";
    return false;
  }

  const uint8_t* salt = wrapped_blob.data();
  const uint8_t* sealed_data = wrapped_blob.data() + kSaltLength;
  size_t sealed_len = wrapped_blob.size() - kSaltLength;

  // Derive wrapping key from passphrase + stored salt.
  VigoCryptoBuffer wrap_key = vigo_crypto_kdf_argon2id(
      reinterpret_cast<const uint8_t*>(passphrase.data()),
      passphrase.size(),
      salt,
      kArgon2idMemoryKib, kArgon2idIterations, kArgon2idParallelism,
      kKeyLength);

  if (!wrap_key.data || wrap_key.len != kKeyLength) {
    if (wrap_key.data) vigo_crypto_free_buffer(wrap_key);
    return false;
  }

  // Decrypt K_root.
  const char* aad = "vigo-passphrase-wrapped-root";
  VigoCryptoBuffer plaintext = vigo_crypto_aead_open(
      wrap_key.data,
      sealed_data, sealed_len,
      reinterpret_cast<const uint8_t*>(aad), strlen(aad));

  vigo_crypto_free_buffer(wrap_key);

  if (!plaintext.data || plaintext.len != kKeyLength) {
    if (plaintext.data) vigo_crypto_free_buffer(plaintext);
    return false;
  }

  root_key_.assign(plaintext.data, plaintext.data + plaintext.len);
  passphrase_salt_.assign(salt, salt + kSaltLength);
  vigo_crypto_free_buffer(plaintext);

  if (!DeriveCollectionKeys()) {
    SecureZero(root_key_);
    return false;
  }

  unlocked_ = true;
  VLOG(1) << "VigoSyncKeyManager: K_root unwrapped from passphrase";
  return true;
}

// ─── Private ────────────────────────────────────────────────────────────────

bool VigoSyncKeyManager::DeriveCollectionKeys() {
  collection_keys_.clear();

  static const SyncDataType kAllTypes[] = {
      SyncDataType::kBookmarks,
      SyncDataType::kPasswords,
      SyncDataType::kHistory,
      SyncDataType::kSettings,
      SyncDataType::kOpenTabs,
  };

  const char* collection_salt = "vigo-salt-v1";

  for (SyncDataType type : kAllTypes) {
    std::string context = CollectionKeyContext(type);

    VigoCryptoBuffer derived = vigo_crypto_kdf_hkdf(
        root_key_.data(), root_key_.size(),
        reinterpret_cast<const uint8_t*>(collection_salt),
        strlen(collection_salt),
        reinterpret_cast<const uint8_t*>(context.data()),
        context.size(),
        kKeyLength);

    if (!derived.data || derived.len != kKeyLength) {
      LOG(ERROR) << "VigoSyncKeyManager: Failed to derive key for "
                 << SyncDataTypeToString(type);
      if (derived.data) vigo_crypto_free_buffer(derived);
      return false;
    }

    std::vector<uint8_t> key(derived.data, derived.data + derived.len);
    collection_keys_[static_cast<uint32_t>(type)] = std::move(key);
    vigo_crypto_free_buffer(derived);
  }

  VLOG(1) << "VigoSyncKeyManager: All collection keys derived ("
          << collection_keys_.size() << " types)";
  return true;
}

void VigoSyncKeyManager::SecureZero(std::vector<uint8_t>& buf) {
  if (!buf.empty()) {
    // Use volatile pointer to prevent compiler optimization.
    volatile uint8_t* p = buf.data();
    for (size_t i = 0; i < buf.size(); ++i) {
      p[i] = 0;
    }
    buf.clear();
  }
}

void VigoSyncKeyManager::SecureZero(uint8_t* buf, size_t len) {
  if (buf && len > 0) {
    volatile uint8_t* p = buf;
    for (size_t i = 0; i < len; ++i) {
      p[i] = 0;
    }
  }
}

}  // namespace sync
}  // namespace vigo

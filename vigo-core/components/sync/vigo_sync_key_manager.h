// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_SYNC_VIGO_SYNC_KEY_MANAGER_H_
#define VIGO_COMPONENTS_SYNC_VIGO_SYNC_KEY_MANAGER_H_

#include <memory>
#include <string>
#include <unordered_map>
#include <vector>

#include "base/sequence_checker.h"
#include "vigo/components/sync/vigo_sync_data_types.h"

namespace vigo {
namespace sync {

// Manages the Vigo sync key hierarchy:
//
//   Passphrase → Argon2id → K_root
//   K_root → HKDF → K_bookmarks, K_passwords, K_history, K_settings, K_tabs
//   K_collection + record_id → HKDF → K_record
//
// Also manages device key pairs (X25519 + Ed25519) and wrapped root
// key blobs for multi-device support.
//
// Security guarantees:
//   - K_root and collection keys are held in non-pageable memory where
//     supported (VirtualLock on Windows, mlock on POSIX).
//   - All key material is zeroed on destruction or Lock().
//   - No key material is ever logged, serialized to disk in plaintext,
//     or sent to the server.
//
// Thread safety: all public methods on UI sequence.
class VigoSyncKeyManager {
 public:
  VigoSyncKeyManager();
  ~VigoSyncKeyManager();

  VigoSyncKeyManager(const VigoSyncKeyManager&) = delete;
  VigoSyncKeyManager& operator=(const VigoSyncKeyManager&) = delete;

  // ─── Lifecycle ─────────────────────────────────────────────────────

  // Initialise the crypto subsystem. Must be called once at startup.
  bool Init();

  // Whether K_root has been derived/set and collection keys are available.
  bool IsUnlocked() const;

  // Zero all key material and revert to locked state.
  void Lock();

  // ─── Root Key ──────────────────────────────────────────────────────

  // Derive K_root from a passphrase using Argon2id.
  // |passphrase|: user-provided UTF-8 sync passphrase.
  // |salt|: 16-byte salt. If nullptr, a new random salt is generated.
  //         The salt must be stored alongside the encrypted K_root wrap.
  // Returns true on success. After this, collection keys are available.
  bool DeriveRootFromPassphrase(const std::string& passphrase,
                                const uint8_t* salt = nullptr);

  // Generate a new random K_root (first device only).
  // The root key should then be wrapped for each device.
  bool GenerateNewRoot();

  // Set K_root directly from 32 raw bytes (e.g., after unwrapping).
  // Used when receiving K_root from another device via key exchange.
  bool SetRootKey(const std::vector<uint8_t>& root_key);

  // Get the Argon2id salt used during DeriveRootFromPassphrase.
  // Empty if root was generated or set directly.
  std::vector<uint8_t> GetPassphraseSalt() const;

  // ─── Collection Keys ───────────────────────────────────────────────

  // Get the derived encryption key for a specific collection.
  // Returns nullptr if not unlocked or derivation fails.
  // The returned pointer is valid until Lock() or destruction.
  const uint8_t* GetCollectionKey(SyncDataType type) const;

  // Get a per-record key derived from the collection key + record_id.
  // Returns 32 bytes in |out_key|. Returns false on failure.
  bool GetRecordKey(SyncDataType type,
                    const std::string& record_id,
                    uint8_t out_key[32]) const;

  // ─── Device Key Pairs ──────────────────────────────────────────────

  // Generate new device key pairs (X25519 for key exchange, Ed25519
  // for signing). Called once during device registration.
  bool GenerateDeviceKeys();

  // Get the device's X25519 public key (32 bytes).
  std::vector<uint8_t> GetDeviceKxPublicKey() const;

  // Get the device's Ed25519 verification key (32 bytes).
  std::vector<uint8_t> GetDeviceVerifyKey() const;

  // Sign a message using the device's Ed25519 signing key.
  // Returns 64-byte signature, or empty on failure.
  std::vector<uint8_t> SignMessage(const std::vector<uint8_t>& message) const;

  // Verify a signature from a peer device.
  bool VerifyMessage(const std::vector<uint8_t>& verify_key,
                     const std::vector<uint8_t>& message,
                     const std::vector<uint8_t>& signature) const;

  // ─── Wrapped Root Key ──────────────────────────────────────────────

  // Wrap K_root for a specific device's X25519 public key.
  // Returns the wrapped blob (ephemeral_pub || AEAD sealed K_root).
  // Used when adding a new device to the sync chain.
  std::vector<uint8_t> WrapRootForDevice(
      const std::vector<uint8_t>& device_kx_public_key) const;

  // Unwrap K_root from a wrapped blob using our device's X25519
  // secret key. Sets K_root and derives collection keys on success.
  bool UnwrapRootFromBlob(const std::vector<uint8_t>& wrapped_blob);

  // Wrap K_root with a passphrase-derived key (for recovery).
  // Returns salt || AEAD sealed K_root.
  std::vector<uint8_t> WrapRootWithPassphrase(
      const std::string& passphrase) const;

  // Unwrap K_root from a passphrase-wrapped blob.
  bool UnwrapRootWithPassphrase(const std::string& passphrase,
                                const std::vector<uint8_t>& wrapped_blob);

 private:
  // Derive all collection keys from K_root via HKDF.
  bool DeriveCollectionKeys();

  // Zero a buffer using crypto-safe zeroing.
  static void SecureZero(std::vector<uint8_t>& buf);
  static void SecureZero(uint8_t* buf, size_t len);

  bool initialized_ = false;
  bool unlocked_ = false;

  // K_root — 32 bytes. Held in memory only while unlocked.
  std::vector<uint8_t> root_key_;

  // Argon2id salt used during passphrase derivation (16 bytes).
  std::vector<uint8_t> passphrase_salt_;

  // Per-collection derived keys: SyncDataType → 32-byte key.
  std::unordered_map<uint32_t, std::vector<uint8_t>> collection_keys_;

  // Device X25519 key pair for key exchange.
  std::vector<uint8_t> device_kx_secret_;   // 32 bytes
  std::vector<uint8_t> device_kx_public_;   // 32 bytes

  // Device Ed25519 key pair for signing.
  std::vector<uint8_t> device_sign_secret_; // 32 bytes
  std::vector<uint8_t> device_sign_public_; // 32 bytes

  SEQUENCE_CHECKER(sequence_checker_);
};

}  // namespace sync
}  // namespace vigo

#endif  // VIGO_COMPONENTS_SYNC_VIGO_SYNC_KEY_MANAGER_H_

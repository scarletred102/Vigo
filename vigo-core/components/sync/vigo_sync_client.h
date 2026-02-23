// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_SYNC_VIGO_SYNC_CLIENT_H_
#define VIGO_COMPONENTS_SYNC_VIGO_SYNC_CLIENT_H_

#include <cstdint>
#include <memory>
#include <string>
#include <vector>

#include "base/sequence_checker.h"

namespace vigo {
namespace sync {

// Sync data types that can be synchronised across devices.
enum class SyncDataType {
  kBookmarks,
  kPasswords,
  kHistory,
  kSettings,
  kOpenTabs,
};

// Connection state with the self-hosted sync server.
enum class SyncState {
  kDisconnected,
  kConnecting,
  kConnected,
  kSyncing,
  kError,
};

// VigoSyncClient manages the E2E encrypted sync channel.
//
// Architecture:
//   - All encryption/decryption happens client-side via vigo_crypto Rust crate
//   - Server stores only opaque encrypted blobs (zero-knowledge)
//   - Key hierarchy: K_root → HKDF → per-collection keys
//   - Conflict resolution: CRDT (bookmarks), LWW (settings/passwords),
//     append (history)
//
// Thread safety: all public methods on UI sequence.
class VigoSyncClient {
 public:
  VigoSyncClient();
  ~VigoSyncClient();

  VigoSyncClient(const VigoSyncClient&) = delete;
  VigoSyncClient& operator=(const VigoSyncClient&) = delete;

  // Configure the sync server URL. Empty string disables sync.
  void SetServerUrl(const std::string& url);

  // Get current connection state.
  SyncState GetState() const;

  // Initiate device onboarding with passphrase (Argon2id → unwrap K_root).
  // |passphrase|: user-provided sync passphrase.
  // Returns true if setup succeeded.
  bool SetupWithPassphrase(const std::string& passphrase);

  // Enable or disable sync for a specific data type.
  void SetDataTypeEnabled(SyncDataType type, bool enabled);
  bool IsDataTypeEnabled(SyncDataType type) const;

  // Trigger an immediate sync cycle.
  void SyncNow();

  // Disconnect and wipe local sync state (does NOT delete server data).
  void Disconnect();

 private:
  std::string server_url_;
  SyncState state_ = SyncState::kDisconnected;
  uint32_t enabled_types_ = 0;  // Bitmask of SyncDataType.

  SEQUENCE_CHECKER(sequence_checker_);
};

}  // namespace sync
}  // namespace vigo

#endif  // VIGO_COMPONENTS_SYNC_VIGO_SYNC_CLIENT_H_

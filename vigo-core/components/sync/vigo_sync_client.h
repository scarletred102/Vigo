// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_SYNC_VIGO_SYNC_CLIENT_H_
#define VIGO_COMPONENTS_SYNC_VIGO_SYNC_CLIENT_H_

#include <memory>
#include <string>
#include <vector>

#include "base/sequence_checker.h"
#include "vigo/components/sync/vigo_sync_data_types.h"
#include "vigo/components/sync/vigo_sync_engine.h"
#include "vigo/components/sync/vigo_sync_encryptor.h"
#include "vigo/components/sync/vigo_sync_key_manager.h"
#include "vigo/components/sync/vigo_sync_transport.h"

namespace vigo {
namespace sync {

// VigoSyncClient is the public-facing sync facade for the Vigo browser.
//
// It owns the full sync stack:
//   VigoSyncKeyManager  – key derivation & key hierarchy
//   VigoSyncEncryptor   – per-record AEAD seal / open
//   VigoSyncTransport   – HTTP communication with the sync server
//   VigoSyncEngine      – sync cycle orchestrator
//
// The browser shell creates a single VigoSyncClient instance and interacts
// only through this class. It delegates all real work to the engine.
//
// Thread safety: all public methods on UI sequence.
class VigoSyncClient : public VigoSyncEngineObserver {
 public:
  VigoSyncClient();
  ~VigoSyncClient() override;

  VigoSyncClient(const VigoSyncClient&) = delete;
  VigoSyncClient& operator=(const VigoSyncClient&) = delete;

  // ─── Configuration ─────────────────────────────────────────────────

  // Configure the sync server URL. Empty string disables sync.
  void SetServerUrl(const std::string& url);

  // Get current connection state.
  SyncState GetState() const;

  // ─── Setup ─────────────────────────────────────────────────────────

  // Initiate device onboarding with passphrase:
  //   1. Derive K_root via Argon2id(passphrase, salt).
  //   2. Generate X25519 + Ed25519 device keys.
  //   3. Register this device with the sync server.
  //   4. Initialize the key manager and start the engine.
  //
  // |passphrase|: user-provided sync passphrase (≥ 8 chars recommended).
  // Returns true if setup succeeded.
  bool SetupWithPassphrase(const std::string& passphrase);

  // ─── Data Types ────────────────────────────────────────────────────

  // Enable or disable sync for a specific data type.
  void SetDataTypeEnabled(SyncDataType type, bool enabled);
  bool IsDataTypeEnabled(SyncDataType type) const;

  // ─── Actions ───────────────────────────────────────────────────────

  // Trigger an immediate sync cycle.
  void SyncNow();

  // Disconnect and wipe local sync state (does NOT delete server data).
  void Disconnect();

  // ─── Device Management ─────────────────────────────────────────────

  // Get the list of devices in the sync group.
  std::vector<SyncDevice> GetDevices() const;

  // Get this device's ID.
  std::string GetDeviceId() const;

  // ─── VigoSyncEngineObserver ─────────────────────────────────────────

  void OnSyncCycleCompleted(const SyncCycleResult& result) override;
  void OnSyncStateChanged(SyncState new_state) override;
  void OnCollectionUpdated(SyncDataType type) override;

 private:
  // Owned sync stack components.
  std::unique_ptr<VigoSyncKeyManager> key_manager_;
  std::unique_ptr<VigoSyncEncryptor> encryptor_;
  std::unique_ptr<VigoSyncTransport> transport_;
  std::unique_ptr<VigoSyncEngine> engine_;

  std::string server_url_;
  std::string device_id_;
  SyncState state_ = SyncState::kDisconnected;
  uint32_t enabled_types_ = 0;  // Bitmask of SyncDataType.
  bool is_setup_ = false;

  SEQUENCE_CHECKER(sequence_checker_);
};

}  // namespace sync
}  // namespace vigo

#endif  // VIGO_COMPONENTS_SYNC_VIGO_SYNC_CLIENT_H_

// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_SYNC_VIGO_SYNC_ENGINE_H_
#define VIGO_COMPONENTS_SYNC_VIGO_SYNC_ENGINE_H_

#include <functional>
#include <memory>
#include <string>
#include <vector>

#include "base/memory/raw_ptr.h"
#include "base/memory/weak_ptr.h"
#include "base/sequence_checker.h"
#include "base/timer/timer.h"
#include "vigo/components/sync/vigo_sync_data_types.h"
#include "vigo/components/sync/vigo_sync_encryptor.h"
#include "vigo/components/sync/vigo_sync_key_manager.h"

namespace vigo {
namespace sync {

class VigoSyncTransport;

// Status of a sync cycle.
enum class SyncCycleStatus {
  kSuccess,
  kPartialSuccess,  // Some collections synced, some failed.
  kNetworkError,
  kAuthError,
  kCryptoError,
  kConflictError,
  kServerError,
};

// Result of a single sync cycle.
struct SyncCycleResult {
  SyncCycleStatus status = SyncCycleStatus::kSuccess;
  int records_uploaded = 0;
  int records_downloaded = 0;
  int conflicts_resolved = 0;
  int64_t duration_ms = 0;
  std::string error_message;
};

// Observer interface for sync engine events.
class VigoSyncEngineObserver {
 public:
  virtual ~VigoSyncEngineObserver() = default;

  // Called when a sync cycle completes.
  virtual void OnSyncCycleCompleted(const SyncCycleResult& result) = 0;

  // Called when the sync engine state changes.
  virtual void OnSyncStateChanged(SyncState new_state) = 0;

  // Called when a collection has new data.
  virtual void OnCollectionUpdated(SyncDataType type) = 0;
};

// Callback for providing local records to upload.
using LocalRecordProvider = std::function<std::vector<SyncRecordEnvelope>(
    SyncDataType type,
    int64_t since_timestamp_ms)>;

// Callback for applying downloaded records locally.
using RecordApplier = std::function<bool(
    SyncDataType type,
    const std::vector<SyncRecordEnvelope>& records)>;

// VigoSyncEngine orchestrates the sync lifecycle:
//
// 1. Collects dirty local records via LocalRecordProvider.
// 2. Encrypts them via VigoSyncEncryptor.
// 3. Pushes encrypted envelopes to the server via VigoSyncTransport.
// 4. Pulls new envelopes from the server since last sync timestamp.
// 5. Decrypts and resolves conflicts per ConflictStrategy.
// 6. Applies resolved records via RecordApplier.
//
// Sync cadence:
//   - Periodic: every `sync_interval_seconds_` (default 30s).
//   - On-demand: SyncNow() for user-triggered immediate sync.
//   - Push: future WebSocket push from server.
//
// Thread safety: all public methods on UI sequence.
class VigoSyncEngine {
 public:
  // |key_manager|, |encryptor|, |transport| must outlive this object.
  VigoSyncEngine(VigoSyncKeyManager* key_manager,
                 VigoSyncEncryptor* encryptor,
                 VigoSyncTransport* transport);
  ~VigoSyncEngine();

  VigoSyncEngine(const VigoSyncEngine&) = delete;
  VigoSyncEngine& operator=(const VigoSyncEngine&) = delete;

  // ─── Lifecycle ─────────────────────────────────────────────────────

  // Start the sync engine. Begins periodic sync cycles.
  void Start();

  // Stop the sync engine. Cancels pending cycles.
  void Stop();

  // Whether the engine is actively running.
  bool IsRunning() const;

  // ─── Configuration ─────────────────────────────────────────────────

  // Enable or disable sync for a collection.
  void SetCollectionEnabled(SyncDataType type, bool enabled);
  bool IsCollectionEnabled(SyncDataType type) const;

  // Set the sync interval (default 30 seconds).
  void SetSyncInterval(int seconds);

  // ─── Providers ─────────────────────────────────────────────────────

  // Set the callback that provides dirty local records for upload.
  void SetLocalRecordProvider(LocalRecordProvider provider);

  // Set the callback that applies downloaded records locally.
  void SetRecordApplier(RecordApplier applier);

  // ─── Observers ─────────────────────────────────────────────────────

  void AddObserver(VigoSyncEngineObserver* observer);
  void RemoveObserver(VigoSyncEngineObserver* observer);

  // ─── Manual Sync ───────────────────────────────────────────────────

  // Trigger an immediate sync cycle.
  void SyncNow();

  // Sync a single collection immediately.
  void SyncCollection(SyncDataType type);

  // ─── State ─────────────────────────────────────────────────────────

  SyncState GetState() const;

  // Get the last sync cycle result.
  SyncCycleResult GetLastResult() const;

  // Get the timestamp of the last successful sync for a collection.
  int64_t GetLastSyncTimestamp(SyncDataType type) const;

 private:
  // Execute a full sync cycle across all enabled collections.
  void ExecuteSyncCycle();

  // Sync a single collection: upload dirty → download new → resolve.
  SyncCycleStatus SyncSingleCollection(SyncDataType type,
                                        SyncCycleResult& result);

  // Upload dirty local records for a collection.
  int UploadRecords(SyncDataType type, int64_t since);

  // Download new records from the server for a collection.
  std::vector<SyncRecordEnvelope> DownloadRecords(SyncDataType type,
                                                    int64_t since);

  // Resolve conflicts between local and remote records.
  std::vector<SyncRecordEnvelope> ResolveConflicts(
      SyncDataType type,
      const std::vector<SyncRecordEnvelope>& local_records,
      const std::vector<SyncRecordEnvelope>& remote_records);

  // Apply LWW conflict resolution.
  std::vector<SyncRecordEnvelope> ResolveLWW(
      const std::vector<SyncRecordEnvelope>& local_records,
      const std::vector<SyncRecordEnvelope>& remote_records);

  // Apply append-only conflict resolution.
  std::vector<SyncRecordEnvelope> ResolveAppendOnly(
      const std::vector<SyncRecordEnvelope>& local_records,
      const std::vector<SyncRecordEnvelope>& remote_records);

  // Notify observers of state change.
  void SetState(SyncState state);

  // Timer callback for periodic sync.
  void OnSyncTimer();

  // Not owned.
  raw_ptr<VigoSyncKeyManager> key_manager_;
  raw_ptr<VigoSyncEncryptor> encryptor_;
  raw_ptr<VigoSyncTransport> transport_;

  SyncState state_ = SyncState::kDisconnected;
  SyncCycleResult last_result_;

  // Bitmask of enabled collections.
  uint32_t enabled_collections_ = 0;

  // Periodic sync timer.
  base::RepeatingTimer sync_timer_;
  int sync_interval_seconds_ = 30;

  // Last sync timestamps per collection.
  std::unordered_map<uint32_t, int64_t> last_sync_timestamps_;

  // Callbacks.
  LocalRecordProvider local_record_provider_;
  RecordApplier record_applier_;

  // Observers (raw pointers — observers manage their own lifetime).
  std::vector<VigoSyncEngineObserver*> observers_;

  bool is_running_ = false;

  SEQUENCE_CHECKER(sequence_checker_);
  base::WeakPtrFactory<VigoSyncEngine> weak_factory_{this};
};

}  // namespace sync
}  // namespace vigo

#endif  // VIGO_COMPONENTS_SYNC_VIGO_SYNC_ENGINE_H_

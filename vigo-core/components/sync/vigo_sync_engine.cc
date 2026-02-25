// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/sync/vigo_sync_engine.h"

#include <algorithm>
#include <unordered_set>

#include "base/logging.h"
#include "base/time/time.h"
#include "vigo/components/sync/vigo_sync_transport.h"

namespace vigo {
namespace sync {

VigoSyncEngine::VigoSyncEngine(VigoSyncKeyManager* key_manager,
                               VigoSyncEncryptor* encryptor,
                               VigoSyncTransport* transport)
    : key_manager_(key_manager),
      encryptor_(encryptor),
      transport_(transport) {
  DCHECK(key_manager_);
  DCHECK(encryptor_);
  DCHECK(transport_);
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

VigoSyncEngine::~VigoSyncEngine() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  Stop();
}

// ─── Lifecycle ──────────────────────────────────────────────────────────────

void VigoSyncEngine::Start() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (is_running_) {
    VLOG(1) << "VigoSyncEngine: Already running";
    return;
  }

  if (!key_manager_->IsUnlocked()) {
    LOG(ERROR) << "VigoSyncEngine: Cannot start — key manager locked";
    SetState(SyncState::kError);
    return;
  }

  VLOG(1) << "VigoSyncEngine: Starting with interval "
          << sync_interval_seconds_ << "s";

  is_running_ = true;
  SetState(SyncState::kConnected);

  // Start periodic sync timer.
  sync_timer_.Start(
      FROM_HERE, base::Seconds(sync_interval_seconds_),
      base::BindRepeating(&VigoSyncEngine::OnSyncTimer,
                          weak_factory_.GetWeakPtr()));

  // Trigger an initial sync immediately.
  SyncNow();
}

void VigoSyncEngine::Stop() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (!is_running_) {
    return;
  }

  VLOG(1) << "VigoSyncEngine: Stopping";
  sync_timer_.Stop();
  weak_factory_.InvalidateWeakPtrs();
  is_running_ = false;
  SetState(SyncState::kDisconnected);
}

bool VigoSyncEngine::IsRunning() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return is_running_;
}

// ─── Configuration ──────────────────────────────────────────────────────────

void VigoSyncEngine::SetCollectionEnabled(SyncDataType type, bool enabled) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  uint32_t bit = 1u << static_cast<uint32_t>(type);
  if (enabled) {
    enabled_collections_ |= bit;
  } else {
    enabled_collections_ &= ~bit;
  }
}

bool VigoSyncEngine::IsCollectionEnabled(SyncDataType type) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  uint32_t bit = 1u << static_cast<uint32_t>(type);
  return (enabled_collections_ & bit) != 0;
}

void VigoSyncEngine::SetSyncInterval(int seconds) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  DCHECK_GT(seconds, 0);
  sync_interval_seconds_ = seconds;

  // Restart timer if running.
  if (is_running_ && sync_timer_.IsRunning()) {
    sync_timer_.Stop();
    sync_timer_.Start(
        FROM_HERE, base::Seconds(sync_interval_seconds_),
        base::BindRepeating(&VigoSyncEngine::OnSyncTimer,
                            weak_factory_.GetWeakPtr()));
  }
}

// ─── Providers ──────────────────────────────────────────────────────────────

void VigoSyncEngine::SetLocalRecordProvider(LocalRecordProvider provider) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  local_record_provider_ = std::move(provider);
}

void VigoSyncEngine::SetRecordApplier(RecordApplier applier) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  record_applier_ = std::move(applier);
}

// ─── Observers ──────────────────────────────────────────────────────────────

void VigoSyncEngine::AddObserver(VigoSyncEngineObserver* observer) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  DCHECK(observer);
  observers_.push_back(observer);
}

void VigoSyncEngine::RemoveObserver(VigoSyncEngineObserver* observer) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  auto it = std::find(observers_.begin(), observers_.end(), observer);
  if (it != observers_.end()) {
    observers_.erase(it);
  }
}

// ─── Manual Sync ────────────────────────────────────────────────────────────

void VigoSyncEngine::SyncNow() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (!is_running_) {
    LOG(WARNING) << "VigoSyncEngine: Not running — ignoring SyncNow";
    return;
  }

  ExecuteSyncCycle();
}

void VigoSyncEngine::SyncCollection(SyncDataType type) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (!is_running_) {
    LOG(WARNING) << "VigoSyncEngine: Not running — ignoring SyncCollection";
    return;
  }

  SyncCycleResult result;
  auto start = base::Time::Now();
  result.status = SyncSingleCollection(type, result);
  result.duration_ms =
      (base::Time::Now() - start).InMilliseconds();

  last_result_ = result;
  for (auto* observer : observers_) {
    observer->OnSyncCycleCompleted(result);
  }
}

// ─── State ──────────────────────────────────────────────────────────────────

SyncState VigoSyncEngine::GetState() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return state_;
}

SyncCycleResult VigoSyncEngine::GetLastResult() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return last_result_;
}

int64_t VigoSyncEngine::GetLastSyncTimestamp(SyncDataType type) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  auto it = last_sync_timestamps_.find(static_cast<uint32_t>(type));
  if (it == last_sync_timestamps_.end()) {
    return 0;
  }
  return it->second;
}

// ─── Private ────────────────────────────────────────────────────────────────

void VigoSyncEngine::ExecuteSyncCycle() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (!key_manager_->IsUnlocked()) {
    LOG(ERROR) << "VigoSyncEngine: Key manager locked during sync cycle";
    SetState(SyncState::kError);
    return;
  }

  SetState(SyncState::kSyncing);
  VLOG(1) << "VigoSyncEngine: Starting sync cycle";

  auto cycle_start = base::Time::Now();
  SyncCycleResult result;
  bool any_success = false;
  bool any_failure = false;

  // Iterate over all data type values.
  for (uint32_t i = 0; i <= static_cast<uint32_t>(SyncDataType::kMaxValue);
       ++i) {
    auto type = static_cast<SyncDataType>(i);
    if (!IsCollectionEnabled(type)) {
      continue;
    }

    auto status = SyncSingleCollection(type, result);
    if (status == SyncCycleStatus::kSuccess) {
      any_success = true;
    } else {
      any_failure = true;
      VLOG(1) << "VigoSyncEngine: Collection "
              << SyncDataTypeToString(type) << " failed";
    }
  }

  result.duration_ms =
      (base::Time::Now() - cycle_start).InMilliseconds();

  if (any_success && !any_failure) {
    result.status = SyncCycleStatus::kSuccess;
  } else if (any_success && any_failure) {
    result.status = SyncCycleStatus::kPartialSuccess;
  } else if (any_failure) {
    result.status = SyncCycleStatus::kNetworkError;
  }

  last_result_ = result;
  SetState(SyncState::kConnected);

  VLOG(1) << "VigoSyncEngine: Sync cycle completed in "
          << result.duration_ms << "ms — uploaded "
          << result.records_uploaded << ", downloaded "
          << result.records_downloaded << ", conflicts "
          << result.conflicts_resolved;

  for (auto* observer : observers_) {
    observer->OnSyncCycleCompleted(result);
  }
}

SyncCycleStatus VigoSyncEngine::SyncSingleCollection(
    SyncDataType type,
    SyncCycleResult& result) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  int64_t since = GetLastSyncTimestamp(type);

  // Phase 1: Upload dirty local records.
  int uploaded = UploadRecords(type, since);
  if (uploaded < 0) {
    return SyncCycleStatus::kNetworkError;
  }
  result.records_uploaded += uploaded;

  // Phase 2: Download new remote records.
  auto remote_records = DownloadRecords(type, since);

  // Phase 3: If we have both local and remote records, resolve conflicts.
  if (!remote_records.empty()) {
    // Get local records for the same period to detect conflicts.
    std::vector<SyncRecordEnvelope> local_envelopes;
    if (local_record_provider_) {
      local_envelopes = local_record_provider_(type, since);
    }

    auto resolved = ResolveConflicts(type, local_envelopes, remote_records);
    result.conflicts_resolved +=
        static_cast<int>(resolved.size()) -
        static_cast<int>(remote_records.size());

    // Phase 4: Apply resolved records locally.
    if (record_applier_ && !resolved.empty()) {
      if (!record_applier_(type, resolved)) {
        LOG(ERROR) << "VigoSyncEngine: Failed to apply records for "
                   << SyncDataTypeToString(type);
        return SyncCycleStatus::kConflictError;
      }
    }

    result.records_downloaded += static_cast<int>(resolved.size());

    for (auto* observer : observers_) {
      observer->OnCollectionUpdated(type);
    }
  }

  // Update last sync timestamp.
  last_sync_timestamps_[static_cast<uint32_t>(type)] =
      base::Time::Now().InMillisecondsSinceUnixEpoch();

  return SyncCycleStatus::kSuccess;
}

int VigoSyncEngine::UploadRecords(SyncDataType type, int64_t since) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (!local_record_provider_) {
    return 0;
  }

  auto local_records = local_record_provider_(type, since);
  if (local_records.empty()) {
    return 0;
  }

  // Encrypt and upload each record.
  int uploaded = 0;
  for (const auto& envelope : local_records) {
    // The envelope is already encrypted by the provider (assumed).
    if (!transport_->PushRecord(type, envelope)) {
      LOG(ERROR) << "VigoSyncEngine: Failed to push record "
                 << envelope.record_id;
      return -1;
    }
    ++uploaded;
  }

  VLOG(2) << "VigoSyncEngine: Uploaded " << uploaded << " records for "
          << SyncDataTypeToString(type);
  return uploaded;
}

std::vector<SyncRecordEnvelope> VigoSyncEngine::DownloadRecords(
    SyncDataType type,
    int64_t since) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  auto records = transport_->PullRecords(type, since);

  VLOG(2) << "VigoSyncEngine: Downloaded " << records.size()
          << " records for " << SyncDataTypeToString(type);
  return records;
}

std::vector<SyncRecordEnvelope> VigoSyncEngine::ResolveConflicts(
    SyncDataType type,
    const std::vector<SyncRecordEnvelope>& local_records,
    const std::vector<SyncRecordEnvelope>& remote_records) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  auto strategy = GetConflictStrategy(type);

  switch (strategy) {
    case ConflictStrategy::kCRDT:
      // CRDT merge: for bookmarks, we merge tree structures.
      // For now, fall through to LWW as placeholder — full CRDT in
      // vigo_bookmark_crdt.h.
      return ResolveLWW(local_records, remote_records);

    case ConflictStrategy::kLastWriterWins:
      return ResolveLWW(local_records, remote_records);

    case ConflictStrategy::kAppendOnly:
      return ResolveAppendOnly(local_records, remote_records);
  }

  return remote_records;
}

std::vector<SyncRecordEnvelope> VigoSyncEngine::ResolveLWW(
    const std::vector<SyncRecordEnvelope>& local_records,
    const std::vector<SyncRecordEnvelope>& remote_records) {
  // Build lookup of local records by ID.
  std::unordered_map<std::string, const SyncRecordEnvelope*> local_by_id;
  for (const auto& rec : local_records) {
    local_by_id[rec.record_id] = &rec;
  }

  std::vector<SyncRecordEnvelope> result;
  result.reserve(remote_records.size());

  for (const auto& remote : remote_records) {
    auto it = local_by_id.find(remote.record_id);
    if (it == local_by_id.end()) {
      // No local conflict — accept remote.
      result.push_back(remote);
    } else {
      // Conflict: pick the one with the later timestamp.
      const auto* local = it->second;
      if (remote.modified_at_ms >= local->modified_at_ms) {
        result.push_back(remote);
      } else {
        result.push_back(*local);
      }
    }
  }

  return result;
}

std::vector<SyncRecordEnvelope> VigoSyncEngine::ResolveAppendOnly(
    const std::vector<SyncRecordEnvelope>& local_records,
    const std::vector<SyncRecordEnvelope>& remote_records) {
  // Append-only: keep all remote records. Dedup by record_id.
  std::unordered_set<std::string> local_ids;
  for (const auto& rec : local_records) {
    local_ids.insert(rec.record_id);
  }

  std::vector<SyncRecordEnvelope> result;
  for (const auto& remote : remote_records) {
    if (local_ids.find(remote.record_id) == local_ids.end()) {
      result.push_back(remote);
    }
  }

  return result;
}

void VigoSyncEngine::SetState(SyncState state) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (state_ == state) {
    return;
  }

  state_ = state;
  for (auto* observer : observers_) {
    observer->OnSyncStateChanged(state);
  }
}

void VigoSyncEngine::OnSyncTimer() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  ExecuteSyncCycle();
}

}  // namespace sync
}  // namespace vigo

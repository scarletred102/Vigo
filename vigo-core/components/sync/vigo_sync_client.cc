// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/sync/vigo_sync_client.h"

#include "base/logging.h"

namespace vigo {
namespace sync {

VigoSyncClient::VigoSyncClient() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

VigoSyncClient::~VigoSyncClient() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

void VigoSyncClient::SetServerUrl(const std::string& url) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  server_url_ = url;
  VLOG(1) << "VigoSyncClient: Server URL set to "
          << (url.empty() ? "(disabled)" : url);
}

SyncState VigoSyncClient::GetState() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return state_;
}

bool VigoSyncClient::SetupWithPassphrase(const std::string& passphrase) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (server_url_.empty()) {
    LOG(ERROR) << "VigoSyncClient: Cannot setup — no server URL configured";
    return false;
  }

  if (passphrase.empty()) {
    LOG(ERROR) << "VigoSyncClient: Cannot setup — empty passphrase";
    return false;
  }

  VLOG(1) << "VigoSyncClient: Setting up with passphrase";

  // TODO(Phase 3.2): Call vigo_crypto Rust FFI to derive K_root via Argon2id.
  // TODO(Phase 3.3): Perform key exchange with server, register device.
  state_ = SyncState::kConnecting;

  return true;
}

void VigoSyncClient::SetDataTypeEnabled(SyncDataType type, bool enabled) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  uint32_t bit = 1u << static_cast<uint32_t>(type);
  if (enabled) {
    enabled_types_ |= bit;
  } else {
    enabled_types_ &= ~bit;
  }
}

bool VigoSyncClient::IsDataTypeEnabled(SyncDataType type) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  uint32_t bit = 1u << static_cast<uint32_t>(type);
  return (enabled_types_ & bit) != 0;
}

void VigoSyncClient::SyncNow() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (state_ == SyncState::kDisconnected) {
    LOG(WARNING) << "VigoSyncClient: Cannot sync — not connected";
    return;
  }

  VLOG(1) << "VigoSyncClient: Triggering sync cycle";
  // TODO(Phase 3.3): Implement sync cycle.
}

void VigoSyncClient::Disconnect() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  VLOG(1) << "VigoSyncClient: Disconnecting";
  state_ = SyncState::kDisconnected;
  server_url_.clear();
  enabled_types_ = 0;
  // TODO(Phase 3.3): Wipe local sync metadata and cached keys.
}

}  // namespace sync
}  // namespace vigo

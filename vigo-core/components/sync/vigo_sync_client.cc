// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/sync/vigo_sync_client.h"

#include <utility>

#include "base/logging.h"

namespace vigo {
namespace sync {

VigoSyncClient::VigoSyncClient() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

VigoSyncClient::~VigoSyncClient() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  Disconnect();
}

// ─── Configuration ──────────────────────────────────────────────────────────

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

// ─── Setup ──────────────────────────────────────────────────────────────────

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
  state_ = SyncState::kConnecting;

  // Step 1: Create the key manager and derive K_root from passphrase.
  key_manager_ = std::make_unique<VigoSyncKeyManager>();
  if (!key_manager_->Init()) {
    LOG(ERROR) << "VigoSyncClient: Failed to initialise crypto subsystem";
    state_ = SyncState::kError;
    key_manager_.reset();
    return false;
  }

  if (!key_manager_->DeriveRootFromPassphrase(passphrase)) {
    LOG(ERROR) << "VigoSyncClient: Failed to derive root key from passphrase";
    state_ = SyncState::kError;
    key_manager_.reset();
    return false;
  }

  // Generate device key pairs for key exchange and signing.
  if (!key_manager_->GenerateDeviceKeys()) {
    LOG(ERROR) << "VigoSyncClient: Failed to generate device keys";
    state_ = SyncState::kError;
    key_manager_.reset();
    return false;
  }

  // Step 2: Create the encryptor using the key manager.
  encryptor_ = std::make_unique<VigoSyncEncryptor>(key_manager_.get());

  // Step 3: Create the transport and configure it.
  transport_ = std::make_unique<VigoSyncTransport>();
  transport_->SetServerUrl(server_url_);

  // Step 4: Register this device with the sync server.
  DeviceRegistration registration;
  registration.device_name = "Vigo Desktop";
  registration.kx_public_key = key_manager_->GetDeviceKxPublicKey();
  registration.verify_key = key_manager_->GetDeviceVerifyKey();
  // TODO(Phase 3.5): Sign the registration payload with Ed25519.

  auto reg_result = transport_->RegisterDevice(registration);
  if (reg_result.success) {
    device_id_ = reg_result.device_id;
    transport_->SetDeviceId(device_id_);
    transport_->SetAuthToken(device_id_);  // Simplified auth for now.
    VLOG(1) << "VigoSyncClient: Registered device " << device_id_;
  } else {
    // Registration failure is non-fatal for offline-first — we can still
    // set up locally and sync when the server becomes available.
    LOG(WARNING) << "VigoSyncClient: Device registration failed: "
                 << reg_result.error_message
                 << " — continuing in offline mode";
  }

  // Step 5: Create and start the sync engine.
  engine_ = std::make_unique<VigoSyncEngine>(
      key_manager_.get(), encryptor_.get(), transport_.get());
  engine_->AddObserver(this);

  // Forward enabled types to the engine.
  for (uint32_t i = 0; i <= static_cast<uint32_t>(SyncDataType::kMaxValue);
       ++i) {
    auto type = static_cast<SyncDataType>(i);
    if (IsDataTypeEnabled(type)) {
      engine_->SetCollectionEnabled(type, true);
    }
  }

  // Start syncing.
  engine_->Start();
  is_setup_ = true;

  VLOG(1) << "VigoSyncClient: Setup complete — sync engine started";
  return true;
}

// ─── Data Types ─────────────────────────────────────────────────────────────

void VigoSyncClient::SetDataTypeEnabled(SyncDataType type, bool enabled) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  uint32_t bit = 1u << static_cast<uint32_t>(type);
  if (enabled) {
    enabled_types_ |= bit;
  } else {
    enabled_types_ &= ~bit;
  }

  // Forward to engine if it's running.
  if (engine_) {
    engine_->SetCollectionEnabled(type, enabled);
  }
}

bool VigoSyncClient::IsDataTypeEnabled(SyncDataType type) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  uint32_t bit = 1u << static_cast<uint32_t>(type);
  return (enabled_types_ & bit) != 0;
}

// ─── Actions ────────────────────────────────────────────────────────────────

void VigoSyncClient::SyncNow() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (!is_setup_ || !engine_) {
    LOG(WARNING) << "VigoSyncClient: Cannot sync — not set up";
    return;
  }

  VLOG(1) << "VigoSyncClient: Triggering immediate sync cycle";
  engine_->SyncNow();
}

void VigoSyncClient::Disconnect() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  VLOG(1) << "VigoSyncClient: Disconnecting";

  // Stop and destroy the engine first.
  if (engine_) {
    engine_->RemoveObserver(this);
    engine_->Stop();
    engine_.reset();
  }

  // Destroy crypto components (key manager zeroes key material).
  encryptor_.reset();
  if (key_manager_) {
    key_manager_->Lock();
    key_manager_.reset();
  }

  // Destroy transport.
  transport_.reset();

  // Reset local state.
  state_ = SyncState::kDisconnected;
  server_url_.clear();
  device_id_.clear();
  enabled_types_ = 0;
  is_setup_ = false;
}

// ─── Device Management ──────────────────────────────────────────────────────

std::vector<SyncDevice> VigoSyncClient::GetDevices() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  // TODO(Phase 3.5): Fetch from transport.
  return {};
}

std::string VigoSyncClient::GetDeviceId() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return device_id_;
}

// ─── VigoSyncEngineObserver ─────────────────────────────────────────────────

void VigoSyncClient::OnSyncCycleCompleted(const SyncCycleResult& result) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  VLOG(1) << "VigoSyncClient: Sync cycle completed — uploaded "
          << result.records_uploaded << ", downloaded "
          << result.records_downloaded;
}

void VigoSyncClient::OnSyncStateChanged(SyncState new_state) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  state_ = new_state;
  VLOG(1) << "VigoSyncClient: State changed to "
          << static_cast<int>(new_state);
}

void VigoSyncClient::OnCollectionUpdated(SyncDataType type) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  VLOG(1) << "VigoSyncClient: Collection updated — "
          << SyncDataTypeToString(type);
}

}  // namespace sync
}  // namespace vigo

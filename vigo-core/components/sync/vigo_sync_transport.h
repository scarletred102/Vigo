// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_SYNC_VIGO_SYNC_TRANSPORT_H_
#define VIGO_COMPONENTS_SYNC_VIGO_SYNC_TRANSPORT_H_

#include <functional>
#include <memory>
#include <string>
#include <vector>

#include "base/sequence_checker.h"
#include "vigo/components/sync/vigo_sync_data_types.h"

namespace vigo {
namespace sync {

// HTTP response from the sync server.
struct SyncHttpResponse {
  int status_code = 0;
  std::string body;
  bool is_success() const { return status_code >= 200 && status_code < 300; }
};

// Device registration request.
struct DeviceRegistration {
  std::string device_name;
  std::vector<uint8_t> kx_public_key;     // X25519 public key (32 bytes).
  std::vector<uint8_t> verify_key;         // Ed25519 verify key (32 bytes).
  std::vector<uint8_t> signature;          // Ed25519 signature of the
                                            // registration payload.
};

// Device registration response.
struct DeviceRegistrationResult {
  bool success = false;
  std::string device_id;
  std::string error_message;
  // Wrapped K_root from existing device (empty if first device).
  std::vector<uint8_t> wrapped_root_key;
};

// VigoSyncTransport handles HTTP communication with the self-hosted
// Vigo sync server.
//
// All communication is over HTTPS with:
//   - TLS 1.3 minimum
//   - Certificate pinning for known server endpoints
//   - Ed25519 request signing for authentication
//
// The server only receives encrypted opaque blobs — zero-knowledge.
//
// Thread safety: all public methods on UI sequence.
class VigoSyncTransport {
 public:
  VigoSyncTransport();
  virtual ~VigoSyncTransport();

  VigoSyncTransport(const VigoSyncTransport&) = delete;
  VigoSyncTransport& operator=(const VigoSyncTransport&) = delete;

  // ─── Configuration ─────────────────────────────────────────────────

  // Set the sync server base URL (e.g., "https://sync.example.com").
  void SetServerUrl(const std::string& url);
  std::string GetServerUrl() const;

  // Set the authentication token (received during device registration).
  void SetAuthToken(const std::string& token);

  // Set the device ID (received during device registration).
  void SetDeviceId(const std::string& device_id);

  // ─── Device Registration ──────────────────────────────────────────

  // Register this device with the sync server.
  // Returns the server-assigned device_id and optional wrapped K_root.
  virtual DeviceRegistrationResult RegisterDevice(
      const DeviceRegistration& registration);

  // Deregister (revoke) this device.
  virtual bool DeregisterDevice(const std::string& device_id);

  // ─── Record Operations ────────────────────────────────────────────

  // Push a single encrypted record to the server.
  virtual bool PushRecord(SyncDataType type,
                          const SyncRecordEnvelope& envelope);

  // Push a batch of encrypted records.
  virtual bool PushRecords(SyncDataType type,
                           const std::vector<SyncRecordEnvelope>& envelopes);

  // Pull records modified since |since_timestamp_ms| for a collection.
  virtual std::vector<SyncRecordEnvelope> PullRecords(
      SyncDataType type,
      int64_t since_timestamp_ms);

  // Delete a record from the server (tombstone).
  virtual bool DeleteRecord(SyncDataType type, const std::string& record_id);

  // ─── Key Exchange ─────────────────────────────────────────────────

  // Upload a wrapped K_root blob for a target device.
  virtual bool PushWrappedKey(const std::string& target_device_id,
                              const std::vector<uint8_t>& wrapped_blob);

  // Fetch a wrapped K_root blob for this device.
  virtual std::vector<uint8_t> FetchWrappedKey();

  // ─── Health ───────────────────────────────────────────────────────

  // Ping the server to check connectivity.
  virtual bool Ping();

  // Get the last HTTP error message.
  std::string GetLastError() const;

 protected:
  // Execute an HTTP request. Override in tests.
  virtual SyncHttpResponse DoRequest(const std::string& method,
                                     const std::string& path,
                                     const std::string& body);

 private:
  // Serialise a SyncRecordEnvelope to JSON.
  static std::string EnvelopeToJson(const SyncRecordEnvelope& envelope);

  // Deserialise a JSON string to SyncRecordEnvelope.
  static SyncRecordEnvelope JsonToEnvelope(const std::string& json);

  // Build the full URL for a path.
  std::string BuildUrl(const std::string& path) const;

  std::string server_url_;
  std::string auth_token_;
  std::string device_id_;
  std::string last_error_;

  SEQUENCE_CHECKER(sequence_checker_);
};

}  // namespace sync
}  // namespace vigo

#endif  // VIGO_COMPONENTS_SYNC_VIGO_SYNC_TRANSPORT_H_

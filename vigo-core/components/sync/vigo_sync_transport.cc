// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/sync/vigo_sync_transport.h"

#include <sstream>

#include "base/json/json_reader.h"
#include "base/json/json_writer.h"
#include "base/base64.h"
#include "base/logging.h"
#include "base/values.h"

namespace vigo {
namespace sync {

VigoSyncTransport::VigoSyncTransport() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

VigoSyncTransport::~VigoSyncTransport() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

// ─── Configuration ──────────────────────────────────────────────────────────

void VigoSyncTransport::SetServerUrl(const std::string& url) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  server_url_ = url;
  // Strip trailing slash.
  if (!server_url_.empty() && server_url_.back() == '/') {
    server_url_.pop_back();
  }
}

std::string VigoSyncTransport::GetServerUrl() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return server_url_;
}

void VigoSyncTransport::SetAuthToken(const std::string& token) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  auth_token_ = token;
}

void VigoSyncTransport::SetDeviceId(const std::string& device_id) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  device_id_ = device_id;
}

// ─── Device Registration ────────────────────────────────────────────────────

DeviceRegistrationResult VigoSyncTransport::RegisterDevice(
    const DeviceRegistration& registration) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  DeviceRegistrationResult result;

  base::Value::Dict payload;
  payload.Set("device_name", registration.device_name);
  payload.Set("kx_public_key",
              base::Base64Encode(registration.kx_public_key));
  payload.Set("verify_key",
              base::Base64Encode(registration.verify_key));
  payload.Set("signature",
              base::Base64Encode(registration.signature));

  std::string body;
  base::JSONWriter::Write(base::Value(std::move(payload)), &body);

  auto response = DoRequest("POST", "/api/v1/devices", body);
  if (!response.is_success()) {
    result.error_message = "Registration failed: HTTP " +
                           std::to_string(response.status_code);
    last_error_ = result.error_message;
    return result;
  }

  // Parse response.
  auto parsed = base::JSONReader::Read(response.body);
  if (!parsed || !parsed->is_dict()) {
    result.error_message = "Invalid registration response";
    last_error_ = result.error_message;
    return result;
  }

  const auto& dict = parsed->GetDict();
  const std::string* device_id = dict.FindString("device_id");
  if (device_id) {
    result.device_id = *device_id;
    result.success = true;
  }

  const std::string* wrapped_key_b64 = dict.FindString("wrapped_root_key");
  if (wrapped_key_b64) {
    auto decoded = base::Base64Decode(*wrapped_key_b64);
    if (decoded) {
      result.wrapped_root_key.assign(decoded->begin(), decoded->end());
    }
  }

  return result;
}

bool VigoSyncTransport::DeregisterDevice(const std::string& device_id) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  auto response = DoRequest("DELETE", "/api/v1/devices/" + device_id, "");
  if (!response.is_success()) {
    last_error_ = "Deregister failed: HTTP " +
                  std::to_string(response.status_code);
    return false;
  }
  return true;
}

// ─── Record Operations ──────────────────────────────────────────────────────

bool VigoSyncTransport::PushRecord(SyncDataType type,
                                   const SyncRecordEnvelope& envelope) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  std::string path = std::string("/api/v1/collections/") +
                     SyncDataTypeToString(type) + "/records";

  std::string body = EnvelopeToJson(envelope);
  auto response = DoRequest("PUT", path, body);
  if (!response.is_success()) {
    last_error_ = "Push failed: HTTP " + std::to_string(response.status_code);
    LOG(ERROR) << "VigoSyncTransport: " << last_error_;
    return false;
  }

  return true;
}

bool VigoSyncTransport::PushRecords(
    SyncDataType type,
    const std::vector<SyncRecordEnvelope>& envelopes) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  std::string path = std::string("/api/v1/collections/") +
                     SyncDataTypeToString(type) + "/records/batch";

  base::Value::List records_json;
  for (const auto& env : envelopes) {
    auto parsed = base::JSONReader::Read(EnvelopeToJson(env));
    if (parsed) {
      records_json.Append(std::move(*parsed));
    }
  }

  base::Value::Dict batch;
  batch.Set("records", std::move(records_json));

  std::string body;
  base::JSONWriter::Write(base::Value(std::move(batch)), &body);

  auto response = DoRequest("PUT", path, body);
  if (!response.is_success()) {
    last_error_ = "Batch push failed: HTTP " +
                  std::to_string(response.status_code);
    return false;
  }

  return true;
}

std::vector<SyncRecordEnvelope> VigoSyncTransport::PullRecords(
    SyncDataType type,
    int64_t since_timestamp_ms) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  std::string path = std::string("/api/v1/collections/") +
                     SyncDataTypeToString(type) + "/records?since=" +
                     std::to_string(since_timestamp_ms);

  auto response = DoRequest("GET", path, "");
  if (!response.is_success()) {
    last_error_ = "Pull failed: HTTP " + std::to_string(response.status_code);
    return {};
  }

  auto parsed = base::JSONReader::Read(response.body);
  if (!parsed || !parsed->is_dict()) {
    return {};
  }

  const auto* records_list = parsed->GetDict().FindList("records");
  if (!records_list) {
    return {};
  }

  std::vector<SyncRecordEnvelope> result;
  for (const auto& item : *records_list) {
    std::string item_json;
    base::JSONWriter::Write(item, &item_json);
    result.push_back(JsonToEnvelope(item_json));
  }

  return result;
}

bool VigoSyncTransport::DeleteRecord(SyncDataType type,
                                     const std::string& record_id) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  std::string path = std::string("/api/v1/collections/") +
                     SyncDataTypeToString(type) + "/records/" + record_id;

  auto response = DoRequest("DELETE", path, "");
  return response.is_success();
}

// ─── Key Exchange ───────────────────────────────────────────────────────────

bool VigoSyncTransport::PushWrappedKey(
    const std::string& target_device_id,
    const std::vector<uint8_t>& wrapped_blob) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  base::Value::Dict payload;
  payload.Set("target_device_id", target_device_id);
  payload.Set("wrapped_key", base::Base64Encode(wrapped_blob));

  std::string body;
  base::JSONWriter::Write(base::Value(std::move(payload)), &body);

  auto response = DoRequest("POST", "/api/v1/keys/wrap", body);
  return response.is_success();
}

std::vector<uint8_t> VigoSyncTransport::FetchWrappedKey() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  auto response = DoRequest("GET",
                            "/api/v1/keys/wrap/" + device_id_, "");
  if (!response.is_success()) {
    return {};
  }

  auto parsed = base::JSONReader::Read(response.body);
  if (!parsed || !parsed->is_dict()) {
    return {};
  }

  const std::string* wrapped_b64 =
      parsed->GetDict().FindString("wrapped_key");
  if (!wrapped_b64) {
    return {};
  }

  auto decoded = base::Base64Decode(*wrapped_b64);
  if (!decoded) {
    return {};
  }

  return std::vector<uint8_t>(decoded->begin(), decoded->end());
}

// ─── Health ─────────────────────────────────────────────────────────────────

bool VigoSyncTransport::Ping() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  auto response = DoRequest("GET", "/api/v1/health", "");
  return response.is_success();
}

std::string VigoSyncTransport::GetLastError() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return last_error_;
}

// ─── HTTP Layer ─────────────────────────────────────────────────────────────

SyncHttpResponse VigoSyncTransport::DoRequest(const std::string& method,
                                              const std::string& path,
                                              const std::string& body) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  // TODO(Phase 3.4): Implement actual HTTP requests using
  // network::SimpleURLLoader via the browser's URL loader factory.
  // For now, return a stub error response to indicate not-yet-wired.

  VLOG(1) << "VigoSyncTransport: " << method << " " << BuildUrl(path)
          << " (body=" << body.size() << " bytes)";

  SyncHttpResponse response;
  response.status_code = 503;  // Service Unavailable (not wired yet).
  response.body = "{\"error\": \"transport not yet wired\"}";
  last_error_ = "Transport layer not yet connected to network stack";
  return response;
}

// ─── Serialisation ──────────────────────────────────────────────────────────

std::string VigoSyncTransport::EnvelopeToJson(
    const SyncRecordEnvelope& envelope) {
  base::Value::Dict dict;
  dict.Set("record_id", envelope.record_id);
  dict.Set("collection", SyncDataTypeToString(envelope.collection));
  dict.Set("ciphertext", base::Base64Encode(envelope.ciphertext));
  dict.Set("version", static_cast<int>(envelope.version));
  dict.Set("modified_at", base::NumberToString(envelope.modified_at_ms));

  if (!envelope.content_hash.empty()) {
    dict.Set("content_hash", base::Base64Encode(envelope.content_hash));
  }

  std::string json;
  base::JSONWriter::Write(base::Value(std::move(dict)), &json);
  return json;
}

SyncRecordEnvelope VigoSyncTransport::JsonToEnvelope(
    const std::string& json) {
  SyncRecordEnvelope envelope;

  auto parsed = base::JSONReader::Read(json);
  if (!parsed || !parsed->is_dict()) {
    return envelope;
  }

  const auto& dict = parsed->GetDict();

  const std::string* record_id = dict.FindString("record_id");
  if (record_id) {
    envelope.record_id = *record_id;
  }

  const std::string* collection = dict.FindString("collection");
  if (collection) {
    if (*collection == "bookmarks")
      envelope.collection = SyncDataType::kBookmarks;
    else if (*collection == "passwords")
      envelope.collection = SyncDataType::kPasswords;
    else if (*collection == "history")
      envelope.collection = SyncDataType::kHistory;
    else if (*collection == "settings")
      envelope.collection = SyncDataType::kSettings;
    else if (*collection == "open_tabs")
      envelope.collection = SyncDataType::kOpenTabs;
  }

  const std::string* ciphertext_b64 = dict.FindString("ciphertext");
  if (ciphertext_b64) {
    auto decoded = base::Base64Decode(*ciphertext_b64);
    if (decoded) {
      envelope.ciphertext.assign(decoded->begin(), decoded->end());
    }
  }

  auto version = dict.FindInt("version");
  if (version) {
    envelope.version = static_cast<uint32_t>(*version);
  }

  const std::string* modified_at = dict.FindString("modified_at");
  if (modified_at) {
    // Parse int64 from string.
    int64_t value = 0;
    std::istringstream iss(*modified_at);
    iss >> value;
    envelope.modified_at_ms = value;
  }

  const std::string* content_hash_b64 = dict.FindString("content_hash");
  if (content_hash_b64) {
    auto decoded = base::Base64Decode(*content_hash_b64);
    if (decoded) {
      envelope.content_hash.assign(decoded->begin(), decoded->end());
    }
  }

  return envelope;
}

std::string VigoSyncTransport::BuildUrl(const std::string& path) const {
  return server_url_ + path;
}

}  // namespace sync
}  // namespace vigo

// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/sync/vigo_sync_encryptor.h"

#include <cstring>

#include "base/logging.h"
#include "base/time/time.h"
#include "vigo/components/sync/ffi/vigo_crypto_ffi.h"

namespace vigo {
namespace sync {

VigoSyncEncryptor::VigoSyncEncryptor(const VigoSyncKeyManager* key_manager)
    : key_manager_(key_manager) {
  DCHECK(key_manager_);
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

VigoSyncEncryptor::~VigoSyncEncryptor() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

SyncRecordEnvelope VigoSyncEncryptor::EncryptRecord(
    const std::string& record_id,
    SyncDataType collection,
    const std::vector<uint8_t>& plaintext) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  SyncRecordEnvelope envelope;
  envelope.record_id = record_id;
  envelope.collection = collection;
  envelope.version = 1;
  envelope.modified_at_ms =
      base::Time::Now().InMillisecondsSinceUnixEpoch();

  if (!key_manager_->IsUnlocked()) {
    LOG(ERROR) << "VigoSyncEncryptor: Key manager not unlocked";
    return envelope;
  }

  // Derive per-record key.
  uint8_t record_key[32];
  if (!key_manager_->GetRecordKey(collection, record_id, record_key)) {
    LOG(ERROR) << "VigoSyncEncryptor: Failed to derive record key for "
               << record_id;
    return envelope;
  }

  // Use collection name as AAD to bind ciphertext to its collection.
  const char* aad = SyncDataTypeToString(collection);

  // Seal: nonce || ciphertext || tag.
  VigoCryptoBuffer sealed = vigo_crypto_aead_seal(
      record_key,
      plaintext.data(), plaintext.size(),
      reinterpret_cast<const uint8_t*>(aad), strlen(aad));

  // Securely zero the record key.
  volatile uint8_t* p = record_key;
  for (size_t i = 0; i < 32; ++i) {
    p[i] = 0;
  }

  if (!sealed.data) {
    LOG(ERROR) << "VigoSyncEncryptor: AEAD seal failed for " << record_id;
    return envelope;
  }

  envelope.ciphertext.assign(sealed.data, sealed.data + sealed.len);
  vigo_crypto_free_buffer(sealed);

  VLOG(2) << "VigoSyncEncryptor: Encrypted record " << record_id
          << " (" << plaintext.size() << " → "
          << envelope.ciphertext.size() << " bytes)";
  return envelope;
}

std::vector<uint8_t> VigoSyncEncryptor::DecryptRecord(
    const SyncRecordEnvelope& envelope) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (!key_manager_->IsUnlocked()) {
    LOG(ERROR) << "VigoSyncEncryptor: Key manager not unlocked";
    return {};
  }

  if (envelope.ciphertext.empty()) {
    LOG(ERROR) << "VigoSyncEncryptor: Empty ciphertext";
    return {};
  }

  // Derive per-record key.
  uint8_t record_key[32];
  if (!key_manager_->GetRecordKey(envelope.collection, envelope.record_id,
                                   record_key)) {
    LOG(ERROR) << "VigoSyncEncryptor: Failed to derive record key for "
               << envelope.record_id;
    return {};
  }

  const char* aad = SyncDataTypeToString(envelope.collection);

  VigoCryptoBuffer plaintext = vigo_crypto_aead_open(
      record_key,
      envelope.ciphertext.data(), envelope.ciphertext.size(),
      reinterpret_cast<const uint8_t*>(aad), strlen(aad));

  // Securely zero the record key.
  volatile uint8_t* p = record_key;
  for (size_t i = 0; i < 32; ++i) {
    p[i] = 0;
  }

  if (!plaintext.data) {
    LOG(ERROR) << "VigoSyncEncryptor: AEAD open failed for "
               << envelope.record_id;
    return {};
  }

  std::vector<uint8_t> result(plaintext.data, plaintext.data + plaintext.len);
  vigo_crypto_free_buffer(plaintext);

  VLOG(2) << "VigoSyncEncryptor: Decrypted record " << envelope.record_id
          << " (" << result.size() << " bytes)";
  return result;
}

std::vector<uint8_t> VigoSyncEncryptor::EncryptWithCollectionKey(
    SyncDataType collection,
    const std::vector<uint8_t>& plaintext) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  const uint8_t* key = key_manager_->GetCollectionKey(collection);
  if (!key) {
    return {};
  }

  const char* aad = SyncDataTypeToString(collection);
  VigoCryptoBuffer sealed = vigo_crypto_aead_seal(
      key,
      plaintext.data(), plaintext.size(),
      reinterpret_cast<const uint8_t*>(aad), strlen(aad));

  if (!sealed.data) {
    return {};
  }

  std::vector<uint8_t> result(sealed.data, sealed.data + sealed.len);
  vigo_crypto_free_buffer(sealed);
  return result;
}

std::vector<uint8_t> VigoSyncEncryptor::DecryptWithCollectionKey(
    SyncDataType collection,
    const std::vector<uint8_t>& ciphertext) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  const uint8_t* key = key_manager_->GetCollectionKey(collection);
  if (!key) {
    return {};
  }

  const char* aad = SyncDataTypeToString(collection);
  VigoCryptoBuffer plaintext = vigo_crypto_aead_open(
      key,
      ciphertext.data(), ciphertext.size(),
      reinterpret_cast<const uint8_t*>(aad), strlen(aad));

  if (!plaintext.data) {
    return {};
  }

  std::vector<uint8_t> result(plaintext.data, plaintext.data + plaintext.len);
  vigo_crypto_free_buffer(plaintext);
  return result;
}

}  // namespace sync
}  // namespace vigo

// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_SYNC_VIGO_SYNC_ENCRYPTOR_H_
#define VIGO_COMPONENTS_SYNC_VIGO_SYNC_ENCRYPTOR_H_

#include <string>
#include <vector>

#include "base/sequence_checker.h"
#include "vigo/components/sync/vigo_sync_data_types.h"
#include "vigo/components/sync/vigo_sync_key_manager.h"

namespace vigo {
namespace sync {

// Encrypts and decrypts sync records using the key hierarchy.
//
// Each record is encrypted with a per-record key derived from:
//   K_collection + record_id → HKDF → K_record
//
// The encrypted envelope uses XChaCha20-Poly1305 AEAD with the
// collection name as additional authenticated data (AAD).
//
// Thread safety: all public methods on UI sequence.
class VigoSyncEncryptor {
 public:
  // |key_manager| must outlive this object.
  explicit VigoSyncEncryptor(const VigoSyncKeyManager* key_manager);
  ~VigoSyncEncryptor();

  VigoSyncEncryptor(const VigoSyncEncryptor&) = delete;
  VigoSyncEncryptor& operator=(const VigoSyncEncryptor&) = delete;

  // Encrypt a plaintext record payload into a SyncRecordEnvelope.
  // |record_id|: unique identifier for this record.
  // |collection|: which data type this belongs to.
  // |plaintext|: serialised record data (JSON bytes).
  // Returns a fully populated envelope, or an envelope with empty
  // ciphertext on failure.
  SyncRecordEnvelope EncryptRecord(const std::string& record_id,
                                    SyncDataType collection,
                                    const std::vector<uint8_t>& plaintext) const;

  // Decrypt a SyncRecordEnvelope back to plaintext.
  // Returns the decrypted payload, or empty vector on failure.
  std::vector<uint8_t> DecryptRecord(
      const SyncRecordEnvelope& envelope) const;

  // Encrypt raw bytes with a collection key (no per-record derivation).
  // Used for bulk operations where record-level keys are overkill.
  std::vector<uint8_t> EncryptWithCollectionKey(
      SyncDataType collection,
      const std::vector<uint8_t>& plaintext) const;

  // Decrypt raw bytes with a collection key.
  std::vector<uint8_t> DecryptWithCollectionKey(
      SyncDataType collection,
      const std::vector<uint8_t>& ciphertext) const;

 private:
  const VigoSyncKeyManager* key_manager_;  // Not owned.

  SEQUENCE_CHECKER(sequence_checker_);
};

}  // namespace sync
}  // namespace vigo

#endif  // VIGO_COMPONENTS_SYNC_VIGO_SYNC_ENCRYPTOR_H_

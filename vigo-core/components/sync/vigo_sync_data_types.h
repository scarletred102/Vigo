// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_SYNC_VIGO_SYNC_DATA_TYPES_H_
#define VIGO_COMPONENTS_SYNC_VIGO_SYNC_DATA_TYPES_H_

#include <cstdint>
#include <string>
#include <vector>

namespace vigo {
namespace sync {

// ─── Sync Collections ──────────────────────────────────────────────────────

// Sync data types that can be synchronised across devices.
// Each collection has its own derived encryption key from K_root.
enum class SyncDataType : uint32_t {
  kBookmarks = 0,
  kPasswords = 1,
  kHistory = 2,
  kSettings = 3,
  kOpenTabs = 4,
  kMaxValue = kOpenTabs,
};

// Returns the HKDF context string for a data type.
inline const char* SyncDataTypeToString(SyncDataType type) {
  switch (type) {
    case SyncDataType::kBookmarks:
      return "bookmarks";
    case SyncDataType::kPasswords:
      return "passwords";
    case SyncDataType::kHistory:
      return "history";
    case SyncDataType::kSettings:
      return "settings";
    case SyncDataType::kOpenTabs:
      return "open_tabs";
  }
  return "unknown";
}

// ─── Record Envelope ────────────────────────────────────────────────────────

// An encrypted sync record as stored on the server.
// The server sees only opaque ciphertext — zero-knowledge.
//
// Wire format (JSON):
// {
//   "record_id": "uuid",
//   "collection": "passwords",
//   "ciphertext": "base64...",           // AEAD sealed blob
//   "version": 1,
//   "modified_at": 1708886400000
// }
struct SyncRecordEnvelope {
  // Unique record identifier (UUID).
  std::string record_id;

  // Collection this record belongs to.
  SyncDataType collection = SyncDataType::kBookmarks;

  // AEAD-sealed ciphertext: nonce (24) || encrypted_payload || tag (16).
  std::vector<uint8_t> ciphertext;

  // Schema version for forward compatibility.
  uint32_t version = 1;

  // Last modification timestamp (milliseconds since epoch).
  int64_t modified_at_ms = 0;

  // Content hash for server-side conflict detection (blinded).
  // H = HMAC(K_collection, ciphertext)
  std::vector<uint8_t> content_hash;
};

// ─── Device Identity ────────────────────────────────────────────────────────

// A registered sync device.
struct SyncDevice {
  // Server-assigned device identifier.
  std::string device_id;

  // Human-readable device name (e.g., "Ryzen Desktop").
  std::string device_name;

  // Ed25519 verification (public) key — 32 bytes.
  std::vector<uint8_t> verify_key;

  // X25519 public key for receiving wrapped K_root — 32 bytes.
  std::vector<uint8_t> kx_public_key;

  // Registration timestamp.
  int64_t registered_at_ms = 0;

  // Last seen timestamp.
  int64_t last_seen_at_ms = 0;

  // Whether this device has been revoked.
  bool is_revoked = false;
};

// ─── Wrapped Root Key ───────────────────────────────────────────────────────

// K_root encrypted (wrapped) for a specific device.
// Each device has its own wrapped copy.
struct WrappedRootKey {
  // Device this blob is for.
  std::string device_id;

  // Ephemeral X25519 public key used for the DH wrap.
  std::vector<uint8_t> ephemeral_public_key;  // 32 bytes

  // AEAD-sealed K_root: nonce || encrypted_k_root || tag.
  std::vector<uint8_t> wrapped_data;

  // Wrapping algorithm identifier.
  std::string wrap_algorithm = "X25519-HKDF-XChaCha20Poly1305";

  // Creation timestamp.
  int64_t created_at_ms = 0;

  // Version for rotation tracking.
  uint32_t version = 1;
};

// ─── Conflict Resolution ────────────────────────────────────────────────────

// Conflict resolution strategy per collection.
enum class ConflictStrategy {
  // CRDT merge: both changes preserved, structural merge.
  // Used for: bookmarks (tree structure).
  kCRDT,

  // Last-Writer-Wins: most recent modification timestamp wins.
  // Used for: settings, passwords.
  kLastWriterWins,

  // Append-only: both records kept, no deduplication.
  // Used for: history.
  kAppendOnly,
};

// Returns the conflict strategy for a data type.
inline ConflictStrategy GetConflictStrategy(SyncDataType type) {
  switch (type) {
    case SyncDataType::kBookmarks:
      return ConflictStrategy::kCRDT;
    case SyncDataType::kPasswords:
    case SyncDataType::kSettings:
    case SyncDataType::kOpenTabs:
      return ConflictStrategy::kLastWriterWins;
    case SyncDataType::kHistory:
      return ConflictStrategy::kAppendOnly;
  }
  return ConflictStrategy::kLastWriterWins;
}

// ─── Sync State ─────────────────────────────────────────────────────────────

// Connection state with the self-hosted sync server.
enum class SyncState {
  kDisconnected,
  kConnecting,
  kConnected,
  kSyncing,
  kError,
};

}  // namespace sync
}  // namespace vigo

#endif  // VIGO_COMPONENTS_SYNC_VIGO_SYNC_DATA_TYPES_H_

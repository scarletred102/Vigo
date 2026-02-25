// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_SYNC_VIGO_BOOKMARK_CRDT_H_
#define VIGO_COMPONENTS_SYNC_VIGO_BOOKMARK_CRDT_H_

#include <string>
#include <unordered_map>
#include <unordered_set>
#include <vector>

#include "base/sequence_checker.h"

namespace vigo {
namespace sync {

// ─── Bookmark CRDT Types ────────────────────────────────────────────────────

// A CRDT-based bookmark node for conflict-free merging across devices.
//
// Design: state-based grow-only set CRDT with LWW fields.
//   - Each bookmark has a stable UUID that survives moves/renames.
//   - Parent-child relationships use a "parent_id" pointer.
//   - Moves/renames use LWW (last modified timestamp wins).
//   - Deletes are tombstones (is_deleted + deleted_at timestamp).
//   - Position within a folder uses a fractional index string for
//     conflict-free reordering (similar to Figma's approach).
//
// The CRDT layer operates on *decrypted* bookmark data — it receives
// plaintext bookmark JSON from the sync encryptor and produces merged
// plaintext output.
struct CRDTBookmarkNode {
  // Stable UUID for this bookmark.
  std::string id;

  // Parent folder UUID. Empty for root node.
  std::string parent_id;

  // Fractional position index within the parent folder.
  // Uses lexicographic ordering (e.g., "a0", "a1", "b0").
  std::string position;

  // Bookmark title.
  std::string title;

  // URL (empty for folders).
  std::string url;

  // Whether this is a folder.
  bool is_folder = false;

  // Tombstone: deleted nodes are kept for conflict resolution.
  bool is_deleted = false;

  // Device that last modified this node.
  std::string last_modified_by;

  // Modification timestamps (LWW tiebreaker for fields).
  int64_t title_modified_at = 0;
  int64_t url_modified_at = 0;
  int64_t parent_modified_at = 0;   // Move timestamp.
  int64_t position_modified_at = 0;
  int64_t deleted_at = 0;
};

// Result of a CRDT merge operation.
struct CRDTMergeResult {
  // Merged bookmark tree (all nodes, including tombstones).
  std::vector<CRDTBookmarkNode> merged_nodes;

  // Number of conflicts resolved.
  int conflicts_resolved = 0;

  // IDs of nodes that were updated by the merge.
  std::unordered_set<std::string> updated_ids;

  // Whether any cycle was detected and broken.
  bool cycle_detected = false;
};

// ─── Bookmark CRDT Merger ───────────────────────────────────────────────────

// Merges bookmark trees from two devices using CRDT semantics.
//
// Invariants maintained:
//   1. Every node has exactly one parent (tree structure).
//   2. No cycles in the parent chain.
//   3. Tombstoned nodes are retained for merge but excluded from UI.
//   4. Position ordering is consistent within each folder.
//
// Thread safety: all public methods on UI sequence.
class VigoBookmarkCRDT {
 public:
  VigoBookmarkCRDT();
  ~VigoBookmarkCRDT();

  VigoBookmarkCRDT(const VigoBookmarkCRDT&) = delete;
  VigoBookmarkCRDT& operator=(const VigoBookmarkCRDT&) = delete;

  // Merge two bookmark trees.
  // |local_nodes|: the current device's bookmark tree.
  // |remote_nodes|: the incoming tree from another device.
  // Returns the merged result.
  CRDTMergeResult Merge(
      const std::vector<CRDTBookmarkNode>& local_nodes,
      const std::vector<CRDTBookmarkNode>& remote_nodes);

  // Generate a fractional position between two existing positions.
  // Used when inserting a bookmark between two existing ones.
  static std::string GeneratePositionBetween(
      const std::string& before,
      const std::string& after);

 private:
  // Merge a single node pair (local vs remote).
  CRDTBookmarkNode MergeNodes(const CRDTBookmarkNode& local,
                               const CRDTBookmarkNode& remote);

  // Detect and break cycles in the parent chain.
  bool DetectAndBreakCycles(
      std::unordered_map<std::string, CRDTBookmarkNode>& nodes);

  // Get the ancestry chain for a node (up to root).
  std::vector<std::string> GetAncestors(
      const std::unordered_map<std::string, CRDTBookmarkNode>& nodes,
      const std::string& node_id);

  SEQUENCE_CHECKER(sequence_checker_);
};

}  // namespace sync
}  // namespace vigo

#endif  // VIGO_COMPONENTS_SYNC_VIGO_BOOKMARK_CRDT_H_

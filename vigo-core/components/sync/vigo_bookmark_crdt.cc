// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/sync/vigo_bookmark_crdt.h"

#include <algorithm>
#include <sstream>

#include "base/logging.h"

namespace vigo {
namespace sync {

VigoBookmarkCRDT::VigoBookmarkCRDT() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

VigoBookmarkCRDT::~VigoBookmarkCRDT() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

CRDTMergeResult VigoBookmarkCRDT::Merge(
    const std::vector<CRDTBookmarkNode>& local_nodes,
    const std::vector<CRDTBookmarkNode>& remote_nodes) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  CRDTMergeResult result;

  // Build lookup maps by ID.
  std::unordered_map<std::string, CRDTBookmarkNode> merged;

  // Step 1: Insert all local nodes.
  for (const auto& node : local_nodes) {
    merged[node.id] = node;
  }

  // Step 2: Merge remote nodes.
  for (const auto& remote : remote_nodes) {
    auto it = merged.find(remote.id);
    if (it == merged.end()) {
      // New node from remote — add directly.
      merged[remote.id] = remote;
      result.updated_ids.insert(remote.id);
    } else {
      // Conflict: merge using LWW per field.
      auto merged_node = MergeNodes(it->second, remote);
      if (merged_node.title != it->second.title ||
          merged_node.url != it->second.url ||
          merged_node.parent_id != it->second.parent_id ||
          merged_node.position != it->second.position ||
          merged_node.is_deleted != it->second.is_deleted) {
        result.updated_ids.insert(remote.id);
        ++result.conflicts_resolved;
      }
      merged[remote.id] = merged_node;
    }
  }

  // Step 3: Detect and break cycles.
  result.cycle_detected = DetectAndBreakCycles(merged);

  // Step 4: Flatten to output vector.
  result.merged_nodes.reserve(merged.size());
  for (auto& [id, node] : merged) {
    result.merged_nodes.push_back(std::move(node));
  }

  // Sort by parent_id then position for deterministic output.
  std::sort(result.merged_nodes.begin(), result.merged_nodes.end(),
            [](const CRDTBookmarkNode& a, const CRDTBookmarkNode& b) {
              if (a.parent_id != b.parent_id) {
                return a.parent_id < b.parent_id;
              }
              return a.position < b.position;
            });

  VLOG(1) << "VigoBookmarkCRDT: Merged " << local_nodes.size()
          << " local + " << remote_nodes.size()
          << " remote → " << result.merged_nodes.size()
          << " nodes (" << result.conflicts_resolved << " conflicts)";

  return result;
}

// static
std::string VigoBookmarkCRDT::GeneratePositionBetween(
    const std::string& before,
    const std::string& after) {
  // Simple fractional indexing: find the midpoint between two strings.
  // For production, this would use a proper fractional index library
  // (e.g., Figma-style). For now, simple ASCII midpoint.

  if (before.empty() && after.empty()) {
    return "m";  // Midpoint of alphabet.
  }

  if (before.empty()) {
    // Insert before |after|.
    if (after.size() == 1 && after[0] <= 'b') {
      return std::string(1, 'a') + "m";
    }
    char mid = static_cast<char>('a' + (after[0] - 'a') / 2);
    return std::string(1, mid);
  }

  if (after.empty()) {
    // Insert after |before|.
    if (before[0] >= 'y') {
      return before + "m";
    }
    char mid = static_cast<char>(before[0] + (('z' - before[0]) / 2) + 1);
    return std::string(1, mid);
  }

  // Both non-empty: find midpoint.
  if (before[0] + 1 < after[0]) {
    char mid = static_cast<char>(before[0] + (after[0] - before[0]) / 2);
    return std::string(1, mid);
  }

  // Same first character or adjacent — append to go deeper.
  return before + "m";
}

CRDTBookmarkNode VigoBookmarkCRDT::MergeNodes(
    const CRDTBookmarkNode& local,
    const CRDTBookmarkNode& remote) {
  CRDTBookmarkNode result = local;

  // LWW per field: pick the version with the later timestamp.

  // Title.
  if (remote.title_modified_at > local.title_modified_at) {
    result.title = remote.title;
    result.title_modified_at = remote.title_modified_at;
  }

  // URL.
  if (remote.url_modified_at > local.url_modified_at) {
    result.url = remote.url;
    result.url_modified_at = remote.url_modified_at;
  }

  // Parent (move).
  if (remote.parent_modified_at > local.parent_modified_at) {
    result.parent_id = remote.parent_id;
    result.parent_modified_at = remote.parent_modified_at;
  }

  // Position within folder.
  if (remote.position_modified_at > local.position_modified_at) {
    result.position = remote.position;
    result.position_modified_at = remote.position_modified_at;
  }

  // Deletion: a delete is permanent (tombstone wins if newer).
  if (remote.is_deleted && remote.deleted_at > local.deleted_at) {
    result.is_deleted = true;
    result.deleted_at = remote.deleted_at;
  }
  // Resurrection: if local un-deleted after remote deleted.
  if (!remote.is_deleted && local.is_deleted &&
      remote.title_modified_at > local.deleted_at) {
    result.is_deleted = false;
    result.deleted_at = 0;
  }

  return result;
}

bool VigoBookmarkCRDT::DetectAndBreakCycles(
    std::unordered_map<std::string, CRDTBookmarkNode>& nodes) {
  bool any_cycle = false;

  for (auto& [id, node] : nodes) {
    if (node.parent_id.empty() || node.is_deleted) {
      continue;
    }

    // Walk up the parent chain; if we see ourselves, there's a cycle.
    std::unordered_set<std::string> visited;
    std::string current = id;

    while (!current.empty()) {
      if (visited.count(current)) {
        // Cycle detected — break by moving this node to root.
        LOG(WARNING) << "VigoBookmarkCRDT: Cycle detected at node "
                     << id << " — reparenting to root";
        node.parent_id.clear();
        any_cycle = true;
        break;
      }

      visited.insert(current);
      auto it = nodes.find(current);
      if (it == nodes.end()) {
        break;
      }
      current = it->second.parent_id;
    }
  }

  return any_cycle;
}

std::vector<std::string> VigoBookmarkCRDT::GetAncestors(
    const std::unordered_map<std::string, CRDTBookmarkNode>& nodes,
    const std::string& node_id) {
  std::vector<std::string> ancestors;
  std::string current = node_id;
  std::unordered_set<std::string> seen;

  while (!current.empty()) {
    if (seen.count(current)) {
      break;  // Cycle — stop.
    }
    seen.insert(current);
    ancestors.push_back(current);

    auto it = nodes.find(current);
    if (it == nodes.end()) {
      break;
    }
    current = it->second.parent_id;
  }

  return ancestors;
}

}  // namespace sync
}  // namespace vigo

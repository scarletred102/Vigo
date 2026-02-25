// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/sync/vigo_bookmark_crdt.h"

#include "testing/gtest/include/gtest/gtest.h"

namespace vigo {
namespace sync {

class VigoBookmarkCRDTTest : public ::testing::Test {
 protected:
  VigoBookmarkCRDT crdt_;
};

TEST_F(VigoBookmarkCRDTTest, MergeEmptyTrees) {
  auto result = crdt_.Merge({}, {});
  EXPECT_TRUE(result.merged_nodes.empty());
  EXPECT_EQ(result.conflicts_resolved, 0);
}

TEST_F(VigoBookmarkCRDTTest, MergeLocalOnlyNoConflict) {
  CRDTBookmarkNode node;
  node.id = "bm-1";
  node.title = "Example";
  node.url = "https://example.com";
  node.title_modified_at = 100;

  auto result = crdt_.Merge({node}, {});
  ASSERT_EQ(result.merged_nodes.size(), 1u);
  EXPECT_EQ(result.merged_nodes[0].title, "Example");
  EXPECT_EQ(result.conflicts_resolved, 0);
}

TEST_F(VigoBookmarkCRDTTest, MergeRemoteOnlyNoConflict) {
  CRDTBookmarkNode node;
  node.id = "bm-2";
  node.title = "Remote";
  node.url = "https://remote.com";
  node.title_modified_at = 200;

  auto result = crdt_.Merge({}, {node});
  ASSERT_EQ(result.merged_nodes.size(), 1u);
  EXPECT_EQ(result.merged_nodes[0].title, "Remote");
  EXPECT_TRUE(result.updated_ids.count("bm-2"));
}

TEST_F(VigoBookmarkCRDTTest, MergeNoOverlap) {
  CRDTBookmarkNode local;
  local.id = "local-1";
  local.title = "Local";
  local.title_modified_at = 100;

  CRDTBookmarkNode remote;
  remote.id = "remote-1";
  remote.title = "Remote";
  remote.title_modified_at = 200;

  auto result = crdt_.Merge({local}, {remote});
  EXPECT_EQ(result.merged_nodes.size(), 2u);
  EXPECT_EQ(result.conflicts_resolved, 0);
}

TEST_F(VigoBookmarkCRDTTest, TitleConflictLWW) {
  CRDTBookmarkNode local;
  local.id = "bm-1";
  local.title = "Old Title";
  local.url = "https://example.com";
  local.title_modified_at = 100;
  local.url_modified_at = 100;

  CRDTBookmarkNode remote;
  remote.id = "bm-1";
  remote.title = "New Title";
  remote.url = "https://example.com";
  remote.title_modified_at = 200;
  remote.url_modified_at = 100;

  auto result = crdt_.Merge({local}, {remote});
  ASSERT_EQ(result.merged_nodes.size(), 1u);
  EXPECT_EQ(result.merged_nodes[0].title, "New Title");
  EXPECT_EQ(result.conflicts_resolved, 1);
}

TEST_F(VigoBookmarkCRDTTest, MoveConflictLWW) {
  CRDTBookmarkNode local;
  local.id = "bm-1";
  local.title = "Moved";
  local.parent_id = "folder-A";
  local.parent_modified_at = 100;
  local.title_modified_at = 50;

  CRDTBookmarkNode remote;
  remote.id = "bm-1";
  remote.title = "Moved";
  remote.parent_id = "folder-B";
  remote.parent_modified_at = 200;
  remote.title_modified_at = 50;

  auto result = crdt_.Merge({local}, {remote});
  ASSERT_EQ(result.merged_nodes.size(), 1u);
  EXPECT_EQ(result.merged_nodes[0].parent_id, "folder-B");
}

TEST_F(VigoBookmarkCRDTTest, DeleteTombstoneWins) {
  CRDTBookmarkNode local;
  local.id = "bm-1";
  local.title = "Alive";
  local.is_deleted = false;
  local.title_modified_at = 100;
  local.deleted_at = 0;

  CRDTBookmarkNode remote;
  remote.id = "bm-1";
  remote.title = "Alive";
  remote.is_deleted = true;
  remote.deleted_at = 200;
  remote.title_modified_at = 100;

  auto result = crdt_.Merge({local}, {remote});
  ASSERT_EQ(result.merged_nodes.size(), 1u);
  EXPECT_TRUE(result.merged_nodes[0].is_deleted);
}

TEST_F(VigoBookmarkCRDTTest, ResurrectionAfterDelete) {
  CRDTBookmarkNode local;
  local.id = "bm-1";
  local.title = "Deleted";
  local.is_deleted = true;
  local.deleted_at = 100;
  local.title_modified_at = 50;

  CRDTBookmarkNode remote;
  remote.id = "bm-1";
  remote.title = "Resurrected";
  remote.is_deleted = false;
  remote.deleted_at = 0;
  remote.title_modified_at = 200;  // Modified after local deletion.

  auto result = crdt_.Merge({local}, {remote});
  ASSERT_EQ(result.merged_nodes.size(), 1u);
  EXPECT_FALSE(result.merged_nodes[0].is_deleted);
  EXPECT_EQ(result.merged_nodes[0].title, "Resurrected");
}

TEST_F(VigoBookmarkCRDTTest, CycleDetection) {
  // Create a cycle: A → B → A.
  CRDTBookmarkNode nodeA;
  nodeA.id = "A";
  nodeA.parent_id = "B";
  nodeA.title = "Node A";
  nodeA.title_modified_at = 100;
  nodeA.parent_modified_at = 100;

  CRDTBookmarkNode nodeB;
  nodeB.id = "B";
  nodeB.parent_id = "A";
  nodeB.title = "Node B";
  nodeB.title_modified_at = 100;
  nodeB.parent_modified_at = 100;

  auto result = crdt_.Merge({nodeA, nodeB}, {});
  EXPECT_TRUE(result.cycle_detected);

  // At least one node should have been reparented to root.
  bool any_root = false;
  for (const auto& node : result.merged_nodes) {
    if (node.parent_id.empty()) {
      any_root = true;
    }
  }
  EXPECT_TRUE(any_root);
}

TEST_F(VigoBookmarkCRDTTest, PositionGeneration) {
  // Between empty: midpoint.
  auto mid = VigoBookmarkCRDT::GeneratePositionBetween("", "");
  EXPECT_EQ(mid, "m");

  // Before "m": something less.
  auto before = VigoBookmarkCRDT::GeneratePositionBetween("", "m");
  EXPECT_LT(before, "m");

  // After "m": something greater.
  auto after = VigoBookmarkCRDT::GeneratePositionBetween("m", "");
  EXPECT_GT(after, "m");

  // Between "a" and "c": should be "b".
  auto between = VigoBookmarkCRDT::GeneratePositionBetween("a", "c");
  EXPECT_GT(between, "a");
  EXPECT_LT(between, "c");
}

TEST_F(VigoBookmarkCRDTTest, SortedOutputByParentAndPosition) {
  CRDTBookmarkNode node1;
  node1.id = "bm-1";
  node1.parent_id = "folder-1";
  node1.position = "b";
  node1.title_modified_at = 100;

  CRDTBookmarkNode node2;
  node2.id = "bm-2";
  node2.parent_id = "folder-1";
  node2.position = "a";
  node2.title_modified_at = 100;

  auto result = crdt_.Merge({node1, node2}, {});
  ASSERT_EQ(result.merged_nodes.size(), 2u);
  // Should be sorted: position "a" before "b".
  EXPECT_EQ(result.merged_nodes[0].position, "a");
  EXPECT_EQ(result.merged_nodes[1].position, "b");
}

}  // namespace sync
}  // namespace vigo

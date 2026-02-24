// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/adblock/vigo_filter_list_manager.h"

#include "base/files/file_path.h"
#include "base/files/scoped_temp_dir.h"
#include "testing/gtest/include/gtest/gtest.h"

namespace vigo {
namespace adblock {
namespace {

class VigoFilterListManagerTest : public ::testing::Test {
 protected:
  void SetUp() override {
    ASSERT_TRUE(temp_dir_.CreateUniqueTempDir());
    manager_.Init(temp_dir_.GetPath());
  }

  base::ScopedTempDir temp_dir_;
  VigoFilterListManager manager_;
};

TEST_F(VigoFilterListManagerTest, DefaultListsAreRegistered) {
  const auto& lists = manager_.GetFilterLists();
  ASSERT_EQ(lists.size(), 2u);
  EXPECT_EQ(lists[0].id, "easylist");
  EXPECT_EQ(lists[0].title, "EasyList");
  EXPECT_TRUE(lists[0].enabled);
  EXPECT_TRUE(lists[0].builtin);
  EXPECT_EQ(lists[1].id, "easyprivacy");
  EXPECT_EQ(lists[1].title, "EasyPrivacy");
  EXPECT_TRUE(lists[1].enabled);
  EXPECT_TRUE(lists[1].builtin);
}

TEST_F(VigoFilterListManagerTest, GetEnabledFilterListPaths) {
  auto paths = manager_.GetEnabledFilterListPaths();
  EXPECT_EQ(paths.size(), 2u);
}

TEST_F(VigoFilterListManagerTest, DisableListReducesPaths) {
  manager_.SetListEnabled("easylist", false);
  auto paths = manager_.GetEnabledFilterListPaths();
  EXPECT_EQ(paths.size(), 1u);
}

TEST_F(VigoFilterListManagerTest, AddCustomList) {
  manager_.AddCustomList("My Custom List", "https://example.com/list.txt");
  const auto& lists = manager_.GetFilterLists();
  EXPECT_EQ(lists.size(), 3u);
  EXPECT_EQ(lists[2].title, "My Custom List");
  EXPECT_FALSE(lists[2].builtin);
  EXPECT_TRUE(lists[2].enabled);
}

TEST_F(VigoFilterListManagerTest, RemoveCustomListSucceeds) {
  manager_.AddCustomList("Test List", "https://example.com/test.txt");
  EXPECT_TRUE(manager_.RemoveCustomList("test_list"));
  EXPECT_EQ(manager_.GetFilterLists().size(), 2u);
}

TEST_F(VigoFilterListManagerTest, CannotRemoveBuiltinList) {
  EXPECT_FALSE(manager_.RemoveCustomList("easylist"));
  EXPECT_EQ(manager_.GetFilterLists().size(), 2u);
}

TEST_F(VigoFilterListManagerTest, RemoveNonexistentListReturnsFalse) {
  EXPECT_FALSE(manager_.RemoveCustomList("nonexistent"));
}

TEST_F(VigoFilterListManagerTest, SetListEnabledToggle) {
  manager_.SetListEnabled("easylist", false);
  const auto& lists = manager_.GetFilterLists();
  EXPECT_FALSE(lists[0].enabled);

  manager_.SetListEnabled("easylist", true);
  EXPECT_TRUE(lists[0].enabled);
}

TEST_F(VigoFilterListManagerTest, AutoUpdateCanBeStartedAndStopped) {
  // Should not crash.
  manager_.StartAutoUpdate();
  manager_.StopAutoUpdate();
}

}  // namespace
}  // namespace adblock
}  // namespace vigo

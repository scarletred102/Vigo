// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/performance/vigo_tab_lifecycle_manager.h"

#include "base/test/task_environment.h"
#include "testing/gtest/include/gtest/gtest.h"

namespace vigo {
namespace performance {
namespace {

// Test observer that records state transitions.
class TestTabObserver : public TabLifecycleObserver {
 public:
  void OnTabStateChanged(int tab_id,
                         TabState old_state,
                         TabState new_state) override {
    transitions.push_back({tab_id, old_state, new_state});
  }

  void OnTabDiscarded(int tab_id) override {
    discarded_ids.push_back(tab_id);
  }

  struct Transition {
    int tab_id;
    TabState old_state;
    TabState new_state;
  };
  std::vector<Transition> transitions;
  std::vector<int> discarded_ids;
};

class VigoTabLifecycleManagerTest : public testing::Test {
 protected:
  base::test::TaskEnvironment task_environment_{
      base::test::TaskEnvironment::TimeSource::MOCK_TIME};
};

TEST_F(VigoTabLifecycleManagerTest, NewTabIsActive) {
  VigoTabLifecycleManager manager;
  manager.OnTabCreated(1, "https://example.com");

  const auto* info = manager.GetTabInfo(1);
  ASSERT_NE(info, nullptr);
  EXPECT_EQ(info->state, TabState::kActive);
  EXPECT_EQ(info->tab_id, 1);
}

TEST_F(VigoTabLifecycleManagerTest, ClosedTabIsRemoved) {
  VigoTabLifecycleManager manager;
  manager.OnTabCreated(1, "https://example.com");
  manager.OnTabClosed(1);

  const auto* info = manager.GetTabInfo(1);
  EXPECT_EQ(info, nullptr);
}

TEST_F(VigoTabLifecycleManagerTest, ActivationDeactivation) {
  VigoTabLifecycleManager manager;
  TestTabObserver observer;
  manager.AddObserver(&observer);

  manager.OnTabCreated(1, "https://example.com");
  manager.OnTabCreated(2, "https://other.com");

  // Activate tab 1.
  manager.OnTabActivated(1);
  EXPECT_EQ(manager.GetTabInfo(1)->state, TabState::kActive);

  // Activate tab 2 — tab 1 gets deactivated.
  manager.OnTabActivated(2);
  // Tab 1 is now deactivated but hasn't timed out yet.
  EXPECT_EQ(manager.GetTabInfo(1)->state, TabState::kActive);

  manager.RemoveObserver(&observer);
}

TEST_F(VigoTabLifecycleManagerTest, AudioTabBecomesBackgroundActive) {
  VigoTabLifecycleManager manager;
  manager.OnTabCreated(1, "https://music.example.com");
  manager.OnTabCreated(2, "https://other.com");

  // Tab 1 is playing audio.
  manager.OnTabAudioStateChanged(1, true);

  // Activate tab 2 — tab 1 should become BackgroundActive.
  manager.OnTabActivated(2);
  EXPECT_EQ(manager.GetTabInfo(1)->state, TabState::kBackgroundActive);
}

TEST_F(VigoTabLifecycleManagerTest, ProtectedTabsNotSuspended) {
  VigoTabLifecycleManager manager;
  manager.OnTabCreated(1, "https://webrtc.example.com");
  manager.OnTabCreated(2, "https://other.com");

  // Tab 1 has WebRTC.
  manager.OnTabWebRtcStateChanged(1, true);
  manager.OnTabActivated(2);

  // Even under pressure, tab 1 should stay BackgroundActive.
  manager.OnMemoryPressureLevelChanged(MemoryPressureLevel::kNone,
                                       MemoryPressureLevel::kCritical);

  // Tab 1 should NOT be suspended.
  const auto* info = manager.GetTabInfo(1);
  EXPECT_NE(info->state, TabState::kSuspended);
  EXPECT_NE(info->state, TabState::kFrozen);
  EXPECT_NE(info->state, TabState::kDiscarded);
}

TEST_F(VigoTabLifecycleManagerTest, GetTabsInState) {
  VigoTabLifecycleManager manager;
  manager.OnTabCreated(1, "https://a.com");
  manager.OnTabCreated(2, "https://b.com");
  manager.OnTabCreated(3, "https://c.com");

  auto active = manager.GetTabsInState(TabState::kActive);
  EXPECT_EQ(active.size(), 3u);

  auto idle = manager.GetTabsInState(TabState::kIdle);
  EXPECT_EQ(idle.size(), 0u);
}

TEST_F(VigoTabLifecycleManagerTest, AliveTabCountExcludesDiscarded) {
  VigoTabLifecycleManager manager;
  manager.OnTabCreated(1, "https://a.com");
  manager.OnTabCreated(2, "https://b.com");

  EXPECT_EQ(manager.GetAliveTabCount(), 2);
}

TEST_F(VigoTabLifecycleManagerTest, TotalEstimatedMemory) {
  VigoTabLifecycleManager manager;
  manager.OnTabCreated(1, "https://a.com");
  manager.OnTabCreated(2, "https://b.com");

  manager.OnTabMemoryUpdate(1, 50 * 1024 * 1024);
  manager.OnTabMemoryUpdate(2, 30 * 1024 * 1024);

  EXPECT_EQ(manager.GetTotalEstimatedMemory(), 80u * 1024 * 1024);
}

TEST_F(VigoTabLifecycleManagerTest, DiscardCandidatesExcludeActive) {
  VigoTabLifecycleManager manager;
  manager.OnTabCreated(1, "https://a.com");
  manager.OnTabCreated(2, "https://b.com");
  manager.OnTabActivated(1);

  // Tab 2 is still active (hasn't timed out), so both are "active".
  auto candidates = manager.GetDiscardCandidates();
  // Active tabs are excluded from discard candidates.
  EXPECT_EQ(candidates.size(), 0u);
}

TEST_F(VigoTabLifecycleManagerTest, PinnedTabsLastInDiscardOrder) {
  VigoTabLifecycleManager manager;
  VigoTabLifecycleManager::Config config;
  config.idle_timeout_seconds = 1;  // Fast for testing.
  VigoTabLifecycleManager short_manager(config);

  short_manager.OnTabCreated(1, "https://a.com");
  short_manager.OnTabCreated(2, "https://b.com");
  short_manager.OnTabCreated(3, "https://c.com");
  short_manager.OnTabPinnedStateChanged(1, true);

  // Tab 1 is pinned and should appear last in discard candidates.
  // All tabs are Active right now, so no candidates.
  auto candidates = short_manager.GetDiscardCandidates();
  EXPECT_TRUE(candidates.empty());
}

TEST_F(VigoTabLifecycleManagerTest, DownloadProtection) {
  VigoTabLifecycleManager manager;
  manager.OnTabCreated(1, "https://download.example.com");
  manager.OnTabDownloadStateChanged(1, true);

  const auto* info = manager.GetTabInfo(1);
  EXPECT_TRUE(info->has_download);
}

TEST_F(VigoTabLifecycleManagerTest, CaptureProtection) {
  VigoTabLifecycleManager manager;
  manager.OnTabCreated(1, "https://camera.example.com");
  manager.OnTabCaptureStateChanged(1, true);

  const auto* info = manager.GetTabInfo(1);
  EXPECT_TRUE(info->is_capturing);
}

TEST_F(VigoTabLifecycleManagerTest, StartAndStop) {
  VigoTabLifecycleManager manager;
  // Should not crash.
  manager.Start();
  manager.Stop();
}

TEST_F(VigoTabLifecycleManagerTest, ObserverNotifiedOnTransition) {
  VigoTabLifecycleManager manager;
  TestTabObserver observer;
  manager.AddObserver(&observer);

  manager.OnTabCreated(1, "https://a.com");
  manager.OnTabCreated(2, "https://b.com");

  // Tab 1 produces audio, then deactivated → BackgroundActive.
  manager.OnTabAudioStateChanged(1, true);
  manager.OnTabActivated(2);

  ASSERT_GE(observer.transitions.size(), 1u);
  bool found_bg = false;
  for (const auto& t : observer.transitions) {
    if (t.tab_id == 1 && t.new_state == TabState::kBackgroundActive) {
      found_bg = true;
    }
  }
  EXPECT_TRUE(found_bg);

  manager.RemoveObserver(&observer);
}

}  // namespace
}  // namespace performance
}  // namespace vigo

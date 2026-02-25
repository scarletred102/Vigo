// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/performance/vigo_memory_budget_controller.h"

#include "base/test/task_environment.h"
#include "testing/gtest/include/gtest/gtest.h"

namespace vigo {
namespace performance {
namespace {

// Test observer that records pressure level changes.
class TestBudgetObserver : public MemoryBudgetObserver {
 public:
  void OnMemoryPressureLevelChanged(MemoryPressureLevel old_level,
                                    MemoryPressureLevel new_level) override {
    last_old_level = old_level;
    last_new_level = new_level;
    change_count++;
  }

  void OnMemoryBudgetSnapshot(const MemoryBudgetSnapshot& snapshot) override {
    last_snapshot = snapshot;
    snapshot_count++;
  }

  MemoryPressureLevel last_old_level = MemoryPressureLevel::kNone;
  MemoryPressureLevel last_new_level = MemoryPressureLevel::kNone;
  int change_count = 0;
  MemoryBudgetSnapshot last_snapshot;
  int snapshot_count = 0;
};

class VigoMemoryBudgetControllerTest : public testing::Test {
 protected:
  base::test::TaskEnvironment task_environment_{
      base::test::TaskEnvironment::TimeSource::MOCK_TIME};
};

TEST_F(VigoMemoryBudgetControllerTest, DefaultBudgetIsAutoDetected) {
  VigoMemoryBudgetController controller;
  // Budget should be non-zero (auto-detected from system RAM).
  EXPECT_GT(controller.current_budget_limit(), 0u);
}

TEST_F(VigoMemoryBudgetControllerTest, CustomBudgetIsRespected) {
  VigoMemoryBudgetController::Config config;
  config.base_budget_bytes = 500 * 1024 * 1024;  // 500 MB
  VigoMemoryBudgetController controller(config);
  EXPECT_EQ(controller.current_budget_limit(), 500u * 1024 * 1024);
}

TEST_F(VigoMemoryBudgetControllerTest, BudgetScalesWithTabs) {
  VigoMemoryBudgetController::Config config;
  config.base_budget_bytes = 400 * 1024 * 1024;  // 400 MB
  config.per_extra_tab_bytes = 30 * 1024 * 1024;  // 30 MB per extra tab
  VigoMemoryBudgetController controller(config);

  // 10 tabs = base budget.
  controller.OnTabCountChanged(10);
  EXPECT_EQ(controller.current_budget_limit(), 400u * 1024 * 1024);

  // 15 tabs = base + 5 * 30 MB = 550 MB.
  controller.OnTabCountChanged(15);
  EXPECT_EQ(controller.current_budget_limit(),
            400u * 1024 * 1024 + 5 * 30u * 1024 * 1024);

  // 5 tabs = still base (no negative extra).
  controller.OnTabCountChanged(5);
  EXPECT_EQ(controller.current_budget_limit(), 400u * 1024 * 1024);
}

TEST_F(VigoMemoryBudgetControllerTest, DefaultPressureIsNone) {
  VigoMemoryBudgetController controller;
  EXPECT_EQ(controller.current_pressure_level(), MemoryPressureLevel::kNone);
}

TEST_F(VigoMemoryBudgetControllerTest, StartAndStopWork) {
  VigoMemoryBudgetController controller;
  EXPECT_FALSE(controller.is_running());
  controller.Start();
  EXPECT_TRUE(controller.is_running());
  controller.Stop();
  EXPECT_FALSE(controller.is_running());
}

TEST_F(VigoMemoryBudgetControllerTest, ObserverReceivesSnapshots) {
  VigoMemoryBudgetController controller;
  TestBudgetObserver observer;
  controller.AddObserver(&observer);

  controller.SampleNow();
  EXPECT_EQ(observer.snapshot_count, 1);
  EXPECT_GT(observer.last_snapshot.total_system_memory, 0u);

  controller.RemoveObserver(&observer);
}

TEST_F(VigoMemoryBudgetControllerTest, SampleNowProducesSnapshot) {
  VigoMemoryBudgetController controller;
  controller.SampleNow();

  const auto& snap = controller.last_snapshot();
  EXPECT_GT(snap.total_system_memory, 0u);
  // RSS should be non-zero for the current process.
  EXPECT_GT(snap.current_browser_rss, 0u);
}

TEST_F(VigoMemoryBudgetControllerTest, MinTabCountClampedToOne) {
  VigoMemoryBudgetController::Config config;
  config.base_budget_bytes = 400 * 1024 * 1024;
  VigoMemoryBudgetController controller(config);

  controller.OnTabCountChanged(0);
  // Should be clamped internally; budget should be base.
  EXPECT_EQ(controller.current_budget_limit(), 400u * 1024 * 1024);

  controller.OnTabCountChanged(-5);
  EXPECT_EQ(controller.current_budget_limit(), 400u * 1024 * 1024);
}

TEST_F(VigoMemoryBudgetControllerTest, ThresholdConfig) {
  VigoMemoryBudgetController::Config config;
  config.moderate_threshold = 0.50;
  config.critical_threshold = 0.80;
  config.base_budget_bytes = 100 * 1024 * 1024;
  VigoMemoryBudgetController controller(config);

  // Just verify configuration was accepted.
  EXPECT_EQ(controller.current_budget_limit(), 100u * 1024 * 1024);
}

TEST_F(VigoMemoryBudgetControllerTest, MultipleObserversGetNotified) {
  VigoMemoryBudgetController controller;
  TestBudgetObserver obs1, obs2;
  controller.AddObserver(&obs1);
  controller.AddObserver(&obs2);

  controller.SampleNow();
  EXPECT_EQ(obs1.snapshot_count, 1);
  EXPECT_EQ(obs2.snapshot_count, 1);

  controller.RemoveObserver(&obs1);
  controller.SampleNow();
  EXPECT_EQ(obs1.snapshot_count, 1);  // No longer observing.
  EXPECT_EQ(obs2.snapshot_count, 2);

  controller.RemoveObserver(&obs2);
}

}  // namespace
}  // namespace performance
}  // namespace vigo

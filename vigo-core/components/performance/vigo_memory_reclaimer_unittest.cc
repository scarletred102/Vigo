// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/performance/vigo_memory_reclaimer.h"

#include "base/test/task_environment.h"
#include "testing/gtest/include/gtest/gtest.h"
#include "vigo/components/performance/vigo_process_manager.h"

namespace vigo {
namespace performance {
namespace {

// Mock process manager for reclaimer tests.
class MockReclaimerProcessManager : public VigoProcessManager {
 public:
  ProcessSnapshot SampleProcess(base::ProcessId pid,
                                ProcessType type) const override {
    ProcessSnapshot snap;
    snap.pid = pid;
    snap.type = type;
    snap.working_set_bytes = 80 * 1024 * 1024;  // 80 MB
    snap.private_bytes = 60 * 1024 * 1024;
    snap.cpu_usage = 3.0;
    snap.sample_time = base::TimeTicks::Now();
    return snap;
  }
};

class VigoMemoryReclaimerTest : public testing::Test {
 protected:
  base::test::TaskEnvironment task_environment_{
      base::test::TaskEnvironment::TimeSource::MOCK_TIME};
};

TEST_F(VigoMemoryReclaimerTest, ReclaimNowDoesNotCrashWithoutManager) {
  VigoMemoryReclaimer reclaimer;
  reclaimer.ReclaimNow(MemoryPressureLevel::kModerate);
  EXPECT_EQ(reclaimer.total_reclaim_passes(), 1);
}

TEST_F(VigoMemoryReclaimerTest, ReclaimNoneDoesNothing) {
  VigoMemoryReclaimer reclaimer;
  reclaimer.ReclaimNow(MemoryPressureLevel::kNone);
  EXPECT_EQ(reclaimer.total_reclaim_passes(), 0);
}

TEST_F(VigoMemoryReclaimerTest, ReclaimWithProcessManager) {
  MockReclaimerProcessManager process_manager;
  process_manager.RegisterProcess(100, ProcessType::kRenderer);
  process_manager.RegisterProcess(101, ProcessType::kRenderer);
  process_manager.SampleNow();

  VigoMemoryReclaimer reclaimer;
  reclaimer.SetProcessManager(&process_manager);

  reclaimer.ReclaimNow(MemoryPressureLevel::kCritical);
  EXPECT_EQ(reclaimer.total_reclaim_passes(), 1);
  // Should have attempted to reclaim something.
  EXPECT_GT(reclaimer.total_bytes_reclaimed(), 0u);
}

TEST_F(VigoMemoryReclaimerTest, CooldownPreventsRapidReclamation) {
  VigoMemoryReclaimer::Config config;
  config.reclaim_cooldown_seconds = 60;
  VigoMemoryReclaimer reclaimer(config);

  reclaimer.ReclaimNow(MemoryPressureLevel::kModerate);
  EXPECT_EQ(reclaimer.total_reclaim_passes(), 1);

  // Second call within cooldown should still execute (ReclaimNow bypasses).
  reclaimer.ReclaimNow(MemoryPressureLevel::kCritical);
  EXPECT_EQ(reclaimer.total_reclaim_passes(), 2);
}

TEST_F(VigoMemoryReclaimerTest, PressureEscalationTriggersReclaim) {
  VigoMemoryReclaimer reclaimer;

  reclaimer.OnMemoryPressureLevelChanged(MemoryPressureLevel::kNone,
                                         MemoryPressureLevel::kModerate);
  EXPECT_EQ(reclaimer.total_reclaim_passes(), 1);
}

TEST_F(VigoMemoryReclaimerTest, PressureDeescalationDoesNotReclaim) {
  VigoMemoryReclaimer reclaimer;

  reclaimer.OnMemoryPressureLevelChanged(MemoryPressureLevel::kCritical,
                                         MemoryPressureLevel::kNone);
  EXPECT_EQ(reclaimer.total_reclaim_passes(), 0);
}

TEST_F(VigoMemoryReclaimerTest, DisabledStrategiesSkipped) {
  VigoMemoryReclaimer::Config config;
  config.enable_working_set_trim = false;
  config.enable_v8_gc_hints = false;
  config.enable_cache_purge = false;
  config.enable_heap_compact = false;
  VigoMemoryReclaimer reclaimer(config);

  reclaimer.ReclaimNow(MemoryPressureLevel::kCritical);
  EXPECT_EQ(reclaimer.total_reclaim_passes(), 1);
  EXPECT_EQ(reclaimer.total_bytes_reclaimed(), 0u);
}

TEST_F(VigoMemoryReclaimerTest, MaxTrimPerPassRespected) {
  VigoMemoryReclaimer::Config config;
  config.max_trim_per_pass = 1;
  VigoMemoryReclaimer reclaimer(config);

  MockReclaimerProcessManager pm;
  pm.RegisterProcess(100, ProcessType::kRenderer);
  pm.RegisterProcess(101, ProcessType::kRenderer);
  pm.RegisterProcess(102, ProcessType::kRenderer);
  pm.SampleNow();

  reclaimer.SetProcessManager(&pm);
  reclaimer.ReclaimNow(MemoryPressureLevel::kModerate);

  // Only 1 process should have been trimmed.
  EXPECT_EQ(reclaimer.total_reclaim_passes(), 1);
}

}  // namespace
}  // namespace performance
}  // namespace vigo

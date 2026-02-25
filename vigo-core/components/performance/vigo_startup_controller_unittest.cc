// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/performance/vigo_startup_controller.h"

#include "base/test/task_environment.h"
#include "testing/gtest/include/gtest/gtest.h"

namespace vigo {
namespace performance {
namespace {

class VigoStartupControllerTest : public testing::Test {
 protected:
  base::test::TaskEnvironment task_environment_{
      base::test::TaskEnvironment::TimeSource::MOCK_TIME};
};

TEST_F(VigoStartupControllerTest, CriticalTaskRunsImmediately) {
  VigoStartupController controller;
  bool ran = false;

  controller.RegisterTask("gpu_init", StartupPriority::kCriticalPath, 10,
                          base::BindOnce([](bool* r) { *r = true; }, &ran));

  EXPECT_TRUE(ran);
  EXPECT_TRUE(controller.AllTasksCompleted());
  EXPECT_EQ(controller.PendingTaskCount(), 0);
}

TEST_F(VigoStartupControllerTest, DeferredTasksNotRunUntilFirstPaint) {
  VigoStartupController controller;
  bool ran = false;

  controller.RegisterTask("sync_init", StartupPriority::kLowPriority, 50,
                          base::BindOnce([](bool* r) { *r = true; }, &ran));

  EXPECT_FALSE(ran);
  EXPECT_FALSE(controller.AllTasksCompleted());
  EXPECT_EQ(controller.PendingTaskCount(), 1);
}

TEST_F(VigoStartupControllerTest, HighPriorityRunsAfterFirstPaint) {
  VigoStartupController::Config config;
  config.high_priority_delay_ms = 100;
  VigoStartupController controller(config);

  bool ran = false;
  controller.RegisterTask("adblock_init", StartupPriority::kHighPriority, 20,
                          base::BindOnce([](bool* r) { *r = true; }, &ran));

  controller.OnFirstPaint();
  EXPECT_FALSE(ran);  // Not yet — timer hasn't fired.

  task_environment_.FastForwardBy(base::Milliseconds(150));
  EXPECT_TRUE(ran);
}

TEST_F(VigoStartupControllerTest, MediumPriorityRunsAfterDelay) {
  VigoStartupController::Config config;
  config.medium_priority_delay_ms = 2000;
  VigoStartupController controller(config);

  bool ran = false;
  controller.RegisterTask("media_init", StartupPriority::kMediumPriority, 30,
                          base::BindOnce([](bool* r) { *r = true; }, &ran));

  controller.OnFirstPaint();

  task_environment_.FastForwardBy(base::Milliseconds(1000));
  EXPECT_FALSE(ran);

  task_environment_.FastForwardBy(base::Milliseconds(1500));
  EXPECT_TRUE(ran);
}

TEST_F(VigoStartupControllerTest, LowPriorityRunsAfterLongDelay) {
  VigoStartupController::Config config;
  config.low_priority_delay_ms = 5000;
  VigoStartupController controller(config);

  bool ran = false;
  controller.RegisterTask("extension_scan", StartupPriority::kLowPriority, 30,
                          base::BindOnce([](bool* r) { *r = true; }, &ran));

  controller.OnFirstPaint();

  task_environment_.FastForwardBy(base::Milliseconds(3000));
  EXPECT_FALSE(ran);

  task_environment_.FastForwardBy(base::Milliseconds(3000));
  EXPECT_TRUE(ran);
}

TEST_F(VigoStartupControllerTest, EnsureTaskCompletedRunsEagerly) {
  VigoStartupController controller;
  bool ran = false;

  controller.RegisterTask("vault_init", StartupPriority::kLazy, 100,
                          base::BindOnce([](bool* r) { *r = true; }, &ran));

  EXPECT_FALSE(ran);

  // Lazy task runs on demand.
  controller.EnsureTaskCompleted("vault_init");
  EXPECT_TRUE(ran);
}

TEST_F(VigoStartupControllerTest, EnsureNonExistentTaskIsNoop) {
  VigoStartupController controller;
  // Should not crash.
  controller.EnsureTaskCompleted("nonexistent");
}

TEST_F(VigoStartupControllerTest, TaskProfilesRecorded) {
  VigoStartupController controller;
  controller.RegisterTask("critical_task", StartupPriority::kCriticalPath, 5,
                          base::BindOnce([]() {}));

  auto profiles = controller.GetTaskProfiles();
  ASSERT_EQ(profiles.size(), 1u);
  EXPECT_EQ(profiles[0].name, "critical_task");
  EXPECT_TRUE(profiles[0].executed);
  EXPECT_EQ(profiles[0].priority, StartupPriority::kCriticalPath);
}

TEST_F(VigoStartupControllerTest, TimeToFirstPaint) {
  VigoStartupController controller;
  base::TimeTicks start = base::TimeTicks::Now();
  controller.RecordProcessStartTime(start);

  task_environment_.FastForwardBy(base::Milliseconds(500));
  controller.OnFirstPaint();

  // Should be approximately 500ms.
  auto ttfp = controller.GetTimeToFirstPaint();
  EXPECT_GE(ttfp.InMilliseconds(), 450);
  EXPECT_LE(ttfp.InMilliseconds(), 600);
}

TEST_F(VigoStartupControllerTest, MultiplePrioritiesOrdered) {
  VigoStartupController::Config config;
  config.high_priority_delay_ms = 100;
  config.medium_priority_delay_ms = 200;
  config.low_priority_delay_ms = 300;
  VigoStartupController controller(config);

  int order = 0;
  int high_order = 0, med_order = 0, low_order = 0;

  controller.RegisterTask("high", StartupPriority::kHighPriority, 10,
                          base::BindOnce([](int* o, int* ho) {
                            *ho = ++(*o);
                          }, &order, &high_order));
  controller.RegisterTask("medium", StartupPriority::kMediumPriority, 10,
                          base::BindOnce([](int* o, int* mo) {
                            *mo = ++(*o);
                          }, &order, &med_order));
  controller.RegisterTask("low", StartupPriority::kLowPriority, 10,
                          base::BindOnce([](int* o, int* lo) {
                            *lo = ++(*o);
                          }, &order, &low_order));

  controller.OnFirstPaint();
  task_environment_.FastForwardBy(base::Milliseconds(500));

  EXPECT_GT(high_order, 0);
  EXPECT_GT(med_order, 0);
  EXPECT_GT(low_order, 0);
  EXPECT_LT(high_order, med_order);
  EXPECT_LT(med_order, low_order);
}

TEST_F(VigoStartupControllerTest, DoubleFirstPaintIgnored) {
  VigoStartupController controller;
  controller.OnFirstPaint();
  controller.OnFirstPaint();  // Should not crash or re-arm timers.
}

}  // namespace
}  // namespace performance
}  // namespace vigo

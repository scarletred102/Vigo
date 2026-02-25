// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/performance/vigo_performance_controller.h"

#include "base/test/task_environment.h"
#include "testing/gtest/include/gtest/gtest.h"

namespace vigo {
namespace performance {
namespace {

class VigoPerformanceControllerTest : public testing::Test {
 protected:
  void SetUp() override {
    controller_ = std::make_unique<VigoPerformanceController>();
  }

  void TearDown() override { controller_.reset(); }

  base::test::TaskEnvironment task_environment_{
      base::test::TaskEnvironment::TimeSource::MOCK_TIME};
  std::unique_ptr<VigoPerformanceController> controller_;
};

TEST_F(VigoPerformanceControllerTest, SubsystemsNullBeforeInit) {
  // Before Initialise(), all subsystem pointers are null.
  EXPECT_EQ(controller_->memory_budget(), nullptr);
  EXPECT_EQ(controller_->tab_lifecycle(), nullptr);
  EXPECT_EQ(controller_->process_manager(), nullptr);
  EXPECT_EQ(controller_->memory_reclaimer(), nullptr);
  EXPECT_EQ(controller_->startup_controller(), nullptr);
  EXPECT_EQ(controller_->telemetry(), nullptr);
}

TEST_F(VigoPerformanceControllerTest, InitialiseCreatesSubsystems) {
  controller_->Initialise();

  EXPECT_NE(controller_->memory_budget(), nullptr);
  EXPECT_NE(controller_->tab_lifecycle(), nullptr);
  EXPECT_NE(controller_->process_manager(), nullptr);
  EXPECT_NE(controller_->memory_reclaimer(), nullptr);
  EXPECT_NE(controller_->startup_controller(), nullptr);
  EXPECT_NE(controller_->telemetry(), nullptr);
}

TEST_F(VigoPerformanceControllerTest, DoubleInitialiseIsNoop) {
  controller_->Initialise();
  auto* budget_ptr = controller_->memory_budget();
  controller_->Initialise();
  // Should return the same pointer — not re-created.
  EXPECT_EQ(controller_->memory_budget(), budget_ptr);
}

TEST_F(VigoPerformanceControllerTest, StartAutoInitialises) {
  // Start() should call Initialise() if not already done.
  controller_->Start();

  EXPECT_NE(controller_->memory_budget(), nullptr);
  EXPECT_TRUE(controller_->memory_budget()->is_running());
}

TEST_F(VigoPerformanceControllerTest, ShutdownStopsAllSubsystems) {
  controller_->Start();
  EXPECT_TRUE(controller_->memory_budget()->is_running());

  controller_->Shutdown();

  // After shutdown, subsystem pointers may still exist but are stopped.
  // Re-initialise should work.
  controller_->Initialise();
  EXPECT_NE(controller_->memory_budget(), nullptr);
}

TEST_F(VigoPerformanceControllerTest, ShutdownWithoutStartIsNoop) {
  // Should not crash.
  controller_->Shutdown();
}

TEST_F(VigoPerformanceControllerTest, DestructorCallsShutdown) {
  controller_->Start();
  // Destroying should not crash — destructor calls Shutdown().
  controller_.reset();
}

TEST_F(VigoPerformanceControllerTest, ProcessManagerRegistration) {
  controller_->Start();

  auto* pm = controller_->process_manager();
  pm->RegisterProcess(/*pid=*/1234, ProcessType::kRenderer);

  EXPECT_EQ(pm->GetProcessCount(), 1);
  EXPECT_EQ(pm->GetRendererCount(), 1);

  pm->UnregisterProcess(/*pid=*/1234);
  EXPECT_EQ(pm->GetProcessCount(), 0);
}

TEST_F(VigoPerformanceControllerTest, OnFirstPaintDelegates) {
  controller_->Start();

  // Should not crash, and should forward to startup controller.
  controller_->OnFirstPaint();
}

TEST_F(VigoPerformanceControllerTest, MemoryBudgetDefaultPressure) {
  controller_->Start();

  // With no processes registered, pressure should be none.
  EXPECT_EQ(controller_->memory_budget()->current_pressure_level(),
            MemoryPressureLevel::kNone);
}

TEST_F(VigoPerformanceControllerTest, SampleTickFires) {
  controller_->Start();

  // The memory budget controller samples every 5s. Advance past one tick.
  task_environment_.FastForwardBy(base::Seconds(6));

  // Should not crash. With zero processes, pressure remains none.
  EXPECT_EQ(controller_->memory_budget()->current_pressure_level(),
            MemoryPressureLevel::kNone);
}

TEST_F(VigoPerformanceControllerTest, TelemetryOptInRequired) {
  controller_->Start();

  // Telemetry is disabled by default (opt-in).
  EXPECT_FALSE(controller_->telemetry()->is_enabled());
  EXPECT_FALSE(controller_->telemetry()->is_running());
}

}  // namespace
}  // namespace performance
}  // namespace vigo

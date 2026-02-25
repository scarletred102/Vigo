// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/performance/vigo_performance_telemetry.h"

#include "base/test/task_environment.h"
#include "testing/gtest/include/gtest/gtest.h"

namespace vigo {
namespace performance {
namespace {

class VigoPerformanceTelemetryTest : public testing::Test {
 protected:
  base::test::TaskEnvironment task_environment_{
      base::test::TaskEnvironment::TimeSource::MOCK_TIME};
};

TEST_F(VigoPerformanceTelemetryTest, DisabledByDefault) {
  VigoPerformanceTelemetry telemetry;
  EXPECT_FALSE(telemetry.is_enabled());
  EXPECT_FALSE(telemetry.is_running());
}

TEST_F(VigoPerformanceTelemetryTest, EnableStartsCollection) {
  VigoPerformanceTelemetry::Config config;
  config.enabled = true;
  config.sample_interval_seconds = 1;
  VigoPerformanceTelemetry telemetry(config);

  telemetry.Start();
  EXPECT_TRUE(telemetry.is_running());

  telemetry.Stop();
  EXPECT_FALSE(telemetry.is_running());
}

TEST_F(VigoPerformanceTelemetryTest, StartWithoutEnableIsNoop) {
  VigoPerformanceTelemetry telemetry;
  telemetry.Start();
  EXPECT_FALSE(telemetry.is_running());
}

TEST_F(VigoPerformanceTelemetryTest, SetEnabledToggles) {
  VigoPerformanceTelemetry telemetry;
  telemetry.SetEnabled(true);
  EXPECT_TRUE(telemetry.is_enabled());
  EXPECT_TRUE(telemetry.is_running());

  telemetry.SetEnabled(false);
  EXPECT_FALSE(telemetry.is_enabled());
  EXPECT_FALSE(telemetry.is_running());
}

TEST_F(VigoPerformanceTelemetryTest, SamplesCollectedOverTime) {
  VigoPerformanceTelemetry::Config config;
  config.enabled = true;
  config.sample_interval_seconds = 1;
  VigoPerformanceTelemetry telemetry(config);

  telemetry.Start();

  // Advance 5 seconds — should collect ~5 samples.
  task_environment_.FastForwardBy(base::Seconds(5));

  EXPECT_GE(telemetry.GetSamples().size(), 4u);
  EXPECT_LE(telemetry.GetSamples().size(), 6u);

  telemetry.Stop();
}

TEST_F(VigoPerformanceTelemetryTest, RollingBufferLimit) {
  VigoPerformanceTelemetry::Config config;
  config.enabled = true;
  config.sample_interval_seconds = 1;
  config.max_samples = 5;
  VigoPerformanceTelemetry telemetry(config);

  telemetry.Start();
  task_environment_.FastForwardBy(base::Seconds(10));

  EXPECT_LE(telemetry.GetSamples().size(), 5u);
  telemetry.Stop();
}

TEST_F(VigoPerformanceTelemetryTest, RecordResumeLatency) {
  VigoPerformanceTelemetry::Config config;
  config.enabled = true;
  VigoPerformanceTelemetry telemetry(config);
  telemetry.Start();

  telemetry.RecordTabResumeLatency(1, 250.0);
  telemetry.RecordTabResumeLatency(2, 350.0);

  auto stats = telemetry.ComputeSessionStats();
  EXPECT_DOUBLE_EQ(stats.avg_resume_latency_ms, 300.0);

  telemetry.Stop();
}

TEST_F(VigoPerformanceTelemetryTest, RecordStartupTimes) {
  // RecordColdStartTime/RecordTimeToFirstPaint work even when disabled.
  VigoPerformanceTelemetry telemetry;
  telemetry.RecordColdStartTime(base::Milliseconds(1800));
  telemetry.RecordTimeToFirstPaint(base::Milliseconds(500));

  auto stats = telemetry.ComputeSessionStats();
  EXPECT_EQ(stats.cold_start_time.InMilliseconds(), 1800);
  EXPECT_EQ(stats.time_to_first_paint.InMilliseconds(), 500);
}

TEST_F(VigoPerformanceTelemetryTest, TabStateChangesTracked) {
  VigoPerformanceTelemetry::Config config;
  config.enabled = true;
  VigoPerformanceTelemetry telemetry(config);

  telemetry.OnTabStateChanged(1, TabState::kIdle, TabState::kSuspended);
  telemetry.OnTabStateChanged(2, TabState::kSuspended, TabState::kFrozen);
  telemetry.OnTabDiscarded(3);

  auto stats = telemetry.ComputeSessionStats();
  EXPECT_EQ(stats.total_suspensions, 1);
  EXPECT_EQ(stats.total_freezes, 1);
  EXPECT_EQ(stats.total_discards, 1);
}

TEST_F(VigoPerformanceTelemetryTest, ExportToJsonProducesValidDict) {
  VigoPerformanceTelemetry::Config config;
  config.enabled = true;
  config.sample_interval_seconds = 1;
  VigoPerformanceTelemetry telemetry(config);

  telemetry.RecordColdStartTime(base::Milliseconds(2000));
  telemetry.Start();
  task_environment_.FastForwardBy(base::Seconds(3));
  telemetry.Stop();

  auto json = telemetry.ExportToJson();

  EXPECT_TRUE(json.contains("version"));
  EXPECT_TRUE(json.contains("product"));
  EXPECT_TRUE(json.contains("session_stats"));
  EXPECT_TRUE(json.contains("samples"));

  const std::string* version = json.FindString("version");
  ASSERT_NE(version, nullptr);
  EXPECT_EQ(*version, "1.0");

  const std::string* product = json.FindString("product");
  ASSERT_NE(product, nullptr);
  EXPECT_EQ(*product, "Vigo");
}

TEST_F(VigoPerformanceTelemetryTest, SessionStatsWithNoData) {
  VigoPerformanceTelemetry telemetry;
  auto stats = telemetry.ComputeSessionStats();

  EXPECT_EQ(stats.peak_rss, 0u);
  EXPECT_EQ(stats.total_suspensions, 0);
  EXPECT_EQ(stats.total_freezes, 0);
  EXPECT_EQ(stats.total_discards, 0);
  EXPECT_DOUBLE_EQ(stats.avg_resume_latency_ms, 0.0);
}

TEST_F(VigoPerformanceTelemetryTest, LatestSampleWhenEmpty) {
  VigoPerformanceTelemetry telemetry;
  const auto& sample = telemetry.GetLatestSample();
  EXPECT_EQ(sample.total_rss, 0u);
}

}  // namespace
}  // namespace performance
}  // namespace vigo

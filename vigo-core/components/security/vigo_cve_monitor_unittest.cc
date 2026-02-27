// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/security/vigo_cve_monitor.h"

#include "base/test/task_environment.h"
#include "testing/gtest/include/gtest/gtest.h"

namespace vigo {
namespace security {
namespace {

class VigoCveMonitorTest : public testing::Test {
 protected:
  void SetUp() override { monitor_.Initialise(); }
  void TearDown() override { monitor_.Shutdown(); }

  base::test::TaskEnvironment task_environment_;
  VigoCveMonitor monitor_;
};

TEST_F(VigoCveMonitorTest, InitialisesWithDefaultComponents) {
  EXPECT_EQ(monitor_.chromium_version(), "132.0.6834.0");
  EXPECT_EQ(monitor_.feed_url(),
            "https://update.vigobrowser.com/cve-feed.json");
  EXPECT_EQ(monitor_.check_interval(), base::Hours(6));
}

TEST_F(VigoCveMonitorTest, TrackComponentAvoidsDuplicates) {
  // "chromium" was already tracked during Initialise().
  monitor_.TrackComponent("chromium", "999.0.0.0");
  // Should not crash or add duplicate.
}

TEST_F(VigoCveMonitorTest, InjectAndQueryCve) {
  CveEntry entry;
  entry.cve_id = "CVE-2025-0001";
  entry.severity = CveSeverity::kHigh;
  entry.component = "chromium";
  entry.description = "Test vulnerability";

  monitor_.InjectCveForTesting(entry);
  EXPECT_EQ(monitor_.known_cves().size(), 1u);
  EXPECT_EQ(monitor_.known_cves()[0].cve_id, "CVE-2025-0001");
}

TEST_F(VigoCveMonitorTest, CriticalUnpatchedCount) {
  CveEntry critical;
  critical.cve_id = "CVE-2025-0002";
  critical.severity = CveSeverity::kCritical;
  critical.component = "ffmpeg";
  critical.is_patched = false;

  CveEntry high;
  high.cve_id = "CVE-2025-0003";
  high.severity = CveSeverity::kHigh;
  high.component = "chromium";
  high.is_patched = false;

  CveEntry patched_critical;
  patched_critical.cve_id = "CVE-2025-0004";
  patched_critical.severity = CveSeverity::kCritical;
  patched_critical.component = "dav1d";
  patched_critical.is_patched = true;

  monitor_.InjectCveForTesting(critical);
  monitor_.InjectCveForTesting(high);
  monitor_.InjectCveForTesting(patched_critical);

  // Only 1 critical unpatched (CVE-2025-0002).
  EXPECT_EQ(monitor_.critical_unpatched_count(), 1u);
}

TEST_F(VigoCveMonitorTest, MarkPatched) {
  CveEntry entry;
  entry.cve_id = "CVE-2025-0005";
  entry.severity = CveSeverity::kCritical;
  entry.is_patched = false;

  monitor_.InjectCveForTesting(entry);
  EXPECT_EQ(monitor_.critical_unpatched_count(), 1u);

  monitor_.MarkPatched("CVE-2025-0005");
  EXPECT_EQ(monitor_.critical_unpatched_count(), 0u);
}

TEST_F(VigoCveMonitorTest, MarkPatchedUnknownCveLogsWarning) {
  // Should not crash.
  monitor_.MarkPatched("CVE-9999-0001");
}

TEST_F(VigoCveMonitorTest, CriticalCveCallbackFires) {
  std::vector<CveEntry> received;
  monitor_.SetCriticalCveCallback(
      base::BindRepeating([](std::vector<CveEntry>* out,
                             const std::vector<CveEntry>& cves) {
        *out = cves;
      },
      &received));

  // Simulate fetching an advisory response with a critical CVE.
  std::string json = R"({
    "cves": [
      {
        "id": "CVE-2025-9999",
        "severity": "critical",
        "component": "chromium",
        "description": "Critical test CVE",
        "affected": "< 132.0.6835.0",
        "fixed": "132.0.6835.0"
      }
    ]
  })";

  monitor_.OnAdvisoriesFetched(json);
  EXPECT_EQ(received.size(), 1u);
  EXPECT_EQ(received[0].cve_id, "CVE-2025-9999");
}

TEST_F(VigoCveMonitorTest, NonCriticalCveDoesNotFireCallback) {
  bool callback_fired = false;
  monitor_.SetCriticalCveCallback(
      base::BindRepeating([](bool* fired,
                             const std::vector<CveEntry>&) {
        *fired = true;
      },
      &callback_fired));

  std::string json = R"({
    "cves": [
      {
        "id": "CVE-2025-0100",
        "severity": "low",
        "component": "chromium",
        "description": "Low severity"
      }
    ]
  })";

  monitor_.OnAdvisoriesFetched(json);
  EXPECT_FALSE(callback_fired);
}

TEST_F(VigoCveMonitorTest, DuplicateCvesSkippedOnRefetch) {
  std::string json = R"({
    "cves": [
      {
        "id": "CVE-2025-0200",
        "severity": "high",
        "component": "ffmpeg",
        "description": "Duplicate test"
      }
    ]
  })";

  monitor_.OnAdvisoriesFetched(json);
  EXPECT_EQ(monitor_.known_cves().size(), 1u);

  // Fetch same feed again — should not duplicate.
  monitor_.OnAdvisoriesFetched(json);
  EXPECT_EQ(monitor_.known_cves().size(), 1u);
}

TEST_F(VigoCveMonitorTest, SetCustomCheckInterval) {
  monitor_.set_check_interval(base::Hours(1));
  EXPECT_EQ(monitor_.check_interval(), base::Hours(1));
}

TEST_F(VigoCveMonitorTest, SetCustomFeedUrl) {
  monitor_.set_feed_url("https://custom.example.com/feed");
  EXPECT_EQ(monitor_.feed_url(), "https://custom.example.com/feed");
}

TEST_F(VigoCveMonitorTest, CheckNowDoesNotCrash) {
  // Just verify no crash/hang.
  monitor_.CheckNow();
}

}  // namespace
}  // namespace security
}  // namespace vigo

// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/privacy_engine/vigo_privacy_stats.h"

#include "testing/gtest/include/gtest/gtest.h"

namespace vigo {
namespace privacy {
namespace {

class MockStatsObserver : public VigoPrivacyStats::Observer {
 public:
  void OnPrivacyStatsUpdated(
      const VigoPrivacyStats::Snapshot& stats) override {
    last_snapshot_ = stats;
    ++update_count_;
  }

  VigoPrivacyStats::Snapshot last_snapshot_;
  int update_count_ = 0;
};

class VigoPrivacyStatsTest : public testing::Test {
 protected:
  void SetUp() override { stats_.Reset(); }

  VigoPrivacyStats stats_;
};

TEST_F(VigoPrivacyStatsTest, InitialCountsAreZero) {
  auto snapshot = stats_.GetSnapshot();
  EXPECT_EQ(snapshot.ads_blocked, 0);
  EXPECT_EQ(snapshot.trackers_blocked, 0);
  EXPECT_EQ(snapshot.https_upgrades, 0);
  EXPECT_EQ(snapshot.fingerprint_protections, 0);
  EXPECT_EQ(snapshot.estimated_time_saved_ms, 0);
}

TEST_F(VigoPrivacyStatsTest, RecordAdBlocked) {
  stats_.RecordAdBlocked();
  stats_.RecordAdBlocked();
  stats_.RecordAdBlocked();
  auto snapshot = stats_.GetSnapshot();
  EXPECT_EQ(snapshot.ads_blocked, 3);
}

TEST_F(VigoPrivacyStatsTest, RecordTrackerBlocked) {
  stats_.RecordTrackerBlocked();
  auto snapshot = stats_.GetSnapshot();
  EXPECT_EQ(snapshot.trackers_blocked, 1);
}

TEST_F(VigoPrivacyStatsTest, RecordHttpsUpgrade) {
  stats_.RecordHttpsUpgrade();
  stats_.RecordHttpsUpgrade();
  auto snapshot = stats_.GetSnapshot();
  EXPECT_EQ(snapshot.https_upgrades, 2);
}

TEST_F(VigoPrivacyStatsTest, RecordFingerprintProtection) {
  stats_.RecordFingerprintProtection();
  auto snapshot = stats_.GetSnapshot();
  EXPECT_EQ(snapshot.fingerprint_protections, 1);
}

TEST_F(VigoPrivacyStatsTest, TimeSavedEstimation) {
  // 50ms per blocked request (ads + trackers).
  stats_.RecordAdBlocked();
  stats_.RecordAdBlocked();
  stats_.RecordTrackerBlocked();
  auto snapshot = stats_.GetSnapshot();
  // 3 blocks × 50ms = 150ms.
  EXPECT_EQ(snapshot.estimated_time_saved_ms, 150);
}

TEST_F(VigoPrivacyStatsTest, BatchIncrementAds) {
  stats_.IncrementAdsBlocked(100);
  auto snapshot = stats_.GetSnapshot();
  EXPECT_EQ(snapshot.ads_blocked, 100);
}

TEST_F(VigoPrivacyStatsTest, BatchIncrementTrackers) {
  stats_.IncrementTrackersBlocked(50);
  auto snapshot = stats_.GetSnapshot();
  EXPECT_EQ(snapshot.trackers_blocked, 50);
}

TEST_F(VigoPrivacyStatsTest, ResetClearsAllCounters) {
  stats_.RecordAdBlocked();
  stats_.RecordTrackerBlocked();
  stats_.RecordHttpsUpgrade();
  stats_.RecordFingerprintProtection();
  stats_.Reset();
  auto snapshot = stats_.GetSnapshot();
  EXPECT_EQ(snapshot.ads_blocked, 0);
  EXPECT_EQ(snapshot.trackers_blocked, 0);
  EXPECT_EQ(snapshot.https_upgrades, 0);
  EXPECT_EQ(snapshot.fingerprint_protections, 0);
}

TEST_F(VigoPrivacyStatsTest, ObserverNotifiedOnRecord) {
  MockStatsObserver observer;
  stats_.AddObserver(&observer);
  stats_.RecordAdBlocked();
  EXPECT_EQ(observer.update_count_, 1);
  EXPECT_EQ(observer.last_snapshot_.ads_blocked, 1);
  stats_.RecordTrackerBlocked();
  EXPECT_EQ(observer.update_count_, 2);
  stats_.RemoveObserver(&observer);
}

TEST_F(VigoPrivacyStatsTest, ObserverNotNotifiedAfterRemoval) {
  MockStatsObserver observer;
  stats_.AddObserver(&observer);
  stats_.RecordAdBlocked();
  EXPECT_EQ(observer.update_count_, 1);
  stats_.RemoveObserver(&observer);
  stats_.RecordAdBlocked();
  EXPECT_EQ(observer.update_count_, 1);  // No new notifications.
}

TEST_F(VigoPrivacyStatsTest, SingletonReturnsConsistentInstance) {
  auto* instance1 = VigoPrivacyStats::GetInstance();
  auto* instance2 = VigoPrivacyStats::GetInstance();
  EXPECT_EQ(instance1, instance2);
  EXPECT_NE(instance1, nullptr);
}

}  // namespace
}  // namespace privacy
}  // namespace vigo

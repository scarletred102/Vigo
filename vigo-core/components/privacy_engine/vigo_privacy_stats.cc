// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/privacy_engine/vigo_privacy_stats.h"

#include "base/logging.h"
#include "base/no_destructor.h"

namespace vigo {
namespace privacy {

namespace {

// Estimated time cost of a blocked ad/tracker request.
// Assumes ~50ms saved per blocked request (network RTT + parse + render).
constexpr int64_t kEstimatedTimeSavedPerBlockMs = 50;

}  // namespace

VigoPrivacyStats::VigoPrivacyStats() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  VLOG(1) << "VigoPrivacyStats: Initialised (session counters zeroed)";
}

VigoPrivacyStats::~VigoPrivacyStats() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

// static
VigoPrivacyStats* VigoPrivacyStats::GetInstance() {
  static base::NoDestructor<VigoPrivacyStats> instance;
  return instance.get();
}

void VigoPrivacyStats::RecordAdBlocked() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  ++ads_blocked_;
  NotifyObservers();
}

void VigoPrivacyStats::RecordTrackerBlocked() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  ++trackers_blocked_;
  NotifyObservers();
}

void VigoPrivacyStats::RecordHttpsUpgrade() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  ++https_upgrades_;
  NotifyObservers();
}

void VigoPrivacyStats::RecordFingerprintProtection() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  ++fingerprint_protections_;
  NotifyObservers();
}

void VigoPrivacyStats::IncrementAdsBlocked(int64_t count) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  ads_blocked_ += count;
  NotifyObservers();
}

void VigoPrivacyStats::IncrementTrackersBlocked(int64_t count) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  trackers_blocked_ += count;
  NotifyObservers();
}

VigoPrivacyStats::Snapshot VigoPrivacyStats::GetSnapshot() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  Snapshot snapshot;
  snapshot.ads_blocked = ads_blocked_;
  snapshot.trackers_blocked = trackers_blocked_;
  snapshot.https_upgrades = https_upgrades_;
  snapshot.fingerprint_protections = fingerprint_protections_;
  snapshot.estimated_time_saved_ms =
      (ads_blocked_ + trackers_blocked_) * kEstimatedTimeSavedPerBlockMs;
  return snapshot;
}

void VigoPrivacyStats::Reset() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  ads_blocked_ = 0;
  trackers_blocked_ = 0;
  https_upgrades_ = 0;
  fingerprint_protections_ = 0;
  VLOG(1) << "VigoPrivacyStats: All counters reset";
  NotifyObservers();
}

void VigoPrivacyStats::AddObserver(Observer* observer) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  observers_.AddObserver(observer);
}

void VigoPrivacyStats::RemoveObserver(Observer* observer) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  observers_.RemoveObserver(observer);
}

void VigoPrivacyStats::NotifyObservers() {
  Snapshot snapshot = GetSnapshot();
  for (auto& observer : observers_) {
    observer.OnPrivacyStatsUpdated(snapshot);
  }
}

}  // namespace privacy
}  // namespace vigo

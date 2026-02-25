// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_PRIVACY_ENGINE_VIGO_PRIVACY_STATS_H_
#define VIGO_COMPONENTS_PRIVACY_ENGINE_VIGO_PRIVACY_STATS_H_

#include <cstdint>

#include "base/observer_list.h"
#include "base/observer_list_types.h"
#include "base/sequence_checker.h"

namespace vigo {
namespace privacy {

// VigoPrivacyStats tracks privacy protection statistics for display on
// the New Tab Page and in the settings "About" section.
//
// Counts are accumulated in memory during the current browser session.
// TODO(Phase 1.5): Persist counts to PrefService for cross-session totals.
//
// Thread safety: must be accessed on the UI sequence only.
class VigoPrivacyStats {
 public:
  // Snapshot of the current stats for consumers (NTP, settings, etc.).
  struct Snapshot {
    int64_t ads_blocked = 0;
    int64_t trackers_blocked = 0;
    int64_t https_upgrades = 0;
    int64_t fingerprint_protections = 0;
    // Estimated time saved: ~50ms per blocked ad request.
    int64_t estimated_time_saved_ms = 0;
  };

  // Observer for stats changes (e.g., NTP auto-refresh).
  class Observer : public base::CheckedObserver {
   public:
    virtual void OnPrivacyStatsUpdated(const Snapshot& stats) = 0;
  };

  VigoPrivacyStats();
  ~VigoPrivacyStats();

  VigoPrivacyStats(const VigoPrivacyStats&) = delete;
  VigoPrivacyStats& operator=(const VigoPrivacyStats&) = delete;

  // Singleton accessor — there is one stats tracker per browser process.
  static VigoPrivacyStats* GetInstance();

  // Increment counters. Each call notifies observers.
  void RecordAdBlocked();
  void RecordTrackerBlocked();
  void RecordHttpsUpgrade();
  void RecordFingerprintProtection();

  // Batch increment (used when loading from persisted state).
  void IncrementAdsBlocked(int64_t count);
  void IncrementTrackersBlocked(int64_t count);

  // Get current snapshot.
  Snapshot GetSnapshot() const;

  // Reset all counters (e.g., user clicks "Reset Stats").
  void Reset();

  void AddObserver(Observer* observer);
  void RemoveObserver(Observer* observer);

 private:
  void NotifyObservers();

  int64_t ads_blocked_ = 0;
  int64_t trackers_blocked_ = 0;
  int64_t https_upgrades_ = 0;
  int64_t fingerprint_protections_ = 0;

  base::ObserverList<Observer> observers_;

  SEQUENCE_CHECKER(sequence_checker_);
};

}  // namespace privacy
}  // namespace vigo

#endif  // VIGO_COMPONENTS_PRIVACY_ENGINE_VIGO_PRIVACY_STATS_H_

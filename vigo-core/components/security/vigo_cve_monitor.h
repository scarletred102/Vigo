// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_SECURITY_VIGO_CVE_MONITOR_H_
#define VIGO_COMPONENTS_SECURITY_VIGO_CVE_MONITOR_H_

#include <string>
#include <vector>

#include "base/functional/callback.h"
#include "base/memory/weak_ptr.h"
#include "base/sequence_checker.h"
#include "base/time/time.h"
#include "base/timer/timer.h"

namespace vigo {
namespace security {

// Severity level for CVEs.
enum class CveSeverity {
  kLow,
  kMedium,
  kHigh,
  kCritical,
};

// A single CVE advisory relevant to Vigo's Chromium base or dependencies.
struct CveEntry {
  std::string cve_id;            // e.g. "CVE-2025-1234"
  CveSeverity severity = CveSeverity::kLow;
  std::string component;         // e.g. "chromium", "ffmpeg", "libsodium"
  std::string description;
  std::string affected_versions;
  std::string fixed_version;     // Empty if no fix available yet.
  base::Time published;
  bool is_patched = false;       // Whether Vigo has addressed this CVE.
};

// VigoCveMonitor polls a security advisory feed for CVEs that affect the
// components used by Vigo (Chromium base version, ffmpeg, dav1d, libsodium,
// etc.). When a critical CVE is found, it triggers the 72-hour SLA pipeline
// alert.
//
// This runs strictly in the browser process on the UI thread.
class VigoCveMonitor {
 public:
  // Callback invoked when new critical CVEs are discovered.
  using CriticalCveCallback =
      base::RepeatingCallback<void(const std::vector<CveEntry>&)>;

  VigoCveMonitor();
  ~VigoCveMonitor();

  VigoCveMonitor(const VigoCveMonitor&) = delete;
  VigoCveMonitor& operator=(const VigoCveMonitor&) = delete;

  // Initialise the monitor. Sets up the polling timer and loads the
  // list of tracked components from the Vigo SBOM.
  void Initialise();

  // Shut down cleanly, cancelling timers.
  void Shutdown();

  // Register a callback for critical CVE alerts.
  void SetCriticalCveCallback(CriticalCveCallback callback);

  // Manually trigger a check (used by tests and the "Check now" button).
  void CheckNow();

  // Add a component+version to monitor (e.g., "chromium", "132.0.6834.0").
  void TrackComponent(const std::string& name, const std::string& version);

  // Mark a CVE as patched in Vigo.
  void MarkPatched(const std::string& cve_id);

  // Accessors.
  const std::vector<CveEntry>& known_cves() const { return known_cves_; }
  size_t critical_unpatched_count() const;

  // Get the Chromium base version being tracked.
  std::string chromium_version() const { return chromium_version_; }
  void set_chromium_version(const std::string& v) { chromium_version_ = v; }

  // Get/set the advisory feed URL (default: Chromium security page).
  std::string feed_url() const { return feed_url_; }
  void set_feed_url(const std::string& url) { feed_url_ = url; }

  // Polling interval (default: 6 hours).
  base::TimeDelta check_interval() const { return check_interval_; }
  void set_check_interval(base::TimeDelta interval) {
    check_interval_ = interval;
  }

  // For testing: inject CVEs directly.
  void InjectCveForTesting(const CveEntry& entry);

 private:
  struct TrackedComponent {
    std::string name;
    std::string version;
  };

  // Fetch advisories from the feed.
  void FetchAdvisories();

  // Parse advisory response and update |known_cves_|.
  void OnAdvisoriesFetched(const std::string& response_body);

  // Evaluate newly discovered CVEs and fire alerts if critical.
  void EvaluateNewCves(const std::vector<CveEntry>& new_cves);

  std::string chromium_version_ = "132.0.6834.0";
  std::string feed_url_ = "https://update.vigobrowser.com/cve-feed.json";
  base::TimeDelta check_interval_ = base::Hours(6);

  std::vector<TrackedComponent> tracked_components_;
  std::vector<CveEntry> known_cves_;

  CriticalCveCallback critical_cve_callback_;
  base::RepeatingTimer check_timer_;

  SEQUENCE_CHECKER(sequence_checker_);
  base::WeakPtrFactory<VigoCveMonitor> weak_factory_{this};
};

}  // namespace security
}  // namespace vigo

#endif  // VIGO_COMPONENTS_SECURITY_VIGO_CVE_MONITOR_H_

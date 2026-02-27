// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/security/vigo_cve_monitor.h"

#include <algorithm>

#include "base/json/json_reader.h"
#include "base/logging.h"
#include "base/strings/string_number_conversions.h"
#include "base/values.h"

namespace vigo {
namespace security {

VigoCveMonitor::VigoCveMonitor() {
  DETACH_FROM_SEQUENCE(sequence_checker_);
}

VigoCveMonitor::~VigoCveMonitor() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

void VigoCveMonitor::Initialise() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  // Track default components.
  TrackComponent("chromium", chromium_version_);
  TrackComponent("ffmpeg", "6.1");
  TrackComponent("dav1d", "1.3.0");
  TrackComponent("libsodium", "1.0.19");

  // Start the polling timer.
  check_timer_.Start(FROM_HERE, check_interval_,
                     base::BindRepeating(&VigoCveMonitor::FetchAdvisories,
                                         weak_factory_.GetWeakPtr()));

  VLOG(1) << "VigoCveMonitor: Initialised with " << tracked_components_.size()
          << " tracked components, polling every "
          << check_interval_.InHours() << "h";
}

void VigoCveMonitor::Shutdown() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  check_timer_.Stop();
  weak_factory_.InvalidateWeakPtrs();
  VLOG(1) << "VigoCveMonitor: Shut down";
}

void VigoCveMonitor::SetCriticalCveCallback(CriticalCveCallback callback) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  critical_cve_callback_ = std::move(callback);
}

void VigoCveMonitor::CheckNow() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  FetchAdvisories();
}

void VigoCveMonitor::TrackComponent(const std::string& name,
                                     const std::string& version) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  // Avoid duplicates.
  for (const auto& tc : tracked_components_) {
    if (tc.name == name) {
      return;
    }
  }

  tracked_components_.push_back({name, version});
  VLOG(2) << "VigoCveMonitor: Tracking " << name << " v" << version;
}

void VigoCveMonitor::MarkPatched(const std::string& cve_id) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  for (auto& cve : known_cves_) {
    if (cve.cve_id == cve_id) {
      cve.is_patched = true;
      VLOG(1) << "VigoCveMonitor: Marked " << cve_id << " as patched";
      return;
    }
  }

  LOG(WARNING) << "VigoCveMonitor: CVE " << cve_id << " not found in list";
}

size_t VigoCveMonitor::critical_unpatched_count() const {
  return static_cast<size_t>(std::count_if(
      known_cves_.begin(), known_cves_.end(), [](const CveEntry& e) {
        return e.severity == CveSeverity::kCritical && !e.is_patched;
      }));
}

void VigoCveMonitor::InjectCveForTesting(const CveEntry& entry) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  known_cves_.push_back(entry);
}

void VigoCveMonitor::FetchAdvisories() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  // TODO(Phase 5.6): Implement actual HTTP fetch using SimpleURLLoader.
  // For now, this is a stub that will be connected to the Vigo update
  // server's /cve-feed.json endpoint.
  //
  // The expected JSON schema:
  // {
  //   "cves": [
  //     {
  //       "id": "CVE-2025-XXXX",
  //       "severity": "critical",
  //       "component": "chromium",
  //       "description": "...",
  //       "affected": "< 132.0.6834.100",
  //       "fixed": "132.0.6834.100",
  //       "published": "2025-01-15T00:00:00Z"
  //     }
  //   ]
  // }

  VLOG(2) << "VigoCveMonitor: Advisory fetch stub — awaiting server impl";
}

void VigoCveMonitor::OnAdvisoriesFetched(const std::string& response_body) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  auto parsed = base::JSONReader::Read(response_body);
  if (!parsed || !parsed->is_dict()) {
    LOG(WARNING) << "VigoCveMonitor: Failed to parse advisory feed";
    return;
  }

  const auto* cves_list = parsed->GetDict().FindList("cves");
  if (!cves_list) {
    VLOG(2) << "VigoCveMonitor: No CVEs in feed";
    return;
  }

  std::vector<CveEntry> new_cves;

  for (const auto& item : *cves_list) {
    if (!item.is_dict()) {
      continue;
    }

    const auto& dict = item.GetDict();
    CveEntry entry;

    const std::string* id = dict.FindString("id");
    if (!id) {
      continue;
    }
    entry.cve_id = *id;

    // Skip if already known.
    bool already_known = false;
    for (const auto& existing : known_cves_) {
      if (existing.cve_id == entry.cve_id) {
        already_known = true;
        break;
      }
    }
    if (already_known) {
      continue;
    }

    // Parse severity.
    const std::string* severity_str = dict.FindString("severity");
    if (severity_str) {
      if (*severity_str == "critical") {
        entry.severity = CveSeverity::kCritical;
      } else if (*severity_str == "high") {
        entry.severity = CveSeverity::kHigh;
      } else if (*severity_str == "medium") {
        entry.severity = CveSeverity::kMedium;
      } else {
        entry.severity = CveSeverity::kLow;
      }
    }

    const std::string* component = dict.FindString("component");
    if (component) {
      entry.component = *component;
    }

    const std::string* desc = dict.FindString("description");
    if (desc) {
      entry.description = *desc;
    }

    const std::string* affected = dict.FindString("affected");
    if (affected) {
      entry.affected_versions = *affected;
    }

    const std::string* fixed = dict.FindString("fixed");
    if (fixed) {
      entry.fixed_version = *fixed;
    }

    new_cves.push_back(std::move(entry));
  }

  if (!new_cves.empty()) {
    EvaluateNewCves(new_cves);

    // Add to known list.
    for (auto& cve : new_cves) {
      known_cves_.push_back(std::move(cve));
    }
  }

  VLOG(1) << "VigoCveMonitor: Processed feed — " << known_cves_.size()
          << " total CVEs, " << critical_unpatched_count()
          << " critical unpatched";
}

void VigoCveMonitor::EvaluateNewCves(const std::vector<CveEntry>& new_cves) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  std::vector<CveEntry> critical_cves;

  for (const auto& cve : new_cves) {
    // Check if this CVE affects a tracked component.
    bool affects_tracked = false;
    for (const auto& tc : tracked_components_) {
      if (cve.component == tc.name) {
        affects_tracked = true;
        break;
      }
    }

    if (!affects_tracked) {
      continue;
    }

    if (cve.severity == CveSeverity::kCritical) {
      LOG(WARNING) << "VigoCveMonitor: CRITICAL CVE detected — "
                   << cve.cve_id << " in " << cve.component
                   << ": " << cve.description;
      critical_cves.push_back(cve);
    } else if (cve.severity == CveSeverity::kHigh) {
      LOG(WARNING) << "VigoCveMonitor: HIGH CVE detected — "
                   << cve.cve_id << " in " << cve.component;
    }
  }

  // Fire critical alert callback (triggers 72-hour SLA pipeline).
  if (!critical_cves.empty() && critical_cve_callback_) {
    critical_cve_callback_.Run(critical_cves);
  }
}

}  // namespace security
}  // namespace vigo

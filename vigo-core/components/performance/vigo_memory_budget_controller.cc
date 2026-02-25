// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/performance/vigo_memory_budget_controller.h"

#include <algorithm>

#include "base/logging.h"
#include "base/process/process_metrics.h"
#include "base/system/sys_info.h"

#if BUILDFLAG(IS_WIN)
#include <psapi.h>
#include <windows.h>
#endif  // BUILDFLAG(IS_WIN)

namespace vigo {
namespace performance {

namespace {

// Size thresholds for system RAM classification.
constexpr size_t kLowEndRamThreshold = 4ULL * 1024 * 1024 * 1024;    // 4 GB
constexpr size_t kHighEndRamThreshold = 8ULL * 1024 * 1024 * 1024;   // 8 GB

// Base budgets per tier (bytes).
constexpr size_t kLowEndBaseBudget = 300 * 1024 * 1024;   // 300 MB
constexpr size_t kMidRangeBaseBudget = 400 * 1024 * 1024;  // 400 MB
constexpr size_t kHighEndBaseBudget = 600 * 1024 * 1024;   // 600 MB

}  // namespace

VigoMemoryBudgetController::VigoMemoryBudgetController()
    : VigoMemoryBudgetController(Config()) {}

VigoMemoryBudgetController::VigoMemoryBudgetController(const Config& config)
    : config_(config) {
  DETACH_FROM_SEQUENCE(sequence_checker_);
  if (config_.base_budget_bytes == 0) {
    config_.base_budget_bytes = ComputeBaseBudget();
  }
  budget_limit_ = config_.base_budget_bytes;
}

VigoMemoryBudgetController::~VigoMemoryBudgetController() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  Stop();
}

void VigoMemoryBudgetController::Start() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  if (is_running_)
    return;

  is_running_ = true;
  RecomputeBudget();

  sample_timer_.Start(
      FROM_HERE,
      base::Milliseconds(config_.sample_interval_ms),
      base::BindRepeating(&VigoMemoryBudgetController::OnSampleTimer,
                          base::Unretained(this)));

  VLOG(1) << "VigoMemoryBudgetController: Started with budget "
          << (budget_limit_ / (1024 * 1024)) << " MB for "
          << current_tab_count_ << " tabs";
}

void VigoMemoryBudgetController::Stop() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  if (!is_running_)
    return;

  sample_timer_.Stop();
  is_running_ = false;
  VLOG(1) << "VigoMemoryBudgetController: Stopped";
}

void VigoMemoryBudgetController::SampleNow() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  OnSampleTimer();
}

void VigoMemoryBudgetController::OnTabCountChanged(int tab_count) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  if (tab_count <= 0)
    tab_count = 1;

  current_tab_count_ = tab_count;
  RecomputeBudget();

  VLOG(2) << "VigoMemoryBudgetController: Tab count → " << tab_count
          << ", budget → " << (budget_limit_ / (1024 * 1024)) << " MB";

  // Immediate re-sample after tab count change.
  if (is_running_) {
    SampleNow();
  }
}

void VigoMemoryBudgetController::AddObserver(MemoryBudgetObserver* observer) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  observers_.AddObserver(observer);
}

void VigoMemoryBudgetController::RemoveObserver(
    MemoryBudgetObserver* observer) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  observers_.RemoveObserver(observer);
}

MemoryPressureLevel VigoMemoryBudgetController::current_pressure_level()
    const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return current_pressure_;
}

const MemoryBudgetSnapshot& VigoMemoryBudgetController::last_snapshot() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return last_snapshot_;
}

size_t VigoMemoryBudgetController::current_budget_limit() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return budget_limit_;
}

// --- Private ---

size_t VigoMemoryBudgetController::ComputeBaseBudget() const {
  int64_t ram = base::SysInfo::AmountOfPhysicalMemory();
  if (ram <= 0) {
    // Fallback to mid-range if detection fails.
    return kMidRangeBaseBudget;
  }

  size_t ram_bytes = static_cast<size_t>(ram);
  if (ram_bytes <= kLowEndRamThreshold) {
    return kLowEndBaseBudget;
  } else if (ram_bytes <= kHighEndRamThreshold) {
    return kMidRangeBaseBudget;
  } else {
    return kHighEndBaseBudget;
  }
}

void VigoMemoryBudgetController::RecomputeBudget() {
  size_t base = config_.base_budget_bytes;
  int extra_tabs = std::max(0, current_tab_count_ - 10);
  budget_limit_ = base + (static_cast<size_t>(extra_tabs) *
                           config_.per_extra_tab_bytes);
}

void VigoMemoryBudgetController::OnSampleTimer() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  size_t total_rss = CollectTotalRss();
  double utilisation =
      (budget_limit_ > 0) ? static_cast<double>(total_rss) /
                                static_cast<double>(budget_limit_)
                          : 0.0;

  MemoryPressureLevel new_level = ComputePressureLevel(utilisation);

  // Build the snapshot.
  MemoryBudgetSnapshot snapshot;
  snapshot.total_system_memory =
      static_cast<size_t>(base::SysInfo::AmountOfPhysicalMemory());
  snapshot.current_browser_rss = total_rss;
  snapshot.budget_limit = budget_limit_;
  snapshot.utilisation = utilisation;
  snapshot.pressure_level = new_level;
  snapshot.renderer_count = 0;  // Filled by ProcessManager in integration.
  snapshot.gpu_process_count = 0;
  // Rough reclaimable estimate: if over budget, the excess is reclaimable.
  snapshot.reclaimable_estimate =
      (total_rss > budget_limit_) ? (total_rss - budget_limit_) : 0;

  last_snapshot_ = snapshot;

  // Hysteresis: only lower pressure after consecutive below-threshold samples.
  if (new_level < current_pressure_) {
    consecutive_below_threshold_++;
    if (consecutive_below_threshold_ < config_.hysteresis_samples) {
      // Keep the higher pressure level during hysteresis.
      new_level = current_pressure_;
    }
  } else {
    consecutive_below_threshold_ = 0;
  }

  // Notify on pressure level change.
  if (new_level != current_pressure_) {
    MemoryPressureLevel old_level = current_pressure_;
    current_pressure_ = new_level;
    consecutive_below_threshold_ = 0;
    NotifyPressureLevelChanged(old_level, new_level);
  }

  // Always notify snapshot.
  NotifySnapshot(snapshot);
}

size_t VigoMemoryBudgetController::CollectTotalRss() const {
  // In the real Chromium integration, this iterates all child processes
  // via content::BrowserChildProcessHostIterator and sums their
  // working set sizes. For the standalone component, we use the current
  // process metrics as a baseline.
  auto metrics = base::ProcessMetrics::CreateCurrentProcessMetrics();
  size_t rss = metrics->GetWorkingSetSize();
  return rss;
}

MemoryPressureLevel VigoMemoryBudgetController::ComputePressureLevel(
    double utilisation) const {
  if (utilisation >= config_.critical_threshold) {
    return MemoryPressureLevel::kCritical;
  } else if (utilisation >= config_.moderate_threshold) {
    return MemoryPressureLevel::kModerate;
  }
  return MemoryPressureLevel::kNone;
}

void VigoMemoryBudgetController::NotifyPressureLevelChanged(
    MemoryPressureLevel old_level,
    MemoryPressureLevel new_level) {
  VLOG(1) << "VigoMemoryBudgetController: Pressure "
          << static_cast<int>(old_level) << " → "
          << static_cast<int>(new_level);

  for (auto& observer : observers_) {
    observer.OnMemoryPressureLevelChanged(old_level, new_level);
  }
}

void VigoMemoryBudgetController::NotifySnapshot(
    const MemoryBudgetSnapshot& snapshot) {
  for (auto& observer : observers_) {
    observer.OnMemoryBudgetSnapshot(snapshot);
  }
}

}  // namespace performance
}  // namespace vigo

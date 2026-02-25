// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_PERFORMANCE_VIGO_MEMORY_BUDGET_CONTROLLER_H_
#define VIGO_COMPONENTS_PERFORMANCE_VIGO_MEMORY_BUDGET_CONTROLLER_H_

#include <cstddef>
#include <cstdint>

#include "base/memory/memory_pressure_listener.h"
#include "base/observer_list.h"
#include "base/sequence_checker.h"
#include "base/time/time.h"
#include "base/timer/timer.h"

namespace vigo {
namespace performance {

// Memory pressure level determined by the budget controller.
enum class MemoryPressureLevel {
  kNone,       // Under budget — no action needed.
  kModerate,   // 70–90% budget used — start soft-suspending background tabs.
  kCritical,   // >90% budget used — aggressively freeze/discard tabs.
};

// Snapshot of current memory usage and budget state.
struct MemoryBudgetSnapshot {
  // Total physical memory on the system (bytes).
  size_t total_system_memory = 0;

  // Current RSS of the browser process + all child processes (bytes).
  size_t current_browser_rss = 0;

  // Configured budget limit (bytes).
  size_t budget_limit = 0;

  // Percentage of budget used (0.0–1.0+).
  double utilisation = 0.0;

  // Current pressure level.
  MemoryPressureLevel pressure_level = MemoryPressureLevel::kNone;

  // Number of tracked renderer processes.
  int renderer_count = 0;

  // Number of tracked GPU processes.
  int gpu_process_count = 0;

  // Estimated potential savings from tab discarding (bytes).
  size_t reclaimable_estimate = 0;
};

// Observer interface for memory budget state changes.
class MemoryBudgetObserver : public base::CheckedObserver {
 public:
  // Called when the pressure level transitions.
  virtual void OnMemoryPressureLevelChanged(MemoryPressureLevel old_level,
                                            MemoryPressureLevel new_level) = 0;

  // Called periodically with the full budget snapshot.
  virtual void OnMemoryBudgetSnapshot(const MemoryBudgetSnapshot& snapshot) = 0;
};

// VigoMemoryBudgetController enforces a global memory budget for the
// browser. It periodically samples process memory, computes utilisation
// against the configured budget, determines the pressure level, and
// notifies observers (tab lifecycle manager, memory reclaimer) so they
// can take action.
//
// Budget defaults:
//   - Low-end  (≤4 GB RAM): 300 MB for 10 tabs
//   - Mid-range (4–8 GB):   400 MB for 10 tabs
//   - High-end  (>8 GB):    600 MB for 10 tabs
//
// The budget scales with additional tabs: +30 MB per tab above 10.
class VigoMemoryBudgetController {
 public:
  // Configuration knobs.
  struct Config {
    // Base budget for 10 tabs (bytes). 0 = auto-detect from system RAM.
    size_t base_budget_bytes = 0;

    // Additional budget per tab beyond the first 10 (bytes).
    size_t per_extra_tab_bytes = 30 * 1024 * 1024;  // 30 MB

    // Moderate pressure threshold (fraction of budget).
    double moderate_threshold = 0.70;

    // Critical pressure threshold (fraction of budget).
    double critical_threshold = 0.90;

    // How often to sample process memory (milliseconds).
    int sample_interval_ms = 5000;  // 5 seconds

    // Hysteresis: must drop below threshold for this many consecutive
    // samples before pressure level is lowered.
    int hysteresis_samples = 3;
  };

  VigoMemoryBudgetController();
  explicit VigoMemoryBudgetController(const Config& config);
  ~VigoMemoryBudgetController();

  VigoMemoryBudgetController(const VigoMemoryBudgetController&) = delete;
  VigoMemoryBudgetController& operator=(const VigoMemoryBudgetController&) =
      delete;

  // Start periodic memory sampling.
  void Start();

  // Stop periodic memory sampling.
  void Stop();

  // Force an immediate sample (e.g., after a tab is opened/closed).
  void SampleNow();

  // Notify the controller that the tab count has changed.
  void OnTabCountChanged(int tab_count);

  // Observer management.
  void AddObserver(MemoryBudgetObserver* observer);
  void RemoveObserver(MemoryBudgetObserver* observer);

  // Getters.
  MemoryPressureLevel current_pressure_level() const;
  const MemoryBudgetSnapshot& last_snapshot() const;
  size_t current_budget_limit() const;
  bool is_running() const { return is_running_; }

 private:
  // Determine the auto-detected base budget from system RAM.
  size_t ComputeBaseBudget() const;

  // Recompute the budget limit based on current tab count.
  void RecomputeBudget();

  // Called on each sample interval.
  void OnSampleTimer();

  // Collect RSS from all browser processes (browser + renderers + GPU +
  // utility). Platform-specific implementation.
  size_t CollectTotalRss() const;

  // Determine pressure level from utilisation.
  MemoryPressureLevel ComputePressureLevel(double utilisation) const;

  // Notify observers of pressure level change.
  void NotifyPressureLevelChanged(MemoryPressureLevel old_level,
                                  MemoryPressureLevel new_level);

  // Notify observers with the latest snapshot.
  void NotifySnapshot(const MemoryBudgetSnapshot& snapshot);

  Config config_;
  int current_tab_count_ = 10;
  size_t budget_limit_ = 0;
  MemoryPressureLevel current_pressure_ = MemoryPressureLevel::kNone;
  MemoryBudgetSnapshot last_snapshot_;
  int consecutive_below_threshold_ = 0;
  bool is_running_ = false;

  base::RepeatingTimer sample_timer_;
  base::ObserverList<MemoryBudgetObserver> observers_;

  SEQUENCE_CHECKER(sequence_checker_);
};

}  // namespace performance
}  // namespace vigo

#endif  // VIGO_COMPONENTS_PERFORMANCE_VIGO_MEMORY_BUDGET_CONTROLLER_H_

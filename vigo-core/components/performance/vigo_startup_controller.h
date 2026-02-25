// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_PERFORMANCE_VIGO_STARTUP_CONTROLLER_H_
#define VIGO_COMPONENTS_PERFORMANCE_VIGO_STARTUP_CONTROLLER_H_

#include <functional>
#include <string>
#include <vector>

#include "base/sequence_checker.h"
#include "base/time/time.h"
#include "base/timer/timer.h"

namespace vigo {
namespace performance {

// Priority tiers for subsystem initialisation.
// Higher priority = initialised earlier during startup.
enum class StartupPriority {
  kCriticalPath,  // Must init before first paint  (browser shell, GPU).
  kHighPriority,  // Init immediately after first paint (adblock, privacy).
  kMediumPriority, // Init within 2s of first paint (media, settings prefs).
  kLowPriority,   // Defer until idle (sync, extensions, telemetry).
  kLazy,           // Init on first use only (credential vault, update check).
};

// A deferred initialisation task registered with the startup controller.
struct StartupTask {
  // Human-readable name (for logging/profiling).
  std::string name;

  // Priority tier.
  StartupPriority priority = StartupPriority::kLowPriority;

  // Estimated initialisation cost (milliseconds). Used for scheduling.
  int estimated_cost_ms = 50;

  // The initialisation callback.
  base::OnceClosure callback;

  // Whether this task has been executed.
  bool executed = false;

  // Actual execution time (set after completion).
  base::TimeDelta actual_duration;
};

// VigoStartupController implements lazy-loading for browser subsystems.
// Instead of initialising everything in PreMainMessageLoopRun(), Vigo
// defers non-critical init work to idle periods after first paint.
//
// This reduces cold-start time by front-loading only what's needed for
// the first visible frame and first navigation.
//
// Timeline:
//   T=0:     Browser process starts
//   T+Xms:   First paint / NTP shown  → CriticalPath tasks run here
//   T+Xms:   requestIdleCallback-like → HighPriority tasks run
//   T+2s:    Timer fires              → MediumPriority tasks run
//   T+5s:    Timer fires              → LowPriority tasks run
//   On-use:  Lazy tasks run on first access
//
// KPI targets (PAD §11):
//   Cold start ≤ 2.5s
//   Warm start ≤ 0.5s
class VigoStartupController {
 public:
  // Configuration.
  struct Config {
    // Delay after first paint before running HighPriority tasks (ms).
    int high_priority_delay_ms = 100;

    // Delay after first paint before running MediumPriority tasks (ms).
    int medium_priority_delay_ms = 2000;

    // Delay after first paint before running LowPriority tasks (ms).
    int low_priority_delay_ms = 5000;

    // Maximum time budget per deferred init batch (ms).
    // If a batch exceeds this, remaining tasks are deferred to next idle.
    int batch_budget_ms = 50;
  };

  VigoStartupController();
  explicit VigoStartupController(const Config& config);
  ~VigoStartupController();

  VigoStartupController(const VigoStartupController&) = delete;
  VigoStartupController& operator=(const VigoStartupController&) = delete;

  // Register a deferred init task. CriticalPath tasks are executed
  // immediately at registration time (they cannot be deferred).
  void RegisterTask(const std::string& name,
                    StartupPriority priority,
                    int estimated_cost_ms,
                    base::OnceClosure callback);

  // Called when the browser process reaches first meaningful paint.
  // This arms the deferred init timers.
  void OnFirstPaint();

  // Called by lazy subsystems to ensure their init has run.
  // If the task hasn't executed yet, it runs synchronously now.
  void EnsureTaskCompleted(const std::string& name);

  // --- Profiling ---

  // Record startup timestamps for profiling.
  void RecordProcessStartTime(base::TimeTicks time);
  void RecordFirstPaintTime(base::TimeTicks time);

  // Get startup timing profile.
  base::TimeDelta GetTimeToFirstPaint() const;
  base::TimeDelta GetTotalDeferredInitTime() const;

  // Get all task execution info (for CI benchmarking).
  struct TaskProfile {
    std::string name;
    StartupPriority priority;
    bool executed;
    base::TimeDelta actual_duration;
  };
  std::vector<TaskProfile> GetTaskProfiles() const;

  // Whether all deferred tasks have completed.
  bool AllTasksCompleted() const;

  // Number of pending tasks.
  int PendingTaskCount() const;

 private:
  // Run all tasks at a given priority level.
  void RunTasksAtPriority(StartupPriority priority);

  // Timer callbacks for deferred priorities.
  void OnHighPriorityTimer();
  void OnMediumPriorityTimer();
  void OnLowPriorityTimer();

  Config config_;
  std::vector<StartupTask> tasks_;

  base::TimeTicks process_start_time_;
  base::TimeTicks first_paint_time_;
  base::TimeDelta total_deferred_init_time_;

  base::OneShotTimer high_priority_timer_;
  base::OneShotTimer medium_priority_timer_;
  base::OneShotTimer low_priority_timer_;

  bool first_paint_received_ = false;

  SEQUENCE_CHECKER(sequence_checker_);
};

}  // namespace performance
}  // namespace vigo

#endif  // VIGO_COMPONENTS_PERFORMANCE_VIGO_STARTUP_CONTROLLER_H_

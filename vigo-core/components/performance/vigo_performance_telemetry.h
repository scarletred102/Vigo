// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_PERFORMANCE_VIGO_PERFORMANCE_TELEMETRY_H_
#define VIGO_COMPONENTS_PERFORMANCE_VIGO_PERFORMANCE_TELEMETRY_H_

#include <string>
#include <vector>

#include "base/sequence_checker.h"
#include "base/time/time.h"
#include "base/timer/timer.h"
#include "base/values.h"
#include "vigo/components/performance/vigo_memory_budget_controller.h"
#include "vigo/components/performance/vigo_process_manager.h"
#include "vigo/components/performance/vigo_tab_lifecycle_manager.h"

namespace base {
class FilePath;
}  // namespace base

namespace vigo {
namespace performance {

// A time-stamped performance metric sample.
struct PerfSample {
  base::TimeTicks timestamp;

  // Memory (bytes).
  size_t total_rss = 0;
  size_t budget_limit = 0;
  double budget_utilisation = 0.0;

  // Process counts.
  int renderer_count = 0;
  int total_process_count = 0;

  // Tab states.
  int active_tabs = 0;
  int idle_tabs = 0;
  int suspended_tabs = 0;
  int frozen_tabs = 0;
  int discarded_tabs = 0;

  // CPU (aggregate, fraction of one core).
  double total_cpu_usage = 0.0;

  // Memory pressure level.
  MemoryPressureLevel pressure_level = MemoryPressureLevel::kNone;

  // Tab resume latency (last measured, in milliseconds).
  double last_resume_latency_ms = 0.0;
};

// Aggregate statistics over a telemetry session.
struct PerfSessionStats {
  base::TimeDelta session_duration;

  // Memory.
  size_t peak_rss = 0;
  double avg_budget_utilisation = 0.0;

  // Tab lifecycle.
  int total_suspensions = 0;
  int total_freezes = 0;
  int total_discards = 0;
  double avg_resume_latency_ms = 0.0;
  double p95_resume_latency_ms = 0.0;

  // Startup.
  base::TimeDelta cold_start_time;
  base::TimeDelta time_to_first_paint;

  // Reclamation.
  int total_reclaim_passes = 0;
  size_t total_bytes_reclaimed = 0;
};

// VigoPerformanceTelemetry is the local-only (opt-in) telemetry signal
// bus from PAD §10. It collects periodic performance samples, stores
// them in a rolling buffer, and can export to JSON for CI benchmarking.
//
// PRIVACY: All data is local-only. No network upload. No PII collected.
// Users must explicitly opt-in via vigo://settings.
class VigoPerformanceTelemetry
    : public MemoryBudgetObserver,
      public ProcessManagerObserver,
      public TabLifecycleObserver {
 public:
  // Configuration.
  struct Config {
    // How often to collect a performance sample (seconds).
    int sample_interval_seconds = 10;

    // Maximum number of samples to keep in the rolling buffer.
    int max_samples = 360;  // 1 hour at 10s intervals.

    // Whether telemetry is enabled (opt-in).
    bool enabled = false;
  };

  VigoPerformanceTelemetry();
  explicit VigoPerformanceTelemetry(const Config& config);
  ~VigoPerformanceTelemetry() override;

  VigoPerformanceTelemetry(const VigoPerformanceTelemetry&) = delete;
  VigoPerformanceTelemetry& operator=(const VigoPerformanceTelemetry&) = delete;

  // Wire up data sources. Not owned.
  void SetMemoryBudgetController(VigoMemoryBudgetController* controller);
  void SetProcessManager(VigoProcessManager* manager);
  void SetTabLifecycleManager(VigoTabLifecycleManager* manager);

  // Start/stop collection.
  void Start();
  void Stop();
  bool is_running() const { return is_running_; }

  // Enable/disable (opt-in toggle).
  void SetEnabled(bool enabled);
  bool is_enabled() const { return config_.enabled; }

  // Record specific events.
  void RecordTabResumeLatency(int tab_id, double latency_ms);
  void RecordColdStartTime(base::TimeDelta duration);
  void RecordTimeToFirstPaint(base::TimeDelta duration);

  // Get the current sample buffer.
  const std::vector<PerfSample>& GetSamples() const;

  // Get the latest sample.
  const PerfSample& GetLatestSample() const;

  // Compute aggregate session statistics.
  PerfSessionStats ComputeSessionStats() const;

  // Export all samples to JSON (for CI benchmark reporting).
  base::Value::Dict ExportToJson() const;

  // Write JSON export to a file.
  bool ExportToFile(const base::FilePath& path) const;

  // MemoryBudgetObserver overrides:
  void OnMemoryPressureLevelChanged(MemoryPressureLevel old_level,
                                    MemoryPressureLevel new_level) override;
  void OnMemoryBudgetSnapshot(
      const MemoryBudgetSnapshot& snapshot) override;

  // ProcessManagerObserver overrides:
  void OnProcessAdded(base::ProcessId pid, ProcessType type) override;
  void OnProcessRemoved(base::ProcessId pid, ProcessType type) override;
  void OnProcessSnapshotsUpdated(
      const std::vector<ProcessSnapshot>& snapshots) override;

  // TabLifecycleObserver overrides:
  void OnTabStateChanged(int tab_id,
                         TabState old_state,
                         TabState new_state) override;
  void OnTabDiscarded(int tab_id) override;

 private:
  // Periodic sample collection.
  void OnSampleTimer();

  // Collect a sample from all data sources.
  PerfSample CollectSample() const;

  Config config_;
  bool is_running_ = false;

  // Data sources (not owned).
  VigoMemoryBudgetController* budget_controller_ = nullptr;
  VigoProcessManager* process_manager_ = nullptr;
  VigoTabLifecycleManager* tab_lifecycle_manager_ = nullptr;

  // Rolling sample buffer.
  std::vector<PerfSample> samples_;

  // Counters for session stats.
  int total_suspensions_ = 0;
  int total_freezes_ = 0;
  int total_discards_ = 0;
  std::vector<double> resume_latencies_;
  base::TimeDelta cold_start_time_;
  base::TimeDelta time_to_first_paint_;

  base::TimeTicks session_start_time_;
  base::RepeatingTimer sample_timer_;

  SEQUENCE_CHECKER(sequence_checker_);
};

}  // namespace performance
}  // namespace vigo

#endif  // VIGO_COMPONENTS_PERFORMANCE_VIGO_PERFORMANCE_TELEMETRY_H_

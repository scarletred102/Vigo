// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_PERFORMANCE_VIGO_PROCESS_MANAGER_H_
#define VIGO_COMPONENTS_PERFORMANCE_VIGO_PROCESS_MANAGER_H_

#include <map>
#include <vector>

#include "base/observer_list.h"
#include "base/process/process_handle.h"
#include "base/sequence_checker.h"
#include "base/time/time.h"
#include "base/timer/timer.h"

namespace vigo {
namespace performance {

// Type of Chromium child process.
enum class ProcessType {
  kBrowser,     // The main browser process.
  kRenderer,    // Per-tab/per-site renderer.
  kGpu,         // GPU process.
  kUtility,     // Utility processes (network service, audio, etc.).
  kExtension,   // Extension renderer.
  kPlugin,      // PPAPI plugin (legacy, rare).
};

// Per-process resource snapshot.
struct ProcessSnapshot {
  // OS process ID.
  base::ProcessId pid = 0;

  // Process type.
  ProcessType type = ProcessType::kRenderer;

  // Working set / RSS (bytes).
  size_t working_set_bytes = 0;

  // Private memory (bytes). On Windows, this is private working set.
  size_t private_bytes = 0;

  // CPU usage fraction (0.0–N.0 where N = number of cores).
  double cpu_usage = 0.0;

  // GPU memory allocated by this process (bytes, 0 if unknown).
  size_t gpu_memory_bytes = 0;

  // Number of open file handles.
  int open_handles = 0;

  // Associated tab IDs (for renderer processes).
  std::vector<int> associated_tab_ids;

  // When this snapshot was taken.
  base::TimeTicks sample_time;
};

// Observer interface for process-level events.
class ProcessManagerObserver : public base::CheckedObserver {
 public:
  virtual void OnProcessAdded(base::ProcessId pid, ProcessType type) = 0;
  virtual void OnProcessRemoved(base::ProcessId pid, ProcessType type) = 0;
  virtual void OnProcessSnapshotsUpdated(
      const std::vector<ProcessSnapshot>& snapshots) = 0;
};

// VigoProcessManager tracks all child processes in the browser, samples
// their resource usage periodically, and provides aggregated data to the
// memory budget controller and telemetry system.
//
// It uses the Chromium child process iterator APIs
// (content::BrowserChildProcessHostIterator and
//  content::RenderProcessHost::AllHostsIterator) to discover processes.
//
// For the standalone component, process discovery is abstracted behind
// virtual methods so it can be tested without a full Chromium checkout.
class VigoProcessManager {
 public:
  // Configuration.
  struct Config {
    // How often to sample all process metrics (milliseconds).
    int sample_interval_ms = 5000;  // 5 seconds

    // Whether to collect GPU memory info (may be expensive).
    bool collect_gpu_memory = true;

    // Whether to lower scheduling priority for background renderers.
    bool adjust_background_priority = true;
  };

  VigoProcessManager();
  explicit VigoProcessManager(const Config& config);
  virtual ~VigoProcessManager();

  VigoProcessManager(const VigoProcessManager&) = delete;
  VigoProcessManager& operator=(const VigoProcessManager&) = delete;

  // Start periodic process sampling.
  void Start();

  // Stop periodic process sampling.
  void Stop();

  // Force an immediate sample.
  void SampleNow();

  // --- Process registration (called by browser-process integration) ---

  void RegisterProcess(base::ProcessId pid, ProcessType type);
  void UnregisterProcess(base::ProcessId pid);
  void AssociateTabWithProcess(base::ProcessId pid, int tab_id);
  void DissociateTabFromProcess(base::ProcessId pid, int tab_id);

  // --- OS scheduling hints ---

  // Set a renderer process to background priority (reduced CPU share).
  // Platform-specific: SetProcessPriority on Windows, nice on POSIX.
  void SetProcessBackgroundPriority(base::ProcessId pid, bool background);

  // --- Queries ---

  const ProcessSnapshot* GetSnapshot(base::ProcessId pid) const;
  std::vector<ProcessSnapshot> GetAllSnapshots() const;
  std::vector<ProcessSnapshot> GetSnapshotsByType(ProcessType type) const;

  // Total RSS across all tracked processes.
  size_t GetTotalRss() const;

  // Total private bytes across all tracked processes.
  size_t GetTotalPrivateBytes() const;

  // Number of tracked processes.
  int GetProcessCount() const;

  // Number of renderer processes.
  int GetRendererCount() const;

  // Observer management.
  void AddObserver(ProcessManagerObserver* observer);
  void RemoveObserver(ProcessManagerObserver* observer);

 protected:
  // Virtual for testability — override to inject fake metrics.
  virtual ProcessSnapshot SampleProcess(base::ProcessId pid,
                                        ProcessType type) const;

 private:
  // Timer callback.
  void OnSampleTimer();

  // Registered process info (pre-snapshot).
  struct TrackedProcess {
    base::ProcessId pid = 0;
    ProcessType type = ProcessType::kRenderer;
    std::vector<int> tab_ids;
    ProcessSnapshot last_snapshot;
  };

  Config config_;
  std::map<base::ProcessId, TrackedProcess> processes_;
  base::RepeatingTimer sample_timer_;

  base::ObserverList<ProcessManagerObserver> observers_;

  SEQUENCE_CHECKER(sequence_checker_);
};

}  // namespace performance
}  // namespace vigo

#endif  // VIGO_COMPONENTS_PERFORMANCE_VIGO_PROCESS_MANAGER_H_

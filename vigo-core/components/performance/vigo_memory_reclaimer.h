// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_PERFORMANCE_VIGO_MEMORY_RECLAIMER_H_
#define VIGO_COMPONENTS_PERFORMANCE_VIGO_MEMORY_RECLAIMER_H_

#include <cstddef>

#include "base/sequence_checker.h"
#include "base/time/time.h"
#include "base/timer/timer.h"
#include "vigo/components/performance/vigo_memory_budget_controller.h"
#include "vigo/components/performance/vigo_process_manager.h"

namespace vigo {
namespace performance {

// VigoMemoryReclaimer implements the PAD §5 memory reclamation strategy.
// It listens for memory budget pressure events and actively reclaims
// memory from child processes using OS-level APIs and V8 GC hints.
//
// Reclamation strategies (in escalation order):
//   1. Trim working set (EmptyWorkingSet / MADV_FREE)
//   2. Advise V8 low-memory GC for background renderers
//   3. Purge renderer caches (image, script, font)
//   4. Compact allocator heaps (malloc_trim / HeapCompact)
//   5. Release shared memory segments from idle tabs
//
// This class is owned by the performance controller and operates on
// the browser process UI sequence.
class VigoMemoryReclaimer : public MemoryBudgetObserver {
 public:
  // Configuration knobs.
  struct Config {
    // Minimum interval between reclamation passes.
    int reclaim_cooldown_seconds = 30;

    // Whether to use OS working set trimming.
    bool enable_working_set_trim = true;

    // Whether to advise V8 of memory pressure.
    bool enable_v8_gc_hints = true;

    // Whether to purge renderer caches.
    bool enable_cache_purge = true;

    // Whether to compact the allocator heap.
    bool enable_heap_compact = true;

    // Maximum number of renderer processes to trim per pass.
    int max_trim_per_pass = 5;
  };

  VigoMemoryReclaimer();
  explicit VigoMemoryReclaimer(const Config& config);
  ~VigoMemoryReclaimer() override;

  VigoMemoryReclaimer(const VigoMemoryReclaimer&) = delete;
  VigoMemoryReclaimer& operator=(const VigoMemoryReclaimer&) = delete;

  // Set pointers to the process manager (for enumeration) and budget
  // controller (for pressure level). Not owned.
  void SetProcessManager(VigoProcessManager* process_manager);

  // Force an immediate reclamation pass at the given pressure level.
  void ReclaimNow(MemoryPressureLevel level);

  // MemoryBudgetObserver overrides:
  void OnMemoryPressureLevelChanged(MemoryPressureLevel old_level,
                                    MemoryPressureLevel new_level) override;
  void OnMemoryBudgetSnapshot(
      const MemoryBudgetSnapshot& snapshot) override;

  // Statistics.
  size_t total_bytes_reclaimed() const { return total_bytes_reclaimed_; }
  int total_reclaim_passes() const { return total_reclaim_passes_; }

 private:
  // Execute a reclamation pass at the specified pressure level.
  void ExecuteReclamation(MemoryPressureLevel level);

  // Strategy 1: Trim working set of background processes.
  size_t TrimWorkingSets();

  // Strategy 2: Send V8 memory pressure signal to background renderers.
  void SendV8MemoryPressure();

  // Strategy 3: Purge caches from background renderers.
  void PurgeCaches();

  // Strategy 4: Compact the browser process heap.
  void CompactHeap();

  // Whether we can reclaim (cooldown check).
  bool CanReclaim() const;

  Config config_;
  VigoProcessManager* process_manager_ = nullptr;  // Not owned.
  base::TimeTicks last_reclaim_time_;
  size_t total_bytes_reclaimed_ = 0;
  int total_reclaim_passes_ = 0;

  SEQUENCE_CHECKER(sequence_checker_);
};

}  // namespace performance
}  // namespace vigo

#endif  // VIGO_COMPONENTS_PERFORMANCE_VIGO_MEMORY_RECLAIMER_H_

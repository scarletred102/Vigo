// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_PERFORMANCE_VIGO_PERFORMANCE_CONTROLLER_H_
#define VIGO_COMPONENTS_PERFORMANCE_VIGO_PERFORMANCE_CONTROLLER_H_

#include <memory>

#include "base/sequence_checker.h"
#include "vigo/components/performance/vigo_memory_budget_controller.h"
#include "vigo/components/performance/vigo_memory_reclaimer.h"
#include "vigo/components/performance/vigo_performance_telemetry.h"
#include "vigo/components/performance/vigo_process_manager.h"
#include "vigo/components/performance/vigo_startup_controller.h"
#include "vigo/components/performance/vigo_tab_lifecycle_manager.h"

namespace vigo {
namespace performance {

// VigoPerformanceController is the top-level orchestrator that owns
// and connects all performance subsystems. It is created once in
// VigoBrowserMainParts and wired into the browser process lifecycle.
//
// Ownership graph:
//   VigoPerformanceController
//   ├── VigoMemoryBudgetController
//   │   ├──→ observers: TabLifecycleManager, MemoryReclaimer, Telemetry
//   ├── VigoTabLifecycleManager
//   │   ├──→ observer: Telemetry
//   ├── VigoProcessManager
//   │   ├──→ observer: Telemetry
//   ├── VigoMemoryReclaimer
//   ├── VigoStartupController
//   └── VigoPerformanceTelemetry
class VigoPerformanceController {
 public:
  VigoPerformanceController();
  ~VigoPerformanceController();

  VigoPerformanceController(const VigoPerformanceController&) = delete;
  VigoPerformanceController& operator=(const VigoPerformanceController&) =
      delete;

  // Initialise all subsystems and wire the observer graph.
  void Initialise();

  // Start all periodic timers.
  void Start();

  // Stop all periodic timers and tear down.
  void Shutdown();

  // Called when the first meaningful paint occurs.
  void OnFirstPaint();

  // --- Accessors ---
  VigoMemoryBudgetController* memory_budget() { return memory_budget_.get(); }
  VigoTabLifecycleManager* tab_lifecycle() { return tab_lifecycle_.get(); }
  VigoProcessManager* process_manager() { return process_manager_.get(); }
  VigoMemoryReclaimer* memory_reclaimer() { return memory_reclaimer_.get(); }
  VigoStartupController* startup_controller() {
    return startup_controller_.get();
  }
  VigoPerformanceTelemetry* telemetry() { return telemetry_.get(); }

 private:
  std::unique_ptr<VigoMemoryBudgetController> memory_budget_;
  std::unique_ptr<VigoTabLifecycleManager> tab_lifecycle_;
  std::unique_ptr<VigoProcessManager> process_manager_;
  std::unique_ptr<VigoMemoryReclaimer> memory_reclaimer_;
  std::unique_ptr<VigoStartupController> startup_controller_;
  std::unique_ptr<VigoPerformanceTelemetry> telemetry_;

  bool initialised_ = false;

  SEQUENCE_CHECKER(sequence_checker_);
};

}  // namespace performance
}  // namespace vigo

#endif  // VIGO_COMPONENTS_PERFORMANCE_VIGO_PERFORMANCE_CONTROLLER_H_

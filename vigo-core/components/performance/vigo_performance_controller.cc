// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/performance/vigo_performance_controller.h"

#include "base/logging.h"

namespace vigo {
namespace performance {

VigoPerformanceController::VigoPerformanceController() {
  DETACH_FROM_SEQUENCE(sequence_checker_);
}

VigoPerformanceController::~VigoPerformanceController() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  if (initialised_) {
    Shutdown();
  }
}

void VigoPerformanceController::Initialise() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  if (initialised_)
    return;

  VLOG(1) << "VigoPerformanceController: Initialising subsystems";

  // Create subsystems.
  memory_budget_ = std::make_unique<VigoMemoryBudgetController>();
  tab_lifecycle_ = std::make_unique<VigoTabLifecycleManager>();
  process_manager_ = std::make_unique<VigoProcessManager>();
  memory_reclaimer_ = std::make_unique<VigoMemoryReclaimer>();
  startup_controller_ = std::make_unique<VigoStartupController>();
  telemetry_ = std::make_unique<VigoPerformanceTelemetry>();

  // Wire the observer graph.
  // MemoryBudgetController → TabLifecycleManager (pressure-driven transitions)
  memory_budget_->AddObserver(tab_lifecycle_.get());

  // MemoryBudgetController → MemoryReclaimer (pressure-driven reclamation)
  memory_budget_->AddObserver(memory_reclaimer_.get());

  // MemoryBudgetController → Telemetry (recording)
  memory_budget_->AddObserver(telemetry_.get());

  // ProcessManager → Telemetry (process events)
  process_manager_->AddObserver(telemetry_.get());

  // TabLifecycleManager → Telemetry (state change events)
  tab_lifecycle_->AddObserver(telemetry_.get());

  // MemoryReclaimer needs process manager for enumeration.
  memory_reclaimer_->SetProcessManager(process_manager_.get());

  // Telemetry needs all data sources.
  telemetry_->SetMemoryBudgetController(memory_budget_.get());
  telemetry_->SetProcessManager(process_manager_.get());
  telemetry_->SetTabLifecycleManager(tab_lifecycle_.get());

  initialised_ = true;
  VLOG(1) << "VigoPerformanceController: All subsystems initialised, "
          << "observer graph wired";
}

void VigoPerformanceController::Start() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  if (!initialised_) {
    Initialise();
  }

  VLOG(1) << "VigoPerformanceController: Starting all subsystems";

  memory_budget_->Start();
  tab_lifecycle_->Start();
  process_manager_->Start();
  // MemoryReclaimer is event-driven (no own timer).
  // StartupController is event-driven (armed by OnFirstPaint).
  // Telemetry starts only if enabled (opt-in).
}

void VigoPerformanceController::Shutdown() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  if (!initialised_)
    return;

  VLOG(1) << "VigoPerformanceController: Shutting down";

  // Stop timers first.
  telemetry_->Stop();
  tab_lifecycle_->Stop();
  process_manager_->Stop();
  memory_budget_->Stop();

  // Remove observers before destroying to avoid dangling pointers.
  memory_budget_->RemoveObserver(tab_lifecycle_.get());
  memory_budget_->RemoveObserver(memory_reclaimer_.get());
  memory_budget_->RemoveObserver(telemetry_.get());
  process_manager_->RemoveObserver(telemetry_.get());
  tab_lifecycle_->RemoveObserver(telemetry_.get());

  initialised_ = false;
}

void VigoPerformanceController::OnFirstPaint() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  if (startup_controller_) {
    startup_controller_->OnFirstPaint();
  }
}

}  // namespace performance
}  // namespace vigo

// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/performance/vigo_startup_controller.h"

#include <algorithm>

#include "base/logging.h"
#include "base/time/time.h"

namespace vigo {
namespace performance {

VigoStartupController::VigoStartupController()
    : VigoStartupController(Config()) {}

VigoStartupController::VigoStartupController(const Config& config)
    : config_(config) {
  DETACH_FROM_SEQUENCE(sequence_checker_);
  process_start_time_ = base::TimeTicks::Now();
}

VigoStartupController::~VigoStartupController() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

void VigoStartupController::RegisterTask(const std::string& name,
                                         StartupPriority priority,
                                         int estimated_cost_ms,
                                         base::OnceClosure callback) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  // CriticalPath tasks execute immediately — they are needed before
  // first paint and cannot be deferred.
  if (priority == StartupPriority::kCriticalPath) {
    VLOG(1) << "VigoStartupController: Running critical task '" << name
            << "' immediately";
    base::TimeTicks start = base::TimeTicks::Now();
    std::move(callback).Run();
    base::TimeDelta duration = base::TimeTicks::Now() - start;

    StartupTask task;
    task.name = name;
    task.priority = priority;
    task.estimated_cost_ms = estimated_cost_ms;
    task.executed = true;
    task.actual_duration = duration;
    tasks_.push_back(std::move(task));

    VLOG(1) << "VigoStartupController: '" << name << "' completed in "
            << duration.InMilliseconds() << "ms";
    return;
  }

  StartupTask task;
  task.name = name;
  task.priority = priority;
  task.estimated_cost_ms = estimated_cost_ms;
  task.callback = std::move(callback);
  task.executed = false;
  tasks_.push_back(std::move(task));

  VLOG(2) << "VigoStartupController: Registered deferred task '" << name
          << "' (priority=" << static_cast<int>(priority)
          << ", est=" << estimated_cost_ms << "ms)";
}

void VigoStartupController::OnFirstPaint() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  if (first_paint_received_)
    return;

  first_paint_received_ = true;
  first_paint_time_ = base::TimeTicks::Now();

  base::TimeDelta time_to_first_paint =
      first_paint_time_ - process_start_time_;
  VLOG(1) << "VigoStartupController: First paint at "
          << time_to_first_paint.InMilliseconds() << "ms";

  // Arm the deferred init timers.
  high_priority_timer_.Start(
      FROM_HERE,
      base::Milliseconds(config_.high_priority_delay_ms),
      base::BindOnce(&VigoStartupController::OnHighPriorityTimer,
                     base::Unretained(this)));

  medium_priority_timer_.Start(
      FROM_HERE,
      base::Milliseconds(config_.medium_priority_delay_ms),
      base::BindOnce(&VigoStartupController::OnMediumPriorityTimer,
                     base::Unretained(this)));

  low_priority_timer_.Start(
      FROM_HERE,
      base::Milliseconds(config_.low_priority_delay_ms),
      base::BindOnce(&VigoStartupController::OnLowPriorityTimer,
                     base::Unretained(this)));
}

void VigoStartupController::EnsureTaskCompleted(const std::string& name) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  for (auto& task : tasks_) {
    if (task.name == name && !task.executed && task.callback) {
      VLOG(1) << "VigoStartupController: Eagerly running lazy task '"
              << name << "'";
      base::TimeTicks start = base::TimeTicks::Now();
      std::move(task.callback).Run();
      task.actual_duration = base::TimeTicks::Now() - start;
      task.executed = true;
      total_deferred_init_time_ += task.actual_duration;
      return;
    }
  }
}

void VigoStartupController::RecordProcessStartTime(base::TimeTicks time) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  process_start_time_ = time;
}

void VigoStartupController::RecordFirstPaintTime(base::TimeTicks time) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  first_paint_time_ = time;
}

base::TimeDelta VigoStartupController::GetTimeToFirstPaint() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  if (first_paint_time_.is_null())
    return base::TimeDelta();
  return first_paint_time_ - process_start_time_;
}

base::TimeDelta VigoStartupController::GetTotalDeferredInitTime() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return total_deferred_init_time_;
}

std::vector<VigoStartupController::TaskProfile>
VigoStartupController::GetTaskProfiles() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  std::vector<TaskProfile> profiles;
  profiles.reserve(tasks_.size());
  for (const auto& task : tasks_) {
    TaskProfile profile;
    profile.name = task.name;
    profile.priority = task.priority;
    profile.executed = task.executed;
    profile.actual_duration = task.actual_duration;
    profiles.push_back(profile);
  }
  return profiles;
}

bool VigoStartupController::AllTasksCompleted() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  for (const auto& task : tasks_) {
    if (!task.executed)
      return false;
  }
  return true;
}

int VigoStartupController::PendingTaskCount() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  int count = 0;
  for (const auto& task : tasks_) {
    if (!task.executed)
      count++;
  }
  return count;
}

// --- Private ---

void VigoStartupController::RunTasksAtPriority(StartupPriority priority) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  base::TimeTicks batch_start = base::TimeTicks::Now();
  base::TimeDelta budget = base::Milliseconds(config_.batch_budget_ms);

  for (auto& task : tasks_) {
    if (task.priority != priority || task.executed || !task.callback)
      continue;

    // Check if we've exceeded the batch budget.
    base::TimeDelta elapsed = base::TimeTicks::Now() - batch_start;
    if (elapsed >= budget) {
      VLOG(2) << "VigoStartupController: Batch budget exhausted ("
              << elapsed.InMilliseconds()
              << "ms), deferring remaining priority-"
              << static_cast<int>(priority) << " tasks";
      break;
    }

    VLOG(2) << "VigoStartupController: Running '" << task.name << "'";
    base::TimeTicks start = base::TimeTicks::Now();
    std::move(task.callback).Run();
    task.actual_duration = base::TimeTicks::Now() - start;
    task.executed = true;
    total_deferred_init_time_ += task.actual_duration;

    VLOG(2) << "VigoStartupController: '" << task.name << "' completed in "
            << task.actual_duration.InMilliseconds() << "ms";
  }
}

void VigoStartupController::OnHighPriorityTimer() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  VLOG(1) << "VigoStartupController: Running HighPriority deferred tasks";
  RunTasksAtPriority(StartupPriority::kHighPriority);
}

void VigoStartupController::OnMediumPriorityTimer() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  VLOG(1) << "VigoStartupController: Running MediumPriority deferred tasks";
  RunTasksAtPriority(StartupPriority::kMediumPriority);
}

void VigoStartupController::OnLowPriorityTimer() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  VLOG(1) << "VigoStartupController: Running LowPriority deferred tasks";
  RunTasksAtPriority(StartupPriority::kLowPriority);
}

}  // namespace performance
}  // namespace vigo

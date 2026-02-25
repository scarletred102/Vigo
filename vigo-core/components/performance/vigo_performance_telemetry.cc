// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/performance/vigo_performance_telemetry.h"

#include <algorithm>
#include <cmath>
#include <numeric>

#include "base/files/file_util.h"
#include "base/json/json_writer.h"
#include "base/logging.h"
#include "base/values.h"

namespace vigo {
namespace performance {

namespace {

const char kPressureLevelNone[] = "none";
const char kPressureLevelModerate[] = "moderate";
const char kPressureLevelCritical[] = "critical";

const char* PressureLevelToString(MemoryPressureLevel level) {
  switch (level) {
    case MemoryPressureLevel::kNone:
      return kPressureLevelNone;
    case MemoryPressureLevel::kModerate:
      return kPressureLevelModerate;
    case MemoryPressureLevel::kCritical:
      return kPressureLevelCritical;
  }
  return kPressureLevelNone;
}

}  // namespace

VigoPerformanceTelemetry::VigoPerformanceTelemetry()
    : VigoPerformanceTelemetry(Config()) {}

VigoPerformanceTelemetry::VigoPerformanceTelemetry(const Config& config)
    : config_(config) {
  DETACH_FROM_SEQUENCE(sequence_checker_);
}

VigoPerformanceTelemetry::~VigoPerformanceTelemetry() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  Stop();
}

void VigoPerformanceTelemetry::SetMemoryBudgetController(
    VigoMemoryBudgetController* controller) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  budget_controller_ = controller;
}

void VigoPerformanceTelemetry::SetProcessManager(
    VigoProcessManager* manager) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  process_manager_ = manager;
}

void VigoPerformanceTelemetry::SetTabLifecycleManager(
    VigoTabLifecycleManager* manager) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  tab_lifecycle_manager_ = manager;
}

void VigoPerformanceTelemetry::Start() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  if (!config_.enabled || is_running_)
    return;

  is_running_ = true;
  session_start_time_ = base::TimeTicks::Now();

  sample_timer_.Start(
      FROM_HERE,
      base::Seconds(config_.sample_interval_seconds),
      base::BindRepeating(&VigoPerformanceTelemetry::OnSampleTimer,
                          base::Unretained(this)));

  VLOG(1) << "VigoPerformanceTelemetry: Started (interval="
          << config_.sample_interval_seconds << "s)";
}

void VigoPerformanceTelemetry::Stop() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  if (!is_running_)
    return;

  sample_timer_.Stop();
  is_running_ = false;
  VLOG(1) << "VigoPerformanceTelemetry: Stopped ("
          << samples_.size() << " samples collected)";
}

void VigoPerformanceTelemetry::SetEnabled(bool enabled) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  config_.enabled = enabled;
  if (enabled && !is_running_) {
    Start();
  } else if (!enabled && is_running_) {
    Stop();
  }
}

void VigoPerformanceTelemetry::RecordTabResumeLatency(int tab_id,
                                                      double latency_ms) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  if (!config_.enabled)
    return;
  resume_latencies_.push_back(latency_ms);
  VLOG(2) << "VigoPerformanceTelemetry: Tab " << tab_id
          << " resume latency " << latency_ms << "ms";
}

void VigoPerformanceTelemetry::RecordColdStartTime(base::TimeDelta duration) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  cold_start_time_ = duration;
}

void VigoPerformanceTelemetry::RecordTimeToFirstPaint(
    base::TimeDelta duration) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  time_to_first_paint_ = duration;
}

const std::vector<PerfSample>& VigoPerformanceTelemetry::GetSamples() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return samples_;
}

const PerfSample& VigoPerformanceTelemetry::GetLatestSample() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  static PerfSample empty;
  return samples_.empty() ? empty : samples_.back();
}

PerfSessionStats VigoPerformanceTelemetry::ComputeSessionStats() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  PerfSessionStats stats;
  stats.session_duration = base::TimeTicks::Now() - session_start_time_;
  stats.cold_start_time = cold_start_time_;
  stats.time_to_first_paint = time_to_first_paint_;
  stats.total_suspensions = total_suspensions_;
  stats.total_freezes = total_freezes_;
  stats.total_discards = total_discards_;

  // Peak RSS.
  for (const auto& sample : samples_) {
    stats.peak_rss = std::max(stats.peak_rss, sample.total_rss);
  }

  // Average budget utilisation.
  if (!samples_.empty()) {
    double sum = 0.0;
    for (const auto& sample : samples_) {
      sum += sample.budget_utilisation;
    }
    stats.avg_budget_utilisation = sum / static_cast<double>(samples_.size());
  }

  // Resume latency stats.
  if (!resume_latencies_.empty()) {
    double sum = std::accumulate(resume_latencies_.begin(),
                                 resume_latencies_.end(), 0.0);
    stats.avg_resume_latency_ms = sum / resume_latencies_.size();

    // P95.
    std::vector<double> sorted = resume_latencies_;
    std::sort(sorted.begin(), sorted.end());
    size_t p95_idx = static_cast<size_t>(
        std::ceil(0.95 * static_cast<double>(sorted.size())) - 1);
    stats.p95_resume_latency_ms = sorted[std::min(p95_idx, sorted.size() - 1)];
  }

  return stats;
}

base::Value::Dict VigoPerformanceTelemetry::ExportToJson() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  base::Value::Dict root;
  root.Set("version", "1.0");
  root.Set("product", "Vigo");
  root.Set("sample_count", static_cast<int>(samples_.size()));

  // Session stats.
  auto stats = ComputeSessionStats();
  base::Value::Dict stats_dict;
  stats_dict.Set("session_duration_seconds",
                 stats.session_duration.InSecondsF());
  stats_dict.Set("peak_rss_mb",
                 static_cast<int>(stats.peak_rss / (1024 * 1024)));
  stats_dict.Set("avg_budget_utilisation", stats.avg_budget_utilisation);
  stats_dict.Set("total_suspensions", stats.total_suspensions);
  stats_dict.Set("total_freezes", stats.total_freezes);
  stats_dict.Set("total_discards", stats.total_discards);
  stats_dict.Set("avg_resume_latency_ms", stats.avg_resume_latency_ms);
  stats_dict.Set("p95_resume_latency_ms", stats.p95_resume_latency_ms);
  stats_dict.Set("cold_start_ms",
                 stats.cold_start_time.InMillisecondsF());
  stats_dict.Set("time_to_first_paint_ms",
                 stats.time_to_first_paint.InMillisecondsF());
  root.Set("session_stats", std::move(stats_dict));

  // Individual samples.
  base::Value::List samples_list;
  for (const auto& sample : samples_) {
    base::Value::Dict s;
    s.Set("total_rss_mb",
          static_cast<int>(sample.total_rss / (1024 * 1024)));
    s.Set("budget_limit_mb",
          static_cast<int>(sample.budget_limit / (1024 * 1024)));
    s.Set("budget_utilisation", sample.budget_utilisation);
    s.Set("renderer_count", sample.renderer_count);
    s.Set("total_process_count", sample.total_process_count);
    s.Set("active_tabs", sample.active_tabs);
    s.Set("idle_tabs", sample.idle_tabs);
    s.Set("suspended_tabs", sample.suspended_tabs);
    s.Set("frozen_tabs", sample.frozen_tabs);
    s.Set("discarded_tabs", sample.discarded_tabs);
    s.Set("total_cpu_usage", sample.total_cpu_usage);
    s.Set("pressure_level",
          PressureLevelToString(sample.pressure_level));
    s.Set("last_resume_latency_ms", sample.last_resume_latency_ms);
    samples_list.Append(std::move(s));
  }
  root.Set("samples", std::move(samples_list));

  return root;
}

bool VigoPerformanceTelemetry::ExportToFile(
    const base::FilePath& path) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  auto json_dict = ExportToJson();
  std::string json;
  if (!base::JSONWriter::WriteWithOptions(
          json_dict, base::JSONWriter::OPTIONS_PRETTY_PRINT, &json)) {
    LOG(ERROR) << "VigoPerformanceTelemetry: Failed to serialise JSON";
    return false;
  }
  return base::WriteFile(path, json);
}

// --- Observer overrides ---

void VigoPerformanceTelemetry::OnMemoryPressureLevelChanged(
    MemoryPressureLevel old_level,
    MemoryPressureLevel new_level) {
  // Pressure level changes are captured in the next periodic sample.
}

void VigoPerformanceTelemetry::OnMemoryBudgetSnapshot(
    const MemoryBudgetSnapshot& /*snapshot*/) {
  // Snapshots from the budget controller are captured in periodic samples.
}

void VigoPerformanceTelemetry::OnProcessAdded(base::ProcessId pid,
                                              ProcessType type) {
  VLOG(2) << "VigoPerformanceTelemetry: Process " << pid
          << " added (type=" << static_cast<int>(type) << ")";
}

void VigoPerformanceTelemetry::OnProcessRemoved(base::ProcessId pid,
                                                ProcessType type) {
  VLOG(2) << "VigoPerformanceTelemetry: Process " << pid << " removed";
}

void VigoPerformanceTelemetry::OnProcessSnapshotsUpdated(
    const std::vector<ProcessSnapshot>& /*snapshots*/) {
  // Captured in the periodic telemetry sample.
}

void VigoPerformanceTelemetry::OnTabStateChanged(int tab_id,
                                                 TabState old_state,
                                                 TabState new_state) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  if (!config_.enabled)
    return;

  if (new_state == TabState::kSuspended)
    total_suspensions_++;
  if (new_state == TabState::kFrozen)
    total_freezes_++;
}

void VigoPerformanceTelemetry::OnTabDiscarded(int tab_id) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  if (!config_.enabled)
    return;
  total_discards_++;
}

// --- Private ---

void VigoPerformanceTelemetry::OnSampleTimer() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  PerfSample sample = CollectSample();
  samples_.push_back(sample);

  // Enforce rolling buffer limit.
  while (static_cast<int>(samples_.size()) > config_.max_samples) {
    samples_.erase(samples_.begin());
  }

  VLOG(3) << "VigoPerformanceTelemetry: Sample #" << samples_.size()
          << " RSS=" << (sample.total_rss / (1024 * 1024)) << "MB"
          << " util=" << (sample.budget_utilisation * 100.0) << "%";
}

PerfSample VigoPerformanceTelemetry::CollectSample() const {
  PerfSample sample;
  sample.timestamp = base::TimeTicks::Now();

  // Memory budget data.
  if (budget_controller_) {
    const auto& snap = budget_controller_->last_snapshot();
    sample.total_rss = snap.current_browser_rss;
    sample.budget_limit = snap.budget_limit;
    sample.budget_utilisation = snap.utilisation;
    sample.pressure_level = snap.pressure_level;
  }

  // Process data.
  if (process_manager_) {
    sample.total_process_count = process_manager_->GetProcessCount();
    sample.renderer_count = process_manager_->GetRendererCount();

    // Compute total CPU.
    auto all = process_manager_->GetAllSnapshots();
    for (const auto& ps : all) {
      sample.total_cpu_usage += ps.cpu_usage;
    }
  }

  // Tab lifecycle data.
  if (tab_lifecycle_manager_) {
    sample.active_tabs = static_cast<int>(
        tab_lifecycle_manager_->GetTabsInState(TabState::kActive).size());
    sample.idle_tabs = static_cast<int>(
        tab_lifecycle_manager_->GetTabsInState(TabState::kIdle).size());
    sample.suspended_tabs = static_cast<int>(
        tab_lifecycle_manager_->GetTabsInState(TabState::kSuspended).size());
    sample.frozen_tabs = static_cast<int>(
        tab_lifecycle_manager_->GetTabsInState(TabState::kFrozen).size());
    sample.discarded_tabs = static_cast<int>(
        tab_lifecycle_manager_->GetTabsInState(TabState::kDiscarded).size());
  }

  // Resume latency (latest).
  if (!resume_latencies_.empty()) {
    sample.last_resume_latency_ms = resume_latencies_.back();
  }

  return sample;
}

}  // namespace performance
}  // namespace vigo

// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/performance/vigo_process_manager.h"

#include <algorithm>

#include "base/logging.h"
#include "base/process/process.h"
#include "base/process/process_metrics.h"

#if BUILDFLAG(IS_WIN)
#include <windows.h>
#endif  // BUILDFLAG(IS_WIN)

namespace vigo {
namespace performance {

VigoProcessManager::VigoProcessManager() : VigoProcessManager(Config()) {}

VigoProcessManager::VigoProcessManager(const Config& config)
    : config_(config) {
  DETACH_FROM_SEQUENCE(sequence_checker_);
}

VigoProcessManager::~VigoProcessManager() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  Stop();
}

void VigoProcessManager::Start() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  sample_timer_.Start(
      FROM_HERE,
      base::Milliseconds(config_.sample_interval_ms),
      base::BindRepeating(&VigoProcessManager::OnSampleTimer,
                          base::Unretained(this)));
  VLOG(1) << "VigoProcessManager: Started (interval="
          << config_.sample_interval_ms << "ms, tracking "
          << processes_.size() << " processes)";
}

void VigoProcessManager::Stop() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  sample_timer_.Stop();
}

void VigoProcessManager::SampleNow() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  OnSampleTimer();
}

// --- Process registration ---

void VigoProcessManager::RegisterProcess(base::ProcessId pid,
                                         ProcessType type) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  if (processes_.count(pid))
    return;

  TrackedProcess tp;
  tp.pid = pid;
  tp.type = type;
  processes_[pid] = tp;

  VLOG(2) << "VigoProcessManager: Registered PID " << pid
          << " type=" << static_cast<int>(type);

  for (auto& observer : observers_) {
    observer.OnProcessAdded(pid, type);
  }
}

void VigoProcessManager::UnregisterProcess(base::ProcessId pid) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  auto it = processes_.find(pid);
  if (it == processes_.end())
    return;

  ProcessType type = it->second.type;
  processes_.erase(it);

  VLOG(2) << "VigoProcessManager: Unregistered PID " << pid;

  for (auto& observer : observers_) {
    observer.OnProcessRemoved(pid, type);
  }
}

void VigoProcessManager::AssociateTabWithProcess(base::ProcessId pid,
                                                 int tab_id) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  auto it = processes_.find(pid);
  if (it == processes_.end())
    return;

  auto& tabs = it->second.tab_ids;
  if (std::find(tabs.begin(), tabs.end(), tab_id) == tabs.end()) {
    tabs.push_back(tab_id);
  }
}

void VigoProcessManager::DissociateTabFromProcess(base::ProcessId pid,
                                                  int tab_id) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  auto it = processes_.find(pid);
  if (it == processes_.end())
    return;

  auto& tabs = it->second.tab_ids;
  tabs.erase(std::remove(tabs.begin(), tabs.end(), tab_id), tabs.end());
}

void VigoProcessManager::SetProcessBackgroundPriority(base::ProcessId pid,
                                                      bool background) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  if (!config_.adjust_background_priority)
    return;

#if BUILDFLAG(IS_WIN)
  HANDLE process =
      ::OpenProcess(PROCESS_SET_INFORMATION, FALSE, static_cast<DWORD>(pid));
  if (process) {
    DWORD priority_class =
        background ? IDLE_PRIORITY_CLASS : NORMAL_PRIORITY_CLASS;
    ::SetPriorityClass(process, priority_class);
    ::CloseHandle(process);
    VLOG(2) << "VigoProcessManager: PID " << pid
            << " priority → "
            << (background ? "IDLE" : "NORMAL");
  }
#elif BUILDFLAG(IS_POSIX)
  // On POSIX, use nice value adjustment. Background = nice 10.
  // This requires appropriate permissions.
  int nice_value = background ? 10 : 0;
  if (setpriority(PRIO_PROCESS, static_cast<id_t>(pid), nice_value) == 0) {
    VLOG(2) << "VigoProcessManager: PID " << pid
            << " nice → " << nice_value;
  }
#endif
}

// --- Queries ---

const ProcessSnapshot* VigoProcessManager::GetSnapshot(
    base::ProcessId pid) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  auto it = processes_.find(pid);
  return (it != processes_.end()) ? &it->second.last_snapshot : nullptr;
}

std::vector<ProcessSnapshot> VigoProcessManager::GetAllSnapshots() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  std::vector<ProcessSnapshot> result;
  result.reserve(processes_.size());
  for (const auto& [pid, tp] : processes_) {
    result.push_back(tp.last_snapshot);
  }
  return result;
}

std::vector<ProcessSnapshot> VigoProcessManager::GetSnapshotsByType(
    ProcessType type) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  std::vector<ProcessSnapshot> result;
  for (const auto& [pid, tp] : processes_) {
    if (tp.type == type) {
      result.push_back(tp.last_snapshot);
    }
  }
  return result;
}

size_t VigoProcessManager::GetTotalRss() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  size_t total = 0;
  for (const auto& [pid, tp] : processes_) {
    total += tp.last_snapshot.working_set_bytes;
  }
  return total;
}

size_t VigoProcessManager::GetTotalPrivateBytes() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  size_t total = 0;
  for (const auto& [pid, tp] : processes_) {
    total += tp.last_snapshot.private_bytes;
  }
  return total;
}

int VigoProcessManager::GetProcessCount() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return static_cast<int>(processes_.size());
}

int VigoProcessManager::GetRendererCount() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  int count = 0;
  for (const auto& [pid, tp] : processes_) {
    if (tp.type == ProcessType::kRenderer) {
      count++;
    }
  }
  return count;
}

void VigoProcessManager::AddObserver(ProcessManagerObserver* observer) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  observers_.AddObserver(observer);
}

void VigoProcessManager::RemoveObserver(ProcessManagerObserver* observer) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  observers_.RemoveObserver(observer);
}

// --- Protected ---

ProcessSnapshot VigoProcessManager::SampleProcess(base::ProcessId pid,
                                                   ProcessType type) const {
  ProcessSnapshot snap;
  snap.pid = pid;
  snap.type = type;
  snap.sample_time = base::TimeTicks::Now();

  base::Process process = base::Process::OpenWithExtraPrivileges(pid);
  if (!process.IsValid()) {
    return snap;
  }

  auto metrics = base::ProcessMetrics::CreateProcessMetrics(
      process.Handle());
  if (metrics) {
    snap.working_set_bytes = metrics->GetWorkingSetSize();
#if BUILDFLAG(IS_WIN)
    snap.private_bytes = metrics->GetWorkingSetSize();  // Approximate
#else
    snap.private_bytes = metrics->GetWorkingSetSize();
#endif
    snap.cpu_usage = metrics->GetPlatformIndependentCPUUsage().value_or(0.0);
  }

  return snap;
}

// --- Private ---

void VigoProcessManager::OnSampleTimer() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  std::vector<ProcessSnapshot> all_snapshots;
  all_snapshots.reserve(processes_.size());

  for (auto& [pid, tp] : processes_) {
    ProcessSnapshot snap = SampleProcess(pid, tp.type);
    snap.associated_tab_ids = tp.tab_ids;
    tp.last_snapshot = snap;
    all_snapshots.push_back(snap);
  }

  for (auto& observer : observers_) {
    observer.OnProcessSnapshotsUpdated(all_snapshots);
  }
}

}  // namespace performance
}  // namespace vigo

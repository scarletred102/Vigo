// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/performance/vigo_tab_lifecycle_manager.h"

#include <algorithm>

#include "base/logging.h"
#include "base/time/time.h"

namespace vigo {
namespace performance {

VigoTabLifecycleManager::VigoTabLifecycleManager()
    : VigoTabLifecycleManager(Config()) {}

VigoTabLifecycleManager::VigoTabLifecycleManager(const Config& config)
    : config_(config) {
  DETACH_FROM_SEQUENCE(sequence_checker_);
}

VigoTabLifecycleManager::~VigoTabLifecycleManager() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  Stop();
}

void VigoTabLifecycleManager::Start() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  tick_timer_.Start(
      FROM_HERE,
      base::Seconds(config_.tick_interval_seconds),
      base::BindRepeating(&VigoTabLifecycleManager::OnLifecycleTick,
                          base::Unretained(this)));
  VLOG(1) << "VigoTabLifecycleManager: Started (tick="
          << config_.tick_interval_seconds << "s)";
}

void VigoTabLifecycleManager::Stop() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  tick_timer_.Stop();
}

// --- Tab tracking ---

void VigoTabLifecycleManager::OnTabCreated(int tab_id,
                                           const std::string& url) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  TabInfo info;
  info.tab_id = tab_id;
  info.url = url;
  info.state = TabState::kActive;
  info.last_active_time = base::TimeTicks::Now();
  info.state_entry_time = base::TimeTicks::Now();
  tabs_[tab_id] = info;

  VLOG(2) << "VigoTabLifecycleManager: Tab " << tab_id
          << " created (url=" << url << ")";
}

void VigoTabLifecycleManager::OnTabClosed(int tab_id) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  tabs_.erase(tab_id);
  if (active_tab_id_ == tab_id) {
    active_tab_id_ = -1;
  }
  VLOG(2) << "VigoTabLifecycleManager: Tab " << tab_id << " closed";
}

void VigoTabLifecycleManager::OnTabActivated(int tab_id) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  auto it = tabs_.find(tab_id);
  if (it == tabs_.end())
    return;

  // Deactivate the previously active tab.
  if (active_tab_id_ >= 0 && active_tab_id_ != tab_id) {
    OnTabDeactivated(active_tab_id_);
  }

  active_tab_id_ = tab_id;
  TabInfo& info = it->second;
  info.last_active_time = base::TimeTicks::Now();

  // Any non-active tab that becomes active transitions to Active.
  if (info.state != TabState::kActive) {
    TransitionTab(tab_id, TabState::kActive);
  }
}

void VigoTabLifecycleManager::OnTabDeactivated(int tab_id) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  auto it = tabs_.find(tab_id);
  if (it == tabs_.end())
    return;

  TabInfo& info = it->second;
  if (info.state != TabState::kActive)
    return;

  // Move to BackgroundActive if it has audio/WebRTC/capture, else Idle
  // will be handled by the tick timer after idle_timeout_seconds.
  if (info.is_playing_audio || info.has_webrtc || info.is_capturing) {
    TransitionTab(tab_id, TabState::kBackgroundActive);
  }
  // Otherwise remain Active but with state_entry_time updated;
  // the tick timer will transition to Idle after the timeout.
  info.state_entry_time = base::TimeTicks::Now();
}

void VigoTabLifecycleManager::OnTabAudioStateChanged(int tab_id,
                                                     bool is_playing) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  auto it = tabs_.find(tab_id);
  if (it == tabs_.end())
    return;

  it->second.is_playing_audio = is_playing;

  // If audio started and tab is suspended/frozen, promote back.
  if (is_playing && it->second.state != TabState::kActive) {
    TransitionTab(tab_id, TabState::kBackgroundActive);
  }
}

void VigoTabLifecycleManager::OnTabWebRtcStateChanged(int tab_id,
                                                      bool has_webrtc) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  auto it = tabs_.find(tab_id);
  if (it == tabs_.end())
    return;
  it->second.has_webrtc = has_webrtc;
}

void VigoTabLifecycleManager::OnTabDownloadStateChanged(int tab_id,
                                                        bool has_download) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  auto it = tabs_.find(tab_id);
  if (it == tabs_.end())
    return;
  it->second.has_download = has_download;
}

void VigoTabLifecycleManager::OnTabCaptureStateChanged(int tab_id,
                                                       bool is_capturing) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  auto it = tabs_.find(tab_id);
  if (it == tabs_.end())
    return;
  it->second.is_capturing = is_capturing;
}

void VigoTabLifecycleManager::OnTabPinnedStateChanged(int tab_id,
                                                      bool is_pinned) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  auto it = tabs_.find(tab_id);
  if (it == tabs_.end())
    return;
  it->second.is_pinned = is_pinned;
}

void VigoTabLifecycleManager::OnTabMemoryUpdate(int tab_id,
                                                size_t estimated_bytes) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  auto it = tabs_.find(tab_id);
  if (it == tabs_.end())
    return;
  it->second.estimated_memory = estimated_bytes;
}

// --- Queries ---

const TabInfo* VigoTabLifecycleManager::GetTabInfo(int tab_id) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  auto it = tabs_.find(tab_id);
  return (it != tabs_.end()) ? &it->second : nullptr;
}

std::vector<int> VigoTabLifecycleManager::GetTabsInState(
    TabState state) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  std::vector<int> result;
  for (const auto& [id, info] : tabs_) {
    if (info.state == state) {
      result.push_back(id);
    }
  }
  return result;
}

int VigoTabLifecycleManager::GetAliveTabCount() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  int count = 0;
  for (const auto& [id, info] : tabs_) {
    if (info.state != TabState::kDiscarded) {
      count++;
    }
  }
  return count;
}

size_t VigoTabLifecycleManager::GetTotalEstimatedMemory() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  size_t total = 0;
  for (const auto& [id, info] : tabs_) {
    if (info.state != TabState::kDiscarded) {
      total += info.estimated_memory;
    }
  }
  return total;
}

std::vector<int> VigoTabLifecycleManager::GetDiscardCandidates() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  // Collect non-protected, non-active, non-discarded tabs.
  std::vector<std::pair<int, base::TimeTicks>> candidates;
  for (const auto& [id, info] : tabs_) {
    if (info.state == TabState::kActive ||
        info.state == TabState::kDiscarded) {
      continue;
    }
    if (IsProtected(info)) {
      continue;
    }
    candidates.emplace_back(id, info.last_active_time);
  }

  // Sort by last active time (LRU first).
  std::sort(candidates.begin(), candidates.end(),
            [](const auto& a, const auto& b) {
              return a.second < b.second;
            });

  // Pinned tabs go last.
  std::stable_partition(candidates.begin(), candidates.end(),
                        [this](const auto& pair) {
                          auto it = tabs_.find(pair.first);
                          return it != tabs_.end() && !it->second.is_pinned;
                        });

  std::vector<int> result;
  result.reserve(candidates.size());
  for (const auto& [id, _] : candidates) {
    result.push_back(id);
  }
  return result;
}

void VigoTabLifecycleManager::AddObserver(TabLifecycleObserver* observer) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  observers_.AddObserver(observer);
}

void VigoTabLifecycleManager::RemoveObserver(TabLifecycleObserver* observer) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  observers_.RemoveObserver(observer);
}

// --- MemoryBudgetObserver overrides ---

void VigoTabLifecycleManager::OnMemoryPressureLevelChanged(
    MemoryPressureLevel old_level,
    MemoryPressureLevel new_level) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  current_pressure_ = new_level;

  VLOG(1) << "VigoTabLifecycleManager: Memory pressure "
          << static_cast<int>(old_level) << " → "
          << static_cast<int>(new_level);

  // Immediately act on escalation.
  if (new_level > old_level) {
    OnLifecycleTick();
  }
}

void VigoTabLifecycleManager::OnMemoryBudgetSnapshot(
    const MemoryBudgetSnapshot& /*snapshot*/) {
  // The snapshot is informational; we act on pressure level changes.
}

// --- Private ---

bool VigoTabLifecycleManager::IsProtected(const TabInfo& info) const {
  return info.is_playing_audio || info.has_webrtc || info.has_download ||
         info.is_capturing;
}

void VigoTabLifecycleManager::TransitionTab(int tab_id, TabState new_state) {
  auto it = tabs_.find(tab_id);
  if (it == tabs_.end())
    return;

  TabState old_state = it->second.state;
  if (old_state == new_state)
    return;

  it->second.state = new_state;
  it->second.state_entry_time = base::TimeTicks::Now();

  VLOG(2) << "VigoTabLifecycleManager: Tab " << tab_id << " "
          << static_cast<int>(old_state) << " → "
          << static_cast<int>(new_state);

  for (auto& observer : observers_) {
    observer.OnTabStateChanged(tab_id, old_state, new_state);
    if (new_state == TabState::kDiscarded) {
      observer.OnTabDiscarded(tab_id);
    }
  }
}

void VigoTabLifecycleManager::OnLifecycleTick() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  base::TimeTicks now = base::TimeTicks::Now();

  // Phase 1: Active → Idle (timeout-based, no pressure required).
  for (auto& [id, info] : tabs_) {
    if (id == active_tab_id_)
      continue;

    if (info.state == TabState::kActive) {
      base::TimeDelta since_entry = now - info.state_entry_time;
      if (since_entry >= base::Seconds(config_.idle_timeout_seconds) &&
          !IsProtected(info)) {
        TransitionTab(id, TabState::kIdle);
      } else if (since_entry >= base::Seconds(config_.idle_timeout_seconds) &&
                 IsProtected(info)) {
        TransitionTab(id, TabState::kBackgroundActive);
      }
    }
  }

  // Phase 2: Pressure-driven transitions.
  if (current_pressure_ >= MemoryPressureLevel::kModerate) {
    SuspendIdleTabs();
  }
  if (current_pressure_ >= MemoryPressureLevel::kCritical) {
    FreezeSuspendedTabs();
    DiscardFrozenTabs();
  }
}

void VigoTabLifecycleManager::SuspendIdleTabs() {
  base::TimeTicks now = base::TimeTicks::Now();

  for (auto& [id, info] : tabs_) {
    if (info.state != TabState::kIdle)
      continue;
    if (IsProtected(info))
      continue;

    base::TimeDelta since_idle = now - info.state_entry_time;
    if (since_idle >= base::Seconds(config_.suspend_timeout_seconds)) {
      TransitionTab(id, TabState::kSuspended);
    }
  }
}

void VigoTabLifecycleManager::FreezeSuspendedTabs() {
  base::TimeTicks now = base::TimeTicks::Now();

  for (auto& [id, info] : tabs_) {
    if (info.state != TabState::kSuspended)
      continue;
    if (IsProtected(info))
      continue;

    base::TimeDelta since_suspended = now - info.state_entry_time;
    if (since_suspended >= base::Seconds(config_.freeze_timeout_seconds)) {
      TransitionTab(id, TabState::kFrozen);
    }
  }
}

void VigoTabLifecycleManager::DiscardFrozenTabs() {
  base::TimeTicks now = base::TimeTicks::Now();
  int alive = GetAliveTabCount();
  int discarded_this_tick = 0;

  auto candidates = GetDiscardCandidates();
  for (int id : candidates) {
    if (alive <= config_.min_tabs_alive)
      break;
    if (discarded_this_tick >= config_.max_discards_per_tick)
      break;

    auto it = tabs_.find(id);
    if (it == tabs_.end())
      continue;

    const TabInfo& info = it->second;
    if (info.state != TabState::kFrozen)
      continue;

    base::TimeDelta since_frozen = now - info.state_entry_time;
    if (since_frozen >= base::Seconds(config_.discard_timeout_seconds)) {
      TransitionTab(id, TabState::kDiscarded);
      alive--;
      discarded_this_tick++;
    }
  }
}

}  // namespace performance
}  // namespace vigo

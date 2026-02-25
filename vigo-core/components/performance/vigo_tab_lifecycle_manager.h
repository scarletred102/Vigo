// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_PERFORMANCE_VIGO_TAB_LIFECYCLE_MANAGER_H_
#define VIGO_COMPONENTS_PERFORMANCE_VIGO_TAB_LIFECYCLE_MANAGER_H_

#include <map>
#include <string>
#include <vector>

#include "base/observer_list.h"
#include "base/sequence_checker.h"
#include "base/time/time.h"
#include "base/timer/timer.h"
#include "vigo/components/performance/vigo_memory_budget_controller.h"

namespace vigo {
namespace performance {

// Tab lifecycle states (from the PAD state machine §4.1).
enum class TabState {
  kActive,           // Foreground with recent interaction. Full resources.
  kBackgroundActive, // Background with audio/RTCs/long-running JS.
  kIdle,             // Background, no activity, light throttling eligible.
  kSuspended,        // Script timers paused, paint kept, caches dropped.
  kFrozen,           // Renderer serialised to disk, process may terminate.
  kDiscarded,        // Fully removed from memory; reload to restore.
};

// Per-tab metadata tracked by the lifecycle manager.
struct TabInfo {
  // Unique identifier (maps to content::WebContents*-derived ID).
  int tab_id = 0;

  // Current lifecycle state.
  TabState state = TabState::kActive;

  // When the tab was last in the Active state.
  base::TimeTicks last_active_time;

  // When the tab entered its current state.
  base::TimeTicks state_entry_time;

  // Whether the tab is pinned.
  bool is_pinned = false;

  // Whether the tab is producing audio.
  bool is_playing_audio = false;

  // Whether the tab has an active WebRTC session.
  bool has_webrtc = false;

  // Whether the tab has an ongoing download.
  bool has_download = false;

  // Whether the tab is capturing media (camera/mic).
  bool is_capturing = false;

  // Estimated private memory footprint (bytes).
  size_t estimated_memory = 0;

  // URL (for debug logging only — never persisted to telemetry).
  std::string url;
};

// Observer for tab lifecycle state transitions.
class TabLifecycleObserver : public base::CheckedObserver {
 public:
  virtual void OnTabStateChanged(int tab_id,
                                 TabState old_state,
                                 TabState new_state) = 0;
  virtual void OnTabDiscarded(int tab_id) = 0;
};

// VigoTabLifecycleManager implements the PAD §4 state machine.
// It tracks all open tabs, transitions them through the lifecycle
// states based on activity, memory pressure, and protection rules,
// and coordinates with the MemoryBudgetController.
//
// Protection rules (PAD §4.3 — tabs NOT eligible for suspension):
//   - Tabs producing audio (kBackgroundActive)
//   - Tabs with active WebRTC
//   - Tabs with ongoing downloads
//   - Tabs capturing media (camera/microphone)
//   - Pinned tabs: last to be discarded, fast-resume cached
//
// Timer policy:
//   - Idle transition: 60s after going to background with no activity
//   - Suspend transition: 5 min idle + moderate pressure
//   - Freeze transition: 15 min suspended + critical pressure
//   - Discard eligibility: 30 min frozen + critical pressure
class VigoTabLifecycleManager : public MemoryBudgetObserver {
 public:
  // Configuration knobs.
  struct Config {
    // Seconds after going to background before transition to Idle.
    int idle_timeout_seconds = 60;

    // Seconds of idle before eligible for Suspended (moderate+ pressure).
    int suspend_timeout_seconds = 300;  // 5 min

    // Seconds of suspended before eligible for Frozen (critical pressure).
    int freeze_timeout_seconds = 900;  // 15 min

    // Seconds of frozen before eligible for Discard (critical pressure).
    int discard_timeout_seconds = 1800;  // 30 min

    // How often to check tab timeouts.
    int tick_interval_seconds = 10;

    // Maximum number of tabs to discard per tick.
    int max_discards_per_tick = 2;

    // Never discard below this many tabs.
    int min_tabs_alive = 3;
  };

  VigoTabLifecycleManager();
  explicit VigoTabLifecycleManager(const Config& config);
  ~VigoTabLifecycleManager() override;

  VigoTabLifecycleManager(const VigoTabLifecycleManager&) = delete;
  VigoTabLifecycleManager& operator=(const VigoTabLifecycleManager&) = delete;

  // Start the lifecycle tick timer.
  void Start();

  // Stop the lifecycle tick timer.
  void Stop();

  // --- Tab tracking ---

  // Register a new tab.
  void OnTabCreated(int tab_id, const std::string& url);

  // Remove a tab (closed by user).
  void OnTabClosed(int tab_id);

  // Tab became the foreground tab.
  void OnTabActivated(int tab_id);

  // Tab moved to background.
  void OnTabDeactivated(int tab_id);

  // Update tab properties.
  void OnTabAudioStateChanged(int tab_id, bool is_playing);
  void OnTabWebRtcStateChanged(int tab_id, bool has_webrtc);
  void OnTabDownloadStateChanged(int tab_id, bool has_download);
  void OnTabCaptureStateChanged(int tab_id, bool is_capturing);
  void OnTabPinnedStateChanged(int tab_id, bool is_pinned);
  void OnTabMemoryUpdate(int tab_id, size_t estimated_bytes);

  // --- Queries ---

  // Get current info for a tab.
  const TabInfo* GetTabInfo(int tab_id) const;

  // Get all tabs currently in a given state.
  std::vector<int> GetTabsInState(TabState state) const;

  // Get number of trackable (non-discarded) tabs.
  int GetAliveTabCount() const;

  // Get total estimated memory of all alive tabs.
  size_t GetTotalEstimatedMemory() const;

  // Get the ordered list of discard candidates (least-recently-used first,
  // protected tabs excluded).
  std::vector<int> GetDiscardCandidates() const;

  // Observer management.
  void AddObserver(TabLifecycleObserver* observer);
  void RemoveObserver(TabLifecycleObserver* observer);

  // MemoryBudgetObserver overrides:
  void OnMemoryPressureLevelChanged(MemoryPressureLevel old_level,
                                    MemoryPressureLevel new_level) override;
  void OnMemoryBudgetSnapshot(
      const MemoryBudgetSnapshot& snapshot) override;

 private:
  // Whether a tab is protected from suspension/freeze/discard.
  bool IsProtected(const TabInfo& info) const;

  // Transition a tab to a new state.
  void TransitionTab(int tab_id, TabState new_state);

  // Periodic check of all tab timeouts and pressure-driven transitions.
  void OnLifecycleTick();

  // Attempt to suspend idle tabs to relieve moderate pressure.
  void SuspendIdleTabs();

  // Attempt to freeze suspended tabs under critical pressure.
  void FreezeSuspendedTabs();

  // Attempt to discard frozen tabs under sustained critical pressure.
  void DiscardFrozenTabs();

  Config config_;
  MemoryPressureLevel current_pressure_ = MemoryPressureLevel::kNone;
  std::map<int, TabInfo> tabs_;
  int active_tab_id_ = -1;
  base::RepeatingTimer tick_timer_;

  base::ObserverList<TabLifecycleObserver> observers_;

  SEQUENCE_CHECKER(sequence_checker_);
};

}  // namespace performance
}  // namespace vigo

#endif  // VIGO_COMPONENTS_PERFORMANCE_VIGO_TAB_LIFECYCLE_MANAGER_H_

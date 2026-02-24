// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_ADBLOCK_VIGO_FILTER_LIST_MANAGER_H_
#define VIGO_COMPONENTS_ADBLOCK_VIGO_FILTER_LIST_MANAGER_H_

#include <string>
#include <vector>

#include "base/files/file_path.h"
#include "base/functional/callback.h"
#include "base/memory/weak_ptr.h"
#include "base/sequence_checker.h"
#include "base/time/time.h"

namespace vigo {
namespace adblock {

// Metadata for a single filter list.
struct FilterListInfo {
  // Unique identifier (e.g., "easylist", "easyprivacy").
  std::string id;

  // Human-readable title.
  std::string title;

  // URL to download the filter list from.
  std::string url;

  // Local cache file path (inside user data directory).
  base::FilePath cache_path;

  // Whether this list is enabled.
  bool enabled = true;

  // Whether this list is built-in (cannot be removed, only disabled).
  bool builtin = false;

  // Timestamp of last successful update.
  base::Time last_updated;

  // Number of rules in this list.
  size_t rule_count = 0;
};

// Callback invoked after filter list update completes.
// |success| is true if at least one list was updated successfully.
using FilterListUpdateCallback = base::OnceCallback<void(bool success)>;

// VigoFilterListManager handles downloading, caching, and auto-updating
// adblock filter lists. It manages the on-disk cache and provides the
// rule text to VigoAdblockService for loading into the Rust engine.
//
// Default lists (shipped with Vigo):
//   - EasyList (ad blocking)
//   - EasyPrivacy (tracker blocking)
//
// Auto-update interval: every 24 hours (configurable).
//
// Thread safety: all public methods must be called on the UI sequence.
// Network requests and file I/O are dispatched to the thread pool.
class VigoFilterListManager {
 public:
  VigoFilterListManager();
  ~VigoFilterListManager();

  VigoFilterListManager(const VigoFilterListManager&) = delete;
  VigoFilterListManager& operator=(const VigoFilterListManager&) = delete;

  // Initialise with the user data directory for cache storage.
  void Init(const base::FilePath& user_data_dir);

  // Returns all registered filter lists.
  const std::vector<FilterListInfo>& GetFilterLists() const;

  // Returns only the cache file paths of enabled lists.
  std::vector<base::FilePath> GetEnabledFilterListPaths() const;

  // Trigger an immediate update of all enabled filter lists.
  // |callback| is called on the UI sequence when complete.
  void UpdateAllLists(FilterListUpdateCallback callback);

  // Enable or disable a specific filter list.
  void SetListEnabled(const std::string& list_id, bool enabled);

  // Add a custom filter list URL.
  void AddCustomList(const std::string& title, const std::string& url);

  // Remove a custom (non-builtin) filter list.
  bool RemoveCustomList(const std::string& list_id);

  // Start the auto-update timer (24-hour interval).
  void StartAutoUpdate();

  // Stop the auto-update timer.
  void StopAutoUpdate();

 private:
  // Register the default built-in filter lists.
  void RegisterDefaultLists();

  // Download a single filter list and write to cache.
  void DownloadList(const std::string& list_id);

  // Called when a list download completes.
  void OnListDownloaded(const std::string& list_id,
                        const std::string& content,
                        bool success);

  // Called when the auto-update timer fires.
  void OnAutoUpdateTimer();

  base::FilePath cache_dir_;
  std::vector<FilterListInfo> filter_lists_;
  bool auto_update_active_ = false;

  // Pending update callback.
  FilterListUpdateCallback pending_update_callback_;
  int pending_downloads_ = 0;
  int successful_downloads_ = 0;

  SEQUENCE_CHECKER(sequence_checker_);

  base::WeakPtrFactory<VigoFilterListManager> weak_factory_{this};
};

}  // namespace adblock
}  // namespace vigo

#endif  // VIGO_COMPONENTS_ADBLOCK_VIGO_FILTER_LIST_MANAGER_H_

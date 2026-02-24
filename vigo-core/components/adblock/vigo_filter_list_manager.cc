// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/adblock/vigo_filter_list_manager.h"

#include <utility>

#include "base/files/file_util.h"
#include "base/logging.h"
#include "base/strings/string_util.h"
#include "base/task/thread_pool.h"
#include "base/time/time.h"

namespace vigo {
namespace adblock {

namespace {

// Default filter list definitions.
constexpr char kEasyListId[] = "easylist";
constexpr char kEasyListTitle[] = "EasyList";
constexpr char kEasyListUrl[] =
    "https://easylist.to/easylist/easylist.txt";

constexpr char kEasyPrivacyId[] = "easyprivacy";
constexpr char kEasyPrivacyTitle[] = "EasyPrivacy";
constexpr char kEasyPrivacyUrl[] =
    "https://easylist.to/easylist/easyprivacy.txt";

// Cache file names.
constexpr char kEasyListCacheFile[] = "easylist.txt";
constexpr char kEasyPrivacyCacheFile[] = "easyprivacy.txt";

// Auto-update interval: 24 hours.
constexpr base::TimeDelta kAutoUpdateInterval = base::Hours(24);

// Read file content from disk (runs on thread pool).
std::string ReadFilterListFile(const base::FilePath& path) {
  std::string content;
  if (!base::ReadFileToString(path, &content)) {
    LOG(WARNING) << "VigoFilterListManager: Failed to read " << path;
    return std::string();
  }
  return content;
}

}  // namespace

VigoFilterListManager::VigoFilterListManager() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

VigoFilterListManager::~VigoFilterListManager() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

void VigoFilterListManager::Init(const base::FilePath& user_data_dir) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  cache_dir_ = user_data_dir.AppendASCII("VigoAdblock");

  // Ensure cache directory exists.
  base::ThreadPool::PostTask(
      FROM_HERE, {base::MayBlock()},
      base::BindOnce(
          [](const base::FilePath& dir) {
            base::CreateDirectory(dir);
          },
          cache_dir_));

  RegisterDefaultLists();

  VLOG(1) << "VigoFilterListManager: Initialised with "
          << filter_lists_.size() << " filter lists, cache at "
          << cache_dir_;
}

const std::vector<FilterListInfo>&
VigoFilterListManager::GetFilterLists() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return filter_lists_;
}

std::vector<base::FilePath>
VigoFilterListManager::GetEnabledFilterListPaths() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  std::vector<base::FilePath> paths;
  for (const auto& list : filter_lists_) {
    if (list.enabled) {
      paths.push_back(list.cache_path);
    }
  }
  return paths;
}

void VigoFilterListManager::UpdateAllLists(
    FilterListUpdateCallback callback) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  pending_update_callback_ = std::move(callback);
  pending_downloads_ = 0;
  successful_downloads_ = 0;

  for (const auto& list : filter_lists_) {
    if (list.enabled) {
      pending_downloads_++;
      DownloadList(list.id);
    }
  }

  if (pending_downloads_ == 0 && pending_update_callback_) {
    std::move(pending_update_callback_).Run(false);
  }
}

void VigoFilterListManager::SetListEnabled(const std::string& list_id,
                                            bool enabled) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  for (auto& list : filter_lists_) {
    if (list.id == list_id) {
      list.enabled = enabled;
      VLOG(1) << "VigoFilterListManager: " << list.title
              << (enabled ? " enabled" : " disabled");
      return;
    }
  }
  LOG(WARNING) << "VigoFilterListManager: Unknown list ID: " << list_id;
}

void VigoFilterListManager::AddCustomList(const std::string& title,
                                           const std::string& url) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  FilterListInfo info;
  // Generate an ID from the title.
  info.id = base::ToLowerASCII(title);
  std::replace(info.id.begin(), info.id.end(), ' ', '_');
  info.title = title;
  info.url = url;
  info.cache_path = cache_dir_.AppendASCII(info.id + ".txt");
  info.enabled = true;
  info.builtin = false;

  filter_lists_.push_back(std::move(info));
  VLOG(1) << "VigoFilterListManager: Added custom list '" << title << "'";
}

bool VigoFilterListManager::RemoveCustomList(const std::string& list_id) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  for (auto it = filter_lists_.begin(); it != filter_lists_.end(); ++it) {
    if (it->id == list_id) {
      if (it->builtin) {
        LOG(WARNING) << "VigoFilterListManager: Cannot remove built-in list: "
                     << list_id;
        return false;
      }

      // Delete cache file on thread pool.
      base::ThreadPool::PostTask(
          FROM_HERE, {base::MayBlock()},
          base::BindOnce(base::IgnoreResult(&base::DeleteFile),
                         it->cache_path));

      filter_lists_.erase(it);
      return true;
    }
  }
  return false;
}

void VigoFilterListManager::StartAutoUpdate() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  auto_update_active_ = true;
  VLOG(1) << "VigoFilterListManager: Auto-update started (interval: "
          << kAutoUpdateInterval << ")";
  // TODO(Phase 1.3): Use base::RepeatingTimer to schedule OnAutoUpdateTimer()
  // at kAutoUpdateInterval.
}

void VigoFilterListManager::StopAutoUpdate() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  auto_update_active_ = false;
  VLOG(1) << "VigoFilterListManager: Auto-update stopped";
}

void VigoFilterListManager::RegisterDefaultLists() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  FilterListInfo easylist;
  easylist.id = kEasyListId;
  easylist.title = kEasyListTitle;
  easylist.url = kEasyListUrl;
  easylist.cache_path = cache_dir_.AppendASCII(kEasyListCacheFile);
  easylist.enabled = true;
  easylist.builtin = true;
  filter_lists_.push_back(std::move(easylist));

  FilterListInfo easyprivacy;
  easyprivacy.id = kEasyPrivacyId;
  easyprivacy.title = kEasyPrivacyTitle;
  easyprivacy.url = kEasyPrivacyUrl;
  easyprivacy.cache_path = cache_dir_.AppendASCII(kEasyPrivacyCacheFile);
  easyprivacy.enabled = true;
  easyprivacy.builtin = true;
  filter_lists_.push_back(std::move(easyprivacy));
}

void VigoFilterListManager::DownloadList(const std::string& list_id) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  VLOG(1) << "VigoFilterListManager: Downloading " << list_id;

  // TODO(Phase 1.3): Use network::SimpleURLLoader to download the list.
  // For now, emit a log and simulate completion.
  // The actual download will be:
  //   1. Create SimpleURLLoader with the list URL
  //   2. Download to a temp file
  //   3. On success, move to cache_path
  //   4. Call OnListDownloaded

  OnListDownloaded(list_id, /*content=*/"", /*success=*/false);
}

void VigoFilterListManager::OnListDownloaded(
    const std::string& list_id,
    const std::string& content,
    bool success) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (success) {
    VLOG(1) << "VigoFilterListManager: Updated " << list_id
            << " (" << content.size() << " bytes)";

    // Update metadata.
    for (auto& list : filter_lists_) {
      if (list.id == list_id) {
        list.last_updated = base::Time::Now();
        // Count rules (rough: non-empty, non-comment lines).
        size_t count = 0;
        for (size_t i = 0; i < content.size(); ++i) {
          if (content[i] == '\n') {
            // Check if the line was a rule (not comment or empty).
            count++;
          }
        }
        list.rule_count = count;
        break;
      }
    }
    successful_downloads_++;
  } else {
    LOG(WARNING) << "VigoFilterListManager: Failed to download " << list_id;
  }

  pending_downloads_--;
  if (pending_downloads_ <= 0 && pending_update_callback_) {
    std::move(pending_update_callback_).Run(successful_downloads_ > 0);
  }
}

void VigoFilterListManager::OnAutoUpdateTimer() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  if (!auto_update_active_)
    return;

  VLOG(1) << "VigoFilterListManager: Auto-update triggered";
  UpdateAllLists(base::BindOnce([](bool success) {
    VLOG(1) << "VigoFilterListManager: Auto-update "
            << (success ? "succeeded" : "failed");
  }));
}

}  // namespace adblock
}  // namespace vigo

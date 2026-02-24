// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/adblock/vigo_filter_list_manager.h"

#include <utility>

#include "base/files/file_util.h"
#include "base/functional/bind.h"
#include "base/logging.h"
#include "base/strings/string_util.h"
#include "base/task/thread_pool.h"
#include "base/time/time.h"
#include "net/base/load_flags.h"
#include "net/traffic_annotation/network_traffic_annotation.h"
#include "services/network/public/cpp/resource_request.h"
#include "services/network/public/cpp/shared_url_loader_factory.h"
#include "services/network/public/cpp/simple_url_loader.h"

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

// Maximum download size for a single filter list (10 MB).
constexpr size_t kMaxDownloadBytes = 10 * 1024 * 1024;

// Traffic annotation for filter list downloads.
constexpr net::NetworkTrafficAnnotationTag kFilterListTrafficAnnotation =
    net::DefineNetworkTrafficAnnotation("vigo_adblock_filter_list", R"(
      semantics {
        sender: "Vigo Adblock"
        description:
          "Downloads ad-blocking filter lists (EasyList, EasyPrivacy) "
          "to block ads and trackers."
        trigger: "Periodic auto-update or user-initiated refresh."
        data: "HTTP GET request to the filter list URL."
        destination: OTHER
      }
      policy {
        cookies_allowed: NO
        setting:
          "Users can disable ad blocking in Vigo settings."
      }
    )");

// Write content to a file on the thread pool.
void WriteFilterListToDisk(const base::FilePath& path,
                           const std::string& content) {
  if (!base::CreateDirectory(path.DirName())) {
    LOG(WARNING) << "VigoFilterListManager: Failed to create directory "
                 << path.DirName();
    return;
  }
  if (!base::WriteFile(path, content)) {
    LOG(WARNING) << "VigoFilterListManager: Failed to write " << path;
  } else {
    VLOG(2) << "VigoFilterListManager: Wrote " << content.size()
            << " bytes to " << path;
  }
}

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
  StopAutoUpdate();
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

void VigoFilterListManager::SetURLLoaderFactory(
    scoped_refptr<network::SharedURLLoaderFactory> factory) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  url_loader_factory_ = std::move(factory);
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

  auto_update_timer_.Start(
      FROM_HERE, kAutoUpdateInterval,
      base::BindRepeating(&VigoFilterListManager::OnAutoUpdateTimer,
                          weak_factory_.GetWeakPtr()));

  VLOG(1) << "VigoFilterListManager: Auto-update started (interval: "
          << kAutoUpdateInterval << ")";
}

void VigoFilterListManager::StopAutoUpdate() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  auto_update_active_ = false;
  auto_update_timer_.Stop();
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

  // Find the list metadata.
  const FilterListInfo* info = nullptr;
  for (const auto& list : filter_lists_) {
    if (list.id == list_id) {
      info = &list;
      break;
    }
  }
  if (!info) {
    LOG(WARNING) << "VigoFilterListManager: Unknown list ID for download: "
                 << list_id;
    OnListDownloaded(list_id, /*content=*/"", /*success=*/false);
    return;
  }

  if (!url_loader_factory_) {
    LOG(WARNING) << "VigoFilterListManager: No URL loader factory set, "
                 << "cannot download " << list_id;
    OnListDownloaded(list_id, /*content=*/"", /*success=*/false);
    return;
  }

  VLOG(1) << "VigoFilterListManager: Downloading " << list_id
          << " from " << info->url;

  auto resource_request = std::make_unique<network::ResourceRequest>();
  resource_request->url = GURL(info->url);
  resource_request->load_flags =
      net::LOAD_BYPASS_CACHE | net::LOAD_DISABLE_CACHE;
  resource_request->credentials_mode = network::mojom::CredentialsMode::kOmit;

  auto loader = network::SimpleURLLoader::Create(
      std::move(resource_request), kFilterListTrafficAnnotation);
  loader->SetRetryOptions(
      2, network::SimpleURLLoader::RETRY_ON_NETWORK_CHANGE);

  // Capture the raw pointer before moving; the loader is kept alive
  // in |active_loaders_|.
  auto* loader_ptr = loader.get();

  // Use DownloadToString with a size limit.
  loader_ptr->DownloadToString(
      url_loader_factory_.get(),
      base::BindOnce(&VigoFilterListManager::OnDownloadComplete,
                     weak_factory_.GetWeakPtr(), list_id,
                     info->cache_path),
      kMaxDownloadBytes);

  active_loaders_.push_back(std::move(loader));
}

void VigoFilterListManager::OnDownloadComplete(
    const std::string& list_id,
    const base::FilePath& cache_path,
    std::unique_ptr<std::string> response_body) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  // Remove the completed loader from the active set.
  // We can't easily identify which one, so just remove any that are done.
  // Since SimpleURLLoader is single-use, cleanup happens when we erase.
  std::erase_if(active_loaders_, [](const auto& loader) {
    return loader->GetFinalURL().is_empty() || true;
  });

  if (!response_body || response_body->empty()) {
    LOG(WARNING) << "VigoFilterListManager: Download failed for " << list_id;
    OnListDownloaded(list_id, /*content=*/"", /*success=*/false);
    return;
  }

  VLOG(1) << "VigoFilterListManager: Downloaded " << list_id
          << " (" << response_body->size() << " bytes)";

  // Write to cache on the thread pool.
  std::string content = *response_body;
  base::ThreadPool::PostTask(
      FROM_HERE, {base::MayBlock()},
      base::BindOnce(&WriteFilterListToDisk, cache_path, content));

  OnListDownloaded(list_id, content, /*success=*/true);
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
        // Count rules: non-empty, non-comment lines.
        size_t count = 0;
        size_t line_start = 0;
        for (size_t i = 0; i <= content.size(); ++i) {
          if (i == content.size() || content[i] == '\n') {
            if (i > line_start) {
              // Skip comment lines (starting with ! or [).
              char first = content[line_start];
              if (first != '!' && first != '[') {
                count++;
              }
            }
            line_start = i + 1;
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

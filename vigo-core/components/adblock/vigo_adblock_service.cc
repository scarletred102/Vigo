// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/adblock/vigo_adblock_service.h"

#include <utility>

#include "base/files/file_util.h"
#include "base/logging.h"
#include "base/task/thread_pool.h"
#include "vigo/components/adblock/ffi/vigo_adblock_ffi.h"
#include "vigo/components/adblock/vigo_filter_list_manager.h"

namespace vigo {
namespace adblock {

namespace {

// Read a filter list file from disk. Runs on the thread pool.
std::string ReadFilterListFromDisk(const base::FilePath& path) {
  std::string content;
  if (!base::PathExists(path)) {
    VLOG(2) << "VigoAdblockService: Filter list not cached yet: " << path;
    return std::string();
  }
  if (!base::ReadFileToString(path, &content)) {
    LOG(WARNING) << "VigoAdblockService: Failed to read filter list: " << path;
    return std::string();
  }
  VLOG(2) << "VigoAdblockService: Read " << content.size()
          << " bytes from " << path;
  return content;
}

}  // namespace

VigoAdblockService::VigoAdblockService() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

VigoAdblockService::~VigoAdblockService() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  if (engine_handle_) {
    vigo_adblock_destroy(engine_handle_);
    engine_handle_ = nullptr;
  }
}

void VigoAdblockService::Init(
    const std::vector<base::FilePath>& filter_list_paths) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  DCHECK(!is_ready_) << "AdblockService already initialised";

  VLOG(1) << "VigoAdblockService: Initialising with "
          << filter_list_paths.size() << " filter lists";

  engine_handle_ = vigo_adblock_create();
  if (!engine_handle_) {
    LOG(ERROR) << "VigoAdblockService: Failed to create Rust engine";
    return;
  }

  // Load each cached filter list file. If a cache file doesn't exist yet
  // (first run), the download manager will fetch it asynchronously and
  // we'll call LoadRules() once the download completes.
  for (const auto& path : filter_list_paths) {
    std::string content = ReadFilterListFromDisk(path);
    if (!content.empty()) {
      if (vigo_adblock_load_rules(engine_handle_, content.c_str())) {
        VLOG(1) << "VigoAdblockService: Loaded filter list: " << path;
      } else {
        LOG(WARNING) << "VigoAdblockService: Failed to parse rules from: "
                     << path;
      }
    }
  }

  is_ready_ = true;
  VLOG(1) << "VigoAdblockService: Engine ready";
}

bool VigoAdblockService::ShouldBlock(const GURL& url,
                                      const GURL& source_url) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  if (!is_ready_ || !engine_handle_)
    return false;

  return vigo_adblock_check_url(engine_handle_,
                                 url.spec().c_str(),
                                 source_url.spec().c_str());
}

bool VigoAdblockService::IsReady() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return is_ready_;
}

bool VigoAdblockService::LoadRules(const std::string& rules_text) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  if (!engine_handle_) {
    LOG(ERROR) << "VigoAdblockService: LoadRules called before Init";
    return false;
  }
  bool success = vigo_adblock_load_rules(engine_handle_, rules_text.c_str());
  if (success) {
    VLOG(1) << "VigoAdblockService: Loaded " << rules_text.size()
            << " bytes of rules";
  } else {
    LOG(WARNING) << "VigoAdblockService: Failed to parse rules";
  }
  return success;
}

void VigoAdblockService::SetFilterListManager(
    std::unique_ptr<VigoFilterListManager> manager) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  filter_list_manager_ = std::move(manager);
}

void VigoAdblockService::UpdateFilterLists() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  VLOG(1) << "VigoAdblockService: Updating filter lists";

  if (!filter_list_manager_) {
    LOG(WARNING) << "VigoAdblockService: No filter list manager available";
    return;
  }

  // Trigger async download of all enabled filter lists. Once downloads
  // complete, the manager notifies us and we reload the Rust engine.
  filter_list_manager_->UpdateAllLists(
      base::BindOnce([](bool success) {
        VLOG(1) << "VigoAdblockService: Filter list update "
                << (success ? "succeeded" : "failed");
      }));
}

void VigoAdblockService::Shutdown() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  VLOG(1) << "VigoAdblockService: Shutting down";

  if (filter_list_manager_) {
    filter_list_manager_->StopAutoUpdate();
    filter_list_manager_.reset();
  }

  if (engine_handle_) {
    vigo_adblock_destroy(engine_handle_);
    engine_handle_ = nullptr;
  }
  is_ready_ = false;
}

}  // namespace adblock
}  // namespace vigo

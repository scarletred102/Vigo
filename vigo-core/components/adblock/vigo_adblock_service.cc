// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/adblock/vigo_adblock_service.h"

#include "base/logging.h"
#include "vigo/components/adblock/ffi/vigo_adblock_ffi.h"

namespace vigo {
namespace adblock {

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

  for (const auto& path : filter_list_paths) {
    // TODO(Phase 1.3): Load filter list files and pass to Rust engine.
    VLOG(2) << "VigoAdblockService: Loading filter list: " << path;
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

void VigoAdblockService::UpdateFilterLists() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  VLOG(1) << "VigoAdblockService: Updating filter lists";
  // TODO(Phase 1.3): Trigger async download + reload of filter lists.
}

void VigoAdblockService::Shutdown() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  VLOG(1) << "VigoAdblockService: Shutting down";
  if (engine_handle_) {
    vigo_adblock_destroy(engine_handle_);
    engine_handle_ = nullptr;
  }
  is_ready_ = false;
}

}  // namespace adblock
}  // namespace vigo

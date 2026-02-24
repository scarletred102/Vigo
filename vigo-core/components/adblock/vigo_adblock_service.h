// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_ADBLOCK_VIGO_ADBLOCK_SERVICE_H_
#define VIGO_COMPONENTS_ADBLOCK_VIGO_ADBLOCK_SERVICE_H_

#include <memory>
#include <string>
#include <vector>

#include "base/files/file_path.h"
#include "base/sequence_checker.h"
#include "components/keyed_service/core/keyed_service.h"
#include "url/gurl.h"

namespace vigo {
namespace adblock {

// VigoAdblockService manages the Rust adblock engine lifecycle, filter list
// updates, and provides the C++ interface for URL blocking decisions.
//
// Lifecycle: created per-profile via VigoAdblockServiceFactory. The
// underlying Rust engine is initialised lazily on first use.
//
// Thread safety: all public methods must be called on the UI sequence.
class VigoAdblockService : public KeyedService {
 public:
  VigoAdblockService();
  ~VigoAdblockService() override;

  VigoAdblockService(const VigoAdblockService&) = delete;
  VigoAdblockService& operator=(const VigoAdblockService&) = delete;

  // Initialise the Rust adblock engine with the given filter list paths.
  // Must be called before ShouldBlock().
  void Init(const std::vector<base::FilePath>& filter_list_paths);

  // Returns true if |url| should be blocked, given the |source_url| of
  // the page that initiated the request.
  // Performance target: < 1ms per call.
  bool ShouldBlock(const GURL& url, const GURL& source_url) const;

  // Returns true if the engine is initialised and ready.
  bool IsReady() const;

  // Force an immediate filter list update from upstream sources.
  void UpdateFilterLists();

  // Load filter list rules from a raw text string into the Rust engine.
  // Typically called after reading a cached filter list file from disk.
  bool LoadRules(const std::string& rules_text);

  // Takes ownership of the filter list manager. Called by the factory
  // after construction.
  void SetFilterListManager(
      std::unique_ptr<class VigoFilterListManager> manager);

  // Returns the filter list manager (may be nullptr before factory init).
  VigoFilterListManager* filter_list_manager() const {
    return filter_list_manager_.get();
  }

  // KeyedService:
  void Shutdown() override;

 private:
  // Opaque handle to the Rust adblock engine (managed via FFI).
  // nullptr until Init() completes.
  void* engine_handle_ = nullptr;

  bool is_ready_ = false;

  // Manages filter list downloading, caching, and auto-update.
  std::unique_ptr<class VigoFilterListManager> filter_list_manager_;

  SEQUENCE_CHECKER(sequence_checker_);
};

}  // namespace adblock
}  // namespace vigo

#endif  // VIGO_COMPONENTS_ADBLOCK_VIGO_ADBLOCK_SERVICE_H_

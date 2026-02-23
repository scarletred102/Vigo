// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_BROWSER_VIGO_BROWSER_MAIN_PARTS_H_
#define VIGO_BROWSER_VIGO_BROWSER_MAIN_PARTS_H_

#include "chrome/browser/chrome_browser_main.h"

namespace vigo {

// VigoBrowserMainParts extends Chrome's browser main parts to inject
// Vigo-specific initialisation: privacy engine, adblock, media
// orchestration, and sync client startup.
class VigoBrowserMainParts : public ChromeBrowserMainParts {
 public:
  VigoBrowserMainParts(const content::MainFunctionParams& parameters,
                       StartupData* startup_data);
  ~VigoBrowserMainParts() override;

  VigoBrowserMainParts(const VigoBrowserMainParts&) = delete;
  VigoBrowserMainParts& operator=(const VigoBrowserMainParts&) = delete;

  // ChromeBrowserMainParts overrides:
  int PreCreateThreads() override;
  void PostProfileInit(Profile* profile, bool is_initial_profile) override;
  void PreMainMessageLoopRun() override;
  void PostMainMessageLoopRun() override;

 private:
  // Initialise the Rust adblock engine filter lists.
  void InitAdblockEngine();

  // Start the privacy engine (DoH, anti-fingerprinting, tracker param strip).
  void InitPrivacyEngine();

  // Initialise the Media Orchestration Layer.
  void InitMediaOrchestration();

  // Connect to the self-hosted sync server (if configured).
  void InitSyncClient();
};

}  // namespace vigo

#endif  // VIGO_BROWSER_VIGO_BROWSER_MAIN_PARTS_H_

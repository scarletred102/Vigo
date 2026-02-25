// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_BROWSER_VIGO_BROWSER_MAIN_PARTS_H_
#define VIGO_BROWSER_VIGO_BROWSER_MAIN_PARTS_H_

#include <memory>

#include "chrome/browser/chrome_browser_main.h"

namespace vigo {

namespace media {
class VigoMediaPipelineIntegration;
}  // namespace media

namespace performance {
class VigoPerformanceController;
}  // namespace performance

namespace privacy {
class VigoDoHConfig;
class VigoFingerprintProtection;
class VigoPrivacyEngine;
}  // namespace privacy

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

  // Accessors for subsystem instances (non-owning, may be nullptr).
  privacy::VigoFingerprintProtection* fingerprint_protection() const {
    return fingerprint_protection_.get();
  }
  privacy::VigoDoHConfig* doh_config() const { return doh_config_.get(); }
  privacy::VigoPrivacyEngine* privacy_engine() const {
    return privacy_engine_.get();
  }

  media::VigoMediaPipelineIntegration* media_pipeline() const {
    return media_pipeline_.get();
  }

  performance::VigoPerformanceController* performance_controller() const {
    return performance_controller_.get();
  }

 private:
  // Initialise the Rust adblock engine via the keyed service factory.
  void InitAdblockEngine(Profile* profile);

  // Start the privacy engine (DoH, anti-fingerprinting, tracker param strip).
  void InitPrivacyEngine();

  // Initialise the Media Orchestration Layer.
  void InitMediaOrchestration();

  // Connect to the self-hosted sync server (if configured).
  void InitSyncClient();

  // Start the performance optimisation controller.
  void InitPerformanceController();

  // Owned subsystem instances.
  std::unique_ptr<privacy::VigoPrivacyEngine> privacy_engine_;
  std::unique_ptr<privacy::VigoFingerprintProtection>
      fingerprint_protection_;
  std::unique_ptr<privacy::VigoDoHConfig> doh_config_;
  std::unique_ptr<media::VigoMediaPipelineIntegration> media_pipeline_;
  std::unique_ptr<performance::VigoPerformanceController>
      performance_controller_;
};

}  // namespace vigo

#endif  // VIGO_BROWSER_VIGO_BROWSER_MAIN_PARTS_H_

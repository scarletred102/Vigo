// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/browser/vigo_browser_main_parts.h"

#include "base/logging.h"
#include "vigo/build/config/vigo_buildflags.h"

#if BUILDFLAG(VIGO_ENABLE_ADBLOCK)
#include "vigo/components/adblock/vigo_adblock_service.h"
#include "vigo/components/adblock/vigo_adblock_service_factory.h"
#endif

#if BUILDFLAG(VIGO_ENABLE_PRIVACY_ENGINE)
#include "vigo/components/privacy_engine/vigo_doh_config.h"
#include "vigo/components/privacy_engine/vigo_fingerprint_protection.h"
#include "vigo/components/privacy_engine/vigo_privacy_engine.h"
#endif

#if BUILDFLAG(VIGO_ENABLE_MEDIA_ORCHESTRATION)
#include "vigo/components/media_orchestration/vigo_media_orchestration_layer.h"
#endif

#if BUILDFLAG(VIGO_ENABLE_SYNC)
#include "vigo/components/sync/vigo_sync_client.h"
#endif

namespace vigo {

VigoBrowserMainParts::VigoBrowserMainParts(
    const content::MainFunctionParams& parameters,
    StartupData* startup_data)
    : ChromeBrowserMainParts(parameters, startup_data) {}

VigoBrowserMainParts::~VigoBrowserMainParts() = default;

int VigoBrowserMainParts::PreCreateThreads() {
  int result = ChromeBrowserMainParts::PreCreateThreads();
  if (result != content::RESULT_CODE_NORMAL_EXIT)
    return result;

  VLOG(1) << "VigoBrowserMainParts: PreCreateThreads completed";
  return content::RESULT_CODE_NORMAL_EXIT;
}

void VigoBrowserMainParts::PostProfileInit(Profile* profile,
                                           bool is_initial_profile) {
  ChromeBrowserMainParts::PostProfileInit(profile, is_initial_profile);

  if (is_initial_profile) {
    InitAdblockEngine(profile);
    InitPrivacyEngine();
  }
}

void VigoBrowserMainParts::PreMainMessageLoopRun() {
  ChromeBrowserMainParts::PreMainMessageLoopRun();

  InitMediaOrchestration();
  InitSyncClient();

  VLOG(1) << "VigoBrowserMainParts: All Vigo subsystems initialised";
}

void VigoBrowserMainParts::PostMainMessageLoopRun() {
  VLOG(1) << "VigoBrowserMainParts: Shutting down Vigo subsystems";
  ChromeBrowserMainParts::PostMainMessageLoopRun();
}

void VigoBrowserMainParts::InitAdblockEngine(Profile* profile) {
#if BUILDFLAG(VIGO_ENABLE_ADBLOCK)
  VLOG(1) << "VigoBrowserMainParts: Initialising adblock engine";

  // Force-create the adblock service for the initial profile. This
  // ensures the Rust engine is loaded and filter lists are cached before
  // the first page load. Subsequent profiles will lazily create their
  // services via the factory when the first request arrives.
  auto* service =
      adblock::VigoAdblockServiceFactory::GetForBrowserContext(profile);
  if (service && service->IsReady()) {
    VLOG(1) << "VigoBrowserMainParts: Adblock engine ready for profile";
  } else {
    LOG(WARNING) << "VigoBrowserMainParts: Adblock engine not ready yet "
                 << "(filter lists may still be downloading)";
  }
#endif
}

void VigoBrowserMainParts::InitPrivacyEngine() {
#if BUILDFLAG(VIGO_ENABLE_PRIVACY_ENGINE)
  VLOG(1) << "VigoBrowserMainParts: Initialising privacy engine";

  // Initialise the privacy engine with default configuration.
  privacy::PrivacyConfig config;
  privacy_engine_ = std::make_unique<privacy::VigoPrivacyEngine>();
  privacy_engine_->Init(config);

  // Initialise fingerprint protection (generates per-session noise seed).
  fingerprint_protection_ =
      std::make_unique<privacy::VigoFingerprintProtection>();
  VLOG(1) << "VigoBrowserMainParts: Fingerprint protection active "
          << "(session seed generated)";

  // Initialise DoH configuration.
  doh_config_ = std::make_unique<privacy::VigoDoHConfig>();
  if (doh_config_->IsEnabled()) {
    VLOG(1) << "VigoBrowserMainParts: DoH enabled with provider "
            << doh_config_->GetProviderUrl()
            << " (mode="
            << (doh_config_->GetMode() == privacy::VigoDoHConfig::Mode::kSecure
                    ? "Secure"
                    : "Automatic")
            << ")";
    // TODO(Phase 1.4): Apply DoH config to the network service via
    // chrome::prefs::kDnsOverHttpsMode and kDnsOverHttpsTemplates.
    // This requires hooking into PrefService at profile init time.
  }
#endif
}

void VigoBrowserMainParts::InitMediaOrchestration() {
#if BUILDFLAG(VIGO_ENABLE_MEDIA_ORCHESTRATION)
  VLOG(1) << "VigoBrowserMainParts: Initialising Media Orchestration Layer";
  // TODO(Phase 2.3): Initialise ABR controller, buffer heuristics, DRM policy.
#endif
}

void VigoBrowserMainParts::InitSyncClient() {
#if BUILDFLAG(VIGO_ENABLE_SYNC)
  VLOG(1) << "VigoBrowserMainParts: Initialising sync client";
  // TODO(Phase 3.3): Connect to self-hosted sync server if configured.
#endif
}

}  // namespace vigo

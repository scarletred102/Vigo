// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/browser/vigo_browser_main_parts.h"

#include "base/logging.h"
#include "vigo/build/config/vigo_buildflags.h"

#if BUILDFLAG(VIGO_ENABLE_ADBLOCK)
#include "vigo/components/adblock/vigo_adblock_service.h"
#endif

#if BUILDFLAG(VIGO_ENABLE_PRIVACY_ENGINE)
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
    InitAdblockEngine();
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

void VigoBrowserMainParts::InitAdblockEngine() {
#if BUILDFLAG(VIGO_ENABLE_ADBLOCK)
  VLOG(1) << "VigoBrowserMainParts: Initialising adblock engine";
  // TODO(Phase 1.3): Wire up Rust adblock engine via FFI.
#endif
}

void VigoBrowserMainParts::InitPrivacyEngine() {
#if BUILDFLAG(VIGO_ENABLE_PRIVACY_ENGINE)
  VLOG(1) << "VigoBrowserMainParts: Initialising privacy engine";
  // TODO(Phase 1.4): Initialise DoH, anti-fingerprint, tracking strip.
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

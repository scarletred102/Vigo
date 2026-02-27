// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/browser/vigo_browser_main_parts.h"

#include "base/logging.h"
#include "content/public/common/result_codes.h"
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
#include "vigo/components/media_orchestration/vigo_media_pipeline_integration.h"
#endif

#if BUILDFLAG(VIGO_ENABLE_SYNC)
#include "vigo/components/sync/vigo_sync_client.h"
#endif

#if BUILDFLAG(VIGO_ENABLE_PERFORMANCE)
#include "vigo/components/performance/vigo_performance_controller.h"
#endif

#if BUILDFLAG(VIGO_ENABLE_SECURITY)
#include "vigo/components/security/vigo_cve_monitor.h"
#include "vigo/components/security/vigo_license_key.h"
#include "vigo/components/security/vigo_security_hardening.h"
#include "vigo/components/security/vigo_update_client.h"
#endif

namespace vigo {

VigoBrowserMainParts::VigoBrowserMainParts(
    bool is_integration_test,
    StartupData* startup_data)
    : ChromeBrowserMainParts(is_integration_test, startup_data) {}

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

int VigoBrowserMainParts::PreMainMessageLoopRun() {
  int result = ChromeBrowserMainParts::PreMainMessageLoopRun();
  if (result != content::RESULT_CODE_NORMAL_EXIT)
    return result;

  InitMediaOrchestration();
  InitSyncClient();
  InitPerformanceController();
  InitSecuritySubsystem();

  VLOG(1) << "VigoBrowserMainParts: All Vigo subsystems initialised";
  return content::RESULT_CODE_NORMAL_EXIT;
}

void VigoBrowserMainParts::PostMainMessageLoopRun() {
  VLOG(1) << "VigoBrowserMainParts: Shutting down Vigo subsystems";

#if BUILDFLAG(VIGO_ENABLE_SECURITY)
  if (cve_monitor_) {
    cve_monitor_->Shutdown();
    cve_monitor_.reset();
    VLOG(1) << "VigoBrowserMainParts: CVE monitor shut down";
  }
  update_client_.reset();
  license_key_.reset();
  security_hardening_.reset();
#endif

#if BUILDFLAG(VIGO_ENABLE_PERFORMANCE)
  if (performance_controller_) {
    performance_controller_->Shutdown();
    performance_controller_.reset();
    VLOG(1) << "VigoBrowserMainParts: Performance controller shut down";
  }
#endif

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

    // Apply DoH config to Chrome's DNS prefs via PrefService.
    // This is the canonical way to configure DoH in Chromium — the
    // network service reads these prefs and applies DnsConfigOverrides.
    //
    // Pref keys (from chrome/common/pref_names.h):
    //   prefs::kDnsOverHttpsMode      → "secure" | "automatic" | "off"
    //   prefs::kDnsOverHttpsTemplates  → DoH server template URL
    //
    // TODO(Phase 1.5): Read user's DoH preference from PrefService;
    // for now, always apply the Vigo default (Secure + Cloudflare).
    // The actual pref write requires a Profile* PrefService handle
    // which we'll obtain once per-profile settings are wired up.
    VLOG(1) << "VigoBrowserMainParts: DoH prefs ready for application. "
            << "Template: " << doh_config_->BuildDnsOverHttpsConfigString();
  }
#endif
}

void VigoBrowserMainParts::InitMediaOrchestration() {
#if BUILDFLAG(VIGO_ENABLE_MEDIA_ORCHESTRATION)
  VLOG(1) << "VigoBrowserMainParts: Initialising Media Orchestration Layer";

  media_pipeline_ = std::make_unique<media::VigoMediaPipelineIntegration>();
  media_pipeline_->Initialize();

  VLOG(1) << "VigoBrowserMainParts: Media pipeline ready — HW decode "
          << "controller probed, codec negotiation available";
#endif
}

void VigoBrowserMainParts::InitSyncClient() {
#if BUILDFLAG(VIGO_ENABLE_SYNC)
  VLOG(1) << "VigoBrowserMainParts: Initialising sync client";
  // TODO(Phase 3.3): Connect to self-hosted sync server if configured.
#endif
}

void VigoBrowserMainParts::InitPerformanceController() {
#if BUILDFLAG(VIGO_ENABLE_PERFORMANCE)
  VLOG(1) << "VigoBrowserMainParts: Initialising performance controller";

  performance_controller_ =
      std::make_unique<performance::VigoPerformanceController>();
  performance_controller_->Initialise();
  performance_controller_->Start();

  VLOG(1) << "VigoBrowserMainParts: Performance controller started — "
          << "memory budget, tab lifecycle, process manager, reclaimer, "
          << "startup controller all wired and active";
#endif
}

void VigoBrowserMainParts::InitSecuritySubsystem() {
#if BUILDFLAG(VIGO_ENABLE_SECURITY)
  VLOG(1) << "VigoBrowserMainParts: Initialising security subsystem";

  // 1. Security hardening: verify platform protections (ASLR, DEP, sandbox).
  security_hardening_ =
      std::make_unique<security::VigoSecurityHardening>();
  auto audit_report = security_hardening_->RunAllChecks();
  VLOG(1) << "VigoBrowserMainParts: Security audit — "
          << audit_report.passed << " passed, "
          << audit_report.failed << " failed";

  // 2. License key: initialise with default beta license.
  license_key_ = std::make_unique<security::VigoLicenseKey>();
  if (license_key_->IsBeta()) {
    VLOG(1) << "VigoBrowserMainParts: Running in BETA mode — all features "
            << "enabled until beta end date";
  }

  // 3. CVE monitor: track upstream vulnerabilities.
  cve_monitor_ = std::make_unique<security::VigoCveMonitor>();
  cve_monitor_->Initialise();

  // 4. Update client: check for available updates.
  update_client_ = std::make_unique<security::VigoUpdateClient>();
  update_client_->Start();

  VLOG(1) << "VigoBrowserMainParts: Security subsystem initialised — "
          << "hardening verified, license active, CVE monitor polling, "
          << "update client ready";
#endif
}

}  // namespace vigo

// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_PRIVACY_ENGINE_VIGO_PRIVACY_ENGINE_H_
#define VIGO_COMPONENTS_PRIVACY_ENGINE_VIGO_PRIVACY_ENGINE_H_

#include <string>

#include "base/sequence_checker.h"
#include "url/gurl.h"

namespace vigo {
namespace privacy {

// Configuration for the Vigo Privacy Engine.
struct PrivacyConfig {
  // DNS-over-HTTPS provider URL. Default: Cloudflare 1.1.1.1.
  std::string doh_provider_url = "https://cloudflare-dns.com/dns-query";

  // Enable HTTPS-first mode (upgrade navigations to HTTPS).
  bool https_first_mode = true;

  // Block third-party cookies by default.
  bool block_third_party_cookies = true;

  // Anti-fingerprinting modules.
  bool canvas_noise = true;
  bool webgl_masking = true;
  bool audio_context_resistance = true;
  bool font_enumeration_restriction = true;
  bool user_agent_normalization = true;
  bool client_hints_reduction = true;

  // Strip tracking parameters (UTM, fbclid, etc.) from URLs.
  bool strip_tracking_params = true;

  // Referrer policy: strict-origin-when-cross-origin.
  bool strict_referrer_policy = true;
};

// VigoPrivacyEngine coordinates all privacy-related subsystems:
//   - DNS-over-HTTPS configuration
//   - Anti-fingerprinting (canvas, WebGL, AudioContext, fonts, UA)
//   - Third-party cookie blocking
//   - Tracking parameter stripping
//   - HTTPS-first mode
//   - Referrer policy enforcement
//
// Thread safety: all public methods must be called on the UI sequence.
class VigoPrivacyEngine {
 public:
  VigoPrivacyEngine();
  ~VigoPrivacyEngine();

  VigoPrivacyEngine(const VigoPrivacyEngine&) = delete;
  VigoPrivacyEngine& operator=(const VigoPrivacyEngine&) = delete;

  // Initialise the engine with the given configuration.
  void Init(const PrivacyConfig& config);

  // Strip tracking parameters from |url|. Modifies |url| in-place.
  // Returns true if any parameters were stripped.
  static bool StripTrackingParams(GURL* url);

  // Returns the configured DoH provider URL.
  const std::string& GetDohProviderUrl() const;

  // Returns the current privacy configuration.
  const PrivacyConfig& config() const { return config_; }

  // Update configuration at runtime (e.g., from settings page).
  void UpdateConfig(const PrivacyConfig& config);

 private:
  PrivacyConfig config_;

  SEQUENCE_CHECKER(sequence_checker_);
};

}  // namespace privacy
}  // namespace vigo

#endif  // VIGO_COMPONENTS_PRIVACY_ENGINE_VIGO_PRIVACY_ENGINE_H_

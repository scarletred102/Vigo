// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_PRIVACY_ENGINE_VIGO_DOH_CONFIG_H_
#define VIGO_COMPONENTS_PRIVACY_ENGINE_VIGO_DOH_CONFIG_H_

#include <string>
#include <vector>

#include "base/sequence_checker.h"

namespace vigo {
namespace privacy {

// Known DoH providers with their template URLs.
struct DohProvider {
  const char* name;
  const char* template_url;
  const char* description;
};

// VigoDoHConfig manages DNS-over-HTTPS configuration for the Vigo browser.
//
// Default: Cloudflare 1.1.1.1 (privacy-focused, no logging, fast).
//
// The DoH configuration is applied to the network service via
// DnsConfigOverrides when the browser starts or when the user changes
// their DoH preference in settings.
//
// Supported modes:
//   - Automatic: use DoH if the system resolver supports it (Chrome default)
//   - Secure: always use DoH, block insecure DNS (Vigo default)
//   - Off: disable DoH entirely
//
// Thread safety: all public methods must be called on the UI sequence.
class VigoDoHConfig {
 public:
  // DoH operation mode.
  enum class Mode {
    // Use DoH with automatic fallback to system resolver if DoH fails.
    kAutomatic = 0,
    // Always use DoH; if DoH fails, DNS resolution fails. Most private.
    kSecure = 1,
    // Disable DoH. Use system resolver only.
    kOff = 2,
  };

  VigoDoHConfig();
  ~VigoDoHConfig();

  VigoDoHConfig(const VigoDoHConfig&) = delete;
  VigoDoHConfig& operator=(const VigoDoHConfig&) = delete;

  // Returns the list of built-in DoH providers.
  static const std::vector<DohProvider>& GetBuiltInProviders();

  // Returns the default DoH provider template URL.
  static const char* GetDefaultProviderUrl();

  // Returns the default DoH provider name.
  static const char* GetDefaultProviderName();

  // Set the DoH mode.
  void SetMode(Mode mode);
  Mode GetMode() const;

  // Set the DoH provider template URL.
  // Can be a built-in provider URL or a custom URL.
  void SetProviderUrl(const std::string& template_url);
  const std::string& GetProviderUrl() const;

  // Set a custom DoH server URL (for power users).
  void SetCustomServer(const std::string& template_url);

  // Returns true if DoH is currently enabled (mode != kOff).
  bool IsEnabled() const;

  // Build the DnsOverHttpsConfig string suitable for passing to
  // chrome::prefs or network::mojom::DnsConfigOverrides.
  // Format: "https://example.com/dns-query{?dns}"
  std::string BuildDnsOverHttpsConfigString() const;

  // Validate a DoH template URL. Returns true if the URL is a valid
  // RFC 8484 compatible DoH template.
  static bool IsValidTemplate(const std::string& template_url);

 private:
  Mode mode_ = Mode::kSecure;
  std::string provider_url_;

  SEQUENCE_CHECKER(sequence_checker_);
};

}  // namespace privacy
}  // namespace vigo

#endif  // VIGO_COMPONENTS_PRIVACY_ENGINE_VIGO_DOH_CONFIG_H_

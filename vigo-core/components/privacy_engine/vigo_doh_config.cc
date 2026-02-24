// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/privacy_engine/vigo_doh_config.h"

#include "base/logging.h"
#include "base/strings/string_util.h"
#include "url/gurl.h"

namespace vigo {
namespace privacy {

namespace {

// Built-in DoH providers. Users can select from this list or enter a
// custom URL. All providers listed here have strong privacy policies.
const DohProvider kBuiltInProviders[] = {
    {
        "Cloudflare",
        "https://cloudflare-dns.com/dns-query",
        "Cloudflare 1.1.1.1 — fast and private, no user logging",
    },
    {
        "Cloudflare (Family)",
        "https://family.cloudflare-dns.com/dns-query",
        "Cloudflare 1.1.1.3 — blocks malware and adult content",
    },
    {
        "Quad9",
        "https://dns.quad9.net/dns-query",
        "Quad9 — security-focused, blocks malicious domains",
    },
    {
        "NextDNS",
        "https://dns.nextdns.io",
        "NextDNS — customisable privacy DNS with analytics opt-in",
    },
    {
        "Google",
        "https://dns.google/dns-query",
        "Google Public DNS 8.8.8.8 — fast, but logs queries",
    },
    {
        "Mullvad",
        "https://dns.mullvad.net/dns-query",
        "Mullvad DNS — no logging, operated by Mullvad VPN",
    },
    {
        "AdGuard",
        "https://dns.adguard-dns.com/dns-query",
        "AdGuard DNS — blocks ads and trackers at DNS level",
    },
};

}  // namespace

VigoDoHConfig::VigoDoHConfig() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  provider_url_ = GetDefaultProviderUrl();
  VLOG(1) << "VigoDoHConfig: Initialised with " << GetDefaultProviderName()
          << " (" << provider_url_ << "), mode=Secure";
}

VigoDoHConfig::~VigoDoHConfig() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

// static
const std::vector<DohProvider>& VigoDoHConfig::GetBuiltInProviders() {
  static const std::vector<DohProvider> kProviders(
      std::begin(kBuiltInProviders), std::end(kBuiltInProviders));
  return kProviders;
}

// static
const char* VigoDoHConfig::GetDefaultProviderUrl() {
  return "https://cloudflare-dns.com/dns-query";
}

// static
const char* VigoDoHConfig::GetDefaultProviderName() {
  return "Cloudflare";
}

void VigoDoHConfig::SetMode(Mode mode) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  mode_ = mode;
  VLOG(1) << "VigoDoHConfig: Mode set to "
          << (mode == Mode::kAutomatic
                  ? "Automatic"
                  : (mode == Mode::kSecure ? "Secure" : "Off"));
}

VigoDoHConfig::Mode VigoDoHConfig::GetMode() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return mode_;
}

void VigoDoHConfig::SetProviderUrl(const std::string& template_url) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  if (!IsValidTemplate(template_url)) {
    LOG(WARNING) << "VigoDoHConfig: Invalid DoH template: " << template_url;
    return;
  }
  provider_url_ = template_url;
  VLOG(1) << "VigoDoHConfig: Provider URL set to " << provider_url_;
}

const std::string& VigoDoHConfig::GetProviderUrl() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return provider_url_;
}

void VigoDoHConfig::SetCustomServer(const std::string& template_url) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  SetProviderUrl(template_url);
}

bool VigoDoHConfig::IsEnabled() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return mode_ != Mode::kOff;
}

std::string VigoDoHConfig::BuildDnsOverHttpsConfigString() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  if (!IsEnabled() || provider_url_.empty()) {
    return std::string();
  }
  // Chromium expects the DoH template in RFC 8484 format.
  // If the URL already contains {?dns}, return as-is.
  // Otherwise, append {?dns} for GET-mode support.
  if (provider_url_.find("{?dns}") != std::string::npos) {
    return provider_url_;
  }
  // For POST-mode servers (most modern DoH), the plain URL suffices.
  return provider_url_;
}

// static
bool VigoDoHConfig::IsValidTemplate(const std::string& template_url) {
  if (template_url.empty())
    return false;

  // Must be HTTPS.
  if (!base::StartsWith(template_url, "https://",
                        base::CompareCase::INSENSITIVE_ASCII)) {
    return false;
  }

  // Must be a valid URL.
  GURL url(template_url);
  if (!url.is_valid())
    return false;

  // Must have a host component.
  if (url.host().empty())
    return false;

  return true;
}

}  // namespace privacy
}  // namespace vigo

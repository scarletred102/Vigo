// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/privacy_engine/vigo_privacy_engine.h"

#include <string>
#include <vector>

#include "base/logging.h"
#include "base/strings/string_util.h"
#include "net/base/url_util.h"

namespace vigo {
namespace privacy {

namespace {

// Known tracking query parameters to strip from URLs.
// This list is intentionally conservative — only well-known, widely-used
// tracking parameters are included.
constexpr const char* kTrackingParams[] = {
    // Google Analytics / Campaign
    "utm_source",
    "utm_medium",
    "utm_campaign",
    "utm_term",
    "utm_content",
    "utm_id",
    "utm_source_platform",
    "utm_creative_format",
    "utm_marketing_tactic",
    // Facebook / Meta
    "fbclid",
    "fb_action_ids",
    "fb_action_types",
    "fb_ref",
    "fb_source",
    // Microsoft / Bing
    "msclkid",
    // Google Ads
    "gclid",
    "gclsrc",
    "dclid",
    // Hubspot
    "hsa_cam",
    "hsa_grp",
    "hsa_mt",
    "hsa_src",
    "hsa_ad",
    "hsa_acc",
    "hsa_net",
    "hsa_ver",
    "hsa_la",
    "hsa_ol",
    "hsa_kw",
    // Mailchimp
    "mc_cid",
    "mc_eid",
    // Twitter / X
    "twclid",
    // Yahoo
    "yclid",
    // General
    "_ga",
    "_gl",
    "_hsenc",
    "_hsmi",
    "ref_",
    "ref_src",
};

}  // namespace

VigoPrivacyEngine::VigoPrivacyEngine() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

VigoPrivacyEngine::~VigoPrivacyEngine() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

void VigoPrivacyEngine::Init(const PrivacyConfig& config) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  config_ = config;

  VLOG(1) << "VigoPrivacyEngine: Initialised"
          << " DoH=" << config_.doh_provider_url
          << " HTTPS-first=" << config_.https_first_mode
          << " 3P-cookies-blocked=" << config_.block_third_party_cookies
          << " Canvas-noise=" << config_.canvas_noise
          << " Tracking-strip=" << config_.strip_tracking_params;
}

// static
bool VigoPrivacyEngine::StripTrackingParams(GURL* url) {
  if (!url || !url->is_valid() || !url->has_query())
    return false;

  bool stripped = false;
  GURL::Replacements replacements;
  std::string new_query;
  bool first = true;

  // Parse existing query parameters and rebuild without tracking ones.
  // TODO(Phase 1.4): Use a more efficient query string parser.
  const std::string& query = url->query();
  size_t start = 0;
  while (start < query.size()) {
    size_t end = query.find('&', start);
    if (end == std::string::npos)
      end = query.size();

    std::string param = query.substr(start, end - start);
    size_t eq = param.find('=');
    std::string key = (eq != std::string::npos) ? param.substr(0, eq) : param;

    bool is_tracking = false;
    for (const char* tracking_param : kTrackingParams) {
      if (base::EqualsCaseInsensitiveASCII(key, tracking_param)) {
        is_tracking = true;
        stripped = true;
        break;
      }
    }

    if (!is_tracking) {
      if (!first)
        new_query += '&';
      new_query += param;
      first = false;
    }

    start = end + 1;
  }

  if (stripped) {
    if (new_query.empty()) {
      replacements.ClearQuery();
    } else {
      replacements.SetQueryStr(new_query);
    }
    *url = url->ReplaceComponents(replacements);
  }

  return stripped;
}

const std::string& VigoPrivacyEngine::GetDohProviderUrl() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return config_.doh_provider_url;
}

void VigoPrivacyEngine::UpdateConfig(const PrivacyConfig& config) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  config_ = config;
  VLOG(1) << "VigoPrivacyEngine: Configuration updated";
  // TODO(Phase 1.4): Propagate config changes to active subsystems.
}

}  // namespace privacy
}  // namespace vigo

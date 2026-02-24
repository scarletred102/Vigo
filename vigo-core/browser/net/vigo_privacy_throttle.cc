// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/browser/net/vigo_privacy_throttle.h"

#include "base/logging.h"
#include "net/http/http_request_headers.h"
#include "services/network/public/cpp/resource_request.h"
#include "url/gurl.h"
#include "vigo/build/config/vigo_buildflags.h"
#include "vigo/components/privacy_engine/vigo_privacy_engine.h"

namespace vigo {

VigoPrivacyThrottle::VigoPrivacyThrottle() = default;
VigoPrivacyThrottle::~VigoPrivacyThrottle() = default;

void VigoPrivacyThrottle::WillStartRequest(
    network::ResourceRequest* request,
    bool* defer) {
  if (!request)
    return;

  GURL url = request->url;
  bool modified = false;

  // Strip tracking parameters from URL.
  if (MaybeStripTrackingParams(&url)) {
    request->url = url;
    modified = true;
  }

  // Upgrade HTTP to HTTPS for navigations.
  if (MaybeUpgradeToHttps(&url)) {
    request->url = url;
    modified = true;
  }

  // Sanitize request headers to remove privacy-invasive ones.
  SanitizeHeaders(&request->headers);

  if (modified) {
    VLOG(2) << "VigoPrivacyThrottle: Modified request to "
            << request->url.spec();
  }
}

void VigoPrivacyThrottle::WillRedirectRequest(
    net::RedirectInfo* redirect_info,
    const network::mojom::URLResponseHead& response_head,
    bool* defer,
    std::vector<std::string>* to_be_removed_request_headers,
    net::HttpRequestHeaders* modified_request_headers,
    net::HttpRequestHeaders* modified_cors_exempt_request_headers) {
  if (!redirect_info)
    return;

  GURL url = redirect_info->new_url;

  // Strip tracking params from redirect targets too.
  if (MaybeStripTrackingParams(&url)) {
    redirect_info->new_url = url;
    VLOG(2) << "VigoPrivacyThrottle: Stripped tracking params from redirect to "
            << url.spec();
  }

  // Remove fingerprinting-relevant headers from redirects.
  if (modified_request_headers) {
    SanitizeHeaders(modified_request_headers);
  }

  // Remove headers that leak cross-origin info on redirects.
  if (to_be_removed_request_headers) {
    // Remove X-Client-Data (Chrome Variations header — Google tracking).
    to_be_removed_request_headers->push_back("X-Client-Data");
  }
}

bool VigoPrivacyThrottle::MaybeStripTrackingParams(GURL* url) const {
#if BUILDFLAG(VIGO_ENABLE_PRIVACY_ENGINE)
  return vigo::privacy::VigoPrivacyEngine::StripTrackingParams(url);
#else
  return false;
#endif
}

bool VigoPrivacyThrottle::MaybeUpgradeToHttps(GURL* url) const {
  if (!url || !url->is_valid())
    return false;

  // Only upgrade plain HTTP navigations.
  if (!url->SchemeIs("http"))
    return false;

  // Don't upgrade localhost / loopback — local dev servers use HTTP.
  if (url->host() == "localhost" || url->host() == "127.0.0.1" ||
      url->host() == "[::1]") {
    return false;
  }

  // Don't upgrade .local domains (mDNS).
  if (url->host().ends_with(".local")) {
    return false;
  }

  // Upgrade to HTTPS.
  GURL::Replacements replacements;
  replacements.SetSchemeStr("https");
  *url = url->ReplaceComponents(replacements);

  VLOG(2) << "VigoPrivacyThrottle: Upgraded to HTTPS: " << url->spec();
  return true;
}

void VigoPrivacyThrottle::SanitizeHeaders(
    net::HttpRequestHeaders* headers) const {
  if (!headers)
    return;

  // Remove Chrome Variations header (used by Google for A/B experiments).
  headers->RemoveHeader("X-Client-Data");

  // Remove Topics API header (Privacy Sandbox / Google tracking).
  headers->RemoveHeader("Sec-Browsing-Topics");

  // Remove Attribution Reporting header (Google ad tracking).
  headers->RemoveHeader("Attribution-Reporting-Eligible");
  headers->RemoveHeader("Attribution-Reporting-Support");
}

}  // namespace vigo

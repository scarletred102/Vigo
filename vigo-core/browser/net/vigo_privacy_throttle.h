// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_BROWSER_NET_VIGO_PRIVACY_THROTTLE_H_
#define VIGO_BROWSER_NET_VIGO_PRIVACY_THROTTLE_H_

#include "third_party/blink/public/common/loader/url_loader_throttle.h"

namespace vigo {

// VigoPrivacyThrottle performs privacy-preserving transformations on
// outgoing network requests:
//   - Strips tracking query parameters (utm_*, fbclid, gclid, etc.)
//   - Upgrades HTTP navigations to HTTPS (HTTPS-first mode)
//   - Enforces strict referrer policy on cross-origin requests
//   - Removes known fingerprinting headers
//
// Attached to every URL loader via
// VigoContentBrowserClient::CreateURLLoaderThrottles().
class VigoPrivacyThrottle : public blink::URLLoaderThrottle {
 public:
  VigoPrivacyThrottle();
  ~VigoPrivacyThrottle() override;

  VigoPrivacyThrottle(const VigoPrivacyThrottle&) = delete;
  VigoPrivacyThrottle& operator=(const VigoPrivacyThrottle&) = delete;

  // blink::URLLoaderThrottle overrides:
  void WillStartRequest(network::ResourceRequest* request,
                        bool* defer) override;

  void WillRedirectRequest(
      net::RedirectInfo* redirect_info,
      const network::mojom::URLResponseHead& response_head,
      bool* defer,
      std::vector<std::string>* to_be_removed_request_headers,
      net::HttpRequestHeaders* modified_request_headers,
      net::HttpRequestHeaders* modified_cors_exempt_request_headers) override;

 private:
  // Strip tracking parameters from the URL. Returns true if modified.
  bool MaybeStripTrackingParams(GURL* url) const;

  // Upgrade HTTP → HTTPS for navigations. Returns true if upgraded.
  bool MaybeUpgradeToHttps(GURL* url) const;

  // Remove privacy-invasive request headers.
  void SanitizeHeaders(net::HttpRequestHeaders* headers) const;
};

}  // namespace vigo

#endif  // VIGO_BROWSER_NET_VIGO_PRIVACY_THROTTLE_H_

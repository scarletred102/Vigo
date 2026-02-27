// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_BROWSER_NET_VIGO_ADBLOCK_THROTTLE_H_
#define VIGO_BROWSER_NET_VIGO_ADBLOCK_THROTTLE_H_

#include "base/memory/raw_ptr.h"
#include "content/public/browser/browser_context.h"
#include "third_party/blink/public/common/loader/url_loader_throttle.h"

namespace vigo {

// VigoAdblockThrottle intercepts network requests and cancels those
// matching adblock filter rules. Attached to every URL loader via
// VigoContentBrowserClient::CreateURLLoaderThrottles().
//
// Lifecycle: one instance per network request, destroyed when the
// request completes or is cancelled.
class VigoAdblockThrottle : public blink::URLLoaderThrottle {
 public:
  explicit VigoAdblockThrottle(content::BrowserContext* browser_context);
  ~VigoAdblockThrottle() override;

  VigoAdblockThrottle(const VigoAdblockThrottle&) = delete;
  VigoAdblockThrottle& operator=(const VigoAdblockThrottle&) = delete;

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
  // Check a URL against the adblock engine and cancel if blocked.
  // Returns true if the request should be cancelled.
  bool ShouldBlockRequest(const GURL& url, const GURL& source_url) const;

  // Non-owning. The BrowserContext (and its keyed services) outlive
  // individual request throttles.
  raw_ptr<content::BrowserContext> browser_context_;
};

}  // namespace vigo

#endif  // VIGO_BROWSER_NET_VIGO_ADBLOCK_THROTTLE_H_

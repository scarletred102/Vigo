// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/browser/net/vigo_adblock_throttle.h"

#include "base/logging.h"
#include "net/base/net_errors.h"
#include "services/network/public/cpp/resource_request.h"
#include "url/gurl.h"
#include "vigo/build/config/vigo_buildflags.h"

#if BUILDFLAG(VIGO_ENABLE_ADBLOCK)
#include "vigo/components/adblock/vigo_adblock_service.h"
#endif

namespace vigo {

VigoAdblockThrottle::VigoAdblockThrottle(
    content::BrowserContext* browser_context)
    : browser_context_(browser_context) {}

VigoAdblockThrottle::~VigoAdblockThrottle() = default;

void VigoAdblockThrottle::WillStartRequest(
    network::ResourceRequest* request,
    bool* defer) {
  if (!request)
    return;

  const GURL& url = request->url;
  const GURL& source_url = request->request_initiator.has_value()
      ? request->request_initiator->GetURL()
      : GURL::EmptyGURL();

  if (ShouldBlockRequest(url, source_url)) {
    VLOG(2) << "VigoAdblockThrottle: Blocked " << url.spec();
    delegate_->CancelWithError(net::ERR_BLOCKED_BY_ADMINISTRATOR,
                               "Vigo Adblock");
  }
}

void VigoAdblockThrottle::WillRedirectRequest(
    net::RedirectInfo* redirect_info,
    const network::mojom::URLResponseHead& response_head,
    bool* defer,
    std::vector<std::string>* to_be_removed_request_headers,
    net::HttpRequestHeaders* modified_request_headers,
    net::HttpRequestHeaders* modified_cors_exempt_request_headers) {
  if (!redirect_info)
    return;

  const GURL& url = redirect_info->new_url;

  if (ShouldBlockRequest(url, GURL::EmptyGURL())) {
    VLOG(2) << "VigoAdblockThrottle: Blocked redirect to " << url.spec();
    delegate_->CancelWithError(net::ERR_BLOCKED_BY_ADMINISTRATOR,
                               "Vigo Adblock");
  }
}

bool VigoAdblockThrottle::ShouldBlockRequest(
    const GURL& url,
    const GURL& source_url) const {
#if BUILDFLAG(VIGO_ENABLE_ADBLOCK)
  if (!browser_context_)
    return false;

  // TODO(Phase 1.3): Retrieve VigoAdblockService from BrowserContext
  // keyed service factory. For now, use the FFI bridge directly.
  // auto* service = VigoAdblockServiceFactory::GetForBrowserContext(
  //     browser_context_);
  // if (!service || !service->IsReady())
  //   return false;
  // return service->ShouldBlock(url, source_url);

  // Stub — blocked domains are hardcoded until factory wiring is done.
  // This list will be replaced by the Rust engine once the factory exists.
  static const char* const kHardcodedBlockDomains[] = {
      "doubleclick.net",
      "googlesyndication.com",
      "googleadservices.com",
      "google-analytics.com",
      "googletagmanager.com",
      "facebook.net",
      "fbcdn.net",
      "analytics.yahoo.com",
      "ads.twitter.com",
      "ad.doubleclick.net",
  };

  const std::string host = url.host();
  for (const char* domain : kHardcodedBlockDomains) {
    if (host == domain || host.ends_with(std::string(".") + domain)) {
      return true;
    }
  }
#endif  // BUILDFLAG(VIGO_ENABLE_ADBLOCK)

  return false;
}

}  // namespace vigo

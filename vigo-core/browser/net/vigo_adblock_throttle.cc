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
#include "vigo/components/adblock/vigo_adblock_service_factory.h"
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
      : GURL();

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

  if (ShouldBlockRequest(url, GURL())) {
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

  // Retrieve the per-profile VigoAdblockService via the keyed service
  // factory. The factory creates and initialises the service (including
  // the Rust engine + filter lists) on first access.
  auto* service =
      adblock::VigoAdblockServiceFactory::GetForBrowserContext(
          browser_context_);
  if (service && service->IsReady()) {
    return service->ShouldBlock(url, source_url);
  }

  // Fallback: if the Rust engine isn't ready yet (first-run before filter
  // list download completes), block the most egregious ad/tracker domains
  // with a hardcoded list. This ensures ads are blocked immediately on
  // first launch.
  static const char* const kFallbackBlockDomains[] = {
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
  for (const char* domain : kFallbackBlockDomains) {
    if (host == domain || host.ends_with(std::string(".") + domain)) {
      return true;
    }
  }
#endif  // BUILDFLAG(VIGO_ENABLE_ADBLOCK)

  return false;
}

}  // namespace vigo

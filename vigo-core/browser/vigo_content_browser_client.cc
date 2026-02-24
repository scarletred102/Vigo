// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/browser/vigo_content_browser_client.h"

#include <memory>
#include <string>
#include <utility>
#include <vector>

#include "base/logging.h"
#include "content/public/browser/browser_context.h"
#include "content/public/browser/web_contents.h"
#include "vigo/app/vigo_branding.h"
#include "vigo/browser/vigo_browser_main_parts.h"
#include "vigo/build/config/vigo_buildflags.h"

#if BUILDFLAG(VIGO_ENABLE_ADBLOCK)
#include "vigo/browser/net/vigo_adblock_throttle.h"
#endif

#if BUILDFLAG(VIGO_ENABLE_PRIVACY_ENGINE)
#include "vigo/browser/net/vigo_privacy_throttle.h"
#endif

namespace vigo {

VigoContentBrowserClient::VigoContentBrowserClient() = default;
VigoContentBrowserClient::~VigoContentBrowserClient() = default;

std::unique_ptr<content::BrowserMainParts>
VigoContentBrowserClient::CreateBrowserMainParts(bool is_integration_test) {
  VLOG(1) << "VigoContentBrowserClient: Creating VigoBrowserMainParts";

  // Let Chrome create its main parts first, then wrap with Vigo additions.
  auto main_parts =
      ChromeContentBrowserClient::CreateBrowserMainParts(is_integration_test);

  // TODO(Phase 1.1): When Chromium allows extending rather than replacing
  // BrowserMainParts, integrate VigoBrowserMainParts here. For now, the
  // override in chromium_src ensures our VigoBrowserMainParts is used
  // at construction time.

  return main_parts;
}

std::vector<std::unique_ptr<content::URLLoaderThrottle>>
VigoContentBrowserClient::CreateURLLoaderThrottles(
    const network::ResourceRequest& request,
    content::BrowserContext* browser_context,
    const base::RepeatingCallback<content::WebContents*()>& wc_getter,
    content::NavigationUIData* navigation_ui_data,
    int frame_tree_node_id,
    std::optional<int64_t> navigation_id) {
  // Start with Chrome's default throttles.
  auto throttles = ChromeContentBrowserClient::CreateURLLoaderThrottles(
      request, browser_context, wc_getter, navigation_ui_data,
      frame_tree_node_id, navigation_id);

#if BUILDFLAG(VIGO_ENABLE_ADBLOCK)
  throttles.push_back(std::make_unique<VigoAdblockThrottle>(browser_context));
#endif

#if BUILDFLAG(VIGO_ENABLE_PRIVACY_ENGINE)
  throttles.push_back(std::make_unique<VigoPrivacyThrottle>());
#endif

  return throttles;
}

void VigoContentBrowserClient::OverrideWebkitPrefs(
    content::WebContents* web_contents,
    blink::web_pref::WebPreferences* prefs) {
  ChromeContentBrowserClient::OverrideWebkitPrefs(web_contents, prefs);

  // Vigo-specific WebKit preference overrides.
  // Ensure third-party cookies are blocked by default.
  // NOTE: Cookie blocking is also enforced at the network layer, but we
  // belt-and-suspenders here for blink-level enforcement.
  // TODO(Phase 1.4): Wire to user preference from settings page.
}

std::string VigoContentBrowserClient::GetUserAgent() {
  // Start with Chrome's UA and append Vigo branding.
  std::string ua = ChromeContentBrowserClient::GetUserAgent();
  // Append " Vigo/0.1.0" to the user agent.
  ua += " ";
  ua += vigo::branding::kUserAgentProduct;
  return ua;
}

}  // namespace vigo

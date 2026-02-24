// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_BROWSER_VIGO_CONTENT_BROWSER_CLIENT_H_
#define VIGO_BROWSER_VIGO_CONTENT_BROWSER_CLIENT_H_

#include "chrome/browser/chrome_content_browser_client.h"

namespace vigo {

// VigoContentBrowserClient extends Chrome's content browser client to
// inject Vigo-specific behaviour:
//   - Custom BrowserMainParts (VigoBrowserMainParts)
//   - Request throttling for adblock & privacy
//   - Custom New Tab Page URL remapping
//   - Google service endpoint disabling
//   - DoH configuration
//
// This class is instantiated instead of ChromeContentBrowserClient via
// the chromium_src shadow override pattern.
class VigoContentBrowserClient : public ChromeContentBrowserClient {
 public:
  VigoContentBrowserClient();
  ~VigoContentBrowserClient() override;

  VigoContentBrowserClient(const VigoContentBrowserClient&) = delete;
  VigoContentBrowserClient& operator=(const VigoContentBrowserClient&) = delete;

  // ContentBrowserClient overrides:

  // Creates VigoBrowserMainParts instead of ChromeBrowserMainParts.
  std::unique_ptr<content::BrowserMainParts> CreateBrowserMainParts(
      bool is_integration_test) override;

  // Appends Vigo-specific URL request throttles (adblock, privacy).
  std::vector<std::unique_ptr<content::URLLoaderThrottle>>
  CreateURLLoaderThrottles(
      const network::ResourceRequest& request,
      content::BrowserContext* browser_context,
      const base::RepeatingCallback<content::WebContents*()>& wc_getter,
      content::NavigationUIData* navigation_ui_data,
      int frame_tree_node_id,
      std::optional<int64_t> navigation_id) override;

  // Override to strip Google service URLs from allowed endpoints.
  void OverrideWebkitPrefs(content::WebContents* web_contents,
                           blink::web_pref::WebPreferences* prefs) override;

  // Override the user agent string to include Vigo branding.
  std::string GetUserAgent() override;
};

}  // namespace vigo

#endif  // VIGO_BROWSER_VIGO_CONTENT_BROWSER_CLIENT_H_

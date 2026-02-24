// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_BROWSER_UI_WEBUI_VIGO_WEB_UI_CONTROLLER_FACTORY_H_
#define VIGO_BROWSER_UI_WEBUI_VIGO_WEB_UI_CONTROLLER_FACTORY_H_

#include "content/public/browser/web_ui.h"
#include "content/public/browser/web_ui_controller.h"

class GURL;

namespace vigo {

// Factory methods for Vigo-specific WebUI pages.
// Called from ChromeWebUIControllerFactory (via chromium_src override or
// registration hook) to handle vigo:// and chrome:// URL schemes.
//
// Registered pages:
//   chrome://newtab   → VigoNewTabPageUI
//   vigo://settings   → VigoSettingsUI (Phase 1.2)
//   vigo://adblock    → VigoAdblockUI (Phase 1.3)
//   vigo://privacy    → VigoPrivacyUI (Phase 1.4)

// Returns a WebUIController for the given URL, or nullptr if the URL
// is not handled by Vigo.
std::unique_ptr<content::WebUIController> CreateVigoWebUIController(
    content::WebUI* web_ui,
    const GURL& url);

// Returns true if |url| is a Vigo-handled WebUI URL.
bool IsVigoWebUIURL(const GURL& url);

}  // namespace vigo

#endif  // VIGO_BROWSER_UI_WEBUI_VIGO_WEB_UI_CONTROLLER_FACTORY_H_

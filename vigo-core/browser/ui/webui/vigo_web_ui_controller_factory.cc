// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/browser/ui/webui/vigo_web_ui_controller_factory.h"

#include <memory>

#include "base/logging.h"
#include "url/gurl.h"
#include "vigo/browser/ui/webui/vigo_new_tab_page_ui.h"
#include "vigo/browser/ui/webui/vigo_settings_ui.h"

namespace vigo {

namespace {

// Vigo's custom URL scheme for internal pages.
constexpr char kVigoScheme[] = "vigo";
constexpr char kNewTabHost[] = "newtab";
constexpr char kSettingsHost[] = "settings";

}  // namespace

std::unique_ptr<content::WebUIController> CreateVigoWebUIController(
    content::WebUI* web_ui,
    const GURL& url) {
  if (!IsVigoWebUIURL(url))
    return nullptr;

  const std::string& host = url.host();

  if (host == kNewTabHost) {
    VLOG(1) << "VigoWebUIFactory: Creating NewTabPageUI";
    return std::make_unique<VigoNewTabPageUI>(web_ui);
  }

  if (host == kSettingsHost) {
    VLOG(1) << "VigoWebUIFactory: Creating SettingsUI";
    return std::make_unique<VigoSettingsUI>(web_ui);
  }

  // TODO(Phase 1.3): vigo://adblock
  // TODO(Phase 1.4): vigo://privacy

  return nullptr;
}

bool IsVigoWebUIURL(const GURL& url) {
  if (!url.is_valid())
    return false;

  // Handle both chrome:// and vigo:// schemes.
  if (url.SchemeIs("chrome") || url.SchemeIs(kVigoScheme)) {
    const std::string& host = url.host();
    return host == kNewTabHost || host == kSettingsHost;
  }

  return false;
}

}  // namespace vigo

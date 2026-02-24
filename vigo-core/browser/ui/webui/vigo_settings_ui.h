// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_BROWSER_UI_WEBUI_VIGO_SETTINGS_UI_H_
#define VIGO_BROWSER_UI_WEBUI_VIGO_SETTINGS_UI_H_

#include "content/public/browser/web_ui_controller.h"

namespace vigo {

// VigoSettingsUI serves the Vigo settings page.
//
// Settings sections:
//   - Privacy & Security (DoH, fingerprint resistance, tracking, cookies)
//   - Ad Blocking (filter lists, whitelist, stats)
//   - Media (ABR mode, codec preferences, hardware acceleration)
//   - Sync (server URL, passphrase, data type toggles)
//   - Appearance (theme, NTP background, fonts)
//   - Search Engine (default engine, suggestions)
//   - About (version, license, update check)
//
// Registered at vigo://settings via VigoWebUIControllerFactory.
class VigoSettingsUI : public content::WebUIController {
 public:
  explicit VigoSettingsUI(content::WebUI* web_ui);
  ~VigoSettingsUI() override;

  VigoSettingsUI(const VigoSettingsUI&) = delete;
  VigoSettingsUI& operator=(const VigoSettingsUI&) = delete;
};

}  // namespace vigo

#endif  // VIGO_BROWSER_UI_WEBUI_VIGO_SETTINGS_UI_H_

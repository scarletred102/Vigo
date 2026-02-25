// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_BROWSER_UI_WEBUI_VIGO_SETTINGS_HANDLER_H_
#define VIGO_BROWSER_UI_WEBUI_VIGO_SETTINGS_HANDLER_H_

#include <string>

#include "base/values.h"
#include "content/public/browser/web_ui_message_handler.h"

namespace vigo {

namespace privacy {
class VigoDoHConfig;
class VigoFingerprintProtection;
class VigoPrivacyEngine;
}  // namespace privacy

// VigoSettingsHandler implements the WebUI message handler for the
// Vigo settings page. It bridges chrome.send() calls from the
// settings JS to browser-process subsystems.
//
// JS messages handled:
//   - "setSetting"      → applies key/value setting change
//   - "getSettings"     → returns all current settings as a dict
//   - "getFilterLists"  → returns adblock filter list info
//   - "updateFilters"   → triggers manual filter list update
//   - "getAboutInfo"    → returns version, build info
//
// All settings are currently applied in-memory to the running subsystem
// instances. TODO(Phase 1.5): Persist via PrefService for cross-session.
//
// Lifetime: owned by VigoSettingsUI; destroyed when settings tab closes.
class VigoSettingsHandler : public content::WebUIMessageHandler {
 public:
  VigoSettingsHandler();
  ~VigoSettingsHandler() override;

  VigoSettingsHandler(const VigoSettingsHandler&) = delete;
  VigoSettingsHandler& operator=(const VigoSettingsHandler&) = delete;

  // content::WebUIMessageHandler:
  void RegisterMessages() override;
  void OnJavascriptAllowed() override;
  void OnJavascriptDisallowed() override;

 private:
  // Message handlers.
  void HandleGetSettings(const base::Value::List& args);
  void HandleSetSetting(const base::Value::List& args);
  void HandleGetFilterLists(const base::Value::List& args);
  void HandleUpdateFilters(const base::Value::List& args);
  void HandleGetAboutInfo(const base::Value::List& args);

  // Apply a single setting change. Returns true if the key was
  // recognised and the value was applied.
  bool ApplySetting(const std::string& key, const base::Value& value);

  // Build a dict of all current settings for initial page load.
  base::Value::Dict BuildCurrentSettings() const;
};

}  // namespace vigo

#endif  // VIGO_BROWSER_UI_WEBUI_VIGO_SETTINGS_HANDLER_H_

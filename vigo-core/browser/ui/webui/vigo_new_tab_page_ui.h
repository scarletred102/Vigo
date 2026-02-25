// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_BROWSER_UI_WEBUI_VIGO_NEW_TAB_PAGE_UI_H_
#define VIGO_BROWSER_UI_WEBUI_VIGO_NEW_TAB_PAGE_UI_H_

#include "content/public/browser/web_ui_controller.h"

namespace vigo {

// VigoNewTabPageUI serves the custom New Tab Page for Vigo.
//
// Features:
//   - Speed dial (most visited / pinned sites)
//   - Search bar (configurable: DuckDuckGo, Brave Search, Google, etc.)
//   - Privacy stats (ads blocked, trackers blocked, HTTPS upgrades)
//   - Dark/light mode support following system preference
//   - No Google integration, no Google Doodles, no Chrome promo cards
//
// Communication: uses WebUIMessageHandler (VigoNtpHandler) to
// provide privacy stats and speed dial data to the NTP JS.
//
// Registered at chrome://newtab and vigo://newtab via
// VigoWebUIControllerFactory.
class VigoNewTabPageUI : public content::WebUIController {
 public:
  explicit VigoNewTabPageUI(content::WebUI* web_ui);
  ~VigoNewTabPageUI() override;

  VigoNewTabPageUI(const VigoNewTabPageUI&) = delete;
  VigoNewTabPageUI& operator=(const VigoNewTabPageUI&) = delete;
};

}  // namespace vigo

#endif  // VIGO_BROWSER_UI_WEBUI_VIGO_NEW_TAB_PAGE_UI_H_

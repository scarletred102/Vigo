// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/browser/ui/webui/vigo_new_tab_page_ui.h"

#include "base/values.h"
#include "content/public/browser/web_ui.h"
#include "content/public/browser/web_ui_data_source.h"
#include "grit/vigo_webui_resources.h"
#include "vigo/app/vigo_branding.h"
#include "vigo/browser/ui/webui/vigo_ntp_handler.h"
#include "vigo/build/config/vigo_buildflags.h"

namespace vigo {

namespace {

// Resource IDs for the NTP — in production these map to grit resource files.
// For now, we embed the HTML directly via the data source.
constexpr char kNewTabPageHost[] = "newtab";

void CreateAndAddDataSource(content::WebUI* web_ui) {
  content::WebUIDataSource* source =
      content::WebUIDataSource::CreateAndAdd(
          web_ui->GetWebContents()->GetBrowserContext(), kNewTabPageHost);

  // Set the default resource (main page).
  source->SetDefaultResource(IDR_VIGO_NEW_TAB_PAGE_HTML);

  // Add additional resources.
  source->AddResourcePath("new_tab_page.css", IDR_VIGO_NEW_TAB_PAGE_CSS);
  source->AddResourcePath("new_tab_page.js", IDR_VIGO_NEW_TAB_PAGE_JS);

  // Inject localised strings and dynamic data.
  source->AddString("productName", vigo::branding::kProductName);
  source->AddString("version", vigo::branding::kVersionString);

  // TODO(Phase 1.1): Wire up Mojo handler to provide:
  //   - Top sites / speed dial data
  //   - Privacy stats (ads blocked, trackers blocked, HTTPS upgrades)
  //   - Search engine configuration
  //   - User background image preference

  // Content Security Policy: only allow same-origin resources.
  source->OverrideContentSecurityPolicy(
      network::mojom::CSPDirectiveName::ScriptSrc,
      "script-src chrome://resources 'self';");
  source->OverrideContentSecurityPolicy(
      network::mojom::CSPDirectiveName::StyleSrc,
      "style-src 'self' 'unsafe-inline';");
}

}  // namespace

VigoNewTabPageUI::VigoNewTabPageUI(content::WebUI* web_ui)
    : content::WebUIController(web_ui) {
  CreateAndAddDataSource(web_ui);

  // Register the NTP message handler for privacy stats and speed dials.
  web_ui->AddMessageHandler(std::make_unique<VigoNtpHandler>());
}

VigoNewTabPageUI::~VigoNewTabPageUI() = default;

}  // namespace vigo

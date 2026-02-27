// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/browser/ui/webui/vigo_settings_ui.h"

#include "content/public/browser/web_ui.h"
#include "content/public/browser/web_ui_data_source.h"
#include "grit/vigo_webui_resources.h"
#include "vigo/app/vigo_branding.h"
#include "vigo/browser/ui/webui/vigo_settings_handler.h"
#include "vigo/build/config/vigo_buildflags.h"

namespace vigo {

namespace {

constexpr char kSettingsHost[] = "settings";

void CreateAndAddDataSource(content::WebUI* web_ui) {
  content::WebUIDataSource* source =
      content::WebUIDataSource::CreateAndAdd(
          web_ui->GetWebContents()->GetBrowserContext(), kSettingsHost);

  source->SetDefaultResource(IDR_VIGO_SETTINGS_HTML);
  source->AddResourcePath("settings.css", IDR_VIGO_SETTINGS_CSS);
  source->AddResourcePath("settings.js", IDR_VIGO_SETTINGS_JS);

  // Inject product info.
  source->AddString("productName", vigo::branding::kProductName);
  source->AddString("version", vigo::branding::kVersionString);

  // Inject feature flags so the UI knows which sections to show.
  source->AddBoolean("adblockEnabled", BUILDFLAG(VIGO_ENABLE_ADBLOCK));
  source->AddBoolean("privacyEngineEnabled",
                      BUILDFLAG(VIGO_ENABLE_PRIVACY_ENGINE));
  source->AddBoolean("syncEnabled", BUILDFLAG(VIGO_ENABLE_SYNC));
  source->AddBoolean("mediaOrchestrationEnabled",
                      BUILDFLAG(VIGO_ENABLE_MEDIA_ORCHESTRATION));

  source->OverrideContentSecurityPolicy(
      network::mojom::CSPDirectiveName::ScriptSrc,
      "script-src chrome://resources 'self';");
  source->OverrideContentSecurityPolicy(
      network::mojom::CSPDirectiveName::StyleSrc,
      "style-src 'self' 'unsafe-inline';");
}

}  // namespace

VigoSettingsUI::VigoSettingsUI(content::WebUI* web_ui)
    : content::WebUIController(web_ui) {
  CreateAndAddDataSource(web_ui);

  // Register the settings message handler for all setting read/write.
  web_ui->AddMessageHandler(std::make_unique<VigoSettingsHandler>());
}

VigoSettingsUI::~VigoSettingsUI() = default;

}  // namespace vigo

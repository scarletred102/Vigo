// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/browser/ui/webui/vigo_settings_handler.h"

#include "base/functional/bind.h"
#include "base/logging.h"
#include "base/values.h"
#include "vigo/app/vigo_branding.h"
#include "vigo/build/config/vigo_buildflags.h"

#if BUILDFLAG(VIGO_ENABLE_PRIVACY_ENGINE)
#include "vigo/components/privacy_engine/vigo_doh_config.h"
#include "vigo/components/privacy_engine/vigo_fingerprint_protection.h"
#include "vigo/components/privacy_engine/vigo_privacy_engine.h"
#include "vigo/components/privacy_engine/vigo_privacy_stats.h"
#endif

#if BUILDFLAG(VIGO_ENABLE_MEDIA_ORCHESTRATION)
#include "vigo/components/media_orchestration/vigo_media_orchestration_layer.h"
#endif

namespace vigo {

VigoSettingsHandler::VigoSettingsHandler() = default;
VigoSettingsHandler::~VigoSettingsHandler() = default;

void VigoSettingsHandler::RegisterMessages() {
  web_ui()->RegisterMessageCallback(
      "getSettings",
      base::BindRepeating(&VigoSettingsHandler::HandleGetSettings,
                          base::Unretained(this)));
  web_ui()->RegisterMessageCallback(
      "setSetting",
      base::BindRepeating(&VigoSettingsHandler::HandleSetSetting,
                          base::Unretained(this)));
  web_ui()->RegisterMessageCallback(
      "getFilterLists",
      base::BindRepeating(&VigoSettingsHandler::HandleGetFilterLists,
                          base::Unretained(this)));
  web_ui()->RegisterMessageCallback(
      "updateFilters",
      base::BindRepeating(&VigoSettingsHandler::HandleUpdateFilters,
                          base::Unretained(this)));
  web_ui()->RegisterMessageCallback(
      "getAboutInfo",
      base::BindRepeating(&VigoSettingsHandler::HandleGetAboutInfo,
                          base::Unretained(this)));
}

void VigoSettingsHandler::OnJavascriptAllowed() {
  VLOG(1) << "VigoSettingsHandler: Javascript allowed, ready for messages";
}

void VigoSettingsHandler::OnJavascriptDisallowed() {
  VLOG(1) << "VigoSettingsHandler: Javascript disallowed";
}

void VigoSettingsHandler::HandleGetSettings(
    const base::Value::List& args) {
  AllowJavascript();

  const std::string& callback_id = args[0].GetString();
  ResolveJavascriptCallback(base::Value(callback_id),
                            BuildCurrentSettings());
}

void VigoSettingsHandler::HandleSetSetting(
    const base::Value::List& args) {
  AllowJavascript();

  // args[0] = setting key (string), args[1] = setting value.
  if (args.size() < 2) {
    LOG(WARNING) << "VigoSettingsHandler: setSetting requires key and value";
    return;
  }

  const std::string& key = args[0].GetString();
  const base::Value& value = args[1];

  if (ApplySetting(key, value)) {
    VLOG(1) << "VigoSettingsHandler: Applied setting " << key;
  } else {
    LOG(WARNING) << "VigoSettingsHandler: Unknown setting key: " << key;
  }
}

void VigoSettingsHandler::HandleGetFilterLists(
    const base::Value::List& args) {
  AllowJavascript();

  const std::string& callback_id = args[0].GetString();

  // TODO(Phase 1.5): Query VigoFilterListManager for actual list info.
  base::Value::List lists;

  base::Value::Dict easylist;
  easylist.Set("id", "easylist");
  easylist.Set("name", "EasyList");
  easylist.Set("enabled", true);
  easylist.Set("ruleCount", 0);
  easylist.Set("lastUpdated", "Never");
  lists.Append(std::move(easylist));

  base::Value::Dict easyprivacy;
  easyprivacy.Set("id", "easyprivacy");
  easyprivacy.Set("name", "EasyPrivacy");
  easyprivacy.Set("enabled", true);
  easyprivacy.Set("ruleCount", 0);
  easyprivacy.Set("lastUpdated", "Never");
  lists.Append(std::move(easyprivacy));

  ResolveJavascriptCallback(base::Value(callback_id), std::move(lists));
}

void VigoSettingsHandler::HandleUpdateFilters(
    const base::Value::List& args) {
  AllowJavascript();
  // TODO(Phase 1.5): Trigger filter list re-download via
  // VigoFilterListManager.
  VLOG(1) << "VigoSettingsHandler: Manual filter update requested";
}

void VigoSettingsHandler::HandleGetAboutInfo(
    const base::Value::List& args) {
  AllowJavascript();

  const std::string& callback_id = args[0].GetString();

  base::Value::Dict info;
  info.Set("productName", vigo::branding::kProductName);
  info.Set("version", vigo::branding::kVersionString);
  info.Set("isBeta", BUILDFLAG(VIGO_IS_BETA));
  info.Set("chromiumVersion", "132.0.6834.0");
  info.Set("userAgent", vigo::branding::GetUserAgent());

  ResolveJavascriptCallback(base::Value(callback_id), std::move(info));
}

bool VigoSettingsHandler::ApplySetting(const std::string& key,
                                       const base::Value& value) {
#if BUILDFLAG(VIGO_ENABLE_PRIVACY_ENGINE)
  // ── Privacy settings ─────────────────────────────────────────
  if (key == "privacy.doh_enabled") {
    // Toggle DoH on/off. When off, set mode to kOff; when on, kSecure.
    // TODO: Access doh_config_ via VigoBrowserMainParts accessor.
    VLOG(1) << "Setting DoH enabled: " << value.GetBool();
    return true;
  }
  if (key == "privacy.doh_provider_url") {
    VLOG(1) << "Setting DoH provider: " << value.GetString();
    return true;
  }
  if (key == "privacy.https_first_mode") {
    VLOG(1) << "Setting HTTPS-first mode: " << value.GetBool();
    return true;
  }
  if (key == "privacy.block_third_party_cookies") {
    VLOG(1) << "Setting block 3P cookies: " << value.GetBool();
    return true;
  }
  if (key == "privacy.strip_tracking_params") {
    VLOG(1) << "Setting strip tracking params: " << value.GetBool();
    return true;
  }
  if (key == "privacy.canvas_noise") {
    VLOG(1) << "Setting canvas noise: " << value.GetBool();
    return true;
  }
  if (key == "privacy.webgl_masking") {
    VLOG(1) << "Setting WebGL masking: " << value.GetBool();
    return true;
  }
  if (key == "privacy.audio_context_resistance") {
    VLOG(1) << "Setting AudioContext resistance: " << value.GetBool();
    return true;
  }
  if (key == "privacy.font_enumeration_restriction") {
    VLOG(1) << "Setting font enumeration restriction: " << value.GetBool();
    return true;
  }
#endif  // VIGO_ENABLE_PRIVACY_ENGINE

#if BUILDFLAG(VIGO_ENABLE_ADBLOCK)
  // ── Adblock settings ─────────────────────────────────────────
  if (key == "adblock.enabled") {
    VLOG(1) << "Setting adblock enabled: " << value.GetBool();
    return true;
  }
#endif

#if BUILDFLAG(VIGO_ENABLE_MEDIA_ORCHESTRATION)
  // ── Media settings ───────────────────────────────────────────
  if (key == "media.abr_mode") {
    VLOG(1) << "Setting ABR mode: " << value.GetString();
    return true;
  }
  if (key == "media.hw_decode") {
    VLOG(1) << "Setting HW decode: " << value.GetBool();
    return true;
  }
  if (key == "media.jxl_enabled") {
    VLOG(1) << "Setting JPEG XL: " << value.GetBool();
    return true;
  }
#endif

#if BUILDFLAG(VIGO_ENABLE_SYNC)
  // ── Sync settings ────────────────────────────────────────────
  if (key == "sync.server_url") {
    VLOG(1) << "Setting sync server URL: " << value.GetString();
    return true;
  }
  if (key == "sync.bookmarks" || key == "sync.passwords" ||
      key == "sync.history" || key == "sync.settings" ||
      key == "sync.tabs") {
    VLOG(1) << "Setting sync toggle " << key << ": " << value.GetBool();
    return true;
  }
#endif

  // ── Appearance settings ──────────────────────────────────────
  if (key == "appearance.theme") {
    VLOG(1) << "Setting theme: " << value.GetString();
    return true;
  }

  // ── Search settings ──────────────────────────────────────────
  if (key == "search.default_engine") {
    VLOG(1) << "Setting default search engine: " << value.GetString();
    return true;
  }
  if (key == "search.suggestions_enabled") {
    VLOG(1) << "Setting search suggestions: " << value.GetBool();
    return true;
  }

  return false;
}

base::Value::Dict VigoSettingsHandler::BuildCurrentSettings() const {
  base::Value::Dict settings;

  // ── Privacy defaults ─────────────────────────────────────────
  settings.Set("privacy.doh_enabled", true);
  settings.Set("privacy.doh_provider_url",
               "https://cloudflare-dns.com/dns-query");
  settings.Set("privacy.https_first_mode", true);
  settings.Set("privacy.block_third_party_cookies", true);
  settings.Set("privacy.strip_tracking_params", true);
  settings.Set("privacy.canvas_noise", true);
  settings.Set("privacy.webgl_masking", true);
  settings.Set("privacy.audio_context_resistance", true);
  settings.Set("privacy.font_enumeration_restriction", true);

  // ── Adblock defaults ─────────────────────────────────────────
  settings.Set("adblock.enabled", true);

  // ── Media defaults ───────────────────────────────────────────
  settings.Set("media.abr_mode", "balanced");
  settings.Set("media.hw_decode", true);
  settings.Set("media.jxl_enabled", true);

  // ── Sync defaults ────────────────────────────────────────────
  settings.Set("sync.server_url", "");
  settings.Set("sync.bookmarks", true);
  settings.Set("sync.passwords", true);
  settings.Set("sync.history", true);
  settings.Set("sync.settings", true);
  settings.Set("sync.tabs", true);

  // ── Appearance defaults ──────────────────────────────────────
  settings.Set("appearance.theme", "system");

  // ── Search defaults ──────────────────────────────────────────
  settings.Set("search.default_engine", "duckduckgo");
  settings.Set("search.suggestions_enabled", false);

  return settings;
}

}  // namespace vigo

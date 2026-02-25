// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_BROWSER_UI_WEBUI_VIGO_NTP_HANDLER_H_
#define VIGO_BROWSER_UI_WEBUI_VIGO_NTP_HANDLER_H_

#include "base/memory/raw_ptr.h"
#include "base/values.h"
#include "content/public/browser/web_ui_message_handler.h"
#include "vigo/components/privacy_engine/vigo_privacy_stats.h"

namespace vigo {

// VigoNtpHandler implements the WebUI message handler for the Vigo
// New Tab Page. It provides:
//
//   - Privacy stats (ads blocked, trackers blocked, HTTPS upgrades)
//   - Speed dial site data (default sites, later from top-sites/history)
//   - Stats observer — pushes live updates when stats change
//
// JS messages handled:
//   - "getPrivacyStats" → returns {adsBlocked, trackersBlocked, ...}
//   - "getSpeedDials"   → returns array of {title, url, letter}
//   - "resetStats"      → resets all privacy counters
//
// JS events fired:
//   - "privacy-stats-updated" → pushed whenever a stat increments
//
// Lifetime: owned by the VigoNewTabPageUI controller; destroyed when
// the WebUI page is closed.
class VigoNtpHandler : public content::WebUIMessageHandler,
                       public privacy::VigoPrivacyStats::Observer {
 public:
  VigoNtpHandler();
  ~VigoNtpHandler() override;

  VigoNtpHandler(const VigoNtpHandler&) = delete;
  VigoNtpHandler& operator=(const VigoNtpHandler&) = delete;

  // content::WebUIMessageHandler:
  void RegisterMessages() override;
  void OnJavascriptAllowed() override;
  void OnJavascriptDisallowed() override;

  // privacy::VigoPrivacyStats::Observer:
  void OnPrivacyStatsUpdated(
      const privacy::VigoPrivacyStats::Snapshot& stats) override;

 private:
  // Message handlers.
  void HandleGetPrivacyStats(const base::Value::List& args);
  void HandleGetSpeedDials(const base::Value::List& args);
  void HandleResetStats(const base::Value::List& args);

  // Converts a privacy stats snapshot to a base::Value::Dict for JS.
  static base::Value::Dict SnapshotToDict(
      const privacy::VigoPrivacyStats::Snapshot& snapshot);

  // Whether we're observing the stats singleton.
  bool observing_stats_ = false;
};

}  // namespace vigo

#endif  // VIGO_BROWSER_UI_WEBUI_VIGO_NTP_HANDLER_H_

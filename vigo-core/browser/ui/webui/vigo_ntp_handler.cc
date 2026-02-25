// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/browser/ui/webui/vigo_ntp_handler.h"

#include "base/functional/bind.h"
#include "base/logging.h"
#include "base/values.h"

namespace vigo {

namespace {

// Default speed dial sites — shown until user has browsing history.
// Matches the client-side defaults in new_tab_page.js.
struct SpeedDialEntry {
  const char* title;
  const char* url;
  const char* letter;
};

constexpr SpeedDialEntry kDefaultSpeedDials[] = {
    {"DuckDuckGo", "https://duckduckgo.com", "D"},
    {"Wikipedia", "https://wikipedia.org", "W"},
    {"Reddit", "https://reddit.com", "R"},
    {"GitHub", "https://github.com", "G"},
    {"YouTube", "https://youtube.com", "Y"},
    {"Hacker News", "https://news.ycombinator.com", "H"},
    {"Stack Overflow", "https://stackoverflow.com", "S"},
    {"Twitch", "https://twitch.tv", "T"},
};

}  // namespace

VigoNtpHandler::VigoNtpHandler() = default;

VigoNtpHandler::~VigoNtpHandler() {
  if (observing_stats_) {
    privacy::VigoPrivacyStats::GetInstance()->RemoveObserver(this);
  }
}

void VigoNtpHandler::RegisterMessages() {
  web_ui()->RegisterMessageCallback(
      "getPrivacyStats",
      base::BindRepeating(&VigoNtpHandler::HandleGetPrivacyStats,
                          base::Unretained(this)));
  web_ui()->RegisterMessageCallback(
      "getSpeedDials",
      base::BindRepeating(&VigoNtpHandler::HandleGetSpeedDials,
                          base::Unretained(this)));
  web_ui()->RegisterMessageCallback(
      "resetStats",
      base::BindRepeating(&VigoNtpHandler::HandleResetStats,
                          base::Unretained(this)));
}

void VigoNtpHandler::OnJavascriptAllowed() {
  // Start observing stats so we can push live updates to the NTP.
  if (!observing_stats_) {
    privacy::VigoPrivacyStats::GetInstance()->AddObserver(this);
    observing_stats_ = true;
  }
}

void VigoNtpHandler::OnJavascriptDisallowed() {
  // Stop observing when JS is disallowed (page navigated away).
  if (observing_stats_) {
    privacy::VigoPrivacyStats::GetInstance()->RemoveObserver(this);
    observing_stats_ = false;
  }
}

void VigoNtpHandler::OnPrivacyStatsUpdated(
    const privacy::VigoPrivacyStats::Snapshot& stats) {
  // Push live stats to the NTP via WebUI event.
  if (IsJavascriptAllowed()) {
    FireWebUIListener("privacy-stats-updated", SnapshotToDict(stats));
  }
}

void VigoNtpHandler::HandleGetPrivacyStats(
    const base::Value::List& args) {
  AllowJavascript();

  // args[0] is the callback ID for cr.sendWithPromise.
  const std::string& callback_id = args[0].GetString();

  auto snapshot = privacy::VigoPrivacyStats::GetInstance()->GetSnapshot();
  ResolveJavascriptCallback(base::Value(callback_id),
                            SnapshotToDict(snapshot));
}

void VigoNtpHandler::HandleGetSpeedDials(
    const base::Value::List& args) {
  AllowJavascript();

  const std::string& callback_id = args[0].GetString();

  // TODO(Phase 1.5): Query TopSites or history service for user's
  // actual top sites. For now, return the hardcoded defaults.
  base::Value::List dials;
  for (const auto& entry : kDefaultSpeedDials) {
    base::Value::Dict dial;
    dial.Set("title", entry.title);
    dial.Set("url", entry.url);
    dial.Set("letter", entry.letter);
    dials.Append(std::move(dial));
  }

  ResolveJavascriptCallback(base::Value(callback_id), std::move(dials));
}

void VigoNtpHandler::HandleResetStats(const base::Value::List& args) {
  AllowJavascript();
  privacy::VigoPrivacyStats::GetInstance()->Reset();
  VLOG(1) << "VigoNtpHandler: Privacy stats reset by user";
}

// static
base::Value::Dict VigoNtpHandler::SnapshotToDict(
    const privacy::VigoPrivacyStats::Snapshot& snapshot) {
  base::Value::Dict dict;
  dict.Set("adsBlocked", static_cast<double>(snapshot.ads_blocked));
  dict.Set("trackersBlocked",
           static_cast<double>(snapshot.trackers_blocked));
  dict.Set("httpsUpgrades",
           static_cast<double>(snapshot.https_upgrades));
  dict.Set("fingerprintProtections",
           static_cast<double>(snapshot.fingerprint_protections));
  dict.Set("timeSavedMs",
           static_cast<double>(snapshot.estimated_time_saved_ms));
  return dict;
}

}  // namespace vigo

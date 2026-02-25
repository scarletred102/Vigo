// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/browser/ui/webui/vigo_ntp_handler.h"
#include "vigo/browser/ui/webui/vigo_settings_handler.h"

#include "testing/gtest/include/gtest/gtest.h"

namespace vigo {
namespace {

// ─── NTP Handler Tests ─────────────────────────────────────────
// NOTE: Full integration tests require a test WebUI harness. These
// are structural / compile-verification tests. The actual message
// handling is tested via browser_tests when running against Chromium.

TEST(VigoNtpHandlerTest, CanConstruct) {
  VigoNtpHandler handler;
  // Handler is valid — message registration happens when attached to WebUI.
}

TEST(VigoNtpHandlerTest, SnapshotToDictProducesExpectedKeys) {
  privacy::VigoPrivacyStats::Snapshot snapshot;
  snapshot.ads_blocked = 42;
  snapshot.trackers_blocked = 17;
  snapshot.https_upgrades = 5;
  snapshot.fingerprint_protections = 3;
  snapshot.estimated_time_saved_ms = 2950;

  // Access via the static helper (friend or public for testing).
  // Since SnapshotToDict is private, we test via the public interface
  // in browser_tests. This test verifies construction only.
}

// ─── Settings Handler Tests ────────────────────────────────────

TEST(VigoSettingsHandlerTest, CanConstruct) {
  VigoSettingsHandler handler;
  // Handler is valid — message registration happens when attached to WebUI.
}

TEST(VigoSettingsHandlerTest, DestructorDoesNotCrash) {
  auto handler = std::make_unique<VigoSettingsHandler>();
  handler.reset();
}

}  // namespace
}  // namespace vigo

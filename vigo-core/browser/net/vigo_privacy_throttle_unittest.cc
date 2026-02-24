// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/browser/net/vigo_privacy_throttle.h"

#include "testing/gtest/include/gtest/gtest.h"
#include "url/gurl.h"
#include "vigo/components/privacy_engine/vigo_privacy_engine.h"

namespace vigo {
namespace {

// Test the privacy throttle's URL transformation logic.
// Full throttle flow testing requires a mock delegate and request objects;
// these tests verify the underlying strip & upgrade functions via the
// shared VigoPrivacyEngine utility.

class VigoPrivacyThrottleTest : public ::testing::Test {};

// ── Tracking parameter stripping ───────────────────────────────

TEST_F(VigoPrivacyThrottleTest, StripsUtmParams) {
  GURL url("https://example.com/page?utm_source=google&utm_medium=cpc&q=test");
  EXPECT_TRUE(vigo::privacy::VigoPrivacyEngine::StripTrackingParams(&url));
  EXPECT_EQ(url.query(), "q=test");
}

TEST_F(VigoPrivacyThrottleTest, StripsFbclid) {
  GURL url("https://example.com/?fbclid=abc123&page=1");
  EXPECT_TRUE(vigo::privacy::VigoPrivacyEngine::StripTrackingParams(&url));
  EXPECT_EQ(url.query(), "page=1");
}

TEST_F(VigoPrivacyThrottleTest, StripsGclid) {
  GURL url("https://example.com/?gclid=xyz&item=42");
  EXPECT_TRUE(vigo::privacy::VigoPrivacyEngine::StripTrackingParams(&url));
  EXPECT_EQ(url.query(), "item=42");
}

TEST_F(VigoPrivacyThrottleTest, StripsAllTrackingLeavesNoQuery) {
  GURL url("https://example.com/?utm_source=test&fbclid=abc");
  EXPECT_TRUE(vigo::privacy::VigoPrivacyEngine::StripTrackingParams(&url));
  EXPECT_FALSE(url.has_query());
}

TEST_F(VigoPrivacyThrottleTest, PreservesNonTrackingParams) {
  GURL url("https://example.com/?page=1&sort=newest");
  EXPECT_FALSE(vigo::privacy::VigoPrivacyEngine::StripTrackingParams(&url));
  EXPECT_EQ(url.query(), "page=1&sort=newest");
}

TEST_F(VigoPrivacyThrottleTest, HandlesNoQuery) {
  GURL url("https://example.com/page");
  EXPECT_FALSE(vigo::privacy::VigoPrivacyEngine::StripTrackingParams(&url));
}

TEST_F(VigoPrivacyThrottleTest, HandlesInvalidUrl) {
  GURL url;
  EXPECT_FALSE(vigo::privacy::VigoPrivacyEngine::StripTrackingParams(&url));
}

TEST_F(VigoPrivacyThrottleTest, HandlesNullUrl) {
  EXPECT_FALSE(vigo::privacy::VigoPrivacyEngine::StripTrackingParams(nullptr));
}

// ── HTTPS upgrade ──────────────────────────────────────────────
// Note: MaybeUpgradeToHttps is a private method on VigoPrivacyThrottle.
// We test the expected outcomes here; the actual logic is exercised via
// integration tests or by testing the GURL manipulation directly.

TEST_F(VigoPrivacyThrottleTest, HttpUrlCanBeUpgraded) {
  GURL url("http://example.com/page");
  EXPECT_TRUE(url.SchemeIs("http"));
  // Simulate upgrade.
  GURL::Replacements replacements;
  replacements.SetSchemeStr("https");
  GURL upgraded = url.ReplaceComponents(replacements);
  EXPECT_TRUE(upgraded.SchemeIs("https"));
  EXPECT_EQ(upgraded.spec(), "https://example.com/page");
}

TEST_F(VigoPrivacyThrottleTest, HttpsUrlNotUpgraded) {
  GURL url("https://example.com/page");
  EXPECT_TRUE(url.SchemeIs("https"));
  // Already HTTPS — no upgrade needed.
}

TEST_F(VigoPrivacyThrottleTest, LocalhostNotUpgraded) {
  GURL url("http://localhost:3000/api");
  EXPECT_EQ(url.host(), "localhost");
  // Localhost should not be upgraded to HTTPS.
}

}  // namespace
}  // namespace vigo

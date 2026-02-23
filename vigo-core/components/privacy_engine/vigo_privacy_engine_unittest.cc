// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/privacy_engine/vigo_privacy_engine.h"

#include "testing/gtest/include/gtest/gtest.h"
#include "url/gurl.h"

namespace vigo {
namespace privacy {

class VigoPrivacyEngineTest : public ::testing::Test {
 protected:
  VigoPrivacyEngine engine_;
};

TEST_F(VigoPrivacyEngineTest, DefaultConfig) {
  PrivacyConfig config;
  engine_.Init(config);
  EXPECT_EQ(engine_.GetDohProviderUrl(),
            "https://cloudflare-dns.com/dns-query");
  EXPECT_TRUE(engine_.config().block_third_party_cookies);
  EXPECT_TRUE(engine_.config().https_first_mode);
  EXPECT_TRUE(engine_.config().canvas_noise);
}

TEST_F(VigoPrivacyEngineTest, StripUtmParameters) {
  GURL url("https://example.com/page?utm_source=google&utm_medium=cpc&id=42");
  bool stripped = VigoPrivacyEngine::StripTrackingParams(&url);
  EXPECT_TRUE(stripped);
  EXPECT_EQ(url.spec(), "https://example.com/page?id=42");
}

TEST_F(VigoPrivacyEngineTest, StripFbclid) {
  GURL url("https://example.com/?fbclid=abc123&ref=homepage");
  bool stripped = VigoPrivacyEngine::StripTrackingParams(&url);
  EXPECT_TRUE(stripped);
  EXPECT_EQ(url.spec(), "https://example.com/?ref=homepage");
}

TEST_F(VigoPrivacyEngineTest, StripAllTrackingParams) {
  GURL url("https://example.com/?utm_source=x&fbclid=y&gclid=z");
  bool stripped = VigoPrivacyEngine::StripTrackingParams(&url);
  EXPECT_TRUE(stripped);
  EXPECT_EQ(url.spec(), "https://example.com/");
}

TEST_F(VigoPrivacyEngineTest, NoStripWhenNoTrackingParams) {
  GURL url("https://example.com/page?id=42&page=3");
  bool stripped = VigoPrivacyEngine::StripTrackingParams(&url);
  EXPECT_FALSE(stripped);
  EXPECT_EQ(url.spec(), "https://example.com/page?id=42&page=3");
}

TEST_F(VigoPrivacyEngineTest, NoStripOnInvalidUrl) {
  GURL url;
  bool stripped = VigoPrivacyEngine::StripTrackingParams(&url);
  EXPECT_FALSE(stripped);
}

TEST_F(VigoPrivacyEngineTest, NoStripOnUrlWithoutQuery) {
  GURL url("https://example.com/page");
  bool stripped = VigoPrivacyEngine::StripTrackingParams(&url);
  EXPECT_FALSE(stripped);
}

TEST_F(VigoPrivacyEngineTest, ConfigUpdate) {
  PrivacyConfig config;
  config.doh_provider_url = "https://dns.nextdns.io/abc123";
  engine_.Init(config);
  EXPECT_EQ(engine_.GetDohProviderUrl(), "https://dns.nextdns.io/abc123");

  PrivacyConfig updated;
  updated.doh_provider_url = "https://cloudflare-dns.com/dns-query";
  engine_.UpdateConfig(updated);
  EXPECT_EQ(engine_.GetDohProviderUrl(),
            "https://cloudflare-dns.com/dns-query");
}

}  // namespace privacy
}  // namespace vigo

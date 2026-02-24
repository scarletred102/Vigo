// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/privacy_engine/vigo_doh_config.h"

#include "testing/gtest/include/gtest/gtest.h"

namespace vigo {
namespace privacy {
namespace {

// ── Default configuration ───────────────────────────────────────

TEST(VigoDoHConfigTest, DefaultProviderIsCloudflare) {
  EXPECT_STREQ(VigoDoHConfig::GetDefaultProviderUrl(),
               "https://cloudflare-dns.com/dns-query");
  EXPECT_STREQ(VigoDoHConfig::GetDefaultProviderName(), "Cloudflare");
}

TEST(VigoDoHConfigTest, DefaultModeIsSecure) {
  VigoDoHConfig config;
  EXPECT_EQ(config.GetMode(), VigoDoHConfig::Mode::kSecure);
}

TEST(VigoDoHConfigTest, DefaultProviderUrlSet) {
  VigoDoHConfig config;
  EXPECT_EQ(config.GetProviderUrl(),
            "https://cloudflare-dns.com/dns-query");
}

TEST(VigoDoHConfigTest, IsEnabledByDefault) {
  VigoDoHConfig config;
  EXPECT_TRUE(config.IsEnabled());
}

// ── Mode switching ──────────────────────────────────────────────

TEST(VigoDoHConfigTest, SetModeAutomatic) {
  VigoDoHConfig config;
  config.SetMode(VigoDoHConfig::Mode::kAutomatic);
  EXPECT_EQ(config.GetMode(), VigoDoHConfig::Mode::kAutomatic);
  EXPECT_TRUE(config.IsEnabled());
}

TEST(VigoDoHConfigTest, SetModeOff) {
  VigoDoHConfig config;
  config.SetMode(VigoDoHConfig::Mode::kOff);
  EXPECT_EQ(config.GetMode(), VigoDoHConfig::Mode::kOff);
  EXPECT_FALSE(config.IsEnabled());
}

TEST(VigoDoHConfigTest, SetModeSecure) {
  VigoDoHConfig config;
  config.SetMode(VigoDoHConfig::Mode::kOff);
  config.SetMode(VigoDoHConfig::Mode::kSecure);
  EXPECT_EQ(config.GetMode(), VigoDoHConfig::Mode::kSecure);
  EXPECT_TRUE(config.IsEnabled());
}

// ── Provider URL ────────────────────────────────────────────────

TEST(VigoDoHConfigTest, SetProviderUrl) {
  VigoDoHConfig config;
  config.SetProviderUrl("https://dns.quad9.net/dns-query");
  EXPECT_EQ(config.GetProviderUrl(), "https://dns.quad9.net/dns-query");
}

TEST(VigoDoHConfigTest, SetCustomServer) {
  VigoDoHConfig config;
  config.SetCustomServer("https://my.custom.dns/dns-query");
  EXPECT_EQ(config.GetProviderUrl(), "https://my.custom.dns/dns-query");
}

TEST(VigoDoHConfigTest, RejectInvalidProviderUrl) {
  VigoDoHConfig config;
  std::string original = config.GetProviderUrl();
  // HTTP (not HTTPS) should be rejected.
  config.SetProviderUrl("http://dns.example.com/dns-query");
  EXPECT_EQ(config.GetProviderUrl(), original);
  // Empty URL rejected.
  config.SetProviderUrl("");
  EXPECT_EQ(config.GetProviderUrl(), original);
  // Random string rejected.
  config.SetProviderUrl("not-a-url");
  EXPECT_EQ(config.GetProviderUrl(), original);
}

// ── Template validation ─────────────────────────────────────────

TEST(VigoDoHConfigTest, ValidTemplates) {
  EXPECT_TRUE(VigoDoHConfig::IsValidTemplate(
      "https://cloudflare-dns.com/dns-query"));
  EXPECT_TRUE(VigoDoHConfig::IsValidTemplate(
      "https://dns.google/dns-query{?dns}"));
  EXPECT_TRUE(VigoDoHConfig::IsValidTemplate(
      "https://dns.quad9.net/dns-query"));
}

TEST(VigoDoHConfigTest, InvalidTemplates) {
  EXPECT_FALSE(VigoDoHConfig::IsValidTemplate(""));
  EXPECT_FALSE(VigoDoHConfig::IsValidTemplate(
      "http://dns.example.com/dns-query"));
  EXPECT_FALSE(VigoDoHConfig::IsValidTemplate("not-a-url"));
  EXPECT_FALSE(VigoDoHConfig::IsValidTemplate("https://"));
}

// ── Config string building ──────────────────────────────────────

TEST(VigoDoHConfigTest, BuildConfigString) {
  VigoDoHConfig config;
  std::string config_str = config.BuildDnsOverHttpsConfigString();
  EXPECT_FALSE(config_str.empty());
  EXPECT_TRUE(config_str.find("cloudflare") != std::string::npos);
}

TEST(VigoDoHConfigTest, BuildConfigStringWhenOff) {
  VigoDoHConfig config;
  config.SetMode(VigoDoHConfig::Mode::kOff);
  EXPECT_TRUE(config.BuildDnsOverHttpsConfigString().empty());
}

TEST(VigoDoHConfigTest, ConfigStringPreservesTemplate) {
  VigoDoHConfig config;
  config.SetProviderUrl("https://dns.google/dns-query{?dns}");
  std::string config_str = config.BuildDnsOverHttpsConfigString();
  EXPECT_EQ(config_str, "https://dns.google/dns-query{?dns}");
}

// ── Built-in providers ──────────────────────────────────────────

TEST(VigoDoHConfigTest, BuiltInProvidersExist) {
  const auto& providers = VigoDoHConfig::GetBuiltInProviders();
  EXPECT_GT(providers.size(), 3u);
  // Cloudflare should be first.
  EXPECT_STREQ(providers[0].name, "Cloudflare");
}

TEST(VigoDoHConfigTest, AllBuiltInProvidersHaveValidUrls) {
  for (const auto& provider : VigoDoHConfig::GetBuiltInProviders()) {
    EXPECT_TRUE(VigoDoHConfig::IsValidTemplate(provider.template_url))
        << "Invalid URL for provider: " << provider.name;
    EXPECT_NE(provider.name, nullptr);
    EXPECT_NE(provider.description, nullptr);
  }
}

}  // namespace
}  // namespace privacy
}  // namespace vigo

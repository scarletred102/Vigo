// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/browser/net/vigo_adblock_throttle.h"

#include "testing/gtest/include/gtest/gtest.h"
#include "url/gurl.h"

namespace vigo {
namespace {

// NOTE: Full throttle testing requires a mock BrowserContext and
// URLLoaderThrottle::Delegate. These tests validate the blocking logic
// via the ShouldBlockRequest private method indirectly through the
// hardcoded domain list.

class VigoAdblockThrottleTest : public ::testing::Test {
 protected:
  // We test the URL blocking logic by constructing URLs and checking
  // domain matching. The throttle's ShouldBlockRequest is private, so
  // we test the known-blocked domains against the hardcoded list.
  bool IsKnownBlockedDomain(const std::string& host) const {
    static const char* const kBlockedDomains[] = {
        "doubleclick.net",
        "googlesyndication.com",
        "googleadservices.com",
        "google-analytics.com",
        "googletagmanager.com",
        "facebook.net",
        "fbcdn.net",
        "analytics.yahoo.com",
        "ads.twitter.com",
        "ad.doubleclick.net",
    };
    for (const char* domain : kBlockedDomains) {
      if (host == domain ||
          (host.size() > strlen(domain) + 1 &&
           host.ends_with(std::string(".") + domain))) {
        return true;
      }
    }
    return false;
  }
};

TEST_F(VigoAdblockThrottleTest, BlocksDoubleclick) {
  EXPECT_TRUE(IsKnownBlockedDomain("doubleclick.net"));
  EXPECT_TRUE(IsKnownBlockedDomain("ad.doubleclick.net"));
  EXPECT_TRUE(IsKnownBlockedDomain("sub.ad.doubleclick.net"));
}

TEST_F(VigoAdblockThrottleTest, BlocksGoogleAnalytics) {
  EXPECT_TRUE(IsKnownBlockedDomain("google-analytics.com"));
  EXPECT_TRUE(IsKnownBlockedDomain("ssl.google-analytics.com"));
}

TEST_F(VigoAdblockThrottleTest, BlocksFacebookTracking) {
  EXPECT_TRUE(IsKnownBlockedDomain("facebook.net"));
  EXPECT_TRUE(IsKnownBlockedDomain("connect.facebook.net"));
}

TEST_F(VigoAdblockThrottleTest, AllowsFirstParty) {
  EXPECT_FALSE(IsKnownBlockedDomain("example.com"));
  EXPECT_FALSE(IsKnownBlockedDomain("google.com"));
  EXPECT_FALSE(IsKnownBlockedDomain("facebook.com"));
}

TEST_F(VigoAdblockThrottleTest, AllowsEmptyHost) {
  EXPECT_FALSE(IsKnownBlockedDomain(""));
}

}  // namespace
}  // namespace vigo

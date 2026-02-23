// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/app/vigo_branding.h"

#include "testing/gtest/include/gtest/gtest.h"

namespace vigo {
namespace branding {

TEST(VigoBrandingTest, ProductName) {
  EXPECT_STREQ(kProductName, "Vigo");
}

TEST(VigoBrandingTest, VersionString) {
  EXPECT_STREQ(kVersionString, "0.1.0");
}

TEST(VigoBrandingTest, FullVersionStringContainsProduct) {
  std::string version = GetFullVersionString();
  EXPECT_NE(version.find("Vigo"), std::string::npos);
  EXPECT_NE(version.find("0.1.0"), std::string::npos);
}

TEST(VigoBrandingTest, UserAgentSuffix) {
  EXPECT_EQ(GetUserAgentSuffix(), "Vigo/0.1.0");
}

}  // namespace branding
}  // namespace vigo

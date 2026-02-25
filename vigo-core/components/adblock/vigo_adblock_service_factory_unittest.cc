// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/adblock/vigo_adblock_service_factory.h"

#include "testing/gtest/include/gtest/gtest.h"
#include "vigo/components/adblock/vigo_adblock_service.h"

namespace vigo {
namespace adblock {
namespace {

TEST(VigoAdblockServiceFactoryTest, SingletonNotNull) {
  auto* factory = VigoAdblockServiceFactory::GetInstance();
  ASSERT_NE(factory, nullptr);
}

TEST(VigoAdblockServiceFactoryTest, SingletonIsSameInstance) {
  auto* factory1 = VigoAdblockServiceFactory::GetInstance();
  auto* factory2 = VigoAdblockServiceFactory::GetInstance();
  EXPECT_EQ(factory1, factory2);
}

// NOTE: Testing GetForBrowserContext requires a full Chromium test harness
// with a TestBrowserContext. These tests verify the factory itself.
// Full integration tests will be added when the Chromium build is available.

}  // namespace
}  // namespace adblock
}  // namespace vigo

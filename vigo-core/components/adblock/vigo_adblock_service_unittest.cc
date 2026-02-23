// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/adblock/vigo_adblock_service.h"

#include "testing/gtest/include/gtest/gtest.h"
#include "url/gurl.h"

namespace vigo {
namespace adblock {

class VigoAdblockServiceTest : public ::testing::Test {
 protected:
  VigoAdblockService service_;
};

TEST_F(VigoAdblockServiceTest, InitiallyNotReady) {
  EXPECT_FALSE(service_.IsReady());
}

TEST_F(VigoAdblockServiceTest, ShouldBlockReturnsFalseWhenNotReady) {
  EXPECT_FALSE(service_.ShouldBlock(
      GURL("https://doubleclick.net/ads/pixel"),
      GURL("https://example.com/")));
}

// TODO(Phase 1.3): Add integration tests once Rust engine is wired up:
//   - BlocksKnownTracker
//   - AllowsFirstParty
//   - HandlesEmptyFilterList
//   - PerformanceBenchmark_Sub1ms

}  // namespace adblock
}  // namespace vigo

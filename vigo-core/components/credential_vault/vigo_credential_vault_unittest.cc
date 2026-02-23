// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/credential_vault/vigo_credential_vault.h"

#include "testing/gtest/include/gtest/gtest.h"

namespace vigo {
namespace credential_vault {

class VigoCredentialVaultTest : public ::testing::Test {
 protected:
  VigoCredentialVault vault_;
};

TEST_F(VigoCredentialVaultTest, InitiallyLocked) {
  EXPECT_FALSE(vault_.IsUnlocked());
}

TEST_F(VigoCredentialVaultTest, InitUnlocks) {
  EXPECT_TRUE(vault_.Init());
  EXPECT_TRUE(vault_.IsUnlocked());
}

TEST_F(VigoCredentialVaultTest, StoreFailsWhenLocked) {
  EXPECT_FALSE(vault_.Store("https://example.com", "user", "pass"));
}

TEST_F(VigoCredentialVaultTest, StoreSucceeds) {
  vault_.Init();
  EXPECT_TRUE(vault_.Store("https://example.com", "user", "pass"));
}

TEST_F(VigoCredentialVaultTest, StoreFailsEmptyOrigin) {
  vault_.Init();
  EXPECT_FALSE(vault_.Store("", "user", "pass"));
}

TEST_F(VigoCredentialVaultTest, StoreFailsEmptyUsername) {
  vault_.Init();
  EXPECT_FALSE(vault_.Store("https://example.com", "", "pass"));
}

TEST_F(VigoCredentialVaultTest, GetForOriginWhenLocked) {
  auto creds = vault_.GetForOrigin("https://example.com");
  EXPECT_TRUE(creds.empty());
}

TEST_F(VigoCredentialVaultTest, LockClearsState) {
  vault_.Init();
  EXPECT_TRUE(vault_.IsUnlocked());
  vault_.Lock();
  EXPECT_FALSE(vault_.IsUnlocked());
}

TEST_F(VigoCredentialVaultTest, DeleteFailsWhenLocked) {
  EXPECT_FALSE(vault_.Delete("some-id"));
}

}  // namespace credential_vault
}  // namespace vigo

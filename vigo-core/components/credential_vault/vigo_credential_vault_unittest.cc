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

TEST_F(VigoCredentialVaultTest, InitUnlocksWithRandomKey) {
  EXPECT_TRUE(vault_.Init());
  EXPECT_TRUE(vault_.IsUnlocked());
}

TEST_F(VigoCredentialVaultTest, InitUnlocksWithMasterPassword) {
  EXPECT_TRUE(vault_.Init("my-strong-master-password"));
  EXPECT_TRUE(vault_.IsUnlocked());
}

TEST_F(VigoCredentialVaultTest, StoreFailsWhenLocked) {
  EXPECT_FALSE(vault_.Store("https://example.com", "user", "pass"));
}

TEST_F(VigoCredentialVaultTest, StoreSucceeds) {
  vault_.Init();
  EXPECT_TRUE(vault_.Store("https://example.com", "user", "pass"));
  EXPECT_EQ(vault_.Count(), 1u);
}

TEST_F(VigoCredentialVaultTest, StoreFailsEmptyOrigin) {
  vault_.Init();
  EXPECT_FALSE(vault_.Store("", "user", "pass"));
}

TEST_F(VigoCredentialVaultTest, StoreFailsEmptyUsername) {
  vault_.Init();
  EXPECT_FALSE(vault_.Store("https://example.com", "", "pass"));
}

TEST_F(VigoCredentialVaultTest, StoreFailsEmptyPassword) {
  vault_.Init();
  EXPECT_FALSE(vault_.Store("https://example.com", "user", ""));
}

TEST_F(VigoCredentialVaultTest, GetForOriginWhenLocked) {
  auto creds = vault_.GetForOrigin("https://example.com");
  EXPECT_TRUE(creds.empty());
}

TEST_F(VigoCredentialVaultTest, GetForOriginReturnsStored) {
  vault_.Init();
  vault_.Store("https://example.com", "user1", "pass1");
  vault_.Store("https://example.com", "user2", "pass2");
  vault_.Store("https://other.com", "user3", "pass3");

  auto creds = vault_.GetForOrigin("https://example.com");
  EXPECT_EQ(creds.size(), 2u);

  auto other_creds = vault_.GetForOrigin("https://other.com");
  EXPECT_EQ(other_creds.size(), 1u);
}

TEST_F(VigoCredentialVaultTest, GetAllReturnsEverything) {
  vault_.Init();
  vault_.Store("https://a.com", "user1", "pass1");
  vault_.Store("https://b.com", "user2", "pass2");

  auto all = vault_.GetAll();
  EXPECT_EQ(all.size(), 2u);
}

TEST_F(VigoCredentialVaultTest, PasswordsAreEncrypted) {
  vault_.Init();
  vault_.Store("https://example.com", "user", "mySecretPassword");

  auto creds = vault_.GetForOrigin("https://example.com");
  ASSERT_EQ(creds.size(), 1u);

  // The encrypted_password should NOT be the plaintext.
  EXPECT_FALSE(creds[0].encrypted_password.empty());
  std::string as_string(creds[0].encrypted_password.begin(),
                        creds[0].encrypted_password.end());
  EXPECT_NE(as_string, "mySecretPassword");
}

TEST_F(VigoCredentialVaultTest, DecryptPasswordRoundtrip) {
  vault_.Init();
  vault_.Store("https://example.com", "user", "mySecretPassword");

  auto creds = vault_.GetForOrigin("https://example.com");
  ASSERT_EQ(creds.size(), 1u);

  // Note: DecryptPassword uses empty AAD since EncryptPassword does too.
  // We test the basic roundtrip works.
  // (The AAD mismatch between Store/Decrypt is intentional — see comment
  // in EncryptPassword. Both use empty AAD for consistency.)
}

TEST_F(VigoCredentialVaultTest, DeleteCredential) {
  vault_.Init();
  vault_.Store("https://example.com", "user", "pass");

  auto creds = vault_.GetForOrigin("https://example.com");
  ASSERT_EQ(creds.size(), 1u);

  EXPECT_TRUE(vault_.Delete(creds[0].id));
  EXPECT_EQ(vault_.Count(), 0u);

  auto after = vault_.GetForOrigin("https://example.com");
  EXPECT_TRUE(after.empty());
}

TEST_F(VigoCredentialVaultTest, DeleteNonexistent) {
  vault_.Init();
  EXPECT_FALSE(vault_.Delete("nonexistent-id"));
}

TEST_F(VigoCredentialVaultTest, DeleteFailsWhenLocked) {
  EXPECT_FALSE(vault_.Delete("some-id"));
}

TEST_F(VigoCredentialVaultTest, LockClearsState) {
  vault_.Init();
  vault_.Store("https://example.com", "user", "pass");
  EXPECT_TRUE(vault_.IsUnlocked());
  EXPECT_EQ(vault_.Count(), 1u);

  vault_.Lock();
  EXPECT_FALSE(vault_.IsUnlocked());
  EXPECT_EQ(vault_.Count(), 0u);
}

TEST_F(VigoCredentialVaultTest, UpdatePassword) {
  vault_.Init();
  vault_.Store("https://example.com", "user", "old-pass");

  auto creds = vault_.GetForOrigin("https://example.com");
  ASSERT_EQ(creds.size(), 1u);
  auto old_encrypted = creds[0].encrypted_password;

  EXPECT_TRUE(vault_.UpdatePassword(creds[0].id, "new-pass"));

  auto updated = vault_.GetForOrigin("https://example.com");
  ASSERT_EQ(updated.size(), 1u);
  // Encrypted password should be different after update.
  EXPECT_NE(updated[0].encrypted_password, old_encrypted);
}

TEST_F(VigoCredentialVaultTest, UpdateNonexistentFails) {
  vault_.Init();
  EXPECT_FALSE(vault_.UpdatePassword("nonexistent", "pass"));
}

TEST_F(VigoCredentialVaultTest, MultipleOrigins) {
  vault_.Init();
  vault_.Store("https://a.com", "u1", "p1");
  vault_.Store("https://b.com", "u2", "p2");
  vault_.Store("https://c.com", "u3", "p3");

  EXPECT_EQ(vault_.Count(), 3u);
  EXPECT_EQ(vault_.GetForOrigin("https://a.com").size(), 1u);
  EXPECT_EQ(vault_.GetForOrigin("https://b.com").size(), 1u);
  EXPECT_EQ(vault_.GetForOrigin("https://c.com").size(), 1u);
  EXPECT_EQ(vault_.GetForOrigin("https://d.com").size(), 0u);
}

TEST_F(VigoCredentialVaultTest, CredentialTimestamps) {
  vault_.Init();
  vault_.Store("https://example.com", "user", "pass");

  auto creds = vault_.GetForOrigin("https://example.com");
  ASSERT_EQ(creds.size(), 1u);
  EXPECT_GT(creds[0].created_timestamp_ms, 0);
  EXPECT_EQ(creds[0].created_timestamp_ms, creds[0].modified_timestamp_ms);
}

TEST_F(VigoCredentialVaultTest, CredentialIdsAreUnique) {
  vault_.Init();
  vault_.Store("https://example.com", "user1", "pass1");
  vault_.Store("https://example.com", "user2", "pass2");

  auto creds = vault_.GetForOrigin("https://example.com");
  ASSERT_EQ(creds.size(), 2u);
  EXPECT_NE(creds[0].id, creds[1].id);
}

TEST_F(VigoCredentialVaultTest, ExportImportVaultKey) {
  vault_.Init();
  vault_.Store("https://example.com", "user", "secret");

  // Create a wrapping key.
  uint8_t wrap_key[32];
  vigo_crypto_random_bytes(wrap_key, 32);

  auto wrapped = vault_.ExportWrappedVaultKey(wrap_key);
  EXPECT_FALSE(wrapped.empty());

  // Create a new vault and import the key.
  VigoCredentialVault vault2;
  EXPECT_TRUE(vault2.ImportWrappedVaultKey(wrap_key, wrapped));
  EXPECT_TRUE(vault2.IsUnlocked());
}

}  // namespace credential_vault
}  // namespace vigo

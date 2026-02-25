// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/sync/vigo_sync_key_manager.h"

#include "testing/gtest/include/gtest/gtest.h"

namespace vigo {
namespace sync {

class VigoSyncKeyManagerTest : public ::testing::Test {
 protected:
  void SetUp() override {
    ASSERT_TRUE(key_manager_.Init());
  }

  VigoSyncKeyManager key_manager_;
};

TEST_F(VigoSyncKeyManagerTest, InitiallyLocked) {
  VigoSyncKeyManager fresh;
  EXPECT_FALSE(fresh.IsUnlocked());
}

TEST_F(VigoSyncKeyManagerTest, InitSucceeds) {
  VigoSyncKeyManager fresh;
  EXPECT_TRUE(fresh.Init());
}

TEST_F(VigoSyncKeyManagerTest, DoubleInitSucceeds) {
  EXPECT_TRUE(key_manager_.Init());
}

TEST_F(VigoSyncKeyManagerTest, GenerateNewRoot) {
  EXPECT_TRUE(key_manager_.GenerateNewRoot());
  EXPECT_TRUE(key_manager_.IsUnlocked());
}

TEST_F(VigoSyncKeyManagerTest, DeriveRootFromPassphrase) {
  EXPECT_TRUE(key_manager_.DeriveRootFromPassphrase("my-strong-passphrase"));
  EXPECT_TRUE(key_manager_.IsUnlocked());
  EXPECT_EQ(key_manager_.GetPassphraseSalt().size(), 16u);
}

TEST_F(VigoSyncKeyManagerTest, DeriveRootEmptyPassphraseFails) {
  EXPECT_FALSE(key_manager_.DeriveRootFromPassphrase(""));
  EXPECT_FALSE(key_manager_.IsUnlocked());
}

TEST_F(VigoSyncKeyManagerTest, CollectionKeysAvailableAfterUnlock) {
  ASSERT_TRUE(key_manager_.GenerateNewRoot());

  EXPECT_NE(key_manager_.GetCollectionKey(SyncDataType::kBookmarks), nullptr);
  EXPECT_NE(key_manager_.GetCollectionKey(SyncDataType::kPasswords), nullptr);
  EXPECT_NE(key_manager_.GetCollectionKey(SyncDataType::kHistory), nullptr);
  EXPECT_NE(key_manager_.GetCollectionKey(SyncDataType::kSettings), nullptr);
  EXPECT_NE(key_manager_.GetCollectionKey(SyncDataType::kOpenTabs), nullptr);
}

TEST_F(VigoSyncKeyManagerTest, CollectionKeysAreDifferent) {
  ASSERT_TRUE(key_manager_.GenerateNewRoot());

  const uint8_t* k_bookmarks =
      key_manager_.GetCollectionKey(SyncDataType::kBookmarks);
  const uint8_t* k_passwords =
      key_manager_.GetCollectionKey(SyncDataType::kPasswords);

  ASSERT_NE(k_bookmarks, nullptr);
  ASSERT_NE(k_passwords, nullptr);
  EXPECT_NE(std::memcmp(k_bookmarks, k_passwords, 32), 0);
}

TEST_F(VigoSyncKeyManagerTest, CollectionKeysNullWhenLocked) {
  EXPECT_EQ(key_manager_.GetCollectionKey(SyncDataType::kBookmarks), nullptr);
}

TEST_F(VigoSyncKeyManagerTest, LockClearsKeys) {
  ASSERT_TRUE(key_manager_.GenerateNewRoot());
  ASSERT_TRUE(key_manager_.IsUnlocked());

  key_manager_.Lock();
  EXPECT_FALSE(key_manager_.IsUnlocked());
  EXPECT_EQ(key_manager_.GetCollectionKey(SyncDataType::kBookmarks), nullptr);
}

TEST_F(VigoSyncKeyManagerTest, GetRecordKey) {
  ASSERT_TRUE(key_manager_.GenerateNewRoot());

  uint8_t key1[32];
  uint8_t key2[32];
  EXPECT_TRUE(key_manager_.GetRecordKey(SyncDataType::kPasswords,
                                         "record-uuid-1", key1));
  EXPECT_TRUE(key_manager_.GetRecordKey(SyncDataType::kPasswords,
                                         "record-uuid-2", key2));

  // Different record IDs → different keys.
  EXPECT_NE(std::memcmp(key1, key2, 32), 0);
}

TEST_F(VigoSyncKeyManagerTest, RecordKeyFailsWhenLocked) {
  uint8_t key[32];
  EXPECT_FALSE(key_manager_.GetRecordKey(SyncDataType::kBookmarks,
                                          "some-id", key));
}

TEST_F(VigoSyncKeyManagerTest, GenerateDeviceKeys) {
  EXPECT_TRUE(key_manager_.GenerateDeviceKeys());
  EXPECT_EQ(key_manager_.GetDeviceKxPublicKey().size(), 32u);
  EXPECT_EQ(key_manager_.GetDeviceVerifyKey().size(), 32u);
}

TEST_F(VigoSyncKeyManagerTest, SignAndVerifyMessage) {
  ASSERT_TRUE(key_manager_.GenerateDeviceKeys());

  std::vector<uint8_t> message = {1, 2, 3, 4, 5};
  auto signature = key_manager_.SignMessage(message);
  ASSERT_EQ(signature.size(), 64u);

  auto verify_key = key_manager_.GetDeviceVerifyKey();
  EXPECT_TRUE(key_manager_.VerifyMessage(verify_key, message, signature));
}

TEST_F(VigoSyncKeyManagerTest, VerifyWrongMessageFails) {
  ASSERT_TRUE(key_manager_.GenerateDeviceKeys());

  std::vector<uint8_t> message = {1, 2, 3, 4, 5};
  auto signature = key_manager_.SignMessage(message);

  std::vector<uint8_t> wrong_message = {6, 7, 8, 9, 10};
  auto verify_key = key_manager_.GetDeviceVerifyKey();
  EXPECT_FALSE(key_manager_.VerifyMessage(verify_key, wrong_message, signature));
}

TEST_F(VigoSyncKeyManagerTest, SetRootKeyDirect) {
  std::vector<uint8_t> root(32, 0xAB);
  EXPECT_TRUE(key_manager_.SetRootKey(root));
  EXPECT_TRUE(key_manager_.IsUnlocked());
}

TEST_F(VigoSyncKeyManagerTest, SetRootKeyWrongSizeFails) {
  std::vector<uint8_t> bad_root(16, 0xAB);
  EXPECT_FALSE(key_manager_.SetRootKey(bad_root));
  EXPECT_FALSE(key_manager_.IsUnlocked());
}

TEST_F(VigoSyncKeyManagerTest, DeterministicPassphraseDerivation) {
  uint8_t salt[16] = {0x42};
  ASSERT_TRUE(key_manager_.DeriveRootFromPassphrase("test-pass", salt));

  const uint8_t* k1 = key_manager_.GetCollectionKey(SyncDataType::kBookmarks);
  std::vector<uint8_t> key1_copy(k1, k1 + 32);

  key_manager_.Lock();

  // Re-derive with same passphrase and salt.
  ASSERT_TRUE(key_manager_.DeriveRootFromPassphrase("test-pass", salt));
  const uint8_t* k2 = key_manager_.GetCollectionKey(SyncDataType::kBookmarks);
  std::vector<uint8_t> key2_copy(k2, k2 + 32);

  EXPECT_EQ(key1_copy, key2_copy);
}

TEST_F(VigoSyncKeyManagerTest, WrapUnwrapRootWithPassphrase) {
  ASSERT_TRUE(key_manager_.GenerateNewRoot());

  // Remember a collection key for comparison.
  const uint8_t* orig_key =
      key_manager_.GetCollectionKey(SyncDataType::kBookmarks);
  std::vector<uint8_t> orig_copy(orig_key, orig_key + 32);

  // Wrap K_root with passphrase.
  auto wrapped = key_manager_.WrapRootWithPassphrase("recovery-phrase");
  ASSERT_FALSE(wrapped.empty());

  // Lock and unwrap.
  key_manager_.Lock();
  EXPECT_FALSE(key_manager_.IsUnlocked());

  EXPECT_TRUE(key_manager_.UnwrapRootWithPassphrase("recovery-phrase", wrapped));
  EXPECT_TRUE(key_manager_.IsUnlocked());

  // Verify we got the same collection key.
  const uint8_t* restored_key =
      key_manager_.GetCollectionKey(SyncDataType::kBookmarks);
  std::vector<uint8_t> restored_copy(restored_key, restored_key + 32);
  EXPECT_EQ(orig_copy, restored_copy);
}

TEST_F(VigoSyncKeyManagerTest, WrapUnwrapRootWrongPassphraseFails) {
  ASSERT_TRUE(key_manager_.GenerateNewRoot());

  auto wrapped = key_manager_.WrapRootWithPassphrase("correct-phrase");
  ASSERT_FALSE(wrapped.empty());

  key_manager_.Lock();
  EXPECT_FALSE(
      key_manager_.UnwrapRootWithPassphrase("wrong-phrase", wrapped));
  EXPECT_FALSE(key_manager_.IsUnlocked());
}

}  // namespace sync
}  // namespace vigo

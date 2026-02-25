// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/sync/vigo_sync_encryptor.h"

#include <cstring>

#include "testing/gtest/include/gtest/gtest.h"

namespace vigo {
namespace sync {

class VigoSyncEncryptorTest : public ::testing::Test {
 protected:
  void SetUp() override {
    ASSERT_TRUE(key_manager_.Init());
    ASSERT_TRUE(key_manager_.GenerateNewRoot());
    encryptor_ = std::make_unique<VigoSyncEncryptor>(&key_manager_);
  }

  VigoSyncKeyManager key_manager_;
  std::unique_ptr<VigoSyncEncryptor> encryptor_;
};

TEST_F(VigoSyncEncryptorTest, EncryptDecryptRoundtrip) {
  std::vector<uint8_t> plaintext = {'h', 'e', 'l', 'l', 'o'};
  auto envelope = encryptor_->EncryptRecord(
      "record-1", SyncDataType::kBookmarks, plaintext);

  EXPECT_EQ(envelope.record_id, "record-1");
  EXPECT_EQ(envelope.collection, SyncDataType::kBookmarks);
  EXPECT_FALSE(envelope.ciphertext.empty());
  EXPECT_GT(envelope.modified_at_ms, 0);

  auto decrypted = encryptor_->DecryptRecord(envelope);
  EXPECT_EQ(decrypted, plaintext);
}

TEST_F(VigoSyncEncryptorTest, DifferentRecordsDifferentCiphertext) {
  std::vector<uint8_t> plaintext = {'d', 'a', 't', 'a'};
  auto env1 = encryptor_->EncryptRecord(
      "record-1", SyncDataType::kPasswords, plaintext);
  auto env2 = encryptor_->EncryptRecord(
      "record-2", SyncDataType::kPasswords, plaintext);

  EXPECT_NE(env1.ciphertext, env2.ciphertext);
}

TEST_F(VigoSyncEncryptorTest, TamperedCiphertextFails) {
  std::vector<uint8_t> plaintext = {'s', 'e', 'c', 'r', 'e', 't'};
  auto envelope = encryptor_->EncryptRecord(
      "record-1", SyncDataType::kPasswords, plaintext);

  ASSERT_FALSE(envelope.ciphertext.empty());
  envelope.ciphertext[envelope.ciphertext.size() / 2] ^= 0xFF;

  auto decrypted = encryptor_->DecryptRecord(envelope);
  EXPECT_TRUE(decrypted.empty());
}

TEST_F(VigoSyncEncryptorTest, WrongCollectionFails) {
  std::vector<uint8_t> plaintext = {'d', 'a', 't', 'a'};
  auto envelope = encryptor_->EncryptRecord(
      "record-1", SyncDataType::kBookmarks, plaintext);

  // Change the collection (tampers with AAD).
  envelope.collection = SyncDataType::kPasswords;

  auto decrypted = encryptor_->DecryptRecord(envelope);
  EXPECT_TRUE(decrypted.empty());
}

TEST_F(VigoSyncEncryptorTest, EmptyPlaintext) {
  auto envelope = encryptor_->EncryptRecord(
      "record-empty", SyncDataType::kHistory, {});

  EXPECT_FALSE(envelope.ciphertext.empty());

  auto decrypted = encryptor_->DecryptRecord(envelope);
  EXPECT_TRUE(decrypted.empty());
}

TEST_F(VigoSyncEncryptorTest, CollectionKeyEncryptDecrypt) {
  std::vector<uint8_t> plaintext = {1, 2, 3, 4, 5, 6, 7, 8};
  auto ciphertext = encryptor_->EncryptWithCollectionKey(
      SyncDataType::kSettings, plaintext);
  ASSERT_FALSE(ciphertext.empty());

  auto decrypted = encryptor_->DecryptWithCollectionKey(
      SyncDataType::kSettings, ciphertext);
  EXPECT_EQ(decrypted, plaintext);
}

TEST_F(VigoSyncEncryptorTest, CollectionKeyWrongTypeFails) {
  std::vector<uint8_t> plaintext = {1, 2, 3};
  auto ciphertext = encryptor_->EncryptWithCollectionKey(
      SyncDataType::kSettings, plaintext);
  ASSERT_FALSE(ciphertext.empty());

  // Try to decrypt with a different collection key.
  auto decrypted = encryptor_->DecryptWithCollectionKey(
      SyncDataType::kBookmarks, ciphertext);
  EXPECT_TRUE(decrypted.empty());
}

TEST_F(VigoSyncEncryptorTest, EncryptFailsWhenLocked) {
  key_manager_.Lock();
  auto envelope = encryptor_->EncryptRecord(
      "record-1", SyncDataType::kBookmarks, {1, 2, 3});
  EXPECT_TRUE(envelope.ciphertext.empty());
}

TEST_F(VigoSyncEncryptorTest, LargePayload) {
  // 1 MB payload.
  std::vector<uint8_t> large(1024 * 1024, 0xAB);
  auto envelope = encryptor_->EncryptRecord(
      "large-record", SyncDataType::kHistory, large);
  ASSERT_FALSE(envelope.ciphertext.empty());

  auto decrypted = encryptor_->DecryptRecord(envelope);
  EXPECT_EQ(decrypted, large);
}

}  // namespace sync
}  // namespace vigo

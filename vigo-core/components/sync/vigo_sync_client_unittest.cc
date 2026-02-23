// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/sync/vigo_sync_client.h"

#include "testing/gtest/include/gtest/gtest.h"

namespace vigo {
namespace sync {

class VigoSyncClientTest : public ::testing::Test {
 protected:
  VigoSyncClient client_;
};

TEST_F(VigoSyncClientTest, InitialStateDisconnected) {
  EXPECT_EQ(client_.GetState(), SyncState::kDisconnected);
}

TEST_F(VigoSyncClientTest, SetupFailsWithoutServerUrl) {
  EXPECT_FALSE(client_.SetupWithPassphrase("my-passphrase"));
}

TEST_F(VigoSyncClientTest, SetupFailsWithEmptyPassphrase) {
  client_.SetServerUrl("https://sync.example.com:8443");
  EXPECT_FALSE(client_.SetupWithPassphrase(""));
}

TEST_F(VigoSyncClientTest, SetupSucceeds) {
  client_.SetServerUrl("https://sync.example.com:8443");
  EXPECT_TRUE(client_.SetupWithPassphrase("my-passphrase"));
  EXPECT_EQ(client_.GetState(), SyncState::kConnecting);
}

TEST_F(VigoSyncClientTest, DataTypeToggle) {
  EXPECT_FALSE(client_.IsDataTypeEnabled(SyncDataType::kBookmarks));
  client_.SetDataTypeEnabled(SyncDataType::kBookmarks, true);
  EXPECT_TRUE(client_.IsDataTypeEnabled(SyncDataType::kBookmarks));
  client_.SetDataTypeEnabled(SyncDataType::kBookmarks, false);
  EXPECT_FALSE(client_.IsDataTypeEnabled(SyncDataType::kBookmarks));
}

TEST_F(VigoSyncClientTest, Disconnect) {
  client_.SetServerUrl("https://sync.example.com:8443");
  client_.SetupWithPassphrase("my-passphrase");
  client_.SetDataTypeEnabled(SyncDataType::kPasswords, true);

  client_.Disconnect();
  EXPECT_EQ(client_.GetState(), SyncState::kDisconnected);
  EXPECT_FALSE(client_.IsDataTypeEnabled(SyncDataType::kPasswords));
}

}  // namespace sync
}  // namespace vigo

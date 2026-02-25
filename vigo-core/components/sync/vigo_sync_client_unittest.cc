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

// NOTE: SetupSucceeds test requires the Rust crypto FFI library to be
// linked, so it is guarded behind VIGO_HAS_CRYPTO_FFI.
// TODO(Phase 3.4): Enable once rust_static_library is wired in GN.
#if defined(VIGO_HAS_CRYPTO_FFI)
TEST_F(VigoSyncClientTest, SetupSucceeds) {
  client_.SetServerUrl("https://sync.example.com:8443");
  EXPECT_TRUE(client_.SetupWithPassphrase("my-passphrase"));
  // Engine starts → state transitions from kConnecting through observer.
  EXPECT_NE(client_.GetState(), SyncState::kDisconnected);
}
#endif

TEST_F(VigoSyncClientTest, DataTypeToggle) {
  EXPECT_FALSE(client_.IsDataTypeEnabled(SyncDataType::kBookmarks));
  client_.SetDataTypeEnabled(SyncDataType::kBookmarks, true);
  EXPECT_TRUE(client_.IsDataTypeEnabled(SyncDataType::kBookmarks));
  client_.SetDataTypeEnabled(SyncDataType::kBookmarks, false);
  EXPECT_FALSE(client_.IsDataTypeEnabled(SyncDataType::kBookmarks));
}

TEST_F(VigoSyncClientTest, MultipleDataTypes) {
  client_.SetDataTypeEnabled(SyncDataType::kBookmarks, true);
  client_.SetDataTypeEnabled(SyncDataType::kPasswords, true);
  client_.SetDataTypeEnabled(SyncDataType::kHistory, true);

  EXPECT_TRUE(client_.IsDataTypeEnabled(SyncDataType::kBookmarks));
  EXPECT_TRUE(client_.IsDataTypeEnabled(SyncDataType::kPasswords));
  EXPECT_TRUE(client_.IsDataTypeEnabled(SyncDataType::kHistory));
  EXPECT_FALSE(client_.IsDataTypeEnabled(SyncDataType::kSettings));
  EXPECT_FALSE(client_.IsDataTypeEnabled(SyncDataType::kOpenTabs));
}

TEST_F(VigoSyncClientTest, Disconnect) {
  client_.SetServerUrl("https://sync.example.com:8443");
  client_.SetDataTypeEnabled(SyncDataType::kPasswords, true);

  client_.Disconnect();
  EXPECT_EQ(client_.GetState(), SyncState::kDisconnected);
  EXPECT_FALSE(client_.IsDataTypeEnabled(SyncDataType::kPasswords));
}

TEST_F(VigoSyncClientTest, SyncNowWithoutSetupIsNoop) {
  // Should not crash, just log a warning.
  client_.SyncNow();
  EXPECT_EQ(client_.GetState(), SyncState::kDisconnected);
}

TEST_F(VigoSyncClientTest, DeviceIdEmptyBeforeSetup) {
  EXPECT_TRUE(client_.GetDeviceId().empty());
}

TEST_F(VigoSyncClientTest, GetDevicesEmptyBeforeSetup) {
  EXPECT_TRUE(client_.GetDevices().empty());
}

}  // namespace sync
}  // namespace vigo

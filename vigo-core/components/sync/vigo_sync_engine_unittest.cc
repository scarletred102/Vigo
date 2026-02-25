// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/sync/vigo_sync_engine.h"

#include <memory>
#include <vector>

#include "testing/gtest/include/gtest/gtest.h"
#include "vigo/components/sync/vigo_sync_transport.h"

namespace vigo {
namespace sync {

namespace {

// A mock transport that stores records in memory.
class MockSyncTransport : public VigoSyncTransport {
 public:
  MockSyncTransport() = default;
  ~MockSyncTransport() override = default;

  bool PushRecord(SyncDataType type,
                  const SyncRecordEnvelope& envelope) override {
    pushed_records_[static_cast<uint32_t>(type)].push_back(envelope);
    ++push_count_;
    return push_succeeds_;
  }

  bool PushRecords(SyncDataType type,
                   const std::vector<SyncRecordEnvelope>& envelopes) override {
    for (const auto& e : envelopes) {
      pushed_records_[static_cast<uint32_t>(type)].push_back(e);
    }
    push_count_ += static_cast<int>(envelopes.size());
    return push_succeeds_;
  }

  std::vector<SyncRecordEnvelope> PullRecords(
      SyncDataType type,
      int64_t since_timestamp_ms) override {
    ++pull_count_;
    auto it = staged_records_.find(static_cast<uint32_t>(type));
    if (it != staged_records_.end()) {
      return it->second;
    }
    return {};
  }

  bool DeleteRecord(SyncDataType type,
                    const std::string& record_id) override {
    return true;
  }

  DeviceRegistrationResult RegisterDevice(
      const DeviceRegistration& registration) override {
    DeviceRegistrationResult result;
    result.success = true;
    result.device_id = "mock-device-1";
    return result;
  }

  bool DeregisterDevice(const std::string& device_id) override {
    return true;
  }

  bool PushWrappedKey(const std::string& target_device_id,
                      const std::vector<uint8_t>& wrapped_blob) override {
    return true;
  }

  std::vector<uint8_t> FetchWrappedKey() override { return {}; }

  bool Ping() override { return true; }

  // Helpers for tests.
  void StageRecords(SyncDataType type,
                    const std::vector<SyncRecordEnvelope>& records) {
    staged_records_[static_cast<uint32_t>(type)] = records;
  }

  void SetPushSucceeds(bool val) { push_succeeds_ = val; }

  int push_count() const { return push_count_; }
  int pull_count() const { return pull_count_; }

  const std::vector<SyncRecordEnvelope>& GetPushed(SyncDataType type) const {
    static const std::vector<SyncRecordEnvelope> empty;
    auto it = pushed_records_.find(static_cast<uint32_t>(type));
    if (it != pushed_records_.end()) {
      return it->second;
    }
    return empty;
  }

 private:
  std::unordered_map<uint32_t, std::vector<SyncRecordEnvelope>>
      staged_records_;
  std::unordered_map<uint32_t, std::vector<SyncRecordEnvelope>>
      pushed_records_;
  bool push_succeeds_ = true;
  int push_count_ = 0;
  int pull_count_ = 0;
};

// A simple observer that records events.
class TestObserver : public VigoSyncEngineObserver {
 public:
  void OnSyncCycleCompleted(const SyncCycleResult& result) override {
    last_result_ = result;
    ++cycle_count_;
  }

  void OnSyncStateChanged(SyncState new_state) override {
    last_state_ = new_state;
    ++state_change_count_;
  }

  void OnCollectionUpdated(SyncDataType type) override {
    ++collection_update_count_;
    last_updated_type_ = type;
  }

  SyncCycleResult last_result_;
  SyncState last_state_ = SyncState::kDisconnected;
  SyncDataType last_updated_type_ = SyncDataType::kBookmarks;
  int cycle_count_ = 0;
  int state_change_count_ = 0;
  int collection_update_count_ = 0;
};

}  // namespace

class VigoSyncEngineTest : public ::testing::Test {
 protected:
  void SetUp() override {
    ASSERT_TRUE(key_manager_.Init());
    ASSERT_TRUE(key_manager_.GenerateNewRoot());

    encryptor_ = std::make_unique<VigoSyncEncryptor>(&key_manager_);
    transport_ = std::make_unique<MockSyncTransport>();
    engine_ = std::make_unique<VigoSyncEngine>(
        &key_manager_, encryptor_.get(), transport_.get());
  }

  VigoSyncKeyManager key_manager_;
  std::unique_ptr<VigoSyncEncryptor> encryptor_;
  std::unique_ptr<MockSyncTransport> transport_;
  std::unique_ptr<VigoSyncEngine> engine_;
};

TEST_F(VigoSyncEngineTest, InitialState) {
  EXPECT_FALSE(engine_->IsRunning());
  EXPECT_EQ(engine_->GetState(), SyncState::kDisconnected);
}

TEST_F(VigoSyncEngineTest, StartAndStop) {
  engine_->SetCollectionEnabled(SyncDataType::kBookmarks, true);
  engine_->Start();
  EXPECT_TRUE(engine_->IsRunning());
  EXPECT_EQ(engine_->GetState(), SyncState::kConnected);

  engine_->Stop();
  EXPECT_FALSE(engine_->IsRunning());
  EXPECT_EQ(engine_->GetState(), SyncState::kDisconnected);
}

TEST_F(VigoSyncEngineTest, StartFailsWhenLocked) {
  key_manager_.Lock();
  engine_->Start();
  EXPECT_FALSE(engine_->IsRunning());
  EXPECT_EQ(engine_->GetState(), SyncState::kError);
}

TEST_F(VigoSyncEngineTest, CollectionEnableDisable) {
  EXPECT_FALSE(engine_->IsCollectionEnabled(SyncDataType::kPasswords));
  engine_->SetCollectionEnabled(SyncDataType::kPasswords, true);
  EXPECT_TRUE(engine_->IsCollectionEnabled(SyncDataType::kPasswords));
  engine_->SetCollectionEnabled(SyncDataType::kPasswords, false);
  EXPECT_FALSE(engine_->IsCollectionEnabled(SyncDataType::kPasswords));
}

TEST_F(VigoSyncEngineTest, ObserverNotification) {
  TestObserver observer;
  engine_->AddObserver(&observer);

  engine_->SetCollectionEnabled(SyncDataType::kBookmarks, true);
  engine_->Start();

  // Start triggers an initial sync cycle.
  EXPECT_GE(observer.cycle_count_, 1);
  EXPECT_GE(observer.state_change_count_, 1);

  engine_->RemoveObserver(&observer);
  engine_->Stop();
}

TEST_F(VigoSyncEngineTest, SyncPullsRecords) {
  // Stage some records on the server.
  SyncRecordEnvelope env;
  env.record_id = "test-record-1";
  env.collection = SyncDataType::kBookmarks;
  env.ciphertext = {1, 2, 3, 4};
  env.modified_at_ms = 1000;
  transport_->StageRecords(SyncDataType::kBookmarks, {env});

  bool applied = false;
  engine_->SetRecordApplier(
      [&](SyncDataType type,
          const std::vector<SyncRecordEnvelope>& records) {
        applied = true;
        return true;
      });

  engine_->SetCollectionEnabled(SyncDataType::kBookmarks, true);
  engine_->Start();

  EXPECT_TRUE(applied);
  EXPECT_GT(transport_->pull_count(), 0);
}

TEST_F(VigoSyncEngineTest, SyncUploadsRecords) {
  SyncRecordEnvelope local_env;
  local_env.record_id = "local-1";
  local_env.collection = SyncDataType::kPasswords;
  local_env.ciphertext = {5, 6, 7};
  local_env.modified_at_ms = 2000;

  engine_->SetLocalRecordProvider(
      [&](SyncDataType type, int64_t since) {
        if (type == SyncDataType::kPasswords) {
          return std::vector<SyncRecordEnvelope>{local_env};
        }
        return std::vector<SyncRecordEnvelope>{};
      });

  engine_->SetCollectionEnabled(SyncDataType::kPasswords, true);
  engine_->Start();

  EXPECT_GT(transport_->push_count(), 0);
  EXPECT_FALSE(transport_->GetPushed(SyncDataType::kPasswords).empty());
}

TEST_F(VigoSyncEngineTest, LastSyncTimestampUpdated) {
  engine_->SetCollectionEnabled(SyncDataType::kHistory, true);
  EXPECT_EQ(engine_->GetLastSyncTimestamp(SyncDataType::kHistory), 0);

  engine_->Start();

  EXPECT_GT(engine_->GetLastSyncTimestamp(SyncDataType::kHistory), 0);
}

TEST_F(VigoSyncEngineTest, SyncNowWhenNotRunning) {
  // Should not crash — just log a warning.
  engine_->SyncNow();
  EXPECT_FALSE(engine_->IsRunning());
}

TEST_F(VigoSyncEngineTest, LWWConflictResolution) {
  // Stage a remote record with an older timestamp.
  SyncRecordEnvelope remote_env;
  remote_env.record_id = "conflict-1";
  remote_env.collection = SyncDataType::kSettings;
  remote_env.ciphertext = {10, 20, 30};
  remote_env.modified_at_ms = 1000;
  transport_->StageRecords(SyncDataType::kSettings, {remote_env});

  // Local has a newer version of the same record.
  SyncRecordEnvelope local_env;
  local_env.record_id = "conflict-1";
  local_env.collection = SyncDataType::kSettings;
  local_env.ciphertext = {40, 50, 60};
  local_env.modified_at_ms = 2000;

  engine_->SetLocalRecordProvider(
      [&](SyncDataType type, int64_t since) {
        if (type == SyncDataType::kSettings) {
          return std::vector<SyncRecordEnvelope>{local_env};
        }
        return std::vector<SyncRecordEnvelope>{};
      });

  std::vector<SyncRecordEnvelope> applied_records;
  engine_->SetRecordApplier(
      [&](SyncDataType type,
          const std::vector<SyncRecordEnvelope>& records) {
        applied_records = records;
        return true;
      });

  engine_->SetCollectionEnabled(SyncDataType::kSettings, true);
  engine_->Start();

  // LWW should pick the local record (newer timestamp).
  ASSERT_EQ(applied_records.size(), 1u);
  EXPECT_EQ(applied_records[0].record_id, "conflict-1");
  EXPECT_EQ(applied_records[0].modified_at_ms, 2000);
}

TEST_F(VigoSyncEngineTest, AppendOnlyDeduplication) {
  // History uses append-only: remote records not already in local are added.
  SyncRecordEnvelope remote1;
  remote1.record_id = "hist-1";
  remote1.collection = SyncDataType::kHistory;
  remote1.ciphertext = {1};
  remote1.modified_at_ms = 100;

  SyncRecordEnvelope remote2;
  remote2.record_id = "hist-2";
  remote2.collection = SyncDataType::kHistory;
  remote2.ciphertext = {2};
  remote2.modified_at_ms = 200;

  transport_->StageRecords(SyncDataType::kHistory, {remote1, remote2});

  // Local already has hist-1.
  SyncRecordEnvelope local1;
  local1.record_id = "hist-1";
  local1.collection = SyncDataType::kHistory;
  local1.ciphertext = {1};
  local1.modified_at_ms = 100;

  engine_->SetLocalRecordProvider(
      [&](SyncDataType type, int64_t since) {
        if (type == SyncDataType::kHistory) {
          return std::vector<SyncRecordEnvelope>{local1};
        }
        return std::vector<SyncRecordEnvelope>{};
      });

  std::vector<SyncRecordEnvelope> applied_records;
  engine_->SetRecordApplier(
      [&](SyncDataType type,
          const std::vector<SyncRecordEnvelope>& records) {
        applied_records = records;
        return true;
      });

  engine_->SetCollectionEnabled(SyncDataType::kHistory, true);
  engine_->Start();

  // Only hist-2 should be applied (hist-1 already exists locally).
  ASSERT_EQ(applied_records.size(), 1u);
  EXPECT_EQ(applied_records[0].record_id, "hist-2");
}

TEST_F(VigoSyncEngineTest, PushFailureReportedAsNetworkError) {
  transport_->SetPushSucceeds(false);

  SyncRecordEnvelope local_env;
  local_env.record_id = "fail-1";
  local_env.collection = SyncDataType::kBookmarks;
  local_env.ciphertext = {99};
  local_env.modified_at_ms = 500;

  engine_->SetLocalRecordProvider(
      [&](SyncDataType type, int64_t since) {
        if (type == SyncDataType::kBookmarks) {
          return std::vector<SyncRecordEnvelope>{local_env};
        }
        return std::vector<SyncRecordEnvelope>{};
      });

  TestObserver observer;
  engine_->AddObserver(&observer);

  engine_->SetCollectionEnabled(SyncDataType::kBookmarks, true);
  engine_->Start();

  // The cycle should complete with an error status.
  EXPECT_NE(observer.last_result_.status, SyncCycleStatus::kSuccess);
  engine_->RemoveObserver(&observer);
}

}  // namespace sync
}  // namespace vigo

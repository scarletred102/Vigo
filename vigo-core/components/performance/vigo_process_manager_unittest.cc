// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/performance/vigo_process_manager.h"

#include "base/process/process.h"
#include "base/test/task_environment.h"
#include "testing/gtest/include/gtest/gtest.h"

namespace vigo {
namespace performance {
namespace {

// Test observer for process events.
class TestProcessObserver : public ProcessManagerObserver {
 public:
  void OnProcessAdded(base::ProcessId pid, ProcessType type) override {
    added_pids.push_back(pid);
  }
  void OnProcessRemoved(base::ProcessId pid, ProcessType type) override {
    removed_pids.push_back(pid);
  }
  void OnProcessSnapshotsUpdated(
      const std::vector<ProcessSnapshot>& snapshots) override {
    last_snapshots = snapshots;
    update_count++;
  }

  std::vector<base::ProcessId> added_pids;
  std::vector<base::ProcessId> removed_pids;
  std::vector<ProcessSnapshot> last_snapshots;
  int update_count = 0;
};

// Mock process manager that returns fake metrics.
class MockProcessManager : public VigoProcessManager {
 public:
  ProcessSnapshot SampleProcess(base::ProcessId pid,
                                ProcessType type) const override {
    ProcessSnapshot snap;
    snap.pid = pid;
    snap.type = type;
    snap.working_set_bytes = 50 * 1024 * 1024;  // 50 MB
    snap.private_bytes = 40 * 1024 * 1024;       // 40 MB
    snap.cpu_usage = 5.0;
    snap.sample_time = base::TimeTicks::Now();
    return snap;
  }
};

class VigoProcessManagerTest : public testing::Test {
 protected:
  base::test::TaskEnvironment task_environment_{
      base::test::TaskEnvironment::TimeSource::MOCK_TIME};
};

TEST_F(VigoProcessManagerTest, RegisterAndUnregister) {
  VigoProcessManager manager;
  TestProcessObserver observer;
  manager.AddObserver(&observer);

  manager.RegisterProcess(1234, ProcessType::kRenderer);
  EXPECT_EQ(observer.added_pids.size(), 1u);
  EXPECT_EQ(observer.added_pids[0], 1234);

  manager.UnregisterProcess(1234);
  EXPECT_EQ(observer.removed_pids.size(), 1u);
  EXPECT_EQ(observer.removed_pids[0], 1234);

  manager.RemoveObserver(&observer);
}

TEST_F(VigoProcessManagerTest, DuplicateRegisterIgnored) {
  VigoProcessManager manager;
  manager.RegisterProcess(100, ProcessType::kGpu);
  manager.RegisterProcess(100, ProcessType::kGpu);
  EXPECT_EQ(manager.GetProcessCount(), 1);
}

TEST_F(VigoProcessManagerTest, UnregisterNonExistentIgnored) {
  VigoProcessManager manager;
  // Should not crash.
  manager.UnregisterProcess(9999);
}

TEST_F(VigoProcessManagerTest, TabAssociation) {
  VigoProcessManager manager;
  manager.RegisterProcess(100, ProcessType::kRenderer);
  manager.AssociateTabWithProcess(100, 1);
  manager.AssociateTabWithProcess(100, 2);

  const auto* snap = manager.GetSnapshot(100);
  // Snapshot won't have tabs until sampled, but internally tracked.
  ASSERT_NE(snap, nullptr);
}

TEST_F(VigoProcessManagerTest, TabDissociation) {
  VigoProcessManager manager;
  manager.RegisterProcess(100, ProcessType::kRenderer);
  manager.AssociateTabWithProcess(100, 1);
  manager.DissociateTabFromProcess(100, 1);
  // Should not crash; association removed.
  EXPECT_EQ(manager.GetProcessCount(), 1);
}

TEST_F(VigoProcessManagerTest, GetSnapshotsByType) {
  MockProcessManager manager;
  manager.RegisterProcess(100, ProcessType::kRenderer);
  manager.RegisterProcess(101, ProcessType::kRenderer);
  manager.RegisterProcess(200, ProcessType::kGpu);

  manager.SampleNow();

  auto renderers = manager.GetSnapshotsByType(ProcessType::kRenderer);
  EXPECT_EQ(renderers.size(), 2u);

  auto gpu = manager.GetSnapshotsByType(ProcessType::kGpu);
  EXPECT_EQ(gpu.size(), 1u);
}

TEST_F(VigoProcessManagerTest, TotalRss) {
  MockProcessManager manager;
  manager.RegisterProcess(100, ProcessType::kRenderer);
  manager.RegisterProcess(101, ProcessType::kRenderer);
  manager.SampleNow();

  // Each mock process has 50 MB working set.
  EXPECT_EQ(manager.GetTotalRss(), 100u * 1024 * 1024);
}

TEST_F(VigoProcessManagerTest, RendererCount) {
  VigoProcessManager manager;
  manager.RegisterProcess(100, ProcessType::kRenderer);
  manager.RegisterProcess(101, ProcessType::kRenderer);
  manager.RegisterProcess(200, ProcessType::kGpu);

  EXPECT_EQ(manager.GetRendererCount(), 2);
  EXPECT_EQ(manager.GetProcessCount(), 3);
}

TEST_F(VigoProcessManagerTest, StartAndStop) {
  VigoProcessManager manager;
  manager.Start();
  manager.Stop();
  // Should not crash.
}

TEST_F(VigoProcessManagerTest, ObserverSnapshotUpdates) {
  MockProcessManager manager;
  TestProcessObserver observer;
  manager.AddObserver(&observer);

  manager.RegisterProcess(100, ProcessType::kRenderer);
  manager.SampleNow();

  EXPECT_EQ(observer.update_count, 1);
  ASSERT_EQ(observer.last_snapshots.size(), 1u);
  EXPECT_EQ(observer.last_snapshots[0].pid, 100);
  EXPECT_EQ(observer.last_snapshots[0].working_set_bytes, 50u * 1024 * 1024);

  manager.RemoveObserver(&observer);
}

}  // namespace
}  // namespace performance
}  // namespace vigo

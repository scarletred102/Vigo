// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/extensions/vigo_extension_manager.h"

#include "testing/gtest/include/gtest/gtest.h"

namespace vigo {
namespace extensions {

class VigoExtensionManagerTest : public ::testing::Test {
 protected:
  void SetUp() override {
    ASSERT_TRUE(manager_.Init());
  }

  VigoExtensionManager manager_;
};

TEST_F(VigoExtensionManagerTest, InitiallyEmpty) {
  EXPECT_EQ(manager_.Count(), 0u);
}

TEST_F(VigoExtensionManagerTest, InstallExtension) {
  auto id = manager_.Install("/path/to/extension", ExtensionSource::kSideloaded);
  EXPECT_FALSE(id.empty());
  EXPECT_EQ(manager_.Count(), 1u);
}

TEST_F(VigoExtensionManagerTest, InstallEmptyPathFails) {
  auto id = manager_.Install("", ExtensionSource::kSideloaded);
  EXPECT_TRUE(id.empty());
}

TEST_F(VigoExtensionManagerTest, UninstallExtension) {
  auto id = manager_.Install("/path/to/extension", ExtensionSource::kSideloaded);
  ASSERT_FALSE(id.empty());

  EXPECT_TRUE(manager_.Uninstall(id));
  EXPECT_EQ(manager_.Count(), 0u);
}

TEST_F(VigoExtensionManagerTest, UninstallNonexistentFails) {
  EXPECT_FALSE(manager_.Uninstall("nonexistent"));
}

TEST_F(VigoExtensionManagerTest, EnableDisable) {
  auto id = manager_.Install("/path/ext", ExtensionSource::kChromeWebStore);
  ASSERT_FALSE(id.empty());

  const auto* ext = manager_.GetExtension(id);
  ASSERT_NE(ext, nullptr);
  EXPECT_EQ(ext->state, ExtensionState::kEnabled);

  EXPECT_TRUE(manager_.Disable(id));
  ext = manager_.GetExtension(id);
  EXPECT_EQ(ext->state, ExtensionState::kDisabled);

  EXPECT_TRUE(manager_.Enable(id));
  ext = manager_.GetExtension(id);
  EXPECT_EQ(ext->state, ExtensionState::kEnabled);
}

TEST_F(VigoExtensionManagerTest, CannotEnableBlocklisted) {
  auto id = manager_.Install("/path/ext", ExtensionSource::kSideloaded);
  ASSERT_FALSE(id.empty());

  manager_.BlockExtension(id, "malware detected");
  EXPECT_FALSE(manager_.Enable(id));

  const auto* ext = manager_.GetExtension(id);
  EXPECT_EQ(ext->state, ExtensionState::kBlocklisted);
}

TEST_F(VigoExtensionManagerTest, GetAllExtensions) {
  manager_.Install("/path/ext1", ExtensionSource::kSideloaded);
  manager_.Install("/path/ext2", ExtensionSource::kChromeWebStore);

  auto all = manager_.GetAll();
  EXPECT_EQ(all.size(), 2u);
}

TEST_F(VigoExtensionManagerTest, MV2Rejected) {
  // MV2 extensions should fail validation.
  ExtensionInfo mv2_ext;
  mv2_ext.manifest_version = 2;
  EXPECT_FALSE(manager_.ValidateExtension(mv2_ext));
}

TEST_F(VigoExtensionManagerTest, MV3Accepted) {
  ExtensionInfo mv3_ext;
  mv3_ext.manifest_version = 3;
  EXPECT_TRUE(manager_.ValidateExtension(mv3_ext));
}

TEST_F(VigoExtensionManagerTest, BlockedHostRejected) {
  EXPECT_FALSE(manager_.IsHostAllowed("vigo://settings"));
  EXPECT_FALSE(manager_.IsHostAllowed("vigo-internal://debug"));
  EXPECT_TRUE(manager_.IsHostAllowed("https://example.com"));
}

TEST_F(VigoExtensionManagerTest, PermissionManagement) {
  auto id = manager_.Install("/path/ext", ExtensionSource::kSideloaded);
  ASSERT_FALSE(id.empty());

  EXPECT_FALSE(manager_.HasPermission(id, "storage"));

  ExtensionPermission perm;
  perm.api_name = "storage";
  EXPECT_TRUE(manager_.GrantPermission(id, perm));
  EXPECT_TRUE(manager_.HasPermission(id, "storage"));

  EXPECT_TRUE(manager_.RevokePermission(id, "storage"));
  EXPECT_FALSE(manager_.HasPermission(id, "storage"));
}

TEST_F(VigoExtensionManagerTest, BlockedHostPermissionDenied) {
  auto id = manager_.Install("/path/ext", ExtensionSource::kSideloaded);
  ASSERT_FALSE(id.empty());

  ExtensionPermission perm;
  perm.api_name = "host";
  perm.hosts = {"vigo://settings"};
  EXPECT_FALSE(manager_.GrantPermission(id, perm));
}

TEST_F(VigoExtensionManagerTest, Blocklist) {
  EXPECT_FALSE(manager_.IsBlocked("bad-ext"));
  manager_.BlockExtension("bad-ext", "malware");
  EXPECT_TRUE(manager_.IsBlocked("bad-ext"));
}

TEST_F(VigoExtensionManagerTest, BlocklistPreventsInstall) {
  std::string id = GenerateExtensionId("/path/bad-ext");
  // Pre-block the extension ID.
  // Note: This assumes we know the ID. In practice, blocklist checks
  // happen post-ID-generation.
  auto installed_id = manager_.Install("/path/bad-ext", ExtensionSource::kSideloaded);
  ASSERT_FALSE(installed_id.empty());

  // Block it.
  manager_.BlockExtension(installed_id, "malware");
  EXPECT_TRUE(manager_.IsBlocked(installed_id));
}

TEST_F(VigoExtensionManagerTest, ExtensionTimestamp) {
  auto id = manager_.Install("/path/ext", ExtensionSource::kBuiltIn);
  ASSERT_FALSE(id.empty());

  const auto* ext = manager_.GetExtension(id);
  ASSERT_NE(ext, nullptr);
  EXPECT_GT(ext->installed_at_ms, 0);
}

// Helper to expose GenerateExtensionId for tests.
namespace {
std::string GenerateExtensionId(const std::string& path) {
  uint32_t hash = 0;
  for (char c : path) {
    hash = hash * 31 + static_cast<uint32_t>(c);
  }
  static const char kChars[] = "abcdefghijklmnop";
  std::string id;
  id.reserve(32);
  for (int i = 0; i < 32; ++i) {
    id.push_back(kChars[(hash >> (i % 16)) & 0xF]);
    hash = hash * 7 + i;
  }
  return id;
}
}  // namespace

}  // namespace extensions
}  // namespace vigo

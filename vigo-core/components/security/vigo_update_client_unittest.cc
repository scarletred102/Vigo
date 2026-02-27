// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/security/vigo_update_client.h"

#include "base/files/file_util.h"
#include "base/files/scoped_temp_dir.h"
#include "testing/gtest/include/gtest/gtest.h"

namespace vigo {
namespace security {
namespace {

class VigoUpdateClientTest : public testing::Test {
 protected:
  void SetUp() override { ASSERT_TRUE(temp_dir_.CreateUniqueTempDir()); }

  base::ScopedTempDir temp_dir_;
  VigoUpdateClient client_;
};

// ── Platform Detection ──────────────────────────────────────────

TEST_F(VigoUpdateClientTest, GetCurrentPlatform_NotEmpty) {
  std::string platform = VigoUpdateClient::GetCurrentPlatform();
  EXPECT_FALSE(platform.empty());
  // Should contain a known OS prefix.
  EXPECT_TRUE(platform.find("win") != std::string::npos ||
              platform.find("mac") != std::string::npos ||
              platform.find("linux") != std::string::npos ||
              platform == "unknown");
}

// ── Manifest Parsing ────────────────────────────────────────────

TEST_F(VigoUpdateClientTest, ParseManifest_ValidJson) {
  const char json[] = R"({
    "releases": [
      {
        "version": "0.2.0",
        "platform": "win-x64",
        "download_url": "https://updates.vigo-browser.com/0.2.0/vigo-0.2.0-win-x64.exe",
        "sha256": "abcdef1234567890",
        "size_bytes": 85000000,
        "signature": "deadbeef",
        "release_notes": "First beta release",
        "is_critical": false,
        "is_delta": false
      }
    ]
  })";

  auto manifest = VigoUpdateClient::ParseManifest(json, "win-x64");
  ASSERT_TRUE(manifest.has_value());
  EXPECT_EQ("0.2.0", manifest->version);
  EXPECT_EQ("win-x64", manifest->platform);
  EXPECT_EQ("abcdef1234567890", manifest->sha256);
  EXPECT_EQ(85000000, manifest->size_bytes);
  EXPECT_FALSE(manifest->is_critical);
  EXPECT_FALSE(manifest->is_delta);
  EXPECT_EQ("First beta release", manifest->release_notes);
}

TEST_F(VigoUpdateClientTest, ParseManifest_PlatformMismatch) {
  const char json[] = R"({
    "releases": [
      {
        "version": "0.2.0",
        "platform": "mac-arm64",
        "download_url": "https://example.com/update.dmg",
        "sha256": "abc"
      }
    ]
  })";

  // Looking for win-x64, but only mac-arm64 exists.
  auto manifest = VigoUpdateClient::ParseManifest(json, "win-x64");
  EXPECT_FALSE(manifest.has_value());
}

TEST_F(VigoUpdateClientTest, ParseManifest_InvalidJson) {
  auto manifest = VigoUpdateClient::ParseManifest("not json", "win-x64");
  EXPECT_FALSE(manifest.has_value());
}

TEST_F(VigoUpdateClientTest, ParseManifest_MissingReleases) {
  auto manifest = VigoUpdateClient::ParseManifest("{}", "win-x64");
  EXPECT_FALSE(manifest.has_value());
}

TEST_F(VigoUpdateClientTest, ParseManifest_CriticalUpdate) {
  const char json[] = R"({
    "releases": [
      {
        "version": "0.1.1",
        "platform": "win-x64",
        "download_url": "https://example.com/patch.exe",
        "sha256": "sec001",
        "is_critical": true,
        "is_delta": true,
        "min_version": "0.1.0"
      }
    ]
  })";

  auto manifest = VigoUpdateClient::ParseManifest(json, "win-x64");
  ASSERT_TRUE(manifest.has_value());
  EXPECT_TRUE(manifest->is_critical);
  EXPECT_TRUE(manifest->is_delta);
  EXPECT_EQ("0.1.0", manifest->min_version);
}

// ── File Hash Verification ──────────────────────────────────────

TEST_F(VigoUpdateClientTest, VerifyFileHash_CorrectHash) {
  base::FilePath test_file = temp_dir_.GetPath().AppendASCII("test.bin");
  const std::string content = "Vigo Browser Update Package";
  ASSERT_TRUE(base::WriteFile(test_file, content));

  // Pre-compute SHA-256 of "Vigo Browser Update Package".
  std::string hash = crypto::SHA256HashString(content);
  std::string hash_hex = base::HexEncode(hash);

  EXPECT_TRUE(VigoUpdateClient::VerifyFileHash(test_file, hash_hex));
}

TEST_F(VigoUpdateClientTest, VerifyFileHash_WrongHash) {
  base::FilePath test_file = temp_dir_.GetPath().AppendASCII("test.bin");
  ASSERT_TRUE(base::WriteFile(test_file, "some content"));

  EXPECT_FALSE(
      VigoUpdateClient::VerifyFileHash(test_file, "0000000000000000"));
}

TEST_F(VigoUpdateClientTest, VerifyFileHash_MissingFile) {
  base::FilePath missing = temp_dir_.GetPath().AppendASCII("nonexistent.bin");
  EXPECT_FALSE(VigoUpdateClient::VerifyFileHash(missing, "abc"));
}

// ── Manifest Signature Verification ─────────────────────────────

TEST_F(VigoUpdateClientTest, VerifyManifestSignature_ValidStructure) {
  // 32-byte key (64 hex chars) and 64-byte signature (128 hex chars).
  std::string key_hex(64, 'a');
  std::string sig_hex(128, 'b');

  // Structural validation should pass with well-formed hex.
  EXPECT_TRUE(VigoUpdateClient::VerifyManifestSignature(
      "{\"test\": true}", sig_hex, key_hex));
}

TEST_F(VigoUpdateClientTest, VerifyManifestSignature_InvalidKeyLength) {
  std::string bad_key = "abcd";  // Too short.
  std::string sig_hex(128, 'b');

  EXPECT_FALSE(VigoUpdateClient::VerifyManifestSignature(
      "{}", sig_hex, bad_key));
}

TEST_F(VigoUpdateClientTest, VerifyManifestSignature_InvalidSigLength) {
  std::string key_hex(64, 'a');
  std::string bad_sig = "abcd";  // Too short.

  EXPECT_FALSE(VigoUpdateClient::VerifyManifestSignature(
      "{}", bad_sig, key_hex));
}

// ── Client Lifecycle ────────────────────────────────────────────

TEST_F(VigoUpdateClientTest, DefaultStatus_UpToDate) {
  EXPECT_EQ(UpdateStatus::kUpToDate, client_.status());
}

TEST_F(VigoUpdateClientTest, Initialise_SetsConfig) {
  VigoUpdateClient::Config config;
  config.update_server_url = GURL("https://updates.vigo-browser.com");
  config.check_interval = base::Hours(6);

  client_.Initialise(std::move(config));
  // Should not crash or change status.
  EXPECT_EQ(UpdateStatus::kUpToDate, client_.status());
}

TEST_F(VigoUpdateClientTest, Shutdown_DoesNotCrash) {
  client_.Shutdown();
  // Double shutdown should be safe.
  client_.Shutdown();
}

}  // namespace
}  // namespace security
}  // namespace vigo

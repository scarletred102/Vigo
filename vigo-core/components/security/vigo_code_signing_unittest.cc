// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/security/vigo_code_signing.h"

#include "base/files/file_util.h"
#include "base/files/scoped_temp_dir.h"
#include "testing/gtest/include/gtest/gtest.h"

namespace vigo {
namespace security {
namespace {

class VigoCodeSigningTest : public testing::Test {
 protected:
  void SetUp() override { ASSERT_TRUE(temp_dir_.CreateUniqueTempDir()); }

  base::ScopedTempDir temp_dir_;
  VigoCodeSigning signing_;
};

// ── Expected Signer ─────────────────────────────────────────────

TEST_F(VigoCodeSigningTest, ExpectedSignerName) {
  EXPECT_EQ("Vigo Browser", VigoCodeSigning::GetExpectedSignerName());
}

// ── Hash File ───────────────────────────────────────────────────

TEST_F(VigoCodeSigningTest, HashFile_ProducesHash) {
  base::FilePath test_file = temp_dir_.GetPath().AppendASCII("test.bin");
  ASSERT_TRUE(base::WriteFile(test_file, "hello world"));

  std::string hash = VigoCodeSigning::HashFile(test_file);
  EXPECT_FALSE(hash.empty());
  // SHA-256 hex = 64 chars.
  EXPECT_EQ(64u, hash.size());
}

TEST_F(VigoCodeSigningTest, HashFile_DeterministicForSameContent) {
  base::FilePath file1 = temp_dir_.GetPath().AppendASCII("a.bin");
  base::FilePath file2 = temp_dir_.GetPath().AppendASCII("b.bin");
  ASSERT_TRUE(base::WriteFile(file1, "identical content"));
  ASSERT_TRUE(base::WriteFile(file2, "identical content"));

  EXPECT_EQ(VigoCodeSigning::HashFile(file1),
            VigoCodeSigning::HashFile(file2));
}

TEST_F(VigoCodeSigningTest, HashFile_DifferentForDifferentContent) {
  base::FilePath file1 = temp_dir_.GetPath().AppendASCII("a.bin");
  base::FilePath file2 = temp_dir_.GetPath().AppendASCII("b.bin");
  ASSERT_TRUE(base::WriteFile(file1, "content A"));
  ASSERT_TRUE(base::WriteFile(file2, "content B"));

  EXPECT_NE(VigoCodeSigning::HashFile(file1),
            VigoCodeSigning::HashFile(file2));
}

TEST_F(VigoCodeSigningTest, HashFile_MissingFile) {
  base::FilePath missing = temp_dir_.GetPath().AppendASCII("nonexistent.bin");
  EXPECT_TRUE(VigoCodeSigning::HashFile(missing).empty());
}

// ── Hash Manifest Generation ────────────────────────────────────

TEST_F(VigoCodeSigningTest, GenerateHashManifest_EmptyDir) {
  auto manifest = signing_.GenerateHashManifest(temp_dir_.GetPath());
  EXPECT_TRUE(manifest.empty());
}

TEST_F(VigoCodeSigningTest, GenerateHashManifest_WithBinaries) {
  // Create some binary files.
  ASSERT_TRUE(
      base::WriteFile(temp_dir_.GetPath().AppendASCII("vigo.exe"), "binary1"));
  ASSERT_TRUE(
      base::WriteFile(temp_dir_.GetPath().AppendASCII("vigo.dll"), "binary2"));
  // Non-binary file should be excluded.
  ASSERT_TRUE(
      base::WriteFile(temp_dir_.GetPath().AppendASCII("readme.txt"), "text"));

  auto manifest = signing_.GenerateHashManifest(temp_dir_.GetPath());
  EXPECT_EQ(2u, manifest.size());

  for (const auto& entry : manifest) {
    EXPECT_FALSE(entry.relative_path.empty());
    EXPECT_FALSE(entry.sha256.empty());
    EXPECT_GT(entry.size_bytes, 0);
  }
}

// ── Hash Manifest Verification ──────────────────────────────────

TEST_F(VigoCodeSigningTest, VerifyHashManifest_AllCorrect) {
  ASSERT_TRUE(
      base::WriteFile(temp_dir_.GetPath().AppendASCII("vigo.exe"), "binary"));

  auto manifest = signing_.GenerateHashManifest(temp_dir_.GetPath());
  auto failures = signing_.VerifyHashManifest(temp_dir_.GetPath(), manifest);

  EXPECT_TRUE(failures.empty());
}

TEST_F(VigoCodeSigningTest, VerifyHashManifest_TamperedFile) {
  base::FilePath exe = temp_dir_.GetPath().AppendASCII("vigo.exe");
  ASSERT_TRUE(base::WriteFile(exe, "original binary"));

  auto manifest = signing_.GenerateHashManifest(temp_dir_.GetPath());

  // Tamper with the file.
  ASSERT_TRUE(base::WriteFile(exe, "tampered binary!"));

  auto failures = signing_.VerifyHashManifest(temp_dir_.GetPath(), manifest);
  EXPECT_FALSE(failures.empty());
  EXPECT_NE(std::string::npos, failures[0].find("hash mismatch"));
}

TEST_F(VigoCodeSigningTest, VerifyHashManifest_MissingFile) {
  BinaryHashEntry entry;
  entry.relative_path = "deleted.exe";
  entry.sha256 = "abc123";
  entry.size_bytes = 100;

  std::vector<BinaryHashEntry> manifest = {entry};
  auto failures = signing_.VerifyHashManifest(temp_dir_.GetPath(), manifest);

  EXPECT_FALSE(failures.empty());
  EXPECT_NE(std::string::npos, failures[0].find("file missing"));
}

// ── Hash Manifest Serialisation ─────────────────────────────────

TEST_F(VigoCodeSigningTest, WriteAndReadManifest_Roundtrip) {
  ASSERT_TRUE(
      base::WriteFile(temp_dir_.GetPath().AppendASCII("vigo.exe"), "binary"));

  auto original = signing_.GenerateHashManifest(temp_dir_.GetPath());

  base::FilePath manifest_path =
      temp_dir_.GetPath().AppendASCII("manifest.json");
  ASSERT_TRUE(signing_.WriteHashManifest(manifest_path, original));

  auto loaded = signing_.ReadHashManifest(manifest_path);
  ASSERT_EQ(original.size(), loaded.size());

  for (size_t i = 0; i < original.size(); ++i) {
    EXPECT_EQ(original[i].relative_path, loaded[i].relative_path);
    EXPECT_EQ(original[i].sha256, loaded[i].sha256);
    EXPECT_EQ(original[i].size_bytes, loaded[i].size_bytes);
  }
}

// ── Signing Config Validation ───────────────────────────────────

TEST_F(VigoCodeSigningTest, ValidateConfig_EmptyIdentity) {
  EXPECT_FALSE(signing_.ValidateConfig());
}

TEST_F(VigoCodeSigningTest, ValidateConfig_WithIdentity) {
  SigningConfig config;
  config.identity = "Vigo Browser Signing Certificate";
  config.timestamp_url = "http://timestamp.digicert.com";
  config.hash_algorithm = "sha256";
  signing_.SetConfig(std::move(config));

  EXPECT_TRUE(signing_.ValidateConfig());
}

// ── Binary Verification ─────────────────────────────────────────

TEST_F(VigoCodeSigningTest, VerifyBinary_MissingFile) {
  base::FilePath missing = temp_dir_.GetPath().AppendASCII("nonexistent.exe");
  SigningInfo info = signing_.VerifyBinary(missing);

  EXPECT_EQ(SigningStatus::kError, info.status);
}

TEST_F(VigoCodeSigningTest, VerifyBinary_UnsignedFile) {
  base::FilePath exe = temp_dir_.GetPath().AppendASCII("vigo.exe");
  ASSERT_TRUE(base::WriteFile(exe, "unsigned binary"));

  SigningInfo info = signing_.VerifyBinary(exe);
  // Without real signing infrastructure, unsigned is expected.
  EXPECT_EQ(SigningStatus::kUnsigned, info.status);
}

}  // namespace
}  // namespace security
}  // namespace vigo

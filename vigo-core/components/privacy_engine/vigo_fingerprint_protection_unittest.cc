// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/privacy_engine/vigo_fingerprint_protection.h"

#include <cstring>
#include <set>
#include <string>

#include "testing/gtest/include/gtest/gtest.h"

namespace vigo {
namespace privacy {
namespace {

// ── Canvas noise ────────────────────────────────────────────────

TEST(VigoFingerprintProtectionTest, CanvasNoiseDeterministic) {
  VigoFingerprintProtection fp;
  uint8_t noise1 = fp.GetCanvasNoise(10, 20, 0);
  uint8_t noise2 = fp.GetCanvasNoise(10, 20, 0);
  // Same seed + position → same noise.
  EXPECT_EQ(noise1, noise2);
}

TEST(VigoFingerprintProtectionTest, CanvasNoiseVariesByPosition) {
  VigoFingerprintProtection fp;
  // Collect noise values for different positions; expect variation.
  std::set<uint8_t> values;
  for (uint32_t x = 0; x < 100; ++x) {
    values.insert(fp.GetCanvasNoise(x, 0, 0));
  }
  // With 100 samples and range [0,4], should hit multiple values.
  EXPECT_GT(values.size(), 1u);
}

TEST(VigoFingerprintProtectionTest, CanvasNoiseRange) {
  VigoFingerprintProtection fp;
  for (uint32_t x = 0; x < 200; ++x) {
    for (uint8_t ch = 0; ch < 4; ++ch) {
      uint8_t noise = fp.GetCanvasNoise(x, x, ch);
      EXPECT_LE(noise, 4u);  // Range [0, 4].
    }
  }
}

TEST(VigoFingerprintProtectionTest, CanvasNoiseDifferentSessions) {
  VigoFingerprintProtection fp1;
  VigoFingerprintProtection fp2;
  // Different instances → different seeds → (very likely) different noise.
  // There's a 1/5 chance of collision for any single call, so check
  // multiple positions and expect at least one difference.
  bool found_difference = false;
  for (uint32_t x = 0; x < 50 && !found_difference; ++x) {
    if (fp1.GetCanvasNoise(x, 0, 0) != fp2.GetCanvasNoise(x, 0, 0))
      found_difference = true;
  }
  EXPECT_TRUE(found_difference);
}

// ── WebGL masking ───────────────────────────────────────────────

TEST(VigoFingerprintProtectionTest, WebGLVendorMasked) {
  const char* vendor = VigoFingerprintProtection::GetMaskedWebGLVendor();
  ASSERT_NE(vendor, nullptr);
  EXPECT_NE(std::string(vendor).find("Vigo"), std::string::npos);
  // Must NOT contain real GPU vendor names.
  EXPECT_EQ(std::string(vendor).find("NVIDIA"), std::string::npos);
  EXPECT_EQ(std::string(vendor).find("AMD"), std::string::npos);
  EXPECT_EQ(std::string(vendor).find("Intel"), std::string::npos);
}

TEST(VigoFingerprintProtectionTest, WebGLRendererMasked) {
  const char* renderer = VigoFingerprintProtection::GetMaskedWebGLRenderer();
  ASSERT_NE(renderer, nullptr);
  EXPECT_NE(std::string(renderer).find("Generic GPU"), std::string::npos);
  // Must NOT contain specific GPU model strings.
  EXPECT_EQ(std::string(renderer).find("GeForce"), std::string::npos);
  EXPECT_EQ(std::string(renderer).find("Radeon"), std::string::npos);
}

// ── AudioContext resistance ─────────────────────────────────────

TEST(VigoFingerprintProtectionTest, AudioNoiseDeterministic) {
  VigoFingerprintProtection fp;
  float buf1[64] = {0};
  float buf2[64] = {0};
  fp.ApplyAudioNoise(buf1, 64);
  fp.ApplyAudioNoise(buf2, 64);
  // Same seed → same noise on same input.
  for (size_t i = 0; i < 64; ++i) {
    EXPECT_FLOAT_EQ(buf1[i], buf2[i]);
  }
}

TEST(VigoFingerprintProtectionTest, AudioNoiseSmallMagnitude) {
  VigoFingerprintProtection fp;
  float buffer[256] = {0};
  fp.ApplyAudioNoise(buffer, 256);
  for (size_t i = 0; i < 256; ++i) {
    EXPECT_LT(std::abs(buffer[i]), 0.001f);
  }
}

TEST(VigoFingerprintProtectionTest, AudioNoiseNullBuffer) {
  VigoFingerprintProtection fp;
  // Should not crash.
  fp.ApplyAudioNoise(nullptr, 0);
  fp.ApplyAudioNoise(nullptr, 10);
}

// ── Font enumeration restriction ────────────────────────────────

TEST(VigoFingerprintProtectionTest, CommonFontsAllowed) {
  EXPECT_TRUE(VigoFingerprintProtection::IsFontAllowed("Arial"));
  EXPECT_TRUE(VigoFingerprintProtection::IsFontAllowed("Times New Roman"));
  EXPECT_TRUE(VigoFingerprintProtection::IsFontAllowed("Courier New"));
  EXPECT_TRUE(VigoFingerprintProtection::IsFontAllowed("sans-serif"));
}

TEST(VigoFingerprintProtectionTest, FontCheckCaseInsensitive) {
  EXPECT_TRUE(VigoFingerprintProtection::IsFontAllowed("arial"));
  EXPECT_TRUE(VigoFingerprintProtection::IsFontAllowed("ARIAL"));
  EXPECT_TRUE(VigoFingerprintProtection::IsFontAllowed("Arial"));
}

TEST(VigoFingerprintProtectionTest, ExoticFontsBlocked) {
  EXPECT_FALSE(VigoFingerprintProtection::IsFontAllowed("Papyrus"));
  EXPECT_FALSE(VigoFingerprintProtection::IsFontAllowed("Comic Sans MS"));
  EXPECT_FALSE(VigoFingerprintProtection::IsFontAllowed("Wingdings"));
  EXPECT_FALSE(
      VigoFingerprintProtection::IsFontAllowed("Some Custom Font 123"));
}

TEST(VigoFingerprintProtectionTest, AllowedFontsNotEmpty) {
  const auto& fonts = VigoFingerprintProtection::GetAllowedFonts();
  EXPECT_GT(fonts.size(), 20u);
}

// ── Client Hints reduction ──────────────────────────────────────

TEST(VigoFingerprintProtectionTest, PermittedClientHintsMinimal) {
  const auto& hints = VigoFingerprintProtection::GetPermittedClientHints();
  // Only 3 low-entropy hints should be permitted.
  EXPECT_EQ(hints.size(), 3u);
}

TEST(VigoFingerprintProtectionTest, BasicClientHintsPermitted) {
  EXPECT_TRUE(
      VigoFingerprintProtection::IsClientHintPermitted("Sec-CH-UA"));
  EXPECT_TRUE(
      VigoFingerprintProtection::IsClientHintPermitted("Sec-CH-UA-Mobile"));
  EXPECT_TRUE(VigoFingerprintProtection::IsClientHintPermitted(
      "Sec-CH-UA-Platform"));
}

TEST(VigoFingerprintProtectionTest, HighEntropyClientHintsBlocked) {
  EXPECT_FALSE(VigoFingerprintProtection::IsClientHintPermitted(
      "Sec-CH-UA-Full-Version-List"));
  EXPECT_FALSE(
      VigoFingerprintProtection::IsClientHintPermitted("Sec-CH-UA-Arch"));
  EXPECT_FALSE(VigoFingerprintProtection::IsClientHintPermitted(
      "Sec-CH-UA-Bitness"));
  EXPECT_FALSE(
      VigoFingerprintProtection::IsClientHintPermitted("Sec-CH-UA-Model"));
  EXPECT_FALSE(VigoFingerprintProtection::IsClientHintPermitted(
      "Sec-CH-UA-Platform-Version"));
  EXPECT_FALSE(
      VigoFingerprintProtection::IsClientHintPermitted("Sec-CH-Viewport-Width"));
  EXPECT_FALSE(
      VigoFingerprintProtection::IsClientHintPermitted("Sec-CH-DPR"));
  EXPECT_FALSE(
      VigoFingerprintProtection::IsClientHintPermitted("Device-Memory"));
  EXPECT_FALSE(
      VigoFingerprintProtection::IsClientHintPermitted("Downlink"));
  EXPECT_FALSE(VigoFingerprintProtection::IsClientHintPermitted("ECT"));
  EXPECT_FALSE(VigoFingerprintProtection::IsClientHintPermitted("RTT"));
}

TEST(VigoFingerprintProtectionTest, ClientHintCheckCaseInsensitive) {
  EXPECT_TRUE(
      VigoFingerprintProtection::IsClientHintPermitted("sec-ch-ua"));
  EXPECT_TRUE(
      VigoFingerprintProtection::IsClientHintPermitted("SEC-CH-UA"));
}

// ── Session seed ────────────────────────────────────────────────

TEST(VigoFingerprintProtectionTest, SessionSeedNonZero) {
  VigoFingerprintProtection fp;
  // Extremely unlikely to be zero from RandUint64.
  EXPECT_NE(fp.session_seed(), 0u);
}

TEST(VigoFingerprintProtectionTest, RegenerateSessionSeedChanges) {
  VigoFingerprintProtection fp;
  uint64_t seed1 = fp.session_seed();
  fp.GenerateSessionSeed();
  uint64_t seed2 = fp.session_seed();
  // Extremely unlikely to generate the same seed twice.
  EXPECT_NE(seed1, seed2);
}

}  // namespace
}  // namespace privacy
}  // namespace vigo

// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/media_orchestration/vigo_media_orchestration_layer.h"

#include "testing/gtest/include/gtest/gtest.h"

namespace vigo {
namespace media {

class VigoMediaOrchestrationLayerTest : public ::testing::Test {
 protected:
  void SetUp() override { mol_.ProbeCodecCapabilities(); }

  VigoMediaOrchestrationLayer mol_;
};

TEST_F(VigoMediaOrchestrationLayerTest, DefaultAbrModeIsBalanced) {
  EXPECT_EQ(mol_.GetAbrMode(), AbrMode::kBalanced);
}

TEST_F(VigoMediaOrchestrationLayerTest, SetAbrMode) {
  mol_.SetAbrMode(AbrMode::kMaxQuality);
  EXPECT_EQ(mol_.GetAbrMode(), AbrMode::kMaxQuality);

  mol_.SetAbrMode(AbrMode::kDataSaver);
  EXPECT_EQ(mol_.GetAbrMode(), AbrMode::kDataSaver);
}

TEST_F(VigoMediaOrchestrationLayerTest, ProbePopulatesCapabilities) {
  EXPECT_FALSE(mol_.GetCapabilities().empty());

  bool found_h264 = false;
  for (const auto& cap : mol_.GetCapabilities()) {
    if (cap.codec_name == "H.264") {
      found_h264 = true;
      EXPECT_TRUE(cap.sw_decode_available);
    }
  }
  EXPECT_TRUE(found_h264);
}

TEST_F(VigoMediaOrchestrationLayerTest, RecommendBitrateBaseline) {
  // 10 Mbps throughput, healthy buffer, no drops, low CPU.
  int64_t bitrate = mol_.RecommendBitrate(
      /*throughput_bps=*/10000000,
      /*buffer_depth_s=*/25.0,
      /*frame_drop_rate=*/0.0,
      /*cpu_usage=*/0.3);

  // Balanced mode uses ~80% of throughput.
  EXPECT_GE(bitrate, 7000000);
  EXPECT_LE(bitrate, 10000000);
}

TEST_F(VigoMediaOrchestrationLayerTest, RecommendBitrateLowBuffer) {
  // Low buffer should significantly reduce bitrate.
  int64_t normal = mol_.RecommendBitrate(10000000, 25.0, 0.0, 0.3);
  int64_t low_buffer = mol_.RecommendBitrate(10000000, 5.0, 0.0, 0.3);
  EXPECT_LT(low_buffer, normal);
}

TEST_F(VigoMediaOrchestrationLayerTest, RecommendBitrateHighCpu) {
  // High CPU should reduce bitrate.
  int64_t normal = mol_.RecommendBitrate(10000000, 25.0, 0.0, 0.3);
  int64_t high_cpu = mol_.RecommendBitrate(10000000, 25.0, 0.0, 0.9);
  EXPECT_LT(high_cpu, normal);
}

TEST_F(VigoMediaOrchestrationLayerTest, RecommendBitrateFloor) {
  // Even with terrible conditions, never below 500 kbps.
  int64_t bitrate = mol_.RecommendBitrate(100000, 2.0, 0.1, 0.95);
  EXPECT_GE(bitrate, 500000);
}

TEST_F(VigoMediaOrchestrationLayerTest, QualityOscillationLimit) {
  // First 3 switches allowed.
  EXPECT_TRUE(mol_.RecordQualitySwitch(0.0));
  EXPECT_TRUE(mol_.RecordQualitySwitch(10.0));
  EXPECT_TRUE(mol_.RecordQualitySwitch(20.0));

  // 4th switch within 10-minute window should be rejected.
  EXPECT_FALSE(mol_.RecordQualitySwitch(30.0));
}

TEST_F(VigoMediaOrchestrationLayerTest, QualityOscillationWindowReset) {
  EXPECT_TRUE(mol_.RecordQualitySwitch(0.0));
  EXPECT_TRUE(mol_.RecordQualitySwitch(10.0));
  EXPECT_TRUE(mol_.RecordQualitySwitch(20.0));
  EXPECT_FALSE(mol_.RecordQualitySwitch(30.0));

  // After 10-minute window expires, counter resets.
  EXPECT_TRUE(mol_.RecordQualitySwitch(700.0));
}

TEST_F(VigoMediaOrchestrationLayerTest, NegotiateCodecPrefersSWFallback) {
  // Without HW decode, should return first available.
  std::vector<std::string> codecs = {"VP9", "H.264", "AV1"};
  std::string result = mol_.NegotiateCodec(codecs);
  // Default probe has no HW decode, so first codec selected.
  EXPECT_EQ(result, "VP9");
}

TEST_F(VigoMediaOrchestrationLayerTest, NegotiateCodecEmpty) {
  std::vector<std::string> empty;
  EXPECT_EQ(mol_.NegotiateCodec(empty), "");
}

TEST_F(VigoMediaOrchestrationLayerTest, DrmLevelDetectsStub) {
  // Stub implementation returns L3.
  EXPECT_EQ(mol_.DetectDrmLevel(), DrmLevel::kL3);
}

}  // namespace media
}  // namespace vigo

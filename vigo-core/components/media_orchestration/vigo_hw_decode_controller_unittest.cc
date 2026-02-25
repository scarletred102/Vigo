// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/media_orchestration/vigo_hw_decode_controller.h"

#include "testing/gtest/include/gtest/gtest.h"

namespace vigo {
namespace media {
namespace {

class VigoHwDecodeControllerTest : public testing::Test {
 protected:
  VigoHwDecodeController controller_;
};

TEST_F(VigoHwDecodeControllerTest, InitiallyNotProbed) {
  EXPECT_FALSE(controller_.IsProbed());
  EXPECT_TRUE(controller_.GetCapabilities().empty());
}

TEST_F(VigoHwDecodeControllerTest, ProbePopulatesCapabilities) {
  controller_.ProbeCapabilities();
  EXPECT_TRUE(controller_.IsProbed());
  EXPECT_FALSE(controller_.GetCapabilities().empty());
}

TEST_F(VigoHwDecodeControllerTest, AllCoreCodecsPresent) {
  controller_.ProbeCapabilities();
  const auto& caps = controller_.GetCapabilities();

  bool has_h264 = false, has_hevc = false, has_vp9 = false, has_av1 = false;
  for (const auto& cap : caps) {
    if (cap.codec == "H.264") has_h264 = true;
    if (cap.codec == "HEVC") has_hevc = true;
    if (cap.codec == "VP9") has_vp9 = true;
    if (cap.codec == "AV1") has_av1 = true;
  }

  EXPECT_TRUE(has_h264);
  EXPECT_TRUE(has_hevc);
  EXPECT_TRUE(has_vp9);
  EXPECT_TRUE(has_av1);
}

TEST_F(VigoHwDecodeControllerTest, H264AlwaysSupported) {
  controller_.ProbeCapabilities();
  // H.264 must always be supported (either HW or SW).
  HwDecodeApi api = controller_.GetBestApi("H.264");
  EXPECT_NE(api, HwDecodeApi::kSoftware);  // Platform probe gives HW.
}

TEST_F(VigoHwDecodeControllerTest, GetMaxResolution) {
  controller_.ProbeCapabilities();
  int w = 0, h = 0;
  controller_.GetMaxResolution("H.264", &w, &h);
  EXPECT_GT(w, 0);
  EXPECT_GT(h, 0);
}

TEST_F(VigoHwDecodeControllerTest, UnknownCodecReturnsSoftware) {
  controller_.ProbeCapabilities();
  HwDecodeApi api = controller_.GetBestApi("NonExistentCodec");
  EXPECT_EQ(api, HwDecodeApi::kSoftware);
}

TEST_F(VigoHwDecodeControllerTest, UnknownCodecNoHwDecode) {
  controller_.ProbeCapabilities();
  EXPECT_FALSE(controller_.HasHwDecode("NonExistentCodec"));
}

TEST_F(VigoHwDecodeControllerTest, ForceApiForTesting) {
  controller_.ProbeCapabilities();
  controller_.ForceApiForTesting("TestCodec", HwDecodeApi::kD3D11VA);
  EXPECT_TRUE(controller_.HasHwDecode("TestCodec"));
  EXPECT_EQ(controller_.GetBestApi("TestCodec"), HwDecodeApi::kD3D11VA);
}

TEST_F(VigoHwDecodeControllerTest, ForceApiReplacesExisting) {
  controller_.ProbeCapabilities();
  // Force H.264 to software.
  controller_.ForceApiForTesting("H.264", HwDecodeApi::kSoftware);
  EXPECT_FALSE(controller_.HasHwDecode("H.264"));
  EXPECT_EQ(controller_.GetBestApi("H.264"), HwDecodeApi::kSoftware);
}

TEST_F(VigoHwDecodeControllerTest, HdrSupportDetected) {
  controller_.ProbeCapabilities();
  // Platform probes set HDR=true for HEVC on most platforms.
  // At minimum, HEVC should report HDR capability.
  bool hevc_hdr = controller_.SupportsHdr("HEVC");
  EXPECT_TRUE(hevc_hdr);
}

TEST_F(VigoHwDecodeControllerTest, DriverVersionInitiallyEmpty) {
  EXPECT_TRUE(controller_.GetGpuDriverVersion().empty());
}

TEST_F(VigoHwDecodeControllerTest, DoubleProbeDoesNotDuplicate) {
  controller_.ProbeCapabilities();
  size_t first_count = controller_.GetCapabilities().size();
  controller_.ProbeCapabilities();
  size_t second_count = controller_.GetCapabilities().size();
  EXPECT_EQ(first_count, second_count);
}

}  // namespace
}  // namespace media
}  // namespace vigo

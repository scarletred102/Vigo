// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/media_orchestration/vigo_media_pipeline_integration.h"

#include "testing/gtest/include/gtest/gtest.h"

namespace vigo {
namespace media {
namespace {

class VigoMediaPipelineIntegrationTest : public testing::Test {
 protected:
  void SetUp() override { pipeline_.Initialize(); }

  VigoMediaPipelineIntegration pipeline_;
};

// ─── Initialization ────────────────────────────────────────────

TEST_F(VigoMediaPipelineIntegrationTest, InitializesSuccessfully) {
  EXPECT_TRUE(pipeline_.IsInitialized());
  EXPECT_NE(pipeline_.mol(), nullptr);
  EXPECT_NE(pipeline_.hw_decode(), nullptr);
  EXPECT_NE(pipeline_.pip(), nullptr);
}

// ─── Container Detection ───────────────────────────────────────

TEST_F(VigoMediaPipelineIntegrationTest, DetectContainerMp4FromMime) {
  EXPECT_EQ(VigoMediaPipelineIntegration::DetectContainer("", "video/mp4"),
            ContainerFormat::kMP4);
}

TEST_F(VigoMediaPipelineIntegrationTest, DetectContainerWebMFromUrl) {
  EXPECT_EQ(
      VigoMediaPipelineIntegration::DetectContainer("http://x.com/v.webm", ""),
      ContainerFormat::kWebM);
}

TEST_F(VigoMediaPipelineIntegrationTest, DetectContainerMkvFromUrl) {
  EXPECT_EQ(
      VigoMediaPipelineIntegration::DetectContainer("/path/video.mkv", ""),
      ContainerFormat::kMKV);
}

TEST_F(VigoMediaPipelineIntegrationTest, DetectContainerFLACFromMime) {
  EXPECT_EQ(
      VigoMediaPipelineIntegration::DetectContainer("", "audio/flac"),
      ContainerFormat::kFLAC);
}

TEST_F(VigoMediaPipelineIntegrationTest, DetectContainerUnknown) {
  EXPECT_EQ(
      VigoMediaPipelineIntegration::DetectContainer("http://x.com/data", ""),
      ContainerFormat::kUnknown);
}

TEST_F(VigoMediaPipelineIntegrationTest, DetectContainerTS) {
  EXPECT_EQ(
      VigoMediaPipelineIntegration::DetectContainer("segment.ts", ""),
      ContainerFormat::kTS);
}

// ─── Protocol Detection ────────────────────────────────────────

TEST_F(VigoMediaPipelineIntegrationTest, DetectProtocolHLS) {
  EXPECT_EQ(
      VigoMediaPipelineIntegration::DetectProtocol("http://cdn.com/live.m3u8"),
      StreamingProtocol::kHLS);
}

TEST_F(VigoMediaPipelineIntegrationTest, DetectProtocolDASH) {
  EXPECT_EQ(
      VigoMediaPipelineIntegration::DetectProtocol("http://cdn.com/stream.mpd"),
      StreamingProtocol::kDASH);
}

TEST_F(VigoMediaPipelineIntegrationTest, DetectProtocolLocal) {
  EXPECT_EQ(
      VigoMediaPipelineIntegration::DetectProtocol("file:///C:/video.mp4"),
      StreamingProtocol::kLocal);
}

TEST_F(VigoMediaPipelineIntegrationTest, DetectProtocolBlob) {
  EXPECT_EQ(
      VigoMediaPipelineIntegration::DetectProtocol("blob:http://x/uuid"),
      StreamingProtocol::kBlob);
}

TEST_F(VigoMediaPipelineIntegrationTest, DetectProtocolProgressive) {
  EXPECT_EQ(
      VigoMediaPipelineIntegration::DetectProtocol("http://cdn.com/v.mp4"),
      StreamingProtocol::kProgressive);
}

// ─── Subtitle Detection ────────────────────────────────────────

TEST_F(VigoMediaPipelineIntegrationTest, DetectSubtitleVTT) {
  EXPECT_EQ(
      VigoMediaPipelineIntegration::DetectSubtitleFormat("captions.vtt"),
      SubtitleFormat::kWebVTT);
}

TEST_F(VigoMediaPipelineIntegrationTest, DetectSubtitleSRT) {
  EXPECT_EQ(
      VigoMediaPipelineIntegration::DetectSubtitleFormat("subs.srt"),
      SubtitleFormat::kSRT);
}

TEST_F(VigoMediaPipelineIntegrationTest, DetectSubtitleSSA) {
  EXPECT_EQ(
      VigoMediaPipelineIntegration::DetectSubtitleFormat("subs.ass"),
      SubtitleFormat::kSSA);
}

TEST_F(VigoMediaPipelineIntegrationTest, DetectSubtitleUnknown) {
  EXPECT_EQ(
      VigoMediaPipelineIntegration::DetectSubtitleFormat("data.txt"),
      SubtitleFormat::kUnknown);
}

// ─── Codec Selection ───────────────────────────────────────────

TEST_F(VigoMediaPipelineIntegrationTest, SelectOptimalCodecPrefers4K) {
  std::vector<std::string> codecs = {"H.264", "HEVC", "VP9"};
  std::string selected = pipeline_.SelectOptimalCodec(codecs, 3840, 2160);
  // Should prefer HEVC or VP9 for 4K if HW decode available.
  EXPECT_FALSE(selected.empty());
}

TEST_F(VigoMediaPipelineIntegrationTest, SelectOptimalCodecFallback) {
  std::vector<std::string> codecs = {"H.264"};
  std::string selected = pipeline_.SelectOptimalCodec(codecs, 1920, 1080);
  EXPECT_EQ(selected, "H.264");
}

TEST_F(VigoMediaPipelineIntegrationTest, SelectOptimalCodecEmpty) {
  std::vector<std::string> codecs;
  std::string selected = pipeline_.SelectOptimalCodec(codecs, 1920, 1080);
  EXPECT_TRUE(selected.empty());
}

// ─── Support Queries ───────────────────────────────────────────

TEST_F(VigoMediaPipelineIntegrationTest, IsSupportedH264) {
  EXPECT_TRUE(pipeline_.IsSupported(ContainerFormat::kMP4, "H.264"));
}

TEST_F(VigoMediaPipelineIntegrationTest, IsSupportedHEVC) {
  EXPECT_TRUE(pipeline_.IsSupported(ContainerFormat::kMP4, "HEVC"));
}

TEST_F(VigoMediaPipelineIntegrationTest, IsSupportedAV1) {
  EXPECT_TRUE(pipeline_.IsSupported(ContainerFormat::kWebM, "AV1"));
}

TEST_F(VigoMediaPipelineIntegrationTest, IsNotSupportedFakeCodec) {
  EXPECT_FALSE(pipeline_.IsSupported(ContainerFormat::kMP4, "FakeCodec"));
}

// ─── Feature Queries ───────────────────────────────────────────

TEST_F(VigoMediaPipelineIntegrationTest, HevcSupported) {
  // Platform probes should detect HEVC support.
  EXPECT_TRUE(pipeline_.IsHevcSupported());
}

TEST_F(VigoMediaPipelineIntegrationTest, JxlEnabled) {
  EXPECT_TRUE(pipeline_.IsJxlEnabled());
}

TEST_F(VigoMediaPipelineIntegrationTest, HdrModes) {
  auto modes = pipeline_.GetSupportedHdrModes();
  EXPECT_FALSE(modes.empty());
  // SDR is always included.
  EXPECT_EQ(modes[0], HdrMode::kSDR);
}

// ─── Playback Session ──────────────────────────────────────────

TEST_F(VigoMediaPipelineIntegrationTest, MediaSessionStart) {
  MediaSessionInfo info;
  info.url = "http://cdn.com/video.mp4";
  info.codec = "H.264";
  info.width = 1920;
  info.height = 1080;
  info.bitrate_bps = 5000000;

  int64_t bitrate = pipeline_.OnMediaSessionStart(info);
  EXPECT_GT(bitrate, 0);
}

TEST_F(VigoMediaPipelineIntegrationTest, PlaybackUpdate) {
  MediaSessionInfo info;
  info.bitrate_bps = 5000000;
  pipeline_.OnMediaSessionStart(info);

  int64_t recommended = pipeline_.OnPlaybackUpdate(
      10000000,  // 10 Mbps throughput
      20.0,      // 20s buffer
      0.0,       // No frame drops
      0.3);      // 30% CPU
  EXPECT_GT(recommended, 0);
}

TEST_F(VigoMediaPipelineIntegrationTest, CurrentSessionUpdated) {
  MediaSessionInfo info;
  info.codec = "HEVC";
  info.width = 3840;
  info.height = 2160;
  pipeline_.OnMediaSessionStart(info);

  const auto& session = pipeline_.GetCurrentSession();
  EXPECT_EQ(session.codec, "HEVC");
  EXPECT_EQ(session.width, 3840);
}

}  // namespace
}  // namespace media
}  // namespace vigo

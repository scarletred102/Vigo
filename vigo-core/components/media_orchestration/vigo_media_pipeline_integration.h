// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_MEDIA_ORCHESTRATION_VIGO_MEDIA_PIPELINE_INTEGRATION_H_
#define VIGO_COMPONENTS_MEDIA_ORCHESTRATION_VIGO_MEDIA_PIPELINE_INTEGRATION_H_

#include <memory>
#include <string>
#include <vector>

#include "base/sequence_checker.h"

namespace vigo {
namespace media {

class VigoHwDecodeController;
class VigoMediaOrchestrationLayer;
class VigoPipController;

// Supported media container formats.
enum class ContainerFormat {
  kMP4,        // MPEG-4 Part 14 (.mp4, .m4v, .m4a)
  kWebM,       // WebM (.webm)
  kMKV,        // Matroska (.mkv)
  kAVI,        // Audio Video Interleave (.avi)
  kOgg,        // Ogg container (.ogg, .ogv)
  kFLAC,       // FLAC container (.flac)
  kWAV,        // Waveform Audio (.wav)
  kMP3,        // MPEG Audio Layer III (.mp3)
  kAAC,        // Advanced Audio Coding (.aac, .m4a)
  kTS,         // MPEG Transport Stream (.ts, .mts)
  kFLV,        // Flash Video (.flv) — legacy support
  kUnknown,
};

// Streaming protocol types.
enum class StreamingProtocol {
  kHLS,        // HTTP Live Streaming (Apple)
  kDASH,       // Dynamic Adaptive Streaming over HTTP
  kMSS,        // Microsoft Smooth Streaming
  kProgressive,// Direct HTTP progressive download
  kLocal,      // Local file playback
  kBlob,       // Blob URL (in-memory)
  kUnknown,
};

// Subtitle format types.
enum class SubtitleFormat {
  kWebVTT,     // Web Video Text Tracks (.vtt)
  kSRT,        // SubRip (.srt)
  kSSA,        // SubStation Alpha / Advanced SSA (.ssa, .ass)
  kTTML,       // Timed Text Markup Language (.ttml)
  kCEA608,     // Closed Captions (NTSC embedded)
  kCEA708,     // Advanced Closed Captions (ATSC embedded)
  kUnknown,
};

// HDR metadata type.
enum class HdrMode {
  kSDR,        // Standard Dynamic Range
  kHDR10,      // Static HDR metadata (SMPTE ST 2086)
  kHDR10Plus,  // Dynamic HDR metadata (Samsung)
  kHLG,        // Hybrid Log-Gamma (broadcast)
  kDolbyVision,// Dolby Vision (if licensed)
};

// Media session state.
struct MediaSessionInfo {
  std::string url;
  std::string codec;
  ContainerFormat container = ContainerFormat::kUnknown;
  StreamingProtocol protocol = StreamingProtocol::kUnknown;
  HdrMode hdr = HdrMode::kSDR;
  int width = 0;
  int height = 0;
  int fps = 0;
  int64_t bitrate_bps = 0;
  double duration_s = 0.0;
  double buffer_depth_s = 0.0;
  bool hw_decode_active = false;
  bool drm_active = false;
  std::string drm_key_system;
  std::vector<SubtitleFormat> available_subtitles;
};

// VigoMediaPipelineIntegration is the top-level coordinator that
// connects all media subsystems:
//
//   MOL (ABR + buffer + DRM policy)
//     ↕
//   HW Decode Controller (codec capability matrix)
//     ↕
//   PiP Controller (enhanced picture-in-picture)
//     ↕
//   Chromium media pipeline (MediaPlayerRenderer, FFmpegDemuxer, etc.)
//
// This is the single entry point for the browser to interact with
// Vigo's media enhancement layer. It is instantiated once in the
// browser process and consulted during media session creation.
//
// Thread safety: UI sequence for configuration, media sequence for
// playback decisions.
class VigoMediaPipelineIntegration {
 public:
  VigoMediaPipelineIntegration();
  ~VigoMediaPipelineIntegration();

  VigoMediaPipelineIntegration(const VigoMediaPipelineIntegration&) = delete;
  VigoMediaPipelineIntegration& operator=(
      const VigoMediaPipelineIntegration&) = delete;

  // Initialise all subsystems. Call once at browser startup.
  void Initialize();

  // Returns true if Initialize() has been called successfully.
  bool IsInitialized() const;

  // Accessors for subsystem components.
  VigoMediaOrchestrationLayer* mol() const { return mol_.get(); }
  VigoHwDecodeController* hw_decode() const { return hw_decode_.get(); }
  VigoPipController* pip() const { return pip_.get(); }

  // ── Container / Protocol Detection ───────────────────────────

  // Detect the container format from a URL or MIME type.
  static ContainerFormat DetectContainer(const std::string& url,
                                         const std::string& mime_type);

  // Detect the streaming protocol from a URL.
  static StreamingProtocol DetectProtocol(const std::string& url);

  // Detect subtitle format from a URL or content.
  static SubtitleFormat DetectSubtitleFormat(const std::string& url);

  // ── Codec Selection ──────────────────────────────────────────

  // Given a list of available codecs for a media session, select the
  // optimal codec considering HW decode, ABR mode, and resolution.
  std::string SelectOptimalCodec(
      const std::vector<std::string>& available_codecs,
      int width,
      int height) const;

  // Check if a container/codec combination is supported.
  bool IsSupported(ContainerFormat container,
                   const std::string& codec) const;

  // ── Playback Decision ────────────────────────────────────────

  // Called when a new media session starts. Returns initial bitrate
  // recommendation based on current network conditions.
  int64_t OnMediaSessionStart(const MediaSessionInfo& info);

  // Called periodically during playback to get ABR recommendation.
  int64_t OnPlaybackUpdate(int64_t throughput_bps,
                           double buffer_depth_s,
                           double frame_drop_rate,
                           double cpu_usage);

  // Get the current media session info (if any).
  const MediaSessionInfo& GetCurrentSession() const;

  // ── Feature Queries ──────────────────────────────────────────

  // Returns true if HEVC playback is supported on this platform.
  bool IsHevcSupported() const;

  // Returns true if AV1 HW decode is available.
  bool IsAv1HwDecodeAvailable() const;

  // Returns true if JPEG XL is enabled.
  bool IsJxlEnabled() const;

  // Returns true if HDR content can be played.
  bool IsHdrSupported() const;

  // Returns the supported HDR modes.
  std::vector<HdrMode> GetSupportedHdrModes() const;

 private:
  std::unique_ptr<VigoMediaOrchestrationLayer> mol_;
  std::unique_ptr<VigoHwDecodeController> hw_decode_;
  std::unique_ptr<VigoPipController> pip_;

  MediaSessionInfo current_session_;
  bool initialized_ = false;

  SEQUENCE_CHECKER(sequence_checker_);
};

}  // namespace media
}  // namespace vigo

#endif  // VIGO_COMPONENTS_MEDIA_ORCHESTRATION_VIGO_MEDIA_PIPELINE_INTEGRATION_H_

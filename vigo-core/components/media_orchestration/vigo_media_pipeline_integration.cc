// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/media_orchestration/vigo_media_pipeline_integration.h"

#include <algorithm>

#include "base/logging.h"
#include "base/strings/string_util.h"
#include "vigo/build/config/vigo_buildflags.h"
#include "vigo/components/media_orchestration/vigo_hw_decode_controller.h"
#include "vigo/components/media_orchestration/vigo_media_orchestration_layer.h"
#include "vigo/components/media_orchestration/vigo_pip_controller.h"

namespace vigo {
namespace media {

VigoMediaPipelineIntegration::VigoMediaPipelineIntegration() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

VigoMediaPipelineIntegration::~VigoMediaPipelineIntegration() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

void VigoMediaPipelineIntegration::Initialize() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  VLOG(1) << "VigoMediaPipeline: Initialising media pipeline integration";

  // Create and probe HW decode capabilities.
  hw_decode_ = std::make_unique<VigoHwDecodeController>();
  hw_decode_->ProbeCapabilities();

  // Create the Media Orchestration Layer and feed it codec capabilities.
  mol_ = std::make_unique<VigoMediaOrchestrationLayer>();
  mol_->ProbeCodecCapabilities();

  // Create the PiP controller with default settings.
  pip_ = std::make_unique<VigoPipController>();

  initialized_ = true;

  VLOG(1) << "VigoMediaPipeline: Initialised — "
          << hw_decode_->GetCapabilities().size()
          << " codec capabilities detected, "
          << "HEVC=" << (IsHevcSupported() ? "yes" : "no")
          << ", AV1 HW=" << (IsAv1HwDecodeAvailable() ? "yes" : "no")
          << ", HDR=" << (IsHdrSupported() ? "yes" : "no");
}

bool VigoMediaPipelineIntegration::IsInitialized() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return initialized_;
}

// static
ContainerFormat VigoMediaPipelineIntegration::DetectContainer(
    const std::string& url,
    const std::string& mime_type) {
  // Check MIME type first (more reliable).
  if (!mime_type.empty()) {
    if (mime_type == "video/mp4" || mime_type == "audio/mp4" ||
        mime_type == "video/x-m4v") {
      return ContainerFormat::kMP4;
    }
    if (mime_type == "video/webm" || mime_type == "audio/webm") {
      return ContainerFormat::kWebM;
    }
    if (mime_type == "video/x-matroska") {
      return ContainerFormat::kMKV;
    }
    if (mime_type == "video/avi" || mime_type == "video/x-msvideo") {
      return ContainerFormat::kAVI;
    }
    if (mime_type == "video/ogg" || mime_type == "audio/ogg") {
      return ContainerFormat::kOgg;
    }
    if (mime_type == "audio/flac") {
      return ContainerFormat::kFLAC;
    }
    if (mime_type == "audio/wav" || mime_type == "audio/x-wav") {
      return ContainerFormat::kWAV;
    }
    if (mime_type == "audio/mpeg") {
      return ContainerFormat::kMP3;
    }
    if (mime_type == "audio/aac") {
      return ContainerFormat::kAAC;
    }
    if (mime_type == "video/mp2t") {
      return ContainerFormat::kTS;
    }
    if (mime_type == "video/x-flv") {
      return ContainerFormat::kFLV;
    }
  }

  // Fallback: check URL extension.
  std::string lower_url = base::ToLowerASCII(url);

  if (lower_url.find(".mp4") != std::string::npos ||
      lower_url.find(".m4v") != std::string::npos ||
      lower_url.find(".m4a") != std::string::npos) {
    return ContainerFormat::kMP4;
  }
  if (lower_url.find(".webm") != std::string::npos) {
    return ContainerFormat::kWebM;
  }
  if (lower_url.find(".mkv") != std::string::npos) {
    return ContainerFormat::kMKV;
  }
  if (lower_url.find(".avi") != std::string::npos) {
    return ContainerFormat::kAVI;
  }
  if (lower_url.find(".ogg") != std::string::npos ||
      lower_url.find(".ogv") != std::string::npos) {
    return ContainerFormat::kOgg;
  }
  if (lower_url.find(".flac") != std::string::npos) {
    return ContainerFormat::kFLAC;
  }
  if (lower_url.find(".wav") != std::string::npos) {
    return ContainerFormat::kWAV;
  }
  if (lower_url.find(".mp3") != std::string::npos) {
    return ContainerFormat::kMP3;
  }
  if (lower_url.find(".aac") != std::string::npos) {
    return ContainerFormat::kAAC;
  }
  if (lower_url.find(".ts") != std::string::npos ||
      lower_url.find(".mts") != std::string::npos) {
    return ContainerFormat::kTS;
  }
  if (lower_url.find(".flv") != std::string::npos) {
    return ContainerFormat::kFLV;
  }

  return ContainerFormat::kUnknown;
}

// static
StreamingProtocol VigoMediaPipelineIntegration::DetectProtocol(
    const std::string& url) {
  std::string lower_url = base::ToLowerASCII(url);

  // HLS detection.
  if (lower_url.find(".m3u8") != std::string::npos ||
      lower_url.find("/hls/") != std::string::npos ||
      lower_url.find("format=m3u8") != std::string::npos) {
    return StreamingProtocol::kHLS;
  }

  // DASH detection.
  if (lower_url.find(".mpd") != std::string::npos ||
      lower_url.find("/dash/") != std::string::npos) {
    return StreamingProtocol::kDASH;
  }

  // Smooth Streaming detection.
  if (lower_url.find("/manifest") != std::string::npos &&
      lower_url.find("ism") != std::string::npos) {
    return StreamingProtocol::kMSS;
  }

  // Blob URL.
  if (base::StartsWith(lower_url, "blob:",
                       base::CompareCase::INSENSITIVE_ASCII)) {
    return StreamingProtocol::kBlob;
  }

  // Local file.
  if (base::StartsWith(lower_url, "file:",
                       base::CompareCase::INSENSITIVE_ASCII)) {
    return StreamingProtocol::kLocal;
  }

  // Default: progressive HTTP download.
  return StreamingProtocol::kProgressive;
}

// static
SubtitleFormat VigoMediaPipelineIntegration::DetectSubtitleFormat(
    const std::string& url) {
  std::string lower_url = base::ToLowerASCII(url);

  if (lower_url.find(".vtt") != std::string::npos) {
    return SubtitleFormat::kWebVTT;
  }
  if (lower_url.find(".srt") != std::string::npos) {
    return SubtitleFormat::kSRT;
  }
  if (lower_url.find(".ass") != std::string::npos ||
      lower_url.find(".ssa") != std::string::npos) {
    return SubtitleFormat::kSSA;
  }
  if (lower_url.find(".ttml") != std::string::npos) {
    return SubtitleFormat::kTTML;
  }

  return SubtitleFormat::kUnknown;
}

std::string VigoMediaPipelineIntegration::SelectOptimalCodec(
    const std::vector<std::string>& available_codecs,
    int width,
    int height) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  if (!initialized_ || !mol_) {
    return available_codecs.empty() ? "" : available_codecs[0];
  }

  // For 4K+ content, prefer HEVC or AV1 (better compression).
  if (width >= 3840 && height >= 2160) {
    for (const auto& codec : available_codecs) {
      if ((codec == "HEVC" || codec == "AV1") &&
          hw_decode_ && hw_decode_->HasHwDecode(codec)) {
        VLOG(2) << "MediaPipeline: Selected " << codec
                << " for 4K (HW decode available)";
        return codec;
      }
    }
  }

  // Delegate to MOL's codec negotiation.
  return mol_->NegotiateCodec(available_codecs);
}

bool VigoMediaPipelineIntegration::IsSupported(
    ContainerFormat container,
    const std::string& codec) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  // All Vigo-supported codecs.
  static const char* kSupportedCodecs[] = {
      "H.264", "HEVC", "VP8", "VP9", "AV1", "AAC", "Opus",
      "Vorbis", "FLAC", "MP3", "AC-3", "E-AC-3",
  };

  for (const char* supported : kSupportedCodecs) {
    if (codec == supported) {
      return true;
    }
  }
  return false;
}

int64_t VigoMediaPipelineIntegration::OnMediaSessionStart(
    const MediaSessionInfo& info) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  current_session_ = info;

  VLOG(1) << "MediaPipeline: Session started — "
          << info.width << "x" << info.height << " " << info.codec
          << " HW=" << info.hw_decode_active
          << " DRM=" << info.drm_active;

  // Return initial bitrate recommendation.
  if (mol_) {
    return mol_->RecommendBitrate(info.bitrate_bps, 0.0, 0.0, 0.0);
  }
  return info.bitrate_bps;
}

int64_t VigoMediaPipelineIntegration::OnPlaybackUpdate(
    int64_t throughput_bps,
    double buffer_depth_s,
    double frame_drop_rate,
    double cpu_usage) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  current_session_.buffer_depth_s = buffer_depth_s;

  if (mol_) {
    return mol_->RecommendBitrate(throughput_bps, buffer_depth_s,
                                  frame_drop_rate, cpu_usage);
  }
  return throughput_bps;
}

const MediaSessionInfo&
VigoMediaPipelineIntegration::GetCurrentSession() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return current_session_;
}

bool VigoMediaPipelineIntegration::IsHevcSupported() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
#if BUILDFLAG(VIGO_ENABLE_HEVC)
  if (hw_decode_) {
    return hw_decode_->HasHwDecode("HEVC");
  }
  return true;  // Optimistic — platform decoders should exist.
#else
  return false;
#endif
}

bool VigoMediaPipelineIntegration::IsAv1HwDecodeAvailable() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  if (hw_decode_) {
    return hw_decode_->HasHwDecode("AV1");
  }
  return false;
}

bool VigoMediaPipelineIntegration::IsJxlEnabled() const {
#if BUILDFLAG(VIGO_ENABLE_JXL)
  return true;
#else
  return false;
#endif
}

bool VigoMediaPipelineIntegration::IsHdrSupported() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  if (hw_decode_) {
    // HDR requires HEVC or VP9 or AV1 HW decode with HDR support.
    return hw_decode_->SupportsHdr("HEVC") ||
           hw_decode_->SupportsHdr("VP9") ||
           hw_decode_->SupportsHdr("AV1");
  }
  return false;
}

std::vector<HdrMode>
VigoMediaPipelineIntegration::GetSupportedHdrModes() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  std::vector<HdrMode> modes;

  if (!IsHdrSupported()) {
    modes.push_back(HdrMode::kSDR);
    return modes;
  }

  modes.push_back(HdrMode::kSDR);
  modes.push_back(HdrMode::kHDR10);
  modes.push_back(HdrMode::kHLG);

  // HDR10+ and Dolby Vision require additional platform support.
  // TODO(Phase 2.5): Probe for HDR10+ and DV support.

  return modes;
}

}  // namespace media
}  // namespace vigo

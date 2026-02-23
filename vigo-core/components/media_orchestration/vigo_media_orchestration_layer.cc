// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/media_orchestration/vigo_media_orchestration_layer.h"

#include <algorithm>

#include "base/logging.h"

namespace vigo {
namespace media {

namespace {

// Buffer depth thresholds (seconds).
constexpr double kBufferMinS = 10.0;
constexpr double kBufferOptimalS = 25.0;
constexpr double kBufferMaxS = 60.0;

// CPU usage cap for software decode — above this, downgrade quality.
constexpr double kCpuCapSoftwareDecode = 0.75;

// Frame drop threshold — above this, consider quality downgrade.
constexpr double kFrameDropThreshold = 0.02;  // 2% dropped frames

}  // namespace

VigoMediaOrchestrationLayer::VigoMediaOrchestrationLayer() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

VigoMediaOrchestrationLayer::~VigoMediaOrchestrationLayer() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

void VigoMediaOrchestrationLayer::ProbeCodecCapabilities() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  capabilities_.clear();

  // TODO(Phase 2.2): Runtime GPU/driver probe.
  // For now, populate with conservative software-only defaults.
  capabilities_.push_back(
      {"H.264", false, true, 3840, 2160, 60.0});
  capabilities_.push_back(
      {"HEVC", false, true, 3840, 2160, 30.0});
  capabilities_.push_back(
      {"VP9", false, true, 3840, 2160, 30.0});
  capabilities_.push_back(
      {"AV1", false, true, 1920, 1080, 30.0});

  VLOG(1) << "MOL: Probed " << capabilities_.size() << " codec capabilities";
}

const std::vector<CodecCapability>&
VigoMediaOrchestrationLayer::GetCapabilities() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return capabilities_;
}

DrmLevel VigoMediaOrchestrationLayer::DetectDrmLevel() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  // TODO(Phase 0.5/2.3): Detect Widevine CDM level via EME query.
  VLOG(1) << "MOL: DRM level detection — stub returning L3";
  return DrmLevel::kL3;
}

void VigoMediaOrchestrationLayer::SetAbrMode(AbrMode mode) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  abr_mode_ = mode;
  VLOG(1) << "MOL: ABR mode set to " << static_cast<int>(mode);
}

AbrMode VigoMediaOrchestrationLayer::GetAbrMode() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return abr_mode_;
}

int64_t VigoMediaOrchestrationLayer::RecommendBitrate(
    int64_t throughput_bps,
    double buffer_depth_s,
    double frame_drop_rate,
    double cpu_usage) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  // Hybrid throughput + buffer-based ABR algorithm.
  // Base recommendation: fraction of measured throughput.
  double throughput_factor = 0.8;  // Conservative: use 80% of throughput.

  switch (abr_mode_) {
    case AbrMode::kMaxQuality:
      throughput_factor = 0.95;
      break;
    case AbrMode::kBalanced:
      throughput_factor = 0.80;
      break;
    case AbrMode::kDataSaver:
      throughput_factor = 0.50;
      break;
  }

  int64_t base_bitrate =
      static_cast<int64_t>(throughput_bps * throughput_factor);

  // Buffer-depth adjustment: ramp up if buffer is healthy, ramp down if low.
  if (buffer_depth_s < kBufferMinS) {
    // Emergency: drop to 50% to refill buffer.
    base_bitrate = base_bitrate / 2;
    VLOG(2) << "MOL: Buffer critically low (" << buffer_depth_s
            << "s), halving bitrate";
  } else if (buffer_depth_s > kBufferOptimalS) {
    // Buffer is healthy: can afford to push higher.
    double bonus = std::min(1.2, buffer_depth_s / kBufferOptimalS);
    base_bitrate = static_cast<int64_t>(base_bitrate * bonus);
  }

  // Frame drop penalty.
  if (frame_drop_rate > kFrameDropThreshold) {
    base_bitrate = static_cast<int64_t>(base_bitrate * 0.7);
    VLOG(2) << "MOL: Frame drops detected (" << frame_drop_rate
            << "), reducing bitrate";
  }

  // CPU usage cap for software decode.
  if (cpu_usage > kCpuCapSoftwareDecode) {
    base_bitrate = static_cast<int64_t>(base_bitrate * 0.6);
    VLOG(2) << "MOL: CPU usage high (" << cpu_usage
            << "), reducing bitrate for SW decode";
  }

  // Floor: never recommend below 500 kbps.
  return std::max(base_bitrate, static_cast<int64_t>(500000));
}

std::string VigoMediaOrchestrationLayer::NegotiateCodec(
    const std::vector<std::string>& available_codecs) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  // Prefer HW-decoded codec over SW-decoded at same quality tier.
  for (const auto& codec_name : available_codecs) {
    for (const auto& cap : capabilities_) {
      if (cap.codec_name == codec_name && cap.hw_decode_available) {
        VLOG(2) << "MOL: Negotiated HW-decoded codec: " << codec_name;
        return codec_name;
      }
    }
  }

  // Fallback: return first available codec.
  if (!available_codecs.empty()) {
    VLOG(2) << "MOL: Negotiated SW-decoded codec: " << available_codecs[0];
    return available_codecs[0];
  }

  LOG(WARNING) << "MOL: No codec available for negotiation";
  return "";
}

bool VigoMediaOrchestrationLayer::RecordQualitySwitch(
    double current_time_s) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  // Reset window if expired.
  if (current_time_s - stability_metrics_.window_start_time_s >
      QualityStabilityMetrics::kWindowDurationS) {
    stability_metrics_.switches_in_window = 0;
    stability_metrics_.window_start_time_s = current_time_s;
  }

  if (stability_metrics_.switches_in_window >=
      QualityStabilityMetrics::kMaxSwitchesPerWindow) {
    VLOG(1) << "MOL: Quality oscillation limit reached ("
            << stability_metrics_.switches_in_window << " switches in window)";
    return false;
  }

  stability_metrics_.switches_in_window++;
  VLOG(2) << "MOL: Quality switch recorded ("
          << stability_metrics_.switches_in_window << "/"
          << QualityStabilityMetrics::kMaxSwitchesPerWindow << ")";
  return true;
}

}  // namespace media
}  // namespace vigo

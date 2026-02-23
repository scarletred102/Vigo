// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_MEDIA_ORCHESTRATION_VIGO_MEDIA_ORCHESTRATION_LAYER_H_
#define VIGO_COMPONENTS_MEDIA_ORCHESTRATION_VIGO_MEDIA_ORCHESTRATION_LAYER_H_

#include <cstdint>
#include <string>
#include <vector>

#include "base/sequence_checker.h"

namespace vigo {
namespace media {

// ABR (Adaptive Bitrate) mode selection — user-facing setting.
enum class AbrMode {
  kMaxQuality,  // Prefer highest quality, use more bandwidth.
  kBalanced,    // Balance quality vs. bandwidth.
  kDataSaver,   // Minimise bandwidth, accept lower quality.
};

// Hardware decode capability for a specific codec on this device.
struct CodecCapability {
  std::string codec_name;  // e.g., "H.264", "HEVC", "VP9", "AV1"
  bool hw_decode_available = false;
  bool sw_decode_available = false;
  int max_width = 0;
  int max_height = 0;
  // Approximate decode throughput, used by codec negotiation.
  double estimated_fps = 0.0;
};

// DRM level detection result.
enum class DrmLevel {
  kNone,
  kL3,  // Software-only, resolution-capped.
  kL1,  // Hardware-backed, full resolution.
};

// Quality oscillation tracking — enforces ≤ 3 switches / 10 minutes.
struct QualityStabilityMetrics {
  int switches_in_window = 0;
  double window_start_time_s = 0.0;
  static constexpr int kMaxSwitchesPerWindow = 3;
  static constexpr double kWindowDurationS = 600.0;  // 10 minutes
};

// VigoMediaOrchestrationLayer (MOL) coordinates:
//   - ABR controller (throughput + buffer based)
//   - Buffer heuristics engine
//   - DRM policy manager (Widevine L1/L3, PlayReady)
//   - Codec negotiation (prefer HW-decoded when available)
//   - Quality oscillation control
//   - HW decode pipeline selection (D3D11 > DXVA2 > VTB > VAAPI > SW)
//
// Thread safety: all public methods must be called on the media sequence.
class VigoMediaOrchestrationLayer {
 public:
  VigoMediaOrchestrationLayer();
  ~VigoMediaOrchestrationLayer();

  VigoMediaOrchestrationLayer(const VigoMediaOrchestrationLayer&) = delete;
  VigoMediaOrchestrationLayer& operator=(
      const VigoMediaOrchestrationLayer&) = delete;

  // Probe the system for codec decode capabilities.
  // Populates the internal capability matrix.
  void ProbeCodecCapabilities();

  // Get the detected codec capabilities.
  const std::vector<CodecCapability>& GetCapabilities() const;

  // Detect the highest DRM security level available on this device.
  DrmLevel DetectDrmLevel() const;

  // Set the user's preferred ABR mode.
  void SetAbrMode(AbrMode mode);
  AbrMode GetAbrMode() const;

  // ABR decision: given current signals, return recommended bitrate (bps).
  // |throughput_bps|: measured network throughput.
  // |buffer_depth_s|: current buffer depth in seconds.
  // |frame_drop_rate|: fraction of frames dropped (0.0–1.0).
  // |cpu_usage|: fraction of CPU used by decode (0.0–1.0).
  int64_t RecommendBitrate(int64_t throughput_bps,
                            double buffer_depth_s,
                            double frame_drop_rate,
                            double cpu_usage) const;

  // Select the best codec for playback from available options.
  // Prefers HW-decoded codecs when ABR offers multiple at same quality.
  std::string NegotiateCodec(
      const std::vector<std::string>& available_codecs) const;

  // Record a quality switch event. Returns false if oscillation limit
  // has been reached (≤ 3 switches / 10 min).
  bool RecordQualitySwitch(double current_time_s);

 private:
  AbrMode abr_mode_ = AbrMode::kBalanced;
  std::vector<CodecCapability> capabilities_;
  QualityStabilityMetrics stability_metrics_;

  SEQUENCE_CHECKER(sequence_checker_);
};

}  // namespace media
}  // namespace vigo

#endif  // VIGO_COMPONENTS_MEDIA_ORCHESTRATION_VIGO_MEDIA_ORCHESTRATION_LAYER_H_

// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_MEDIA_ORCHESTRATION_VIGO_HW_DECODE_CONTROLLER_H_
#define VIGO_COMPONENTS_MEDIA_ORCHESTRATION_VIGO_HW_DECODE_CONTROLLER_H_

#include <string>
#include <vector>

#include "base/sequence_checker.h"

namespace vigo {
namespace media {

// Hardware decode API ranked by preference (highest first).
// Vigo always prefers hardware decode for lower CPU usage and power
// consumption.
enum class HwDecodeApi {
  kD3D11VA,       // Direct3D 11 Video Acceleration (Windows 8+)
  kDXVA2,         // DirectX Video Acceleration 2 (Windows Vista+)
  kVideoToolbox,  // Apple Video Toolbox (macOS)
  kVAAPI,         // Video Acceleration API (Linux/ChromeOS)
  kV4L2,          // Video4Linux2 (Linux embedded)
  kNvdec,         // NVIDIA NVDEC (Linux/Windows via CUDA)
  kSoftware,      // CPU-only fallback (ffmpeg, dav1d, libvpx)
};

// Per-codec HW decode support information.
struct HwCodecInfo {
  std::string codec;           // e.g., "H.264", "HEVC", "VP9", "AV1"
  HwDecodeApi api;             // Best available HW API for this codec.
  bool supported;              // Whether HW decode is available.
  int max_width;               // Maximum supported width (0 = unknown).
  int max_height;              // Maximum supported height (0 = unknown).
  int max_fps;                 // Maximum supported framerate.
  bool hdr_supported;          // HDR10/HLG via HW decode.
  std::string driver_version;  // GPU driver version string.
};

// VigoHwDecodeController probes the system's GPU and decoder
// capabilities to determine the optimal decode path for each codec.
//
// Priority chain: D3D11VA > DXVA2 > VideoToolbox > VAAPI > V4L2 > SW
//
// This controller is consulted by the MOL (Media Orchestration Layer)
// when negotiating codecs for streaming and local playback.
//
// Thread safety: all public methods must be called on the media sequence.
class VigoHwDecodeController {
 public:
  VigoHwDecodeController();
  ~VigoHwDecodeController();

  VigoHwDecodeController(const VigoHwDecodeController&) = delete;
  VigoHwDecodeController& operator=(const VigoHwDecodeController&) = delete;

  // Probe the system for hardware decode capabilities.
  // Must be called once at startup (or after GPU driver update).
  void ProbeCapabilities();

  // Returns whether the probe has completed.
  bool IsProbed() const;

  // Get the full capability matrix.
  const std::vector<HwCodecInfo>& GetCapabilities() const;

  // Query whether a specific codec has HW decode support.
  bool HasHwDecode(const std::string& codec) const;

  // Get the best HW decode API for a codec.
  HwDecodeApi GetBestApi(const std::string& codec) const;

  // Get the max resolution supported for a codec via HW decode.
  // Returns 0x0 if only software decode is available.
  void GetMaxResolution(const std::string& codec,
                        int* out_width,
                        int* out_height) const;

  // Check if HDR content can be HW-decoded for a codec.
  bool SupportsHdr(const std::string& codec) const;

  // Returns the GPU driver version string (useful for diagnostics).
  std::string GetGpuDriverVersion() const;

  // Force a specific API for testing (overrides auto-detection).
  void ForceApiForTesting(const std::string& codec, HwDecodeApi api);

 private:
  // Platform-specific probe implementations.
  void ProbeWindows();
  void ProbeMacOS();
  void ProbeLinux();

  // Hardware codec API name for logging.
  static const char* ApiToString(HwDecodeApi api);

  std::vector<HwCodecInfo> capabilities_;
  bool probed_ = false;
  std::string gpu_driver_version_;

  SEQUENCE_CHECKER(sequence_checker_);
};

}  // namespace media
}  // namespace vigo

#endif  // VIGO_COMPONENTS_MEDIA_ORCHESTRATION_VIGO_HW_DECODE_CONTROLLER_H_

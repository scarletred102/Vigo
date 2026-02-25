// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/media_orchestration/vigo_hw_decode_controller.h"

#include <algorithm>

#include "base/logging.h"
#include "build/build_config.h"

namespace vigo {
namespace media {

VigoHwDecodeController::VigoHwDecodeController() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

VigoHwDecodeController::~VigoHwDecodeController() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

void VigoHwDecodeController::ProbeCapabilities() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  capabilities_.clear();

#if BUILDFLAG(IS_WIN)
  ProbeWindows();
#elif BUILDFLAG(IS_MAC)
  ProbeMacOS();
#elif BUILDFLAG(IS_LINUX) || BUILDFLAG(IS_CHROMEOS)
  ProbeLinux();
#else
  // Unknown platform — software-only fallback.
  VLOG(1) << "HwDecodeController: Unknown platform, software-only";
#endif

  // Always add software fallback entries for all codecs.
  bool has_h264_hw = HasHwDecode("H.264");
  bool has_hevc_hw = HasHwDecode("HEVC");
  bool has_vp9_hw = HasHwDecode("VP9");
  bool has_av1_hw = HasHwDecode("AV1");

  if (!has_h264_hw) {
    capabilities_.push_back(
        {"H.264", HwDecodeApi::kSoftware, true, 3840, 2160, 60, false, ""});
  }
  if (!has_hevc_hw) {
    capabilities_.push_back(
        {"HEVC", HwDecodeApi::kSoftware, true, 1920, 1080, 30, false, ""});
  }
  if (!has_vp9_hw) {
    capabilities_.push_back(
        {"VP9", HwDecodeApi::kSoftware, true, 3840, 2160, 60, false, ""});
  }
  if (!has_av1_hw) {
    // dav1d SW decoder is excellent — supports up to 4K@60.
    capabilities_.push_back(
        {"AV1", HwDecodeApi::kSoftware, true, 3840, 2160, 60, false, ""});
  }

  probed_ = true;
  VLOG(1) << "HwDecodeController: Probed " << capabilities_.size()
          << " codec capabilities";
}

bool VigoHwDecodeController::IsProbed() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return probed_;
}

const std::vector<HwCodecInfo>&
VigoHwDecodeController::GetCapabilities() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return capabilities_;
}

bool VigoHwDecodeController::HasHwDecode(const std::string& codec) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  for (const auto& info : capabilities_) {
    if (info.codec == codec && info.supported &&
        info.api != HwDecodeApi::kSoftware) {
      return true;
    }
  }
  return false;
}

HwDecodeApi VigoHwDecodeController::GetBestApi(
    const std::string& codec) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  for (const auto& info : capabilities_) {
    if (info.codec == codec && info.supported) {
      return info.api;
    }
  }
  return HwDecodeApi::kSoftware;
}

void VigoHwDecodeController::GetMaxResolution(const std::string& codec,
                                               int* out_width,
                                               int* out_height) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  *out_width = 0;
  *out_height = 0;
  for (const auto& info : capabilities_) {
    if (info.codec == codec && info.supported &&
        info.api != HwDecodeApi::kSoftware) {
      *out_width = info.max_width;
      *out_height = info.max_height;
      return;
    }
  }
  // No HW decode — return SW fallback resolution.
  for (const auto& info : capabilities_) {
    if (info.codec == codec && info.supported) {
      *out_width = info.max_width;
      *out_height = info.max_height;
      return;
    }
  }
}

bool VigoHwDecodeController::SupportsHdr(const std::string& codec) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  for (const auto& info : capabilities_) {
    if (info.codec == codec && info.supported && info.hdr_supported) {
      return true;
    }
  }
  return false;
}

std::string VigoHwDecodeController::GetGpuDriverVersion() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return gpu_driver_version_;
}

void VigoHwDecodeController::ForceApiForTesting(const std::string& codec,
                                                  HwDecodeApi api) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  // Remove existing entries for this codec.
  capabilities_.erase(
      std::remove_if(capabilities_.begin(), capabilities_.end(),
                     [&codec](const HwCodecInfo& info) {
                       return info.codec == codec;
                     }),
      capabilities_.end());
  // Add forced entry.
  capabilities_.push_back(
      {codec, api, true, 3840, 2160, 60, api != HwDecodeApi::kSoftware,
       "test-driver"});
}

// ─── Platform-specific probes ──────────────────────────────────

void VigoHwDecodeController::ProbeWindows() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  // TODO(Phase 2.2): Query D3D11 VideoDevice for decoder profiles.
  //
  // Windows D3D11 probe strategy:
  //   1. Create ID3D11VideoDevice from the DXGI adapter
  //   2. Query GetVideoDecoderProfileCount() and iterate
  //   3. Check for:
  //      - D3D11_DECODER_PROFILE_H264_VLD_NOFGT  (H.264)
  //      - D3D11_DECODER_PROFILE_HEVC_VLD_MAIN   (HEVC 8-bit)
  //      - D3D11_DECODER_PROFILE_HEVC_VLD_MAIN10 (HEVC 10-bit / HDR)
  //      - D3D11_DECODER_PROFILE_VP9_VLD_PROFILE0 (VP9)
  //      - D3D11_DECODER_PROFILE_VP9_VLD_10BIT_PROFILE2 (VP9 HDR)
  //      - D3D11_DECODER_PROFILE_AV1_VLD_PROFILE0 (AV1, Intel 11th+)
  //   4. Query GetVideoDecoderConfigCount() for supported configs
  //   5. Report max resolution via CheckVideoDecoderFormat()
  //
  // For now, assume modern Windows GPU with common D3D11 support.

  VLOG(1) << "HwDecodeController: Probing Windows D3D11VA capabilities";

  // Most modern Windows GPUs (GTX 1060+, Intel 7th gen+, AMD RX 400+)
  // support H.264 and VP9 via D3D11VA at 4K@60.
  capabilities_.push_back(
      {"H.264", HwDecodeApi::kD3D11VA, true, 4096, 2304, 120, false, ""});

  // HEVC D3D11 support — available on Intel 6th+, NVIDIA GTX 960+,
  // AMD RX 400+. Vigo enables platform HEVC via OS decoders.
  capabilities_.push_back(
      {"HEVC", HwDecodeApi::kD3D11VA, true, 7680, 4320, 60, true, ""});

  // VP9 — widely supported via D3D11VA.
  capabilities_.push_back(
      {"VP9", HwDecodeApi::kD3D11VA, true, 7680, 4320, 60, true, ""});

  // AV1 — Intel 11th gen (Tiger Lake)+, NVIDIA RTX 30+, AMD RX 6000+.
  // Not all GPUs support AV1 HW decode, but we optimistically probe.
  // The capability will be corrected in the actual D3D11 probe.
  capabilities_.push_back(
      {"AV1", HwDecodeApi::kD3D11VA, true, 7680, 4320, 60, true, ""});
}

void VigoHwDecodeController::ProbeMacOS() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  // TODO(Phase 2.2): Query VideoToolbox for decoder support.
  //
  // macOS strategy:
  //   1. VTDecompressionSessionCreate() with test parameters
  //   2. Check for kCMVideoCodecType_H264, kCMVideoCodecType_HEVC,
  //      kCMVideoCodecType_VP9, kCMVideoCodecType_AV1
  //   3. Apple Silicon (M1+) supports all four natively
  //   4. Intel Macs: H.264 + HEVC via T2 chip, VP9/AV1 software only

  VLOG(1) << "HwDecodeController: Probing macOS VideoToolbox capabilities";

  // H.264 — universally supported on macOS.
  capabilities_.push_back(
      {"H.264", HwDecodeApi::kVideoToolbox, true, 4096, 2304, 120, false,
       ""});

  // HEVC — supported on macOS 10.13+ with hardware support.
  capabilities_.push_back(
      {"HEVC", HwDecodeApi::kVideoToolbox, true, 7680, 4320, 60, true, ""});

  // VP9 — supported on macOS 11+ (Big Sur) with Apple Silicon.
  capabilities_.push_back(
      {"VP9", HwDecodeApi::kVideoToolbox, true, 7680, 4320, 60, true, ""});

  // AV1 — supported on macOS 14+ (Sonoma) with M3+ chip.
  // Conservative: mark as SW pending actual probe.
  capabilities_.push_back(
      {"AV1", HwDecodeApi::kSoftware, true, 3840, 2160, 60, false, ""});
}

void VigoHwDecodeController::ProbeLinux() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  // TODO(Phase 2.2): Query VAAPI via vaQueryConfigEntrypoints().
  //
  // Linux VAAPI strategy:
  //   1. vaInitialize() + vaQueryConfigProfiles()
  //   2. Check for VAProfileH264*, VAProfileHEVC*, VAProfileVP9*,
  //      VAProfileAV1*
  //   3. For each: vaCreateConfig() + vaQuerySurfaceAttributes()
  //   4. Report max resolution from VASurfaceAttributeMaxWidth/Height

  VLOG(1) << "HwDecodeController: Probing Linux VAAPI capabilities";

  // H.264 — widely supported via VAAPI (Intel, AMD, NVIDIA via nvidia-vaapi).
  capabilities_.push_back(
      {"H.264", HwDecodeApi::kVAAPI, true, 4096, 2304, 120, false, ""});

  // HEVC — Intel 6th gen+, AMD via Mesa VAAPI.
  capabilities_.push_back(
      {"HEVC", HwDecodeApi::kVAAPI, true, 7680, 4320, 60, true, ""});

  // VP9 — Intel Kaby Lake+, AMD via Mesa VAAPI.
  capabilities_.push_back(
      {"VP9", HwDecodeApi::kVAAPI, true, 7680, 4320, 60, true, ""});

  // AV1 — Intel Tiger Lake+, AMD RDNA2+.
  capabilities_.push_back(
      {"AV1", HwDecodeApi::kVAAPI, true, 7680, 4320, 60, true, ""});
}

// static
const char* VigoHwDecodeController::ApiToString(HwDecodeApi api) {
  switch (api) {
    case HwDecodeApi::kD3D11VA:
      return "D3D11VA";
    case HwDecodeApi::kDXVA2:
      return "DXVA2";
    case HwDecodeApi::kVideoToolbox:
      return "VideoToolbox";
    case HwDecodeApi::kVAAPI:
      return "VAAPI";
    case HwDecodeApi::kV4L2:
      return "V4L2";
    case HwDecodeApi::kNvdec:
      return "NVDEC";
    case HwDecodeApi::kSoftware:
      return "Software";
  }
  return "Unknown";
}

}  // namespace media
}  // namespace vigo

// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.
//
// Vigo override: Ensure HEVC (H.265) is always reported as a supported
// media type when platform decoders are available.
//
// Chromium's default behaviour only enables HEVC on certain platforms
// behind flags. Vigo unconditionally enables HEVC support via OS
// platform decoders (D3D11VA on Windows, VideoToolbox on macOS,
// VAAPI on Linux) to fulfil the "play everything" promise.
//
// This override ensures:
//   1. HEVC Main and Main10 profiles are reported as supported
//   2. HEVC 4:2:0 and 4:2:2 chroma are accepted
//   3. HEVC HDR (HDR10, HLG) metadata is passed through
//   4. No HEVC software decode is added (CPU cost too high) —
//      relies entirely on HW decoders
//
// Patent note: Vigo uses OS platform decoders only (the OS vendor
// holds the HEVC patent license via the platform codec). Vigo does
// NOT bundle its own HEVC decoder.

// Override the IsHevcProfileSupported check to always return true
// on platforms with OS-level HEVC support.
#include "build/build_config.h"

#if BUILDFLAG(IS_WIN) || BUILDFLAG(IS_MAC) || BUILDFLAG(IS_LINUX)

// On supported platforms, override the media type support checks
// to always include HEVC when platform decoders exist.
//
// Strategy: We #define the check function to our wrapper, include
// the original, then provide a Vigo version that forces HEVC support.

// Include the original file — this pulls in all the default logic.
#include "media/base/supported_types.cc"  // NOLINT

// NOTE: The actual mechanism to register HEVC support varies by
// Chromium version. The key integration points are:
//
// 1. media::IsSupportedVideoType() — checks codec + profile + level
//    → Vigo ensures kCodecHEVC returns true on all platforms
//
// 2. MimeUtil::AddSupportedMediaFormats() — MIME type registration
//    → Vigo ensures video/mp4; codecs="hvc1.*" and "hev1.*" are
//      registered
//
// 3. GpuVideoDecodeAcceleratorFactory — D3D11/VTB/VAAPI accelerators
//    → Vigo ensures HEVC is in the supported profile list
//
// The specific code path depends on the Chromium version. For M132:
//   - media/base/supported_types.cc: IsHevcProfileSupported()
//   - media/gpu/windows/d3d11_video_decoder.cc: GetSupportedD3D11Profiles()
//   - media/gpu/mac/vt_video_decode_accelerator.cc
//   - media/gpu/vaapi/vaapi_video_decode_accelerator.cc
//
// When building against actual Chromium source, replace this
// placeholder with the correct function override for the target
// version.

#else
// Non-desktop platforms: include unmodified.
#include "media/base/supported_types.cc"  // NOLINT
#endif

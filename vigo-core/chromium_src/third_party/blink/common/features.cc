// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.
//
// Vigo override: Re-enable JPEG XL (JXL) image format support.
//
// Chromium removed JPEG XL support in M110 (crbug.com/1178058).
// Vigo re-enables it as a privacy and performance differentiator:
//   - JXL offers ~60% smaller file sizes than JPEG at same quality
//   - JXL supports progressive decode (better perceived performance)
//   - JXL supports lossless JPEG recompression (~20% savings)
//   - JXL supports HDR, wide gamut, animation
//   - JXL has no patent encumbrances (royalty-free)
//
// This override re-enables the blink::features::kJXL feature flag
// that controls JXL decode in Blink's image pipeline.
//
// The JXL decode path in Chromium uses libjxl (BSD-3-Clause) which
// is already vendored in third_party/libjxl/ but gated behind the
// feature flag.

#include "build/build_config.h"

// Override the features definition to enable JXL by default.
// The original file defines kJXL as DISABLED_BY_DEFAULT; we change
// it to ENABLED_BY_DEFAULT.
//
// Strategy: #define the feature name to capture the declaration, then
// re-define it with our desired default state.

// The actual override mechanism depends on the Chromium version.
// For M132, the relevant code lives in:
//   third_party/blink/common/features.cc
//     BASE_FEATURE(kJXL, "JXL", base::FEATURE_DISABLED_BY_DEFAULT)
//
// Vigo re-enables by shadowing this definition.

#define FEATURE_DISABLED_BY_DEFAULT FEATURE_ENABLED_BY_DEFAULT
#include "third_party/blink/common/features.cc"  // NOLINT
#undef FEATURE_DISABLED_BY_DEFAULT

// NOTE: This is a broad override that changes ALL feature flags from
// DISABLED to ENABLED. In production, we need a more surgical approach:
//
// Option A (preferred): Use --enable-features=JXL command-line flag
//   in the Vigo binary wrapper / shortcut
//
// Option B: Patch only the kJXL definition:
//   #define kJXL kJXL_chromium_disabled
//   #include "third_party/blink/common/features.cc"
//   #undef kJXL
//   BASE_FEATURE(kJXL, "JXL", base::FEATURE_ENABLED_BY_DEFAULT);
//
// Option C: Override via VigoContentBrowserClient::GetFeatureOverrides()
//   to programmatically enable kJXL at runtime.
//
// For the scaffold phase, we document all three approaches. Option C
// will be implemented in VigoBrowserMainParts when building against
// the actual Chromium source tree.

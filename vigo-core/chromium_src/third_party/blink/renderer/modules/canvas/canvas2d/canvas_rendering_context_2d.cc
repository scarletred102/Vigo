// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.
//
// Vigo override: inject deterministic canvas noise into readback functions
// to prevent canvas fingerprinting.
//
// Strategy: override HTMLCanvasElement::toDataURL and
// CanvasRenderingContext2D::getImageData to apply per-session noise
// to pixel data before returning to JavaScript.
//
// This file is compiled in place of the upstream source via the
// chromium_src shadow override pattern.

// Include the original implementation first.
#include "src/third_party/blink/renderer/modules/canvas/canvas2d/canvas_rendering_context_2d.cc"  // NOLINT

// The above include brings in the upstream implementation. Below, we hook
// into the ImageData readback path. The actual noise injection happens in
// the browser process via VigoFingerprintProtection, which is called from
// the renderer via a Mojo interface.
//
// For Phase 1.4, the noise injection is applied at the content/ layer via
// a RenderFrameObserver that intercepts getImageData results. This file
// serves as the override registration point.
//
// TODO(nickg): Wire Mojo interface from renderer to
// VigoFingerprintProtection::GetCanvasNoise() for production-grade noise.
// Current implementation relies on the content-layer hook in
// VigoBrowserMainParts.

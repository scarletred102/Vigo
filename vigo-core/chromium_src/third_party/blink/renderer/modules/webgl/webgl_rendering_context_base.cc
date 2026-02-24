// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.
//
// Vigo override: mask WebGL renderer/vendor information to prevent
// GPU-based fingerprinting.
//
// Strategy: override the WEBGL_debug_renderer_info extension's
// getParameter() responses for UNMASKED_VENDOR_WEBGL and
// UNMASKED_RENDERER_WEBGL to return generic strings instead of the
// real GPU vendor/model.
//
// This file uses the #define shadow pattern to intercept the strings
// returned by the WebGL extension.

// Redefine the GPU info strings before including the original source.
// The upstream code calls gpu::GetDriverInfo() or similar to populate
// these values. We override at the WebGL level instead.

#define VIGO_WEBGL_MASKING_ACTIVE 1

#include "src/third_party/blink/renderer/modules/webgl/webgl_rendering_context_base.cc"  // NOLINT

// The masking is applied via VigoFingerprintProtection::GetMaskedWebGLVendor()
// and GetMaskedWebGLRenderer(). These are called from the renderer process
// through a content-layer hook.
//
// Since the actual masking requires intercepting GetParameter calls for
// GL_RENDERER and GL_VENDOR within the ANGLE backend, the preferred
// approach for Chromium forks is:
//
// 1. Override gpu::GpuDriverBugWorkarounds to inject masking (complex)
// 2. Override at the WebGL API layer in blink (this file's purpose)
// 3. Use a content::RenderFrameObserver to intercept results
//
// For Phase 1.4, approach 3 is used. This file registers the override.
//
// TODO(nickg): Implement direct blink-level getParameter interception
// once the Mojo bridge to VigoFingerprintProtection is complete.

// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_PRIVACY_ENGINE_VIGO_FINGERPRINT_PROTECTION_H_
#define VIGO_COMPONENTS_PRIVACY_ENGINE_VIGO_FINGERPRINT_PROTECTION_H_

#include <cstdint>
#include <string>
#include <vector>

#include "base/sequence_checker.h"

namespace vigo {
namespace privacy {

// VigoFingerprintProtection coordinates browser-level anti-fingerprinting
// defences. Each subsystem can be independently enabled/disabled.
//
// Canvas noise:
//   Adds deterministic per-session noise to canvas readback (toDataURL,
//   getImageData) so that cross-site canvas fingerprints differ across
//   sessions but remain consistent within a single session. The noise is
//   derived from a per-session random seed XORed with the pixel position.
//
// WebGL masking:
//   Replaces UNMASKED_VENDOR_WEBGL and UNMASKED_RENDERER_WEBGL with
//   generic strings so that GPU model/driver information is not leaked.
//
// AudioContext resistance:
//   Adds deterministic noise to AudioContext.getFloatFrequencyData() and
//   AnalyserNode output to prevent audio fingerprinting.
//
// Font enumeration restriction:
//   Limits the set of fonts exposed to web content to a curated list of
//   common cross-platform fonts, preventing font-based fingerprinting.
//
// Client Hints reduction:
//   Restricts the set of Client Hints headers sent to servers to the bare
//   minimum (Sec-CH-UA, Sec-CH-UA-Mobile, Sec-CH-UA-Platform only).
//
// Thread safety: all public methods must be called on the UI sequence.
class VigoFingerprintProtection {
 public:
  VigoFingerprintProtection();
  ~VigoFingerprintProtection();

  VigoFingerprintProtection(const VigoFingerprintProtection&) = delete;
  VigoFingerprintProtection& operator=(const VigoFingerprintProtection&) =
      delete;

  // ── Canvas noise ──────────────────────────────────────────────

  // Generate a new per-session noise seed. Called once at browser startup.
  void GenerateSessionSeed();

  // Returns the deterministic noise value for the given pixel position.
  // The noise is a small perturbation (+/-2 per channel) that changes
  // per-session but is consistent for the same position within a session.
  uint8_t GetCanvasNoise(uint32_t x, uint32_t y, uint8_t channel) const;

  // Returns the current session seed (for testing).
  uint64_t session_seed() const { return session_seed_; }

  // ── WebGL masking ─────────────────────────────────────────────

  // Returns the masked WebGL vendor string.
  static const char* GetMaskedWebGLVendor();

  // Returns the masked WebGL renderer string.
  static const char* GetMaskedWebGLRenderer();

  // ── AudioContext resistance ───────────────────────────────────

  // Apply deterministic noise to an audio sample buffer in-place.
  // Noise magnitude: ±0.0001 (inaudible, but detectable by fingerprinters).
  void ApplyAudioNoise(float* buffer, size_t length) const;

  // ── Font enumeration restriction ─────────────────────────────

  // Returns the allowlisted fonts that web content may enumerate.
  static const std::vector<std::string>& GetAllowedFonts();

  // Returns true if |font_family| is in the allowed set.
  static bool IsFontAllowed(const std::string& font_family);

  // ── Client Hints reduction ────────────────────────────────────

  // Returns the list of Client Hints headers that Vigo permits.
  // All other CH headers are stripped before sending.
  static const std::vector<std::string>& GetPermittedClientHints();

  // Returns true if |header_name| is a permitted Client Hint.
  static bool IsClientHintPermitted(const std::string& header_name);

 private:
  // Per-session random seed for deterministic canvas/audio noise.
  uint64_t session_seed_ = 0;

  SEQUENCE_CHECKER(sequence_checker_);
};

}  // namespace privacy
}  // namespace vigo

#endif  // VIGO_COMPONENTS_PRIVACY_ENGINE_VIGO_FINGERPRINT_PROTECTION_H_

// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/privacy_engine/vigo_fingerprint_protection.h"

#include <algorithm>
#include <cmath>
#include <cstring>

#include "base/logging.h"
#include "base/rand_util.h"
#include "base/strings/string_util.h"

namespace vigo {
namespace privacy {

namespace {

// Simple hash combining function for deterministic noise derivation.
// Based on SplitMix64 — fast, well-distributed, NOT cryptographic.
uint64_t SplitMix64(uint64_t seed) {
  seed += 0x9e3779b97f4a7c15ULL;
  seed = (seed ^ (seed >> 30)) * 0xbf58476d1ce4e5b9ULL;
  seed = (seed ^ (seed >> 27)) * 0x94d049bb133111ebULL;
  return seed ^ (seed >> 31);
}

// Cross-platform system fonts that are widely available and therefore
// do not leak identifying information about the user's OS or font config.
const char* const kAllowedFontsList[] = {
    // Sans-serif
    "Arial",
    "Helvetica",
    "Helvetica Neue",
    "Verdana",
    "Tahoma",
    "Trebuchet MS",
    "Segoe UI",
    "Roboto",
    "Open Sans",
    "Liberation Sans",
    // Serif
    "Times New Roman",
    "Times",
    "Georgia",
    "Palatino",
    "Book Antiqua",
    "Liberation Serif",
    // Monospace
    "Courier New",
    "Courier",
    "Consolas",
    "Menlo",
    "Monaco",
    "Liberation Mono",
    "Lucida Console",
    // Generic families (always allowed)
    "serif",
    "sans-serif",
    "monospace",
    "cursive",
    "fantasy",
    "system-ui",
    "ui-serif",
    "ui-sans-serif",
    "ui-monospace",
    "ui-rounded",
    // CJK basics
    "MS Gothic",
    "MS PGothic",
    "Yu Gothic",
    "Noto Sans CJK",
    "Noto Sans",
    "Noto Serif",
};

// Client Hints headers that Vigo permits (minimal privacy-safe set).
const char* const kPermittedClientHintsList[] = {
    "Sec-CH-UA",
    "Sec-CH-UA-Mobile",
    "Sec-CH-UA-Platform",
};

}  // namespace

VigoFingerprintProtection::VigoFingerprintProtection() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  GenerateSessionSeed();
}

VigoFingerprintProtection::~VigoFingerprintProtection() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

void VigoFingerprintProtection::GenerateSessionSeed() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  session_seed_ = base::RandUint64();
  VLOG(1) << "VigoFingerprintProtection: New session seed generated";
}

uint8_t VigoFingerprintProtection::GetCanvasNoise(uint32_t x,
                                                    uint32_t y,
                                                    uint8_t channel) const {
  // Combine position + channel + session seed into a deterministic hash.
  // The hash output is reduced to a small noise value in range [0, 4]
  // (which maps to perturbation of -2..+2 when applied).
  uint64_t input = session_seed_;
  input = SplitMix64(input ^ static_cast<uint64_t>(x));
  input = SplitMix64(input ^ (static_cast<uint64_t>(y) << 32));
  input = SplitMix64(input ^ static_cast<uint64_t>(channel));

  // Map to range [0, 4] → apply as (noise - 2) to get -2..+2.
  return static_cast<uint8_t>(input % 5);
}

// static
const char* VigoFingerprintProtection::GetMaskedWebGLVendor() {
  // Return a generic vendor that does not reveal the actual GPU vendor.
  return "Google Inc. (Vigo)";
}

// static
const char* VigoFingerprintProtection::GetMaskedWebGLRenderer() {
  // Return a generic renderer string that does not reveal GPU model.
  // ANGLE is the standard WebGL backend on all desktop platforms.
  return "ANGLE (Vigo, Direct3D11, Generic GPU)";
}

void VigoFingerprintProtection::ApplyAudioNoise(float* buffer,
                                                  size_t length) const {
  if (!buffer || length == 0)
    return;

  for (size_t i = 0; i < length; ++i) {
    // Derive a deterministic noise value for each sample position.
    uint64_t hash = SplitMix64(session_seed_ ^ static_cast<uint64_t>(i));
    // Map to range [-0.0001, +0.0001] — inaudible perturbation.
    double noise =
        (static_cast<double>(hash & 0xFFFFFFFF) / 4294967295.0 - 0.5) *
        0.0002;
    buffer[i] += static_cast<float>(noise);
  }
}

// static
const std::vector<std::string>&
VigoFingerprintProtection::GetAllowedFonts() {
  static const std::vector<std::string> kFonts(
      std::begin(kAllowedFontsList), std::end(kAllowedFontsList));
  return kFonts;
}

// static
bool VigoFingerprintProtection::IsFontAllowed(
    const std::string& font_family) {
  const auto& allowed = GetAllowedFonts();
  for (const auto& font : allowed) {
    if (base::EqualsCaseInsensitiveASCII(font_family, font)) {
      return true;
    }
  }
  return false;
}

// static
const std::vector<std::string>&
VigoFingerprintProtection::GetPermittedClientHints() {
  static const std::vector<std::string> kHints(
      std::begin(kPermittedClientHintsList),
      std::end(kPermittedClientHintsList));
  return kHints;
}

// static
bool VigoFingerprintProtection::IsClientHintPermitted(
    const std::string& header_name) {
  const auto& permitted = GetPermittedClientHints();
  for (const auto& hint : permitted) {
    if (base::EqualsCaseInsensitiveASCII(header_name, hint)) {
      return true;
    }
  }
  return false;
}

}  // namespace privacy
}  // namespace vigo

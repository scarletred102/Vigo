// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/security/vigo_license_key.h"

#include "base/logging.h"
#include "base/strings/string_util.h"
#include "base/time/time.h"
#include "build/build_config.h"
#include "crypto/sha2.h"
#include "third_party/re2/src/re2/re2.h"

#if BUILDFLAG(IS_WIN)
#include <windows.h>
#endif

namespace vigo {
namespace security {

namespace {

// Beta end date: 2026-01-01 (generous beta window).
constexpr int kBetaEndYear = 2026;
constexpr int kBetaEndMonth = 1;
constexpr int kBetaEndDay = 1;

// Features that require a purchased license (after beta expires).
constexpr const char* kPaidFeatures[] = {
    "sync",
    "credential_vault",
    "advanced_privacy",
    "extension_platform",
};

constexpr size_t kPaidFeaturesCount =
    sizeof(kPaidFeatures) / sizeof(kPaidFeatures[0]);

}  // namespace

VigoLicenseKey::VigoLicenseKey() {
  DETACH_FROM_SEQUENCE(sequence_checker_);

  // Default to beta license.
  current_license_.valid = true;
  current_license_.tier = LicenseTier::kBeta;

  base::Time::Exploded beta_end = {};
  beta_end.year = kBetaEndYear;
  beta_end.month = kBetaEndMonth;
  beta_end.day_of_month = kBetaEndDay;
  base::Time::FromUTCExploded(beta_end, &current_license_.expires);
}

VigoLicenseKey::~VigoLicenseKey() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

// static
bool VigoLicenseKey::IsValidFormat(const std::string& key) {
  // VIGO-XXXXX-XXXXX-XXXXX-XXXXX
  static const re2::RE2 kPattern(
      R"(^VIGO-[A-Z0-9]{5}-[A-Z0-9]{5}-[A-Z0-9]{5}-[A-Z0-9]{5}$)");
  return re2::RE2::FullMatch(key, kPattern);
}

// static
std::string VigoLicenseKey::MaskKey(const std::string& key) {
  if (key.size() < 10) {
    return "INVALID";
  }
  // Show first 5 and last 5 characters.
  return key.substr(0, 5) + "-*****-*****-*****-" +
         key.substr(key.size() - 5);
}

// static
std::string VigoLicenseKey::GenerateHardwareFingerprint() {
  // Collect machine-specific identifiers and hash them.
  // This is intentionally coarse-grained to avoid false negatives
  // when users upgrade RAM or swap a disk.
  std::string raw;

#if BUILDFLAG(IS_WIN)
  // Use volume serial number + processor info as basic fingerprint.
  DWORD serial = 0;
  if (GetVolumeInformationW(L"C:\\", nullptr, 0, &serial,
                             nullptr, nullptr, nullptr, 0)) {
    raw += std::to_string(serial);
  }

  SYSTEM_INFO si;
  GetSystemInfo(&si);
  raw += std::to_string(si.dwProcessorType);
  raw += std::to_string(si.dwNumberOfProcessors);
#elif BUILDFLAG(IS_MAC)
  // macOS: use IOPlatformSerialNumber via system_profiler.
  raw += "mac-placeholder";
#elif BUILDFLAG(IS_LINUX)
  // Linux: /etc/machine-id.
  raw += "linux-placeholder";
#endif

  if (raw.empty()) {
    raw = "unknown-machine";
  }

  return crypto::SHA256HashString(raw);
}

LicenseInfo VigoLicenseKey::ValidateOffline(const std::string& key) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  LicenseInfo info;
  info.license_key = key;

  // Step 1: Format check.
  if (!IsValidFormat(key)) {
    info.valid = false;
    VLOG(1) << "VigoLicenseKey: Invalid format: " << MaskKey(key);
    return info;
  }

  // Step 2: Signature verification.
  if (!VerifyKeySignature(key)) {
    info.valid = false;
    VLOG(1) << "VigoLicenseKey: Signature verification failed: "
            << MaskKey(key);
    return info;
  }

  // Step 3: Hardware binding.
  info.hardware_fingerprint = GenerateHardwareFingerprint();

  info.valid = true;
  info.tier = LicenseTier::kPurchased;

  VLOG(1) << "VigoLicenseKey: Offline validation passed for " << MaskKey(key);
  return info;
}

void VigoLicenseKey::ValidateOnline(const std::string& key,
                                     ValidationCallback callback) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  // First do offline validation.
  LicenseInfo info = ValidateOffline(key);
  if (!info.valid) {
    std::move(callback).Run(info);
    return;
  }

  // Then do online activation.
  PerformOnlineActivation(key, std::move(callback));
}

LicenseInfo VigoLicenseKey::GetCurrentLicense() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return current_license_;
}

void VigoLicenseKey::StoreLicense(const LicenseInfo& info) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  current_license_ = info;
  VLOG(1) << "VigoLicenseKey: License stored — tier="
          << static_cast<int>(info.tier);

  // TODO(Phase 6): Persist to encrypted storage via credential vault.
}

void VigoLicenseKey::ClearLicense() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  current_license_ = LicenseInfo{};
  current_license_.valid = true;
  current_license_.tier = LicenseTier::kBeta;
  VLOG(1) << "VigoLicenseKey: License cleared, reverted to beta";
}

bool VigoLicenseKey::IsFeatureAllowed(const std::string& feature_name) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  // During beta, everything is allowed.
  if (current_license_.tier == LicenseTier::kBeta && !IsBetaExpired()) {
    return true;
  }

  // Purchased license: everything allowed.
  if (current_license_.tier == LicenseTier::kPurchased) {
    return true;
  }

  // Expired beta: check if it's a paid feature.
  for (size_t i = 0; i < kPaidFeaturesCount; ++i) {
    if (feature_name == kPaidFeatures[i]) {
      return false;  // Requires purchase.
    }
  }

  // Core browsing features are always available.
  return true;
}

bool VigoLicenseKey::IsBeta() const {
  return current_license_.tier == LicenseTier::kBeta;
}

bool VigoLicenseKey::IsBetaExpired() const {
  if (current_license_.tier != LicenseTier::kBeta) {
    return false;
  }
  return base::Time::Now() > current_license_.expires;
}

base::Time VigoLicenseKey::GetBetaEndDate() const {
  return current_license_.expires;
}

void VigoLicenseKey::SetLicenseForTesting(const LicenseInfo& info) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  current_license_ = info;
}

bool VigoLicenseKey::VerifyKeySignature(const std::string& key) const {
  // TODO(Phase 6): Implement Ed25519 signature verification.
  // The key encodes a payload + signature:
  //   payload = base32(tier | issued_ts | expiry_ts | max_activations)
  //   signature = Ed25519(kLicensePublicKey, payload)
  //
  // For now, accept any correctly formatted key during development.
  return IsValidFormat(key);
}

bool VigoLicenseKey::CheckHardwareBinding(const LicenseInfo& info) const {
  std::string current_fp = GenerateHardwareFingerprint();
  return current_fp == info.hardware_fingerprint;
}

void VigoLicenseKey::PerformOnlineActivation(const std::string& key,
                                              ValidationCallback callback) {
  // TODO(Phase 6): HTTP POST to license_server_url_ with:
  //   { "key": key, "hardware_fingerprint": fp, "version": version }
  // Response: { "valid": true, "tier": "purchased", "activations": 1, ... }
  //
  // For now, simulate success.
  LicenseInfo info;
  info.valid = true;
  info.tier = LicenseTier::kPurchased;
  info.license_key = key;
  info.hardware_fingerprint = GenerateHardwareFingerprint();
  info.issued = base::Time::Now();
  info.max_activations = 3;
  info.current_activations = 1;

  std::move(callback).Run(info);
}

void VigoLicenseKey::OnActivationResponse(ValidationCallback callback,
                                            const std::string& response_body) {
  // TODO(Phase 6): Parse JSON response and populate LicenseInfo.
  LicenseInfo info;
  info.valid = false;
  std::move(callback).Run(info);
}

}  // namespace security
}  // namespace vigo

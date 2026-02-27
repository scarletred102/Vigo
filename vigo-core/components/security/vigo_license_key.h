// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_SECURITY_VIGO_LICENSE_KEY_H_
#define VIGO_COMPONENTS_SECURITY_VIGO_LICENSE_KEY_H_

#include <cstdint>
#include <optional>
#include <string>
#include <vector>

#include "base/functional/callback.h"
#include "base/memory/weak_ptr.h"
#include "base/sequence_checker.h"
#include "base/time/time.h"

namespace vigo {
namespace security {

// License tier — determines which features are available.
enum class LicenseTier {
  // Beta / trial: all features enabled, free.
  kBeta,
  // Full purchased license ($29-39 one-time).
  kPurchased,
  // Expired beta without purchase — core browsing only (no advanced privacy,
  // no sync, no vault), with gentle conversion prompt.
  kExpired,
};

// Result of license validation.
struct LicenseInfo {
  bool valid = false;
  LicenseTier tier = LicenseTier::kBeta;
  std::string license_key;           // The key itself (masked in logs).
  std::string hardware_fingerprint;  // Machine binding hash.
  base::Time issued;                 // When the key was issued.
  base::Time expires;                // For beta: beta end date.
  int max_activations = 3;           // Max machines per key.
  int current_activations = 0;
};

// VigolicenseKey handles the one-time purchase license validation for Vigo.
//
// Key format: VIGO-XXXXX-XXXXX-XXXXX-XXXXX (20 alphanumeric, 4 groups)
//
// Validation:
//   1. Format check (regex: VIGO-[A-Z0-9]{5}-...x4)
//   2. Ed25519 signature verification (embedded in the key payload)
//   3. Hardware fingerprint binding (SHA-256 of machine identifiers)
//   4. Online activation check (rate-limited, with offline grace period)
//
// Security model:
//   - License keys are cryptographically signed by the Vigo license server
//   - Each key is bound to ≤3 hardware fingerprints
//   - Beta keys auto-expire at the beta end date
//   - No DRM, no phone-home surveillance — honest users get full access;
//     piracy is accepted as a cost of doing business for a solo developer
class VigoLicenseKey {
 public:
  using ValidationCallback = base::OnceCallback<void(const LicenseInfo&)>;

  VigoLicenseKey();
  ~VigoLicenseKey();

  VigoLicenseKey(const VigoLicenseKey&) = delete;
  VigoLicenseKey& operator=(const VigoLicenseKey&) = delete;

  // Validate a license key string. Returns synchronously for format/signature
  // checks; activation check is async.
  LicenseInfo ValidateOffline(const std::string& key) const;

  // Full validation including online activation.
  void ValidateOnline(const std::string& key, ValidationCallback callback);

  // Get the current license status (from stored/cached state).
  LicenseInfo GetCurrentLicense() const;

  // Store a validated license key.
  void StoreLicense(const LicenseInfo& info);

  // Clear the stored license (for testing / deactivation).
  void ClearLicense();

  // Check if the current license allows a given feature.
  bool IsFeatureAllowed(const std::string& feature_name) const;

  // Generate a hardware fingerprint for the current machine.
  static std::string GenerateHardwareFingerprint();

  // Beta check.
  bool IsBeta() const;
  bool IsBetaExpired() const;
  base::Time GetBetaEndDate() const;

  // Mask a license key for safe logging (VIGO-XXXXX-...-XXXXX → VIGO-*****-...-XXXXX).
  static std::string MaskKey(const std::string& key);

  // Format validation.
  static bool IsValidFormat(const std::string& key);

  // Set the license server URL (for testing).
  void set_license_server_url(const std::string& url) {
    license_server_url_ = url;
  }

  // For testing: force a specific license.
  void SetLicenseForTesting(const LicenseInfo& info);

 private:
  // Verify the Ed25519 signature embedded in the key payload.
  bool VerifyKeySignature(const std::string& key) const;

  // Check hardware binding.
  bool CheckHardwareBinding(const LicenseInfo& info) const;

  // Perform online activation with the license server.
  void PerformOnlineActivation(const std::string& key,
                                ValidationCallback callback);

  // Handle activation response.
  void OnActivationResponse(ValidationCallback callback,
                             const std::string& response_body);

  std::string license_server_url_ =
      "https://license.vigobrowser.com/api/v1/activate";

  // Ed25519 public key for license signature verification.
  // This is the LICENSE signing key — distinct from the UPDATE signing key.
  static constexpr char kLicensePublicKey[] =
      ""; // Populated during build / first release.

  LicenseInfo current_license_;

  SEQUENCE_CHECKER(sequence_checker_);
  base::WeakPtrFactory<VigoLicenseKey> weak_factory_{this};
};

}  // namespace security
}  // namespace vigo

#endif  // VIGO_COMPONENTS_SECURITY_VIGO_LICENSE_KEY_H_

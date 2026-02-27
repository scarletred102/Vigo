// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_SECURITY_VIGO_CODE_SIGNING_H_
#define VIGO_COMPONENTS_SECURITY_VIGO_CODE_SIGNING_H_

#include <string>
#include <vector>

#include "base/files/file_path.h"
#include "base/sequence_checker.h"
#include "base/values.h"

namespace vigo {
namespace security {

// Code signing verification result.
enum class SigningStatus {
  kSigned,              // Valid signature from expected signer.
  kUnsigned,            // No signature present.
  kInvalidSignature,    // Signature present but invalid.
  kExpiredCertificate,  // Certificate chain contains expired cert.
  kUntrustedRoot,       // Certificate chain root not trusted.
  kRevokedCertificate,  // Certificate has been revoked.
  kError,               // System error during verification.
};

// Represents code signing metadata for a binary.
struct SigningInfo {
  // Path to the signed binary.
  base::FilePath binary_path;

  // Signing status.
  SigningStatus status = SigningStatus::kUnsigned;

  // Subject name from the signing certificate (e.g., "Vigo Browser").
  std::string signer_name;

  // Certificate thumbprint (SHA-256 of the DER-encoded certificate).
  std::string cert_thumbprint;

  // Certificate expiry date (ISO 8601).
  std::string cert_expiry;

  // Whether the binary is timestamped (for long-term validity).
  bool is_timestamped = false;

  // Timestamp authority URL (if timestamped).
  std::string timestamp_authority;

  // Platform-specific signing details.
  std::string detail_message;
};

// Represents signing configuration for a release build.
struct SigningConfig {
  // Path to the code signing certificate / key.
  // - Windows: PFX file path or certificate store subject name.
  // - macOS: Identity name (e.g., "Developer ID Application: Vigo Browser").
  // - Linux: GPG key ID for package signing.
  std::string identity;

  // Password for certificate file (Windows PFX). Read from environment
  // variable VIGO_SIGNING_PASSWORD — never hardcoded.
  // NOTE: This field is populated at runtime from env, never persisted.
  std::string password;

  // Timestamp server URL for long-term validity.
  // Recommended: http://timestamp.digicert.com
  std::string timestamp_url;

  // SHA algorithm for signing (e.g., "sha256").
  std::string hash_algorithm = "sha256";

  // Cross-sign with SHA-1 for Windows 7 compatibility (optional).
  bool cross_sign_sha1 = false;

  // Additional files to sign (DLLs, EXEs, etc.).
  std::vector<base::FilePath> additional_files;
};

// Represents a hash entry for binary verification.
struct BinaryHashEntry {
  // Relative path from install root (e.g., "vigo.exe").
  std::string relative_path;

  // SHA-256 hash of the binary (hex-encoded).
  std::string sha256;

  // File size in bytes.
  int64_t size_bytes = 0;
};

// VigoCodeSigning manages code signing verification for Vigo binaries.
//
// Platform support:
// - Windows: Authenticode (signtool.exe) — EXE, DLL, MSI signing
// - macOS: Apple codesign + notarization (codesign + xcrun notarytool)
// - Linux: GPG detached signatures for .deb/.rpm packages
//
// This component provides:
// 1. Signing verification: check if binaries have valid signatures.
// 2. Signing configuration: encapsulate signing identity/cert info.
// 3. Binary hash manifest: generate/verify hash manifests for all
//    distributed binaries (defence against supply chain tampering).
class VigoCodeSigning {
 public:
  VigoCodeSigning();
  ~VigoCodeSigning();

  VigoCodeSigning(const VigoCodeSigning&) = delete;
  VigoCodeSigning& operator=(const VigoCodeSigning&) = delete;

  // ── Signing Verification ──────────────────────────────────────

  // Verify the code signature of a binary.
  // Platform-specific: uses WinVerifyTrust on Windows, codesign -v on macOS.
  SigningInfo VerifyBinary(const base::FilePath& binary_path) const;

  // Verify that a signed binary was signed by the expected Vigo identity.
  bool IsSignedByVigo(const base::FilePath& binary_path) const;

  // ── Binary Hash Manifest ──────────────────────────────────────

  // Generate a hash manifest for all binaries in the install directory.
  // This is used for tamper detection at startup.
  std::vector<BinaryHashEntry> GenerateHashManifest(
      const base::FilePath& install_dir) const;

  // Verify a hash manifest against actual files on disk.
  // Returns list of files that fail verification (mismatch or missing).
  std::vector<std::string> VerifyHashManifest(
      const base::FilePath& install_dir,
      const std::vector<BinaryHashEntry>& manifest) const;

  // Write hash manifest to a JSON file.
  bool WriteHashManifest(const base::FilePath& output_path,
                         const std::vector<BinaryHashEntry>& manifest) const;

  // Read hash manifest from a JSON file.
  std::vector<BinaryHashEntry> ReadHashManifest(
      const base::FilePath& input_path) const;

  // ── Signing Configuration ─────────────────────────────────────

  // Set the signing configuration for this instance.
  void SetConfig(SigningConfig config);

  // Get the current signing configuration.
  const SigningConfig& config() const { return config_; }

  // Validate that the signing configuration is complete for the
  // current platform (identity set, cert accessible, etc.).
  bool ValidateConfig() const;

  // ── Platform Helpers ──────────────────────────────────────────

  // Get the expected signer name for Vigo binaries.
  static std::string GetExpectedSignerName();

  // Generate SHA-256 hash of a file (hex-encoded).
  static std::string HashFile(const base::FilePath& file_path);

 private:
  SigningConfig config_;

  SEQUENCE_CHECKER(sequence_checker_);
};

}  // namespace security
}  // namespace vigo

#endif  // VIGO_COMPONENTS_SECURITY_VIGO_CODE_SIGNING_H_

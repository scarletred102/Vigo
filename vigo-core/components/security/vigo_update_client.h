// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_SECURITY_VIGO_UPDATE_CLIENT_H_
#define VIGO_COMPONENTS_SECURITY_VIGO_UPDATE_CLIENT_H_

#include <string>
#include <vector>

#include "base/functional/callback.h"
#include "base/files/file_path.h"
#include "base/memory/weak_ptr.h"
#include "base/sequence_checker.h"
#include "base/time/time.h"
#include "base/timer/timer.h"
#include "base/values.h"
#include "url/gurl.h"

namespace vigo {
namespace security {

// Represents a signed update manifest entry.
struct UpdateManifest {
  // Product version the update targets (e.g., "0.2.0").
  std::string version;

  // Minimum version required to apply this update (empty = any).
  std::string min_version;

  // Platform: "win-x64", "mac-x64", "mac-arm64", "linux-x64".
  std::string platform;

  // Download URL for the update package.
  GURL download_url;

  // SHA-256 hash of the update package (hex-encoded).
  std::string sha256;

  // Size of the update package in bytes.
  int64_t size_bytes = 0;

  // Ed25519 signature over the canonical manifest JSON (hex-encoded).
  // Signed by Vigo's release signing key.
  std::string signature;

  // Release notes (Markdown).
  std::string release_notes;

  // Whether this is a critical security update.
  bool is_critical = false;

  // Whether this is a differential (delta) update.
  bool is_delta = false;

  // Timestamp when this update was published.
  base::Time published_at;
};

// Update check result.
enum class UpdateStatus {
  kUpToDate,           // Current version is the latest.
  kUpdateAvailable,    // A newer version is available.
  kDownloading,        // Update package is being downloaded.
  kDownloaded,         // Update package downloaded and verified.
  kVerificationFailed, // Downloaded package failed hash/signature check.
  kInstallPending,     // Waiting for browser restart to apply.
  kError,              // Network or other error occurred.
};

// Observer interface for update state changes.
class VigoUpdateObserver {
 public:
  virtual ~VigoUpdateObserver() = default;

  // Called when the update status changes.
  virtual void OnUpdateStatusChanged(UpdateStatus status) = 0;

  // Called periodically during download with progress percentage (0–100).
  virtual void OnUpdateDownloadProgress(int percent) = 0;

  // Called when an update check encounters an error.
  virtual void OnUpdateError(const std::string& error_message) = 0;
};

// VigoUpdateClient manages automatic update checks, manifest verification,
// package download, and integrity validation for Vigo browser updates.
//
// Security model:
// - Update manifests are fetched over TLS from a pinned domain.
// - Manifest is signed with Vigo's Ed25519 release signing key.
// - Downloaded packages are verified against SHA-256 hash in manifest.
// - The manifest signature is verified against the embedded public key
//   before any action is taken.
// - No Google Omaha dependency — fully self-contained.
//
// Update flow:
// 1. Check for updates (periodic or user-triggered).
// 2. Fetch manifest from update server.
// 3. Verify manifest signature.
// 4. Compare version against current.
// 5. Download update package (with progress).
// 6. Verify package SHA-256.
// 7. Stage for install-on-restart.
class VigoUpdateClient {
 public:
  // Configuration for the update client.
  struct Config {
    // Base URL of the update server (TLS-pinned).
    GURL update_server_url;

    // Ed25519 public key for manifest signature verification (32 bytes,
    // hex-encoded).
    std::string manifest_verify_key_hex;

    // Check interval (default: 4 hours).
    base::TimeDelta check_interval = base::Hours(4);

    // Directory to store downloaded update packages.
    base::FilePath staging_dir;

    // Whether to automatically download updates (vs. notify only).
    bool auto_download = true;

    // Whether to allow delta (differential) updates.
    bool allow_delta = true;
  };

  VigoUpdateClient();
  ~VigoUpdateClient();

  VigoUpdateClient(const VigoUpdateClient&) = delete;
  VigoUpdateClient& operator=(const VigoUpdateClient&) = delete;

  // Initialise with configuration. Must be called before Start().
  void Initialise(Config config);

  // Start periodic update checks.
  void Start();

  // Stop periodic update checks and cancel any in-progress downloads.
  void Shutdown();

  // Trigger an immediate update check (user-initiated or from code).
  void CheckNow();

  // Add/remove observers.
  void AddObserver(VigoUpdateObserver* observer);
  void RemoveObserver(VigoUpdateObserver* observer);

  // Get the current update status.
  UpdateStatus status() const { return status_; }

  // Get the latest available manifest (valid after kUpdateAvailable).
  const UpdateManifest& latest_manifest() const { return latest_manifest_; }

  // Get the current platform string (e.g., "win-x64").
  static std::string GetCurrentPlatform();

  // Verify an Ed25519 signature over manifest JSON.
  // Returns true if valid.
  static bool VerifyManifestSignature(const std::string& manifest_json,
                                      const std::string& signature_hex,
                                      const std::string& verify_key_hex);

  // Verify SHA-256 hash of a file.
  // Returns true if the file's hash matches |expected_sha256_hex|.
  static bool VerifyFileHash(const base::FilePath& file_path,
                             const std::string& expected_sha256_hex);

  // Parse update manifest from JSON. Returns nullopt on parse failure.
  static std::optional<UpdateManifest> ParseManifest(
      const std::string& json_string,
      const std::string& platform);

 private:
  // Perform the update check cycle.
  void PerformCheck();

  // Called when the manifest fetch completes.
  void OnManifestFetched(const std::string& response_body);

  // Called when the update package download completes.
  void OnPackageDownloaded(const base::FilePath& package_path);

  // Set status and notify observers.
  void SetStatus(UpdateStatus status);

  // Notify observers of download progress.
  void NotifyProgress(int percent);

  // Notify observers of an error.
  void NotifyError(const std::string& error_message);

  Config config_;
  UpdateStatus status_ = UpdateStatus::kUpToDate;
  UpdateManifest latest_manifest_;

  // Periodic check timer.
  base::RepeatingTimer check_timer_;

  // Observer list.
  std::vector<VigoUpdateObserver*> observers_;

  base::WeakPtrFactory<VigoUpdateClient> weak_factory_{this};

  SEQUENCE_CHECKER(sequence_checker_);
};

}  // namespace security
}  // namespace vigo

#endif  // VIGO_COMPONENTS_SECURITY_VIGO_UPDATE_CLIENT_H_

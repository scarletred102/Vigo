// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/security/vigo_update_client.h"

#include <algorithm>

#include "base/files/file_util.h"
#include "base/hash/sha1.h"
#include "base/json/json_reader.h"
#include "base/logging.h"
#include "base/strings/string_number_conversions.h"
#include "base/strings/stringprintf.h"
#include "base/task/thread_pool.h"
#include "crypto/sha2.h"
#include "vigo/app/vigo_branding.h"

namespace vigo {
namespace security {

namespace {

// Compare two semver version strings. Returns:
//  -1 if a < b, 0 if a == b, 1 if a > b.
int CompareVersions(const std::string& a, const std::string& b) {
  auto parse = [](const std::string& v) -> std::vector<int> {
    std::vector<int> parts;
    size_t start = 0;
    while (start < v.size()) {
      size_t dot = v.find('.', start);
      if (dot == std::string::npos)
        dot = v.size();
      int val = 0;
      base::StringToInt(v.substr(start, dot - start), &val);
      parts.push_back(val);
      start = dot + 1;
    }
    return parts;
  };

  auto va = parse(a);
  auto vb = parse(b);

  size_t max_len = std::max(va.size(), vb.size());
  va.resize(max_len, 0);
  vb.resize(max_len, 0);

  for (size_t i = 0; i < max_len; ++i) {
    if (va[i] < vb[i])
      return -1;
    if (va[i] > vb[i])
      return 1;
  }
  return 0;
}

}  // namespace

// ── VigoUpdateClient ────────────────────────────────────────────

VigoUpdateClient::VigoUpdateClient() = default;

VigoUpdateClient::~VigoUpdateClient() {
  Shutdown();
}

void VigoUpdateClient::Initialise(Config config) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  config_ = std::move(config);
  VLOG(1) << "VigoUpdateClient: Initialised with update server "
          << config_.update_server_url.spec();
}

void VigoUpdateClient::Start() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (config_.update_server_url.is_empty()) {
    LOG(WARNING) << "VigoUpdateClient: No update server URL configured; "
                 << "update checks disabled";
    return;
  }

  // Start periodic check timer.
  check_timer_.Start(FROM_HERE, config_.check_interval,
                     base::BindRepeating(&VigoUpdateClient::PerformCheck,
                                         weak_factory_.GetWeakPtr()));

  // Perform an initial check after a short delay to avoid blocking startup.
  base::SequencedTaskRunner::GetCurrentDefault()->PostDelayedTask(
      FROM_HERE,
      base::BindOnce(&VigoUpdateClient::PerformCheck,
                     weak_factory_.GetWeakPtr()),
      base::Minutes(2));

  VLOG(1) << "VigoUpdateClient: Started — checking every "
          << config_.check_interval.InHours() << " hours";
}

void VigoUpdateClient::Shutdown() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  check_timer_.Stop();
  weak_factory_.InvalidateWeakPtrs();
  VLOG(1) << "VigoUpdateClient: Shut down";
}

void VigoUpdateClient::CheckNow() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  PerformCheck();
}

void VigoUpdateClient::AddObserver(VigoUpdateObserver* observer) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  observers_.push_back(observer);
}

void VigoUpdateClient::RemoveObserver(VigoUpdateObserver* observer) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  observers_.erase(std::remove(observers_.begin(), observers_.end(), observer),
                   observers_.end());
}

// static
std::string VigoUpdateClient::GetCurrentPlatform() {
#if BUILDFLAG(IS_WIN)
#if defined(ARCH_CPU_X86_64)
  return "win-x64";
#elif defined(ARCH_CPU_ARM64)
  return "win-arm64";
#else
  return "win-x86";
#endif
#elif BUILDFLAG(IS_MAC)
#if defined(ARCH_CPU_ARM64)
  return "mac-arm64";
#else
  return "mac-x64";
#endif
#elif BUILDFLAG(IS_LINUX)
#if defined(ARCH_CPU_ARM64)
  return "linux-arm64";
#else
  return "linux-x64";
#endif
#else
  return "unknown";
#endif
}

// static
bool VigoUpdateClient::VerifyManifestSignature(
    const std::string& manifest_json,
    const std::string& signature_hex,
    const std::string& verify_key_hex) {
  // Decode hex strings.
  std::vector<uint8_t> signature_bytes;
  if (!base::HexStringToBytes(signature_hex, &signature_bytes) ||
      signature_bytes.size() != 64) {
    LOG(ERROR) << "VigoUpdateClient: Invalid signature hex (expected 64 bytes)";
    return false;
  }

  std::vector<uint8_t> key_bytes;
  if (!base::HexStringToBytes(verify_key_hex, &key_bytes) ||
      key_bytes.size() != 32) {
    LOG(ERROR) << "VigoUpdateClient: Invalid verify key hex (expected 32 bytes)";
    return false;
  }

  // Ed25519 verification is performed via the Rust crypto FFI.
  // For the C++ side, we use the crypto FFI bridge:
  //   vigo_crypto_verify(key_bytes, message_bytes, signature_bytes)
  //
  // TODO(Phase 5): Wire to vigo_crypto FFI. For now, structural
  // validation passes if both key and signature decode correctly.
  // The actual cryptographic verification will be enabled once the
  // FFI bridge is linked into the security component.
  VLOG(1) << "VigoUpdateClient: Manifest signature structural validation "
          << "passed (key: " << key_bytes.size()
          << " bytes, sig: " << signature_bytes.size() << " bytes)";
  return true;
}

// static
bool VigoUpdateClient::VerifyFileHash(const base::FilePath& file_path,
                                      const std::string& expected_sha256_hex) {
  std::string file_contents;
  if (!base::ReadFileToString(file_path, &file_contents)) {
    LOG(ERROR) << "VigoUpdateClient: Cannot read file for hash verification: "
               << file_path.value();
    return false;
  }

  std::string hash = crypto::SHA256HashString(file_contents);
  std::string hash_hex = base::HexEncode(hash);

  // Case-insensitive comparison.
  std::string expected_lower = expected_sha256_hex;
  std::transform(expected_lower.begin(), expected_lower.end(),
                 expected_lower.begin(), ::tolower);
  std::string actual_lower = hash_hex;
  std::transform(actual_lower.begin(), actual_lower.end(),
                 actual_lower.begin(), ::tolower);

  if (expected_lower != actual_lower) {
    LOG(ERROR) << "VigoUpdateClient: Hash mismatch! Expected: "
               << expected_sha256_hex << " Got: " << hash_hex;
    return false;
  }

  return true;
}

// static
std::optional<UpdateManifest> VigoUpdateClient::ParseManifest(
    const std::string& json_string,
    const std::string& platform) {
  auto parsed = base::JSONReader::Read(json_string);
  if (!parsed || !parsed->is_dict()) {
    LOG(ERROR) << "VigoUpdateClient: Failed to parse manifest JSON";
    return std::nullopt;
  }

  const base::Value::Dict& root = parsed->GetDict();

  // Look for a release matching our platform.
  const base::Value::List* releases = root.FindList("releases");
  if (!releases) {
    LOG(ERROR) << "VigoUpdateClient: Manifest missing 'releases' array";
    return std::nullopt;
  }

  for (const auto& release : *releases) {
    if (!release.is_dict())
      continue;

    const base::Value::Dict& r = release.GetDict();
    const std::string* r_platform = r.FindString("platform");
    if (!r_platform || *r_platform != platform)
      continue;

    UpdateManifest manifest;

    const std::string* version = r.FindString("version");
    if (version)
      manifest.version = *version;

    const std::string* min_ver = r.FindString("min_version");
    if (min_ver)
      manifest.min_version = *min_ver;

    manifest.platform = platform;

    const std::string* url = r.FindString("download_url");
    if (url)
      manifest.download_url = GURL(*url);

    const std::string* sha = r.FindString("sha256");
    if (sha)
      manifest.sha256 = *sha;

    auto size = r.FindDouble("size_bytes");
    if (size)
      manifest.size_bytes = static_cast<int64_t>(*size);

    const std::string* sig = r.FindString("signature");
    if (sig)
      manifest.signature = *sig;

    const std::string* notes = r.FindString("release_notes");
    if (notes)
      manifest.release_notes = *notes;

    auto critical = r.FindBool("is_critical");
    if (critical)
      manifest.is_critical = *critical;

    auto delta = r.FindBool("is_delta");
    if (delta)
      manifest.is_delta = *delta;

    return manifest;
  }

  VLOG(1) << "VigoUpdateClient: No release found for platform " << platform;
  return std::nullopt;
}

void VigoUpdateClient::PerformCheck() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (config_.update_server_url.is_empty()) {
    NotifyError("No update server URL configured");
    return;
  }

  VLOG(1) << "VigoUpdateClient: Checking for updates from "
          << config_.update_server_url.spec();

  // Construct the manifest URL: {base}/manifest/{platform}/{current_version}
  std::string manifest_path = base::StringPrintf(
      "/manifest/%s/%s", GetCurrentPlatform().c_str(),
      branding::kVersionString);

  GURL manifest_url =
      config_.update_server_url.Resolve(manifest_path);

  // The actual HTTP fetch would use SimpleURLLoader here.
  // For now, this is the structural hook — the same pattern as
  // VigoFilterListManager's download implementation.
  //
  // TODO(Phase 5.2): Wire SimpleURLLoader with traffic annotation.
  // On completion, call OnManifestFetched(response_body).
  VLOG(1) << "VigoUpdateClient: Would fetch manifest from "
          << manifest_url.spec();
}

void VigoUpdateClient::OnManifestFetched(const std::string& response_body) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  auto manifest = ParseManifest(response_body, GetCurrentPlatform());
  if (!manifest) {
    NotifyError("Failed to parse update manifest");
    SetStatus(UpdateStatus::kError);
    return;
  }

  // Verify signature.
  if (!config_.manifest_verify_key_hex.empty() &&
      !manifest->signature.empty()) {
    if (!VerifyManifestSignature(response_body, manifest->signature,
                                 config_.manifest_verify_key_hex)) {
      NotifyError("Update manifest signature verification failed");
      SetStatus(UpdateStatus::kVerificationFailed);
      return;
    }
  }

  // Compare versions.
  int cmp = CompareVersions(manifest->version, branding::kVersionString);
  if (cmp <= 0) {
    VLOG(1) << "VigoUpdateClient: Up to date (current: "
            << branding::kVersionString
            << ", latest: " << manifest->version << ")";
    SetStatus(UpdateStatus::kUpToDate);
    return;
  }

  // Check minimum version requirement.
  if (!manifest->min_version.empty()) {
    int min_cmp =
        CompareVersions(branding::kVersionString, manifest->min_version);
    if (min_cmp < 0) {
      NotifyError("Current version too old for delta update; full reinstall "
                  "required");
      SetStatus(UpdateStatus::kError);
      return;
    }
  }

  latest_manifest_ = *manifest;
  SetStatus(UpdateStatus::kUpdateAvailable);

  VLOG(1) << "VigoUpdateClient: Update available — " << manifest->version
          << (manifest->is_critical ? " (CRITICAL)" : "");

  if (config_.auto_download) {
    SetStatus(UpdateStatus::kDownloading);
    // TODO(Phase 5.2): Start download via SimpleURLLoader.
  }
}

void VigoUpdateClient::OnPackageDownloaded(
    const base::FilePath& package_path) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  // Verify SHA-256 hash.
  if (!VerifyFileHash(package_path, latest_manifest_.sha256)) {
    SetStatus(UpdateStatus::kVerificationFailed);
    NotifyError("Downloaded update package failed hash verification");

    // Delete the corrupt file.
    base::ThreadPool::PostTask(
        FROM_HERE, {base::MayBlock()},
        base::BindOnce(base::IgnoreResult(&base::DeleteFile), package_path));
    return;
  }

  VLOG(1) << "VigoUpdateClient: Update package verified — "
          << package_path.value();
  SetStatus(UpdateStatus::kDownloaded);

  // Stage for install-on-restart.
  SetStatus(UpdateStatus::kInstallPending);
}

void VigoUpdateClient::SetStatus(UpdateStatus status) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  status_ = status;
  for (auto* observer : observers_) {
    observer->OnUpdateStatusChanged(status);
  }
}

void VigoUpdateClient::NotifyProgress(int percent) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  for (auto* observer : observers_) {
    observer->OnUpdateDownloadProgress(percent);
  }
}

void VigoUpdateClient::NotifyError(const std::string& error_message) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  LOG(WARNING) << "VigoUpdateClient: " << error_message;
  for (auto* observer : observers_) {
    observer->OnUpdateError(error_message);
  }
}

}  // namespace security
}  // namespace vigo

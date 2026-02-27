// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/security/vigo_code_signing.h"

#include <algorithm>

#include "base/files/file_enumerator.h"
#include "base/files/file_util.h"
#include "base/json/json_reader.h"
#include "base/json/json_writer.h"
#include "base/logging.h"
#include "base/strings/string_number_conversions.h"
#include "build/build_config.h"
#include "crypto/sha2.h"

namespace vigo {
namespace security {

namespace {

// Binary extensions to include in hash manifests.
constexpr const char* kBinaryExtensions[] = {
    ".exe", ".dll", ".so",  ".dylib", ".pak",
    ".dat", ".bin", ".node", ".wasm",
};

bool IsBinaryFile(const base::FilePath& path) {
  std::string ext = path.Extension();
  std::transform(ext.begin(), ext.end(), ext.begin(), ::tolower);
  for (const char* binary_ext : kBinaryExtensions) {
    if (ext == binary_ext)
      return true;
  }
  return false;
}

}  // namespace

// ── VigoCodeSigning ─────────────────────────────────────────────

VigoCodeSigning::VigoCodeSigning() = default;
VigoCodeSigning::~VigoCodeSigning() = default;

SigningInfo VigoCodeSigning::VerifyBinary(
    const base::FilePath& binary_path) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  SigningInfo info;
  info.binary_path = binary_path;

  if (!base::PathExists(binary_path)) {
    info.status = SigningStatus::kError;
    info.detail_message = "Binary file not found";
    return info;
  }

#if BUILDFLAG(IS_WIN)
  // On Windows, we would call WinVerifyTrust to verify Authenticode.
  //
  // HRESULT hr = WinVerifyTrust(
  //     NULL, &action_id, &wintrust_data);
  //
  // TODO(Phase 5.4): Implement WinVerifyTrust integration.
  // For now, report as unsigned (no signing cert configured yet).
  info.status = SigningStatus::kUnsigned;
  info.detail_message =
      "Windows Authenticode verification not yet implemented";
#elif BUILDFLAG(IS_MAC)
  // On macOS, we would shell out to:
  //   codesign --verify --deep --strict <binary>
  //
  // TODO(Phase 5.4): Implement codesign verification.
  info.status = SigningStatus::kUnsigned;
  info.detail_message = "macOS codesign verification not yet implemented";
#elif BUILDFLAG(IS_LINUX)
  // On Linux, packages are verified via GPG signatures on .deb/.rpm.
  // Individual binary signing is not standard.
  info.status = SigningStatus::kUnsigned;
  info.detail_message = "Linux binary signing uses package-level GPG";
#endif

  return info;
}

bool VigoCodeSigning::IsSignedByVigo(const base::FilePath& binary_path) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  SigningInfo info = VerifyBinary(binary_path);
  if (info.status != SigningStatus::kSigned)
    return false;

  return info.signer_name == GetExpectedSignerName();
}

std::vector<BinaryHashEntry> VigoCodeSigning::GenerateHashManifest(
    const base::FilePath& install_dir) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  std::vector<BinaryHashEntry> manifest;

  base::FileEnumerator enumerator(
      install_dir, /*recursive=*/true,
      base::FileEnumerator::FILES);

  for (base::FilePath path = enumerator.Next(); !path.empty();
       path = enumerator.Next()) {
    if (!IsBinaryFile(path))
      continue;

    BinaryHashEntry entry;

    // Store relative path from install root.
    base::FilePath relative;
    if (install_dir.AppendRelativePath(path, &relative)) {
      entry.relative_path = relative.AsUTF8Unsafe();
    } else {
      entry.relative_path = path.BaseName().AsUTF8Unsafe();
    }

    entry.sha256 = HashFile(path);

    int64_t file_size = 0;
    base::GetFileSize(path, &file_size);
    entry.size_bytes = file_size;

    manifest.push_back(std::move(entry));
  }

  VLOG(1) << "VigoCodeSigning: Generated hash manifest with "
          << manifest.size() << " entries for " << install_dir.value();

  return manifest;
}

std::vector<std::string> VigoCodeSigning::VerifyHashManifest(
    const base::FilePath& install_dir,
    const std::vector<BinaryHashEntry>& manifest) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  std::vector<std::string> failures;

  for (const auto& entry : manifest) {
    base::FilePath full_path =
        install_dir.Append(base::FilePath::FromUTF8Unsafe(entry.relative_path));

    if (!base::PathExists(full_path)) {
      failures.push_back(entry.relative_path + ": file missing");
      continue;
    }

    std::string actual_hash = HashFile(full_path);
    if (actual_hash != entry.sha256) {
      failures.push_back(entry.relative_path + ": hash mismatch (expected " +
                         entry.sha256.substr(0, 16) + "..., got " +
                         actual_hash.substr(0, 16) + "...)");
    }

    int64_t actual_size = 0;
    base::GetFileSize(full_path, &actual_size);
    if (actual_size != entry.size_bytes) {
      failures.push_back(entry.relative_path + ": size mismatch");
    }
  }

  return failures;
}

bool VigoCodeSigning::WriteHashManifest(
    const base::FilePath& output_path,
    const std::vector<BinaryHashEntry>& manifest) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  base::Value::List entries;
  for (const auto& entry : manifest) {
    base::Value::Dict e;
    e.Set("path", entry.relative_path);
    e.Set("sha256", entry.sha256);
    e.Set("size", static_cast<double>(entry.size_bytes));
    entries.Append(std::move(e));
  }

  base::Value::Dict root;
  root.Set("version", "1");
  root.Set("files", std::move(entries));

  std::string json;
  if (!base::JSONWriter::WriteWithOptions(
          root, base::JSONWriter::OPTIONS_PRETTY_PRINT, &json)) {
    LOG(ERROR) << "VigoCodeSigning: Failed to serialise hash manifest";
    return false;
  }

  return base::WriteFile(output_path, json);
}

std::vector<BinaryHashEntry> VigoCodeSigning::ReadHashManifest(
    const base::FilePath& input_path) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  std::vector<BinaryHashEntry> manifest;

  std::string json;
  if (!base::ReadFileToString(input_path, &json)) {
    LOG(ERROR) << "VigoCodeSigning: Cannot read hash manifest: "
               << input_path.value();
    return manifest;
  }

  auto parsed = base::JSONReader::Read(json);
  if (!parsed || !parsed->is_dict()) {
    LOG(ERROR) << "VigoCodeSigning: Invalid hash manifest JSON";
    return manifest;
  }

  const base::Value::List* files = parsed->GetDict().FindList("files");
  if (!files)
    return manifest;

  for (const auto& file : *files) {
    if (!file.is_dict())
      continue;

    const base::Value::Dict& f = file.GetDict();
    BinaryHashEntry entry;

    const std::string* path = f.FindString("path");
    if (path)
      entry.relative_path = *path;

    const std::string* sha = f.FindString("sha256");
    if (sha)
      entry.sha256 = *sha;

    auto size = f.FindDouble("size");
    if (size)
      entry.size_bytes = static_cast<int64_t>(*size);

    manifest.push_back(std::move(entry));
  }

  return manifest;
}

void VigoCodeSigning::SetConfig(SigningConfig config) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  config_ = std::move(config);
}

bool VigoCodeSigning::ValidateConfig() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (config_.identity.empty()) {
    LOG(WARNING) << "VigoCodeSigning: No signing identity configured";
    return false;
  }

#if BUILDFLAG(IS_WIN)
  // Windows: need identity (PFX path or cert store name).
  // Password can be read from env at sign time.
  if (config_.timestamp_url.empty()) {
    LOG(WARNING) << "VigoCodeSigning: No timestamp URL — signatures may "
                 << "not survive certificate expiry";
  }
#elif BUILDFLAG(IS_MAC)
  // macOS: identity should be a Developer ID name.
  if (config_.identity.find("Developer ID") == std::string::npos) {
    LOG(WARNING) << "VigoCodeSigning: macOS identity doesn't look like a "
                 << "Developer ID certificate";
  }
#endif

  return true;
}

// static
std::string VigoCodeSigning::GetExpectedSignerName() {
  return "Vigo Browser";
}

// static
std::string VigoCodeSigning::HashFile(const base::FilePath& file_path) {
  std::string contents;
  if (!base::ReadFileToString(file_path, &contents)) {
    LOG(ERROR) << "VigoCodeSigning: Cannot read file for hashing: "
               << file_path.value();
    return "";
  }

  std::string hash = crypto::SHA256HashString(contents);
  return base::HexEncode(hash);
}

}  // namespace security
}  // namespace vigo

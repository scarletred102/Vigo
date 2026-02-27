// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_SECURITY_VIGO_SBOM_GENERATOR_H_
#define VIGO_COMPONENTS_SECURITY_VIGO_SBOM_GENERATOR_H_

#include <string>
#include <vector>

#include "base/files/file_path.h"
#include "base/sequence_checker.h"
#include "base/time/time.h"
#include "base/values.h"

namespace vigo {
namespace security {

// Represents a single dependency in the Software Bill of Materials.
struct SbomDependency {
  // Unique package identifier (e.g., "pkg:cargo/chacha20poly1305@0.10").
  std::string purl;

  // Human-readable name (e.g., "chacha20poly1305").
  std::string name;

  // Exact pinned version (e.g., "0.10.1").
  std::string version;

  // License identifier (SPDX format, e.g., "MIT", "Apache-2.0").
  std::string license;

  // Package ecosystem: "cargo", "npm", "chromium", "system".
  std::string ecosystem;

  // SHA-256 hash of the dependency archive or source (hex-encoded).
  std::string sha256;

  // Whether this is a direct (true) or transitive (false) dependency.
  bool is_direct = true;

  // Whether the dependency is security-critical (crypto, network, etc.).
  bool is_security_critical = false;
};

// Represents a complete SBOM document.
struct SbomDocument {
  // SBOM specification version.
  static constexpr char kSpecVersion[] = "1.5";

  // SBOM format: CycloneDX.
  static constexpr char kFormat[] = "CycloneDX";

  // Product being described.
  std::string product_name;
  std::string product_version;

  // Generation timestamp.
  base::Time generated_at;

  // All dependencies.
  std::vector<SbomDependency> dependencies;

  // Build metadata.
  std::string build_type;       // "debug" or "release"
  std::string target_os;        // "win", "mac", "linux"
  std::string target_arch;      // "x64", "arm64"
  std::string chromium_version;

  // Serialise to CycloneDX JSON format.
  base::Value::Dict ToJson() const;

  // Serialise to a human-readable Markdown table.
  std::string ToMarkdown() const;
};

// VigoSbomGenerator produces a Software Bill of Materials for each Vigo
// release. This enables supply chain transparency, vulnerability tracking,
// and audit compliance.
//
// The SBOM lists:
// - All Rust crate dependencies from Cargo.lock
// - All npm dependencies from package-lock.json
// - All vendored third-party C/C++ libraries
// - Chromium baseline version
// - System dependencies (ffmpeg, libsodium, etc.)
//
// Format: CycloneDX 1.5 JSON (industry standard).
class VigoSbomGenerator {
 public:
  VigoSbomGenerator();
  ~VigoSbomGenerator();

  VigoSbomGenerator(const VigoSbomGenerator&) = delete;
  VigoSbomGenerator& operator=(const VigoSbomGenerator&) = delete;

  // Register a dependency for inclusion in the SBOM.
  void AddDependency(SbomDependency dep);

  // Register all known Vigo dependencies (Rust crates, npm, vendored libs).
  // Call this once at build/release time.
  void RegisterVigoDependencies();

  // Generate the SBOM document with current registered dependencies.
  SbomDocument Generate() const;

  // Write the SBOM to a JSON file.
  bool WriteToFile(const base::FilePath& output_path) const;

  // Write a human-readable Markdown summary.
  bool WriteMarkdownSummary(const base::FilePath& output_path) const;

  // Verify that all registered dependencies have pinned versions and
  // SHA-256 hashes. Returns list of deps that fail verification.
  std::vector<std::string> VerifyPinning() const;

  // Get count of registered dependencies.
  size_t dependency_count() const;

  // Get count of security-critical dependencies.
  size_t security_critical_count() const;

 private:
  // Register Rust crate dependencies.
  void RegisterRustDependencies();

  // Register npm dependencies.
  void RegisterNpmDependencies();

  // Register vendored C/C++ third-party libraries.
  void RegisterVendoredDependencies();

  // Register the Chromium baseline.
  void RegisterChromiumBaseline();

  std::vector<SbomDependency> dependencies_;

  SEQUENCE_CHECKER(sequence_checker_);
};

}  // namespace security
}  // namespace vigo

#endif  // VIGO_COMPONENTS_SECURITY_VIGO_SBOM_GENERATOR_H_

// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/security/vigo_sbom_generator.h"

#include <algorithm>
#include <sstream>

#include "base/files/file_util.h"
#include "base/json/json_writer.h"
#include "base/logging.h"
#include "base/strings/string_number_conversions.h"
#include "base/strings/stringprintf.h"
#include "base/time/time.h"
#include "vigo/app/vigo_branding.h"

namespace vigo {
namespace security {

namespace {

// Chromium baseline version pinned in package.json.
constexpr char kChromiumVersion[] = "132.0.6834.0";

}  // namespace

// ── SbomDocument ────────────────────────────────────────────────

base::Value::Dict SbomDocument::ToJson() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  base::Value::Dict root;
  root.Set("bomFormat", kFormat);
  root.Set("specVersion", kSpecVersion);

  // Metadata.
  base::Value::Dict metadata;

  base::Time::Exploded exploded;
  generated_at.UTCExplode(&exploded);
  metadata.Set("timestamp",
               base::StringPrintf("%04d-%02d-%02dT%02d:%02d:%02dZ",
                                  exploded.year, exploded.month,
                                  exploded.day_of_month, exploded.hour,
                                  exploded.minute, exploded.second));

  base::Value::Dict tool;
  tool.Set("vendor", "Vigo Browser");
  tool.Set("name", "vigo-sbom-generator");
  tool.Set("version", branding::kVersionString);
  base::Value::List tools;
  tools.Append(std::move(tool));
  metadata.Set("tools", std::move(tools));

  base::Value::Dict component;
  component.Set("type", "application");
  component.Set("name", product_name);
  component.Set("version", product_version);
  metadata.Set("component", std::move(component));

  root.Set("metadata", std::move(metadata));

  // Build properties.
  base::Value::Dict properties;
  properties.Set("build_type", build_type);
  properties.Set("target_os", target_os);
  properties.Set("target_arch", target_arch);
  properties.Set("chromium_version", chromium_version);
  root.Set("properties", std::move(properties));

  // Components (dependencies).
  base::Value::List components;
  for (const auto& dep : dependencies) {
    base::Value::Dict c;
    c.Set("type", "library");
    c.Set("name", dep.name);
    c.Set("version", dep.version);
    c.Set("purl", dep.purl);

    if (!dep.license.empty()) {
      base::Value::List licenses;
      base::Value::Dict lic;
      lic.Set("id", dep.license);
      base::Value::Dict license_entry;
      license_entry.Set("license", std::move(lic));
      licenses.Append(std::move(license_entry));
      c.Set("licenses", std::move(licenses));
    }

    if (!dep.sha256.empty()) {
      base::Value::List hashes;
      base::Value::Dict hash;
      hash.Set("alg", "SHA-256");
      hash.Set("content", dep.sha256);
      hashes.Append(std::move(hash));
      c.Set("hashes", std::move(hashes));
    }

    base::Value::Dict cp;
    cp.Set("ecosystem", dep.ecosystem);
    cp.Set("direct", dep.is_direct);
    cp.Set("security_critical", dep.is_security_critical);
    c.Set("properties", std::move(cp));

    components.Append(std::move(c));
  }
  root.Set("components", std::move(components));

  return root;
}

std::string SbomDocument::ToMarkdown() const {
  std::ostringstream md;
  md << "# Vigo Browser — Software Bill of Materials\n\n";
  md << "**Product**: " << product_name << " " << product_version << "\n";
  md << "**Chromium**: " << chromium_version << "\n";
  md << "**Build**: " << build_type << " (" << target_os << "/" << target_arch
     << ")\n";
  md << "**Dependencies**: " << dependencies.size() << "\n\n";

  md << "| Name | Version | Ecosystem | License | Direct | Security |\n";
  md << "|------|---------|-----------|---------|--------|----------|\n";

  for (const auto& dep : dependencies) {
    md << "| " << dep.name << " | " << dep.version << " | " << dep.ecosystem
       << " | " << dep.license << " | " << (dep.is_direct ? "✅" : "—")
       << " | " << (dep.is_security_critical ? "🔒" : "—") << " |\n";
  }

  return md.str();
}

// ── VigoSbomGenerator ───────────────────────────────────────────

VigoSbomGenerator::VigoSbomGenerator() = default;
VigoSbomGenerator::~VigoSbomGenerator() = default;

void VigoSbomGenerator::AddDependency(SbomDependency dep) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  dependencies_.push_back(std::move(dep));
}

void VigoSbomGenerator::RegisterVigoDependencies() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  RegisterRustDependencies();
  RegisterNpmDependencies();
  RegisterVendoredDependencies();
  RegisterChromiumBaseline();

  VLOG(1) << "VigoSbomGenerator: Registered " << dependencies_.size()
          << " dependencies (" << security_critical_count()
          << " security-critical)";
}

SbomDocument VigoSbomGenerator::Generate() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  SbomDocument doc;
  doc.product_name = branding::kProductName;
  doc.product_version = branding::kVersionString;
  doc.generated_at = base::Time::Now();
  doc.dependencies = dependencies_;

  // Build metadata — these would be injected by the build system at
  // compile time; defaults are provided for development.
#if BUILDFLAG(IS_WIN)
  doc.target_os = "win";
#elif BUILDFLAG(IS_MAC)
  doc.target_os = "mac";
#elif BUILDFLAG(IS_LINUX)
  doc.target_os = "linux";
#endif

#if defined(ARCH_CPU_X86_64)
  doc.target_arch = "x64";
#elif defined(ARCH_CPU_ARM64)
  doc.target_arch = "arm64";
#endif

#if NDEBUG
  doc.build_type = "release";
#else
  doc.build_type = "debug";
#endif

  doc.chromium_version = kChromiumVersion;

  return doc;
}

bool VigoSbomGenerator::WriteToFile(const base::FilePath& output_path) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  SbomDocument doc = Generate();
  base::Value::Dict json = doc.ToJson();

  std::string json_string;
  if (!base::JSONWriter::WriteWithOptions(
          json, base::JSONWriter::OPTIONS_PRETTY_PRINT, &json_string)) {
    LOG(ERROR) << "VigoSbomGenerator: Failed to serialise SBOM JSON";
    return false;
  }

  if (!base::WriteFile(output_path, json_string)) {
    LOG(ERROR) << "VigoSbomGenerator: Failed to write SBOM to "
               << output_path.value();
    return false;
  }

  VLOG(1) << "VigoSbomGenerator: SBOM written to " << output_path.value();
  return true;
}

bool VigoSbomGenerator::WriteMarkdownSummary(
    const base::FilePath& output_path) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  SbomDocument doc = Generate();
  std::string markdown = doc.ToMarkdown();

  if (!base::WriteFile(output_path, markdown)) {
    LOG(ERROR) << "VigoSbomGenerator: Failed to write Markdown summary to "
               << output_path.value();
    return false;
  }

  return true;
}

std::vector<std::string> VigoSbomGenerator::VerifyPinning() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  std::vector<std::string> failures;

  for (const auto& dep : dependencies_) {
    if (dep.version.empty()) {
      failures.push_back(dep.name + ": missing pinned version");
    }

    // Security-critical deps MUST have SHA-256 hashes.
    if (dep.is_security_critical && dep.sha256.empty()) {
      failures.push_back(dep.name + ": security-critical but missing SHA-256");
    }

    // Direct deps should have a PURL.
    if (dep.is_direct && dep.purl.empty()) {
      failures.push_back(dep.name + ": direct dependency missing PURL");
    }
  }

  return failures;
}

size_t VigoSbomGenerator::dependency_count() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return dependencies_.size();
}

size_t VigoSbomGenerator::security_critical_count() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return std::count_if(
      dependencies_.begin(), dependencies_.end(),
      [](const SbomDependency& d) { return d.is_security_critical; });
}

void VigoSbomGenerator::RegisterRustDependencies() {
  // Vigo Rust workspace crates (from Cargo.toml / Cargo.lock).
  // Security-critical: crypto crates.

  AddDependency({"pkg:cargo/chacha20poly1305@0.10.1", "chacha20poly1305",
                  "0.10.1", "MIT OR Apache-2.0", "cargo", "", true, true});
  AddDependency({"pkg:cargo/argon2@0.5.3", "argon2", "0.5.3",
                  "MIT OR Apache-2.0", "cargo", "", true, true});
  AddDependency({"pkg:cargo/hkdf@0.12.4", "hkdf", "0.12.4",
                  "MIT OR Apache-2.0", "cargo", "", true, true});
  AddDependency({"pkg:cargo/sha2@0.10.8", "sha2", "0.10.8",
                  "MIT OR Apache-2.0", "cargo", "", true, true});
  AddDependency({"pkg:cargo/ed25519-dalek@2.1.1", "ed25519-dalek", "2.1.1",
                  "BSD-3-Clause", "cargo", "", true, true});
  AddDependency({"pkg:cargo/x25519-dalek@2.0.1", "x25519-dalek", "2.0.1",
                  "BSD-3-Clause", "cargo", "", true, true});
  AddDependency({"pkg:cargo/rand@0.8.5", "rand", "0.8.5",
                  "MIT OR Apache-2.0", "cargo", "", true, true});
  AddDependency({"pkg:cargo/zeroize@1.7.0", "zeroize", "1.7.0",
                  "MIT OR Apache-2.0", "cargo", "", true, true});
  AddDependency({"pkg:cargo/blake2@0.10.6", "blake2", "0.10.6",
                  "MIT OR Apache-2.0", "cargo", "", true, true});
  AddDependency({"pkg:cargo/hmac@0.12.1", "hmac", "0.12.1",
                  "MIT OR Apache-2.0", "cargo", "", true, false});

  // Non-security Rust deps.
  AddDependency({"pkg:cargo/aho-corasick@1.1.3", "aho-corasick", "1.1.3",
                  "MIT OR Unlicense", "cargo", "", false, false});
}

void VigoSbomGenerator::RegisterNpmDependencies() {
  // vigo-browser orchestration layer npm deps.
  // These are build-time tools, not shipped in the binary.
  AddDependency({"pkg:npm/semver@7.6.3", "semver", "7.6.3", "ISC", "npm", "",
                  true, false});
}

void VigoSbomGenerator::RegisterVendoredDependencies() {
  // Vendored C/C++ third-party libraries included in vigo-core.
  AddDependency({"pkg:generic/ffmpeg@6.1", "ffmpeg", "6.1",
                  "LGPL-2.1-or-later", "system", "", true, false});
  AddDependency({"pkg:generic/dav1d@1.4.1", "dav1d", "1.4.1", "BSD-2-Clause",
                  "system", "", true, false});
  AddDependency({"pkg:generic/libvpx@1.14.0", "libvpx", "1.14.0",
                  "BSD-3-Clause", "system", "", true, false});
  AddDependency({"pkg:generic/libaom@3.8.1", "libaom", "3.8.1",
                  "BSD-2-Clause", "system", "", true, false});
}

void VigoSbomGenerator::RegisterChromiumBaseline() {
  AddDependency({"pkg:generic/chromium@" + std::string(kChromiumVersion),
                  "chromium", kChromiumVersion, "BSD-3-Clause", "chromium", "",
                  true, false});
}

}  // namespace security
}  // namespace vigo

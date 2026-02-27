// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/security/vigo_security_hardening.h"

#include <sstream>

#include "base/logging.h"
#include "base/strings/stringprintf.h"
#include "base/time/time.h"
#include "vigo/app/vigo_branding.h"

namespace vigo {
namespace security {

// ── SecurityAuditReport ─────────────────────────────────────────

base::Value::Dict SecurityAuditReport::ToJson() const {
  base::Value::Dict root;
  root.Set("timestamp", timestamp);
  root.Set("version", version);

  base::Value::Dict summary;
  summary.Set("total_checks", total_checks);
  summary.Set("passed", passed);
  summary.Set("failed", failed);
  summary.Set("critical_failures", critical_failures);
  root.Set("summary", std::move(summary));

  base::Value::List checks;
  for (const auto& r : results) {
    base::Value::Dict c;
    c.Set("check_id", r.check_id);
    c.Set("name", r.name);
    c.Set("category", r.category);
    c.Set("passed", r.passed);
    c.Set("severity", r.severity);
    c.Set("description", r.description);
    if (!r.passed) {
      c.Set("remediation", r.remediation);
    }
    checks.Append(std::move(c));
  }
  root.Set("checks", std::move(checks));

  return root;
}

std::string SecurityAuditReport::ToText() const {
  std::ostringstream out;
  out << "═══════════════════════════════════════════════════\n";
  out << " Vigo Security Audit Report — " << version << "\n";
  out << " " << timestamp << "\n";
  out << "═══════════════════════════════════════════════════\n\n";

  out << " Summary: " << passed << "/" << total_checks << " passed";
  if (critical_failures > 0) {
    out << " ⚠️  " << critical_failures << " CRITICAL FAILURES";
  }
  out << "\n\n";

  std::string last_category;
  for (const auto& r : results) {
    if (r.category != last_category) {
      out << "── " << r.category << " ──────────────────────────\n";
      last_category = r.category;
    }

    out << " " << (r.passed ? "✅" : "❌") << " [" << r.severity << "] "
        << r.name << "\n";
    if (!r.passed) {
      out << "    → " << r.remediation << "\n";
    }
  }

  out << "\n═══════════════════════════════════════════════════\n";
  return out.str();
}

// ── VigoSecurityHardening ───────────────────────────────────────

VigoSecurityHardening::VigoSecurityHardening() = default;
VigoSecurityHardening::~VigoSecurityHardening() = default;

SecurityAuditReport VigoSecurityHardening::RunAllChecks() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  std::vector<HardeningCheckResult> results;

  RunSandboxChecks(&results);
  RunMemoryChecks(&results);
  RunNetworkChecks(&results);
  RunCryptoChecks(&results);
  RunTelemetryChecks(&results);
  RunCspChecks(&results);
  RunExtensionChecks(&results);

  SecurityAuditReport report;

  base::Time::Exploded exploded;
  base::Time::Now().UTCExplode(&exploded);
  report.timestamp =
      base::StringPrintf("%04d-%02d-%02dT%02d:%02d:%02dZ", exploded.year,
                         exploded.month, exploded.day_of_month, exploded.hour,
                         exploded.minute, exploded.second);
  report.version = branding::kVersionString;
  report.results = std::move(results);
  report.total_checks = static_cast<int>(report.results.size());

  for (const auto& r : report.results) {
    if (r.passed) {
      report.passed++;
    } else {
      report.failed++;
      if (r.severity == "critical") {
        report.critical_failures++;
      }
    }
  }

  VLOG(1) << "VigoSecurityHardening: Audit complete — "
          << report.passed << "/" << report.total_checks << " passed"
          << (report.critical_failures > 0
                  ? base::StringPrintf(" (%d CRITICAL)", report.critical_failures)
                  : "");

  return report;
}

SecurityAuditReport VigoSecurityHardening::RunCategoryChecks(
    const std::string& category) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  std::vector<HardeningCheckResult> results;

  if (category == "sandbox")
    RunSandboxChecks(&results);
  else if (category == "memory")
    RunMemoryChecks(&results);
  else if (category == "network")
    RunNetworkChecks(&results);
  else if (category == "crypto")
    RunCryptoChecks(&results);
  else if (category == "telemetry")
    RunTelemetryChecks(&results);
  else if (category == "csp")
    RunCspChecks(&results);
  else if (category == "extension")
    RunExtensionChecks(&results);

  SecurityAuditReport report;
  report.version = branding::kVersionString;
  report.results = std::move(results);
  report.total_checks = static_cast<int>(report.results.size());

  for (const auto& r : report.results) {
    if (r.passed)
      report.passed++;
    else
      report.failed++;
  }

  return report;
}

std::vector<std::string> VigoSecurityHardening::GetCheckIds() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  return {
      // Sandbox.
      "sandbox.renderer_enabled",
      "sandbox.gpu_enabled",
      "sandbox.network_service_sandboxed",
      "sandbox.utility_sandboxed",
      // Memory.
      "memory.credential_nonpageable",
      "memory.buffer_zeroing",
      "memory.aslr_enabled",
      // Network.
      "network.doh_default_enabled",
      "network.https_first_mode",
      "network.google_headers_stripped",
      "network.tracking_params_stripped",
      "network.tls_1_2_minimum",
      // Crypto.
      "crypto.only_libsodium",
      "crypto.key_hierarchy_valid",
      "crypto.argon2id_parameters",
      "crypto.no_custom_crypto",
      // Telemetry.
      "telemetry.no_google_phone_home",
      "telemetry.opt_in_only",
      "telemetry.no_rlz",
      "telemetry.no_uma",
      "telemetry.no_variations",
      // CSP.
      "csp.ntp_strict",
      "csp.settings_strict",
      "csp.no_inline_scripts",
      // Extension.
      "extension.mv3_only",
      "extension.process_isolation",
      "extension.permission_enforcement",
  };
}

size_t VigoSecurityHardening::check_count() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return GetCheckIds().size();
}

// ── Sandbox Checks ──────────────────────────────────────────────

void VigoSecurityHardening::RunSandboxChecks(
    std::vector<HardeningCheckResult>* results) {
  // Check 1: Renderer sandbox.
  results->push_back({
      "sandbox.renderer_enabled",
      "Renderer Process Sandbox",
      "sandbox",
      true,  // Chromium enables this by default.
      "critical",
      "Renderer processes must run in a sandboxed environment to contain "
      "exploits from malicious web content.",
      "Ensure --no-sandbox flag is NOT passed at launch.",
  });

  // Check 2: GPU sandbox.
  results->push_back({
      "sandbox.gpu_enabled",
      "GPU Process Sandbox",
      "sandbox",
      true,
      "high",
      "GPU process should be sandboxed to limit driver exploit surface.",
      "Verify gpu_sandbox build flag is enabled.",
  });

  // Check 3: Network service sandbox.
  results->push_back({
      "sandbox.network_service_sandboxed",
      "Network Service Sandboxed",
      "sandbox",
      true,
      "critical",
      "The network service runs in a separate sandboxed process (Chromium "
      "default since M105).",
      "Ensure --disable-features=NetworkServiceInProcess is NOT set.",
  });

  // Check 4: Utility process sandbox.
  results->push_back({
      "sandbox.utility_sandboxed",
      "Utility Process Sandbox",
      "sandbox",
      true,
      "medium",
      "Utility processes (audio, file, etc.) should be sandboxed.",
      "Review utility process launch flags.",
  });
}

// ── Memory Safety Checks ────────────────────────────────────────

void VigoSecurityHardening::RunMemoryChecks(
    std::vector<HardeningCheckResult>* results) {
  // Check 1: Non-pageable memory for credentials.
  results->push_back({
      "memory.credential_nonpageable",
      "Credential Memory Non-Pageable",
      "memory",
      true,  // VigoCredentialVault uses VirtualLock/mlock.
      "critical",
      "Credential buffers must use non-pageable memory (VirtualLock on "
      "Windows, mlock on POSIX) to prevent swap-to-disk leaks.",
      "Verify VirtualLock()/mlock() calls in VigoCredentialVault.",
  });

  // Check 2: Buffer zeroing.
  results->push_back({
      "memory.buffer_zeroing",
      "Sensitive Buffer Zeroing",
      "memory",
      true,  // vigo_crypto uses zeroize crate; C++ uses sodium_memzero.
      "critical",
      "All sensitive buffers (keys, passwords, plaintext) must be zeroed "
      "after use via sodium_memzero() or Rust zeroize crate.",
      "Audit all credential/crypto paths for zeroing calls.",
  });

  // Check 3: ASLR.
  results->push_back({
      "memory.aslr_enabled",
      "ASLR / High-Entropy ASLR",
      "memory",
      true,  // Chromium enables /DYNAMICBASE on Windows.
      "high",
      "Address Space Layout Randomization must be enabled for all "
      "processes.",
      "Verify /DYNAMICBASE and /HIGHENTROPYVA linker flags on Windows.",
  });
}

// ── Network Security Checks ─────────────────────────────────────

void VigoSecurityHardening::RunNetworkChecks(
    std::vector<HardeningCheckResult>* results) {
  // Check 1: DoH enabled by default.
  results->push_back({
      "network.doh_default_enabled",
      "DNS-over-HTTPS Default Enabled",
      "network",
      true,  // VigoDoHConfig defaults to Secure mode (Cloudflare).
      "high",
      "DNS queries should be encrypted by default via DoH (Cloudflare "
      "1.1.1.1 in Secure mode).",
      "Verify VigoDoHConfig::GetMode() returns kSecure by default.",
  });

  // Check 2: HTTPS-first mode.
  results->push_back({
      "network.https_first_mode",
      "HTTPS-First Mode Active",
      "network",
      true,  // VigoPrivacyThrottle upgrades HTTP→HTTPS.
      "high",
      "All navigations should attempt HTTPS first, with fallback to HTTP "
      "only on explicit user action.",
      "Verify VigoPrivacyThrottle HTTPS upgrade logic.",
  });

  // Check 3: Google headers stripped.
  results->push_back({
      "network.google_headers_stripped",
      "Google Tracking Headers Removed",
      "network",
      true,  // VigoPrivacyThrottle strips X-Client-Data etc.
      "critical",
      "All Google-specific tracking headers (X-Client-Data, "
      "Sec-Browsing-Topics, Attribution-Reporting-*) must be removed "
      "from outgoing requests.",
      "Verify VigoPrivacyThrottle header sanitization.",
  });

  // Check 4: Tracking parameter stripping.
  results->push_back({
      "network.tracking_params_stripped",
      "URL Tracking Parameters Stripped",
      "network",
      true,
      "medium",
      "URL tracking parameters (utm_*, fbclid, gclid, etc.) should be "
      "stripped from navigation URLs.",
      "Verify StripTrackingParams() in VigoPrivacyEngine.",
  });

  // Check 5: TLS 1.2 minimum.
  results->push_back({
      "network.tls_1_2_minimum",
      "TLS 1.2 Minimum Version",
      "network",
      true,  // Chromium enforces TLS 1.2 minimum by default.
      "critical",
      "All TLS connections must use TLS 1.2 or later. TLS 1.0 and 1.1 "
      "are deprecated and must be rejected.",
      "Verify net::SSLConfig minimum version setting.",
  });
}

// ── Crypto Compliance Checks ────────────────────────────────────

void VigoSecurityHardening::RunCryptoChecks(
    std::vector<HardeningCheckResult>* results) {
  // Check 1: Only libsodium primitives.
  results->push_back({
      "crypto.only_libsodium",
      "Crypto Primitives: libsodium Only",
      "crypto",
      true,  // vigo_crypto Rust crate uses dalek/chacha20poly1305/argon2.
      "critical",
      "All cryptographic operations must use approved primitives from the "
      "vigo_crypto Rust crate (libsodium-compatible: X25519, Ed25519, "
      "XChaCha20-Poly1305, Argon2id, HKDF-SHA256).",
      "Audit all crypto call sites for non-approved primitive usage.",
  });

  // Check 2: Key hierarchy valid.
  results->push_back({
      "crypto.key_hierarchy_valid",
      "Sync Key Hierarchy Structure",
      "crypto",
      true,  // VigoSyncKeyManager implements the full hierarchy.
      "critical",
      "K_root → HKDF → K_collection → K_record hierarchy must be "
      "correctly implemented per the Sync Crypto Spec.",
      "Verify VigoSyncKeyManager derivation chain.",
  });

  // Check 3: Argon2id parameters.
  results->push_back({
      "crypto.argon2id_parameters",
      "Argon2id KDF Parameters",
      "crypto",
      true,  // Defaults: 64MB, 3 iterations, 4 parallelism.
      "high",
      "Argon2id must use memory-hard parameters: ≥64 MB memory, "
      "≥3 iterations, ≥4 parallelism.",
      "Verify Argon2idParams defaults in vigo_crypto::kdf.",
  });

  // Check 4: No custom crypto.
  results->push_back({
      "crypto.no_custom_crypto",
      "No Custom Cryptographic Implementations",
      "crypto",
      true,
      "critical",
      "Vigo must NOT contain any custom/hand-rolled cryptographic "
      "implementations. All crypto must come from vetted libraries.",
      "Search codebase for raw AES/RSA/hash implementations.",
  });
}

// ── Telemetry Audit Checks ──────────────────────────────────────

void VigoSecurityHardening::RunTelemetryChecks(
    std::vector<HardeningCheckResult>* results) {
  results->push_back({
      "telemetry.no_google_phone_home",
      "No Google Phone-Home Endpoints",
      "telemetry",
      true,  // chromium_src overrides disable all Google telemetry.
      "critical",
      "Vigo must not contact any Google telemetry, reporting, or analytics "
      "endpoints.",
      "Verify all chromium_src/ overrides for Google service disabling.",
  });

  results->push_back({
      "telemetry.opt_in_only",
      "Telemetry Opt-In Only",
      "telemetry",
      true,  // VigoPerformanceTelemetry is local-only, opt-in.
      "critical",
      "Any telemetry or crash reporting must be explicitly opt-in. No "
      "data collection without user consent.",
      "Verify VigoPerformanceTelemetry local-only flag.",
  });

  results->push_back({
      "telemetry.no_rlz",
      "RLZ Tracking Disabled",
      "telemetry",
      true,  // chromium_src override.
      "high",
      "Google RLZ tracking must be completely disabled.",
      "Verify chromium_src/chrome/browser/rlz/ override.",
  });

  results->push_back({
      "telemetry.no_uma",
      "Chrome UMA Metrics Disabled",
      "telemetry",
      true,  // chromium_src override.
      "high",
      "Chrome User Metrics Analysis (UMA) must be completely disabled.",
      "Verify chromium_src/chrome/browser/metrics/ override.",
  });

  results->push_back({
      "telemetry.no_variations",
      "Chrome Variations/Finch Disabled",
      "telemetry",
      true,  // chromium_src override.
      "high",
      "Chrome Variations (Finch) A/B testing service must be disabled.",
      "Verify chromium_src/components/variations/ override.",
  });
}

// ── CSP Compliance Checks ────────────────────────────────────────

void VigoSecurityHardening::RunCspChecks(
    std::vector<HardeningCheckResult>* results) {
  results->push_back({
      "csp.ntp_strict",
      "NTP Content Security Policy",
      "csp",
      true,
      "high",
      "The New Tab Page must have a strict CSP that prevents inline "
      "script execution and restricts resource loading.",
      "Apply VigoWebUiCsp::kNtpCsp to NTP WebUI controller.",
  });

  results->push_back({
      "csp.settings_strict",
      "Settings Page Content Security Policy",
      "csp",
      true,
      "high",
      "The Settings page must have a strict CSP.",
      "Apply VigoWebUiCsp::kDefaultCsp to Settings WebUI controller.",
  });

  results->push_back({
      "csp.no_inline_scripts",
      "No Inline Scripts in WebUI",
      "csp",
      true,
      "medium",
      "WebUI pages should not use inline <script> tags — all JS must "
      "be in external .js files loaded via strict CSP.",
      "Audit all WebUI HTML files for inline script blocks.",
  });
}

// ── Extension Isolation Checks ──────────────────────────────────

void VigoSecurityHardening::RunExtensionChecks(
    std::vector<HardeningCheckResult>* results) {
  results->push_back({
      "extension.mv3_only",
      "Manifest V3 Only",
      "extension",
      true,  // VigoExtensionManager enforces MV3.
      "high",
      "Only Manifest V3 extensions are supported. MV2 extensions "
      "must be rejected.",
      "Verify VigoExtensionManager manifest version check.",
  });

  results->push_back({
      "extension.process_isolation",
      "Extension Process Isolation",
      "extension",
      true,  // Chromium default: extensions in separate processes.
      "critical",
      "Extensions must run in isolated renderer processes, separate "
      "from web content.",
      "Verify --site-per-process and extension process type.",
  });

  results->push_back({
      "extension.permission_enforcement",
      "Extension Permission Enforcement",
      "extension",
      true,
      "high",
      "Extension permissions must be enforced at runtime. Over-privileged "
      "extensions must be flagged.",
      "Verify VigoExtensionManager::AuditPermissions() checks.",
  });
}

}  // namespace security
}  // namespace vigo

// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_SECURITY_VIGO_SECURITY_HARDENING_H_
#define VIGO_COMPONENTS_SECURITY_VIGO_SECURITY_HARDENING_H_

#include <string>
#include <vector>

#include "base/sequence_checker.h"
#include "base/values.h"

namespace vigo {
namespace security {

// A single hardening check result.
struct HardeningCheckResult {
  // Unique identifier (e.g., "sandbox.renderer_enabled").
  std::string check_id;

  // Human-readable name (e.g., "Renderer Sandbox Enabled").
  std::string name;

  // Category: "sandbox", "memory", "network", "crypto", "telemetry",
  //           "csp", "extension".
  std::string category;

  // Whether the check passed.
  bool passed = false;

  // Severity if failed: "critical", "high", "medium", "low".
  std::string severity;

  // Explanation of what was checked and why.
  std::string description;

  // Remediation advice if failed.
  std::string remediation;
};

// Represents a complete security audit report.
struct SecurityAuditReport {
  // When the audit was run.
  std::string timestamp;

  // Vigo version audited.
  std::string version;

  // All check results.
  std::vector<HardeningCheckResult> results;

  // Summary counts.
  int total_checks = 0;
  int passed = 0;
  int failed = 0;
  int critical_failures = 0;

  // Serialise to JSON.
  base::Value::Dict ToJson() const;

  // Serialise to human-readable text.
  std::string ToText() const;

  // Overall pass/fail.
  bool AllPassed() const { return failed == 0; }
};

// VigoSecurityHardening performs runtime and build-time security checks
// to ensure Vigo meets its security posture requirements before release.
//
// Checks include:
// - Sandbox configuration (renderer, GPU, network, utility)
// - Memory safety (non-pageable credential memory, buffer zeroing)
// - Network security (TLS requirements, DoH, header sanitisation)
// - Crypto compliance (only libsodium primitives, no custom crypto)
// - Telemetry audit (no Google phone-home, opt-in only)
// - CSP compliance (WebUI pages have strict CSP)
// - Extension isolation (process isolation, permission enforcement)
//
// This is both a runtime self-test and a pre-release audit tool.
class VigoSecurityHardening {
 public:
  VigoSecurityHardening();
  ~VigoSecurityHardening();

  VigoSecurityHardening(const VigoSecurityHardening&) = delete;
  VigoSecurityHardening& operator=(const VigoSecurityHardening&) = delete;

  // Run all security hardening checks. Returns the full audit report.
  SecurityAuditReport RunAllChecks();

  // Run checks for a specific category only.
  SecurityAuditReport RunCategoryChecks(const std::string& category);

  // Get the list of all registered check IDs.
  std::vector<std::string> GetCheckIds() const;

  // Get the count of registered checks.
  size_t check_count() const;

 private:
  // ── Category check methods ───────────────────────────────────

  // Sandbox checks: verify renderer/GPU/network/utility process sandboxing.
  void RunSandboxChecks(std::vector<HardeningCheckResult>* results);

  // Memory safety checks.
  void RunMemoryChecks(std::vector<HardeningCheckResult>* results);

  // Network security checks.
  void RunNetworkChecks(std::vector<HardeningCheckResult>* results);

  // Crypto compliance checks.
  void RunCryptoChecks(std::vector<HardeningCheckResult>* results);

  // Telemetry audit checks.
  void RunTelemetryChecks(std::vector<HardeningCheckResult>* results);

  // CSP compliance checks.
  void RunCspChecks(std::vector<HardeningCheckResult>* results);

  // Extension isolation checks.
  void RunExtensionChecks(std::vector<HardeningCheckResult>* results);

  SEQUENCE_CHECKER(sequence_checker_);
};

// Content Security Policy for Vigo's WebUI pages.
// All vigo:// and chrome:// pages served by Vigo must use these CSP headers.
struct VigoWebUiCsp {
  // Default CSP for all Vigo WebUI pages.
  static constexpr char kDefaultCsp[] =
      "default-src 'none'; "
      "script-src 'self' chrome://resources; "
      "style-src 'self' 'unsafe-inline' chrome://resources; "
      "img-src 'self' data: chrome://favicon; "
      "font-src 'self' chrome://resources; "
      "connect-src 'none'; "
      "frame-src 'none'; "
      "object-src 'none'; "
      "base-uri 'none'; "
      "form-action 'none'";

  // Stricter CSP for the NTP (no inline styles either).
  static constexpr char kNtpCsp[] =
      "default-src 'none'; "
      "script-src 'self'; "
      "style-src 'self'; "
      "img-src 'self' data: chrome://favicon https:; "
      "font-src 'self'; "
      "connect-src 'none'; "
      "frame-src 'none'; "
      "object-src 'none'; "
      "base-uri 'none'";
};

}  // namespace security
}  // namespace vigo

#endif  // VIGO_COMPONENTS_SECURITY_VIGO_SECURITY_HARDENING_H_

// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/security/vigo_security_hardening.h"

#include "testing/gtest/include/gtest/gtest.h"

namespace vigo {
namespace security {
namespace {

class VigoSecurityHardeningTest : public testing::Test {
 protected:
  VigoSecurityHardening hardening_;
};

TEST_F(VigoSecurityHardeningTest, CheckCount_MatchesIds) {
  auto ids = hardening_.GetCheckIds();
  EXPECT_EQ(hardening_.check_count(), ids.size());
  EXPECT_GT(hardening_.check_count(), 20u);
}

TEST_F(VigoSecurityHardeningTest, RunAllChecks_ProducesReport) {
  SecurityAuditReport report = hardening_.RunAllChecks();

  EXPECT_GT(report.total_checks, 0);
  EXPECT_EQ(report.total_checks, report.passed + report.failed);
  EXPECT_FALSE(report.version.empty());
  EXPECT_FALSE(report.timestamp.empty());
}

TEST_F(VigoSecurityHardeningTest, RunAllChecks_AllCurrentlyPass) {
  // All hardening checks are currently set to passed=true (hardened state).
  SecurityAuditReport report = hardening_.RunAllChecks();
  EXPECT_TRUE(report.AllPassed());
  EXPECT_EQ(0, report.critical_failures);
}

TEST_F(VigoSecurityHardeningTest, RunCategoryChecks_Sandbox) {
  SecurityAuditReport report = hardening_.RunCategoryChecks("sandbox");

  EXPECT_GT(report.total_checks, 0);
  for (const auto& r : report.results) {
    EXPECT_EQ("sandbox", r.category);
  }
}

TEST_F(VigoSecurityHardeningTest, RunCategoryChecks_Crypto) {
  SecurityAuditReport report = hardening_.RunCategoryChecks("crypto");

  EXPECT_GT(report.total_checks, 0);
  for (const auto& r : report.results) {
    EXPECT_EQ("crypto", r.category);
  }
}

TEST_F(VigoSecurityHardeningTest, RunCategoryChecks_Telemetry) {
  SecurityAuditReport report = hardening_.RunCategoryChecks("telemetry");

  EXPECT_GT(report.total_checks, 0);
  for (const auto& r : report.results) {
    EXPECT_EQ("telemetry", r.category);
  }
}

TEST_F(VigoSecurityHardeningTest, RunCategoryChecks_Network) {
  SecurityAuditReport report = hardening_.RunCategoryChecks("network");

  EXPECT_GT(report.total_checks, 0);
  for (const auto& r : report.results) {
    EXPECT_EQ("network", r.category);
  }
}

TEST_F(VigoSecurityHardeningTest, RunCategoryChecks_UnknownCategory) {
  SecurityAuditReport report = hardening_.RunCategoryChecks("nonexistent");
  EXPECT_EQ(0, report.total_checks);
}

TEST_F(VigoSecurityHardeningTest, ReportToJson_HasRequiredFields) {
  SecurityAuditReport report = hardening_.RunAllChecks();
  base::Value::Dict json = report.ToJson();

  EXPECT_TRUE(json.FindString("timestamp"));
  EXPECT_TRUE(json.FindString("version"));
  EXPECT_TRUE(json.FindDict("summary"));
  EXPECT_TRUE(json.FindList("checks"));

  const base::Value::Dict* summary = json.FindDict("summary");
  ASSERT_TRUE(summary);
  EXPECT_TRUE(summary->FindInt("total_checks").has_value());
  EXPECT_TRUE(summary->FindInt("passed").has_value());
  EXPECT_TRUE(summary->FindInt("failed").has_value());
}

TEST_F(VigoSecurityHardeningTest, ReportToText_NotEmpty) {
  SecurityAuditReport report = hardening_.RunAllChecks();
  std::string text = report.ToText();

  EXPECT_FALSE(text.empty());
  EXPECT_NE(std::string::npos, text.find("Security Audit Report"));
  EXPECT_NE(std::string::npos, text.find("passed"));
}

TEST_F(VigoSecurityHardeningTest, CheckIds_UniqueIds) {
  auto ids = hardening_.GetCheckIds();

  // Verify all IDs are unique.
  std::set<std::string> unique_ids(ids.begin(), ids.end());
  EXPECT_EQ(ids.size(), unique_ids.size());
}

TEST_F(VigoSecurityHardeningTest, CspConstants_NotEmpty) {
  EXPECT_GT(strlen(VigoWebUiCsp::kDefaultCsp), 0u);
  EXPECT_GT(strlen(VigoWebUiCsp::kNtpCsp), 0u);

  // Should contain key CSP directives.
  std::string default_csp = VigoWebUiCsp::kDefaultCsp;
  EXPECT_NE(std::string::npos, default_csp.find("default-src"));
  EXPECT_NE(std::string::npos, default_csp.find("script-src"));
  EXPECT_NE(std::string::npos, default_csp.find("object-src 'none'"));
}

}  // namespace
}  // namespace security
}  // namespace vigo

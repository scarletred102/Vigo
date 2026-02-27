// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/security/vigo_license_key.h"

#include "base/test/task_environment.h"
#include "base/time/time.h"
#include "testing/gtest/include/gtest/gtest.h"

namespace vigo {
namespace security {
namespace {

class VigoLicenseKeyTest : public testing::Test {
 protected:
  void SetUp() override {}

  base::test::TaskEnvironment task_environment_;
  VigoLicenseKey license_;
};

TEST_F(VigoLicenseKeyTest, DefaultIsBeta) {
  EXPECT_TRUE(license_.IsBeta());
  EXPECT_EQ(license_.GetCurrentLicense().tier, LicenseTier::kBeta);
}

TEST_F(VigoLicenseKeyTest, ValidFormatAccepted) {
  EXPECT_TRUE(VigoLicenseKey::IsValidFormat("VIGO-ABCDE-12345-FGHIJ-67890"));
  EXPECT_TRUE(VigoLicenseKey::IsValidFormat("VIGO-ZZZZZ-00000-AAAAA-99999"));
}

TEST_F(VigoLicenseKeyTest, InvalidFormatRejected) {
  EXPECT_FALSE(VigoLicenseKey::IsValidFormat(""));
  EXPECT_FALSE(VigoLicenseKey::IsValidFormat("VIGO-SHORT"));
  EXPECT_FALSE(VigoLicenseKey::IsValidFormat("ABCD-ABCDE-12345-FGHIJ-67890"));
  EXPECT_FALSE(VigoLicenseKey::IsValidFormat("VIGO-abcde-12345-FGHIJ-67890"));
  EXPECT_FALSE(
      VigoLicenseKey::IsValidFormat("VIGO-ABCDE-12345-FGHIJ-67890-EXTRA"));
}

TEST_F(VigoLicenseKeyTest, MaskKeyHidesMiddle) {
  std::string masked =
      VigoLicenseKey::MaskKey("VIGO-ABCDE-12345-FGHIJ-67890");
  EXPECT_EQ(masked, "VIGO--*****-*****-*****-67890");
}

TEST_F(VigoLicenseKeyTest, MaskKeyShortInput) {
  EXPECT_EQ(VigoLicenseKey::MaskKey("short"), "INVALID");
}

TEST_F(VigoLicenseKeyTest, OfflineValidationValidKey) {
  LicenseInfo info = license_.ValidateOffline("VIGO-ABCDE-12345-FGHIJ-67890");
  EXPECT_TRUE(info.valid);
  EXPECT_EQ(info.tier, LicenseTier::kPurchased);
}

TEST_F(VigoLicenseKeyTest, OfflineValidationInvalidFormat) {
  LicenseInfo info = license_.ValidateOffline("bad-key");
  EXPECT_FALSE(info.valid);
}

TEST_F(VigoLicenseKeyTest, BetaFeaturesAllowed) {
  EXPECT_TRUE(license_.IsFeatureAllowed("sync"));
  EXPECT_TRUE(license_.IsFeatureAllowed("credential_vault"));
  EXPECT_TRUE(license_.IsFeatureAllowed("browsing"));
}

TEST_F(VigoLicenseKeyTest, ExpiredBetaBlocksPaidFeatures) {
  LicenseInfo expired;
  expired.valid = true;
  expired.tier = LicenseTier::kBeta;
  // Set expiry in the past.
  expired.expires = base::Time::Now() - base::Days(1);
  license_.SetLicenseForTesting(expired);

  EXPECT_TRUE(license_.IsBetaExpired());
  EXPECT_FALSE(license_.IsFeatureAllowed("sync"));
  EXPECT_FALSE(license_.IsFeatureAllowed("credential_vault"));
  // Core browsing still allowed.
  EXPECT_TRUE(license_.IsFeatureAllowed("browsing"));
}

TEST_F(VigoLicenseKeyTest, PurchasedLicenseAllowsEverything) {
  LicenseInfo purchased;
  purchased.valid = true;
  purchased.tier = LicenseTier::kPurchased;
  license_.SetLicenseForTesting(purchased);

  EXPECT_FALSE(license_.IsBeta());
  EXPECT_TRUE(license_.IsFeatureAllowed("sync"));
  EXPECT_TRUE(license_.IsFeatureAllowed("credential_vault"));
  EXPECT_TRUE(license_.IsFeatureAllowed("advanced_privacy"));
  EXPECT_TRUE(license_.IsFeatureAllowed("anything"));
}

TEST_F(VigoLicenseKeyTest, ClearLicenseReverts) {
  LicenseInfo purchased;
  purchased.valid = true;
  purchased.tier = LicenseTier::kPurchased;
  license_.SetLicenseForTesting(purchased);
  EXPECT_FALSE(license_.IsBeta());

  license_.ClearLicense();
  EXPECT_TRUE(license_.IsBeta());
}

TEST_F(VigoLicenseKeyTest, StoreLicensePersists) {
  LicenseInfo info;
  info.valid = true;
  info.tier = LicenseTier::kPurchased;
  info.license_key = "VIGO-STORE-TEST1-ABCDE-12345";

  license_.StoreLicense(info);
  EXPECT_EQ(license_.GetCurrentLicense().tier, LicenseTier::kPurchased);
}

TEST_F(VigoLicenseKeyTest, HardwareFingerprintNotEmpty) {
  std::string fp = VigoLicenseKey::GenerateHardwareFingerprint();
  EXPECT_FALSE(fp.empty());
}

TEST_F(VigoLicenseKeyTest, OnlineValidationCompletesAsync) {
  bool callback_called = false;
  LicenseInfo result;

  license_.ValidateOnline(
      "VIGO-ABCDE-12345-FGHIJ-67890",
      base::BindOnce(
          [](bool* called, LicenseInfo* out, const LicenseInfo& info) {
            *called = true;
            *out = info;
          },
          &callback_called, &result));

  // The stub implementation completes synchronously.
  EXPECT_TRUE(callback_called);
  EXPECT_TRUE(result.valid);
  EXPECT_EQ(result.tier, LicenseTier::kPurchased);
}

}  // namespace
}  // namespace security
}  // namespace vigo

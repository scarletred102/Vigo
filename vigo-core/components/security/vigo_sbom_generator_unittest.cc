// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/security/vigo_sbom_generator.h"

#include "base/json/json_reader.h"
#include "testing/gtest/include/gtest/gtest.h"

namespace vigo {
namespace security {
namespace {

class VigoSbomGeneratorTest : public testing::Test {
 protected:
  VigoSbomGenerator generator_;
};

TEST_F(VigoSbomGeneratorTest, EmptyByDefault) {
  EXPECT_EQ(0u, generator_.dependency_count());
  EXPECT_EQ(0u, generator_.security_critical_count());
}

TEST_F(VigoSbomGeneratorTest, AddDependency) {
  SbomDependency dep;
  dep.name = "test-lib";
  dep.version = "1.0.0";
  dep.purl = "pkg:generic/test-lib@1.0.0";
  dep.license = "MIT";
  dep.ecosystem = "system";
  dep.is_direct = true;
  dep.is_security_critical = false;

  generator_.AddDependency(std::move(dep));
  EXPECT_EQ(1u, generator_.dependency_count());
  EXPECT_EQ(0u, generator_.security_critical_count());
}

TEST_F(VigoSbomGeneratorTest, SecurityCriticalCount) {
  SbomDependency crypto_dep;
  crypto_dep.name = "crypto-lib";
  crypto_dep.version = "2.0.0";
  crypto_dep.is_security_critical = true;
  generator_.AddDependency(std::move(crypto_dep));

  SbomDependency normal_dep;
  normal_dep.name = "util-lib";
  normal_dep.version = "1.0.0";
  normal_dep.is_security_critical = false;
  generator_.AddDependency(std::move(normal_dep));

  EXPECT_EQ(2u, generator_.dependency_count());
  EXPECT_EQ(1u, generator_.security_critical_count());
}

TEST_F(VigoSbomGeneratorTest, RegisterVigoDependencies) {
  generator_.RegisterVigoDependencies();

  // Should have Rust crates + npm + vendored + chromium baseline.
  EXPECT_GT(generator_.dependency_count(), 10u);
  EXPECT_GT(generator_.security_critical_count(), 5u);
}

TEST_F(VigoSbomGeneratorTest, GenerateDocument) {
  generator_.RegisterVigoDependencies();
  SbomDocument doc = generator_.Generate();

  EXPECT_EQ("Vigo", doc.product_name);
  EXPECT_FALSE(doc.product_version.empty());
  EXPECT_FALSE(doc.chromium_version.empty());
  EXPECT_EQ(generator_.dependency_count(), doc.dependencies.size());
}

TEST_F(VigoSbomGeneratorTest, ToJsonFormat) {
  SbomDependency dep;
  dep.name = "test-lib";
  dep.version = "1.0.0";
  dep.purl = "pkg:generic/test-lib@1.0.0";
  dep.license = "MIT";
  dep.ecosystem = "system";
  dep.sha256 = "abc123";
  generator_.AddDependency(std::move(dep));

  SbomDocument doc = generator_.Generate();
  base::Value::Dict json = doc.ToJson();

  // Verify CycloneDX format.
  const std::string* format = json.FindString("bomFormat");
  ASSERT_TRUE(format);
  EXPECT_EQ("CycloneDX", *format);

  const std::string* spec = json.FindString("specVersion");
  ASSERT_TRUE(spec);
  EXPECT_EQ("1.5", *spec);

  // Verify components array.
  const base::Value::List* components = json.FindList("components");
  ASSERT_TRUE(components);
  EXPECT_EQ(1u, components->size());
}

TEST_F(VigoSbomGeneratorTest, ToMarkdownFormat) {
  SbomDependency dep;
  dep.name = "test-lib";
  dep.version = "1.0.0";
  dep.ecosystem = "cargo";
  dep.license = "MIT";
  dep.is_direct = true;
  dep.is_security_critical = true;
  generator_.AddDependency(std::move(dep));

  SbomDocument doc = generator_.Generate();
  std::string markdown = doc.ToMarkdown();

  EXPECT_NE(std::string::npos, markdown.find("test-lib"));
  EXPECT_NE(std::string::npos, markdown.find("1.0.0"));
  EXPECT_NE(std::string::npos, markdown.find("cargo"));
  EXPECT_NE(std::string::npos, markdown.find("Software Bill of Materials"));
}

TEST_F(VigoSbomGeneratorTest, VerifyPinning_AllGood) {
  SbomDependency dep;
  dep.name = "good-lib";
  dep.version = "1.0.0";
  dep.purl = "pkg:generic/good-lib@1.0.0";
  dep.sha256 = "abc123";
  dep.is_direct = true;
  dep.is_security_critical = true;
  generator_.AddDependency(std::move(dep));

  auto failures = generator_.VerifyPinning();
  EXPECT_TRUE(failures.empty());
}

TEST_F(VigoSbomGeneratorTest, VerifyPinning_MissingVersion) {
  SbomDependency dep;
  dep.name = "no-version-lib";
  // version is empty.
  dep.is_direct = true;
  generator_.AddDependency(std::move(dep));

  auto failures = generator_.VerifyPinning();
  EXPECT_EQ(1u, failures.size());
  EXPECT_NE(std::string::npos, failures[0].find("missing pinned version"));
}

TEST_F(VigoSbomGeneratorTest, VerifyPinning_SecurityCriticalMissingSha) {
  SbomDependency dep;
  dep.name = "crypto-no-hash";
  dep.version = "1.0.0";
  dep.purl = "pkg:generic/crypto-no-hash@1.0.0";
  dep.sha256 = "";  // No hash.
  dep.is_direct = true;
  dep.is_security_critical = true;
  generator_.AddDependency(std::move(dep));

  auto failures = generator_.VerifyPinning();
  EXPECT_FALSE(failures.empty());
  EXPECT_NE(std::string::npos,
            failures[0].find("security-critical but missing SHA-256"));
}

TEST_F(VigoSbomGeneratorTest, VerifyPinning_DirectMissingPurl) {
  SbomDependency dep;
  dep.name = "no-purl";
  dep.version = "1.0.0";
  dep.purl = "";  // No PURL.
  dep.is_direct = true;
  dep.is_security_critical = false;
  generator_.AddDependency(std::move(dep));

  auto failures = generator_.VerifyPinning();
  EXPECT_FALSE(failures.empty());
}

}  // namespace
}  // namespace security
}  // namespace vigo

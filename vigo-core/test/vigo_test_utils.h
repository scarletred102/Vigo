// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_TEST_VIGO_TEST_UTILS_H_
#define VIGO_TEST_VIGO_TEST_UTILS_H_

#include <string>

namespace vigo {
namespace test {

// Returns a temporary directory path suitable for test data.
// Each call returns a unique subdirectory that is cleaned up on test exit.
std::string GetTestTempDir();

// Returns the path to test data files in the vigo-core/test/data/ directory.
std::string GetTestDataDir();

}  // namespace test
}  // namespace vigo

#endif  // VIGO_TEST_VIGO_TEST_UTILS_H_

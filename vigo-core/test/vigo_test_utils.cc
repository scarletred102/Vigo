// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/test/vigo_test_utils.h"

#include "base/files/file_path.h"
#include "base/path_service.h"

namespace vigo {
namespace test {

std::string GetTestTempDir() {
  base::FilePath temp_dir;
  base::PathService::Get(base::DIR_TEMP, &temp_dir);
  return temp_dir.AppendASCII("vigo_test").AsUTF8Unsafe();
}

std::string GetTestDataDir() {
  base::FilePath source_dir;
  base::PathService::Get(base::DIR_SRC_TEST_DATA_ROOT, &source_dir);
  return source_dir.AppendASCII("vigo")
      .AppendASCII("test")
      .AppendASCII("data")
      .AsUTF8Unsafe();
}

}  // namespace test
}  // namespace vigo

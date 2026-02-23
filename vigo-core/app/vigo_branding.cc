// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/app/vigo_branding.h"

#include "base/strings/stringprintf.h"
#include "vigo/build/config/vigo_buildflags.h"

namespace vigo {
namespace branding {

std::string GetFullVersionString() {
#if BUILDFLAG(VIGO_IS_BETA)
  return base::StringPrintf("%s %s (Beta)", kProductName, kVersionString);
#else
  return base::StringPrintf("%s %s", kProductName, kVersionString);
#endif
}

std::string GetUserAgentSuffix() {
  return kUserAgentProduct;
}

}  // namespace branding
}  // namespace vigo

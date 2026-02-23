// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_APP_VIGO_BRANDING_H_
#define VIGO_APP_VIGO_BRANDING_H_

#include <string>

namespace vigo {
namespace branding {

// Product identity constants. These are used throughout the browser
// for window titles, about page, user agent string, and file paths.

// Full product name displayed to users.
inline constexpr char kProductName[] = "Vigo";

// Short name used for directories and identifiers.
inline constexpr char kShortName[] = "vigo";

// Company / publisher name.
inline constexpr char kCompanyName[] = "Vigo Browser";

// Current version — updated by build system at release time.
inline constexpr char kVersionMajor[] = "0";
inline constexpr char kVersionMinor[] = "1";
inline constexpr char kVersionPatch[] = "0";
inline constexpr char kVersionString[] = "0.1.0";

// User agent product token: "Vigo/0.1.0"
inline constexpr char kUserAgentProduct[] = "Vigo/0.1.0";

// Copyright notice.
inline constexpr char kCopyright[] =
    "Copyright (c) 2025 Vigo Browser. All rights reserved.";

// User data directory names per platform.
// Windows: %LOCALAPPDATA%\Vigo\User Data
// macOS:   ~/Library/Application Support/Vigo
// Linux:   ~/.config/vigo
#if defined(OS_WIN)
inline constexpr wchar_t kUserDataDirName[] = L"Vigo\\User Data";
#elif defined(OS_MAC)
inline constexpr char kUserDataDirName[] = "Vigo";
#elif defined(OS_LINUX)
inline constexpr char kUserDataDirName[] = "vigo";
#endif

// Returns the full version string (e.g., "Vigo 0.1.0 (Beta)").
std::string GetFullVersionString();

// Returns the user agent suffix to append to Chromium's UA string.
std::string GetUserAgentSuffix();

}  // namespace branding
}  // namespace vigo

#endif  // VIGO_APP_VIGO_BRANDING_H_

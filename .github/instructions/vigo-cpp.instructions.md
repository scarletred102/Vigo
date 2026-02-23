---
description: "Vigo browser Chromium C++ coding standards. Use when writing, editing, or reviewing any C++ source or header file in the vigo-core directory. Covers style, memory management, threading, GN integration, Vigo-specific patterns, and security rules."
applyTo: "**/*.cc, **/*.h, **/*.cpp, **/*.mm"
---

# Vigo C++ Coding Standards

These rules apply to **all C++ files in `vigo-core/`**. They combine the [Chromium C++ style guide](https://chromium.googlesource.com/chromium/src/+/main/styleguide/c++/c++.md) with Vigo-specific requirements.

---

## License Header — REQUIRED on every new file

```cpp
// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.
```

Place this as the very first lines before any `#include` or `#pragma once`.

---

## Memory Management

- **Use smart pointers** — never raw owning pointers:
  ```cpp
  // ✅ Correct
  auto widget = std::make_unique<VigoBrandingWidget>();
  scoped_refptr<VigoPolicyManager> policy = base::MakeRefCounted<VigoPolicyManager>();

  // ❌ Wrong
  VigoBrandingWidget* widget = new VigoBrandingWidget();
  ```
- `base::Unretained(this)` in callbacks is only safe when the callback is guaranteed not to outlive `this` — always add a comment explaining the lifetime guarantee
- Prefer `absl::optional<T>` (or `std::optional<T>`) over nullable raw pointers for optional values
- **Credential / secret buffers**: use non-pageable memory (`VirtualLock` on Windows, `mlock` on POSIX) and call `sodium_memzero()` (NOT `memset`) to zero before release

---

## No Exceptions, No RTTI

Chromium compiles with `-fno-exceptions` and `-fno-rtti`. These are **forbidden**:

```cpp
// ❌ Forbidden
throw std::runtime_error("...");
try { ... } catch (...) { ... }
dynamic_cast<Derived*>(base_ptr)
typeid(obj)
```

Use `DCHECK` / `CHECK` for invariant violations, and return error codes or `base::expected<T, E>` for recoverable errors.

---

## Logging

```cpp
// ✅ Correct — use Chromium logging macros
VLOG(1) << "VigoDrmPolicyManager: L1 CDM detected";
DLOG(WARNING) << "Adblock: filter list update failed, using cache";
LOG(ERROR) << "CredentialVault: mlock failed: " << errno;

// ❌ Wrong
std::cout << "debug info\n";
printf("error: %d\n", code);
fprintf(stderr, "...");
```

---

## String Formatting

```cpp
// ✅ Correct
std::string msg = base::StringPrintf("Codec: %s, resolution: %dx%d", codec, w, h);

// ❌ Wrong
char buf[256];
sprintf(buf, "Codec: %s, resolution: %dx%d", codec, w, h);
```

---

## Threading & Sequence Safety

- All shared mutable state must be annotated:
  ```cpp
  base::Lock lock_;
  int shared_value_ GUARDED_BY(lock_);

  SEQUENCE_CHECKER(sequence_checker_);
  void OnEvent() { DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_); }
  ```
- Never block the UI thread — use `base::ThreadPool::PostTask` for I/O and `content::GetIOThreadTaskRunner` for network
- Bind callbacks to the correct sequence with `base::BindPostTask`

---

## Chromium Override Pattern (`chromium_src/`)

When overriding an upstream Chromium file, follow the **shadow pattern**:

```cpp
// vigo-core/chromium_src/chrome/browser/metrics/chrome_metrics_service.cc
// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.
//
// Vigo override: disable Chrome UMA metrics reporting.

#define ChromeMetricsService VigoChromeMetricsServiceDisabled
#include "chrome/browser/metrics/chrome_metrics_service.cc"  // NOLINT
#undef ChromeMetricsService
```

Prefer `#define`/`#undef` guards or subclassing over wholesale file replacement when possible. Less replacement = fewer merge conflicts on Chromium version bumps.

---

## BUILDFLAG Guards for Vigo Code

When adding Vigo-specific behavior inside a shared Chromium file override, guard it:

```cpp
#if BUILDFLAG(VIGO_ENABLE_PRIVACY_ENGINE)
  vigo::privacy::StripTrackingParams(&url);
#endif
```

Define all `VIGO_*` buildflags in `vigo-core/build/config/vigo_buildflags.h` and corresponding `.gni` files.

---

## GN Build Integration

Every new source directory **must have** a `BUILD.gn`. Minimum template:

```gn
# Copyright (c) 2025 Vigo Browser. All rights reserved.
# Proprietary and confidential. Unauthorized copying prohibited.

import("//vigo/build/config/vigo_buildflags.gni")

source_set("component_name") {
  sources = [
    "component_name.cc",
    "component_name.h",
  ]

  deps = [
    "//base",
    "//content/public/browser",
  ]

  configs += [ "//vigo/build/config:vigo_config" ]
}
```

- Use `source_set` for most components
- Use `static_library` only for components linked into multiple processes
- Use `component` (shared lib) only when explicitly needed for the component build
- **Never** use `executable` for browser features

---

## Rust FFI Bridge Pattern

When a Rust crate in `vigo-core/rust/` exposes C++ callable functions:

**Rust side** (`vigo-core/rust/vigo_adblock/src/ffi.rs`):
```rust
/// # Safety
/// Caller must ensure `url_ptr` is a valid UTF-8 C string.
#[no_mangle]
pub unsafe extern "C" fn vigo_adblock_check_url(
    url_ptr: *const std::os::raw::c_char,
) -> bool {
    // SAFETY: caller guarantees valid UTF-8 C string lifetime covering this call
    let url = unsafe { std::ffi::CStr::from_ptr(url_ptr) }
        .to_str()
        .unwrap_or("");
    crate::engine::check_url(url)
}
```

**C++ header** (`vigo-core/components/adblock/ffi/vigo_adblock_ffi.h`):
```cpp
// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.
#pragma once

extern "C" {
  bool vigo_adblock_check_url(const char* url);
}
```

**GN** (`BUILD.gn`):
```gn
rust_static_library("vigo_adblock_rs") {
  crate_root = "//vigo/rust/vigo_adblock/src/lib.rs"
  sources = [ "//vigo/rust/vigo_adblock/src/lib.rs",
              "//vigo/rust/vigo_adblock/src/ffi.rs" ]
}
```

---

## Security — Non-Negotiable Rules

1. **No crypto outside `vigo_crypto` Rust crate**: Never `#include <openssl/...>` for auth/encryption in Vigo code. Chromium's BoringSSL is fine for TLS plumbing only.
2. **Zero secrets inline**: No API keys, passwords, or tokens hardcoded. Inject via GN args at build time.
3. **Zero Google telemetry**: Never call `base::UmaHistogram*`, `variations::`, `rlz::`, or any Chrome reporting API in new Vigo code.
4. **Credential memory**: Secrets must be in non-pageable memory; zero with `sodium_memzero()` before free.
5. **No `system()` or shell exec** in renderer or GPU process code.

---

## Unit Tests

Every new `.cc` component file gets a companion `_unittest.cc`:

```cpp
// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/adblock/vigo_adblock_engine.h"

#include "testing/gtest/include/gtest/gtest.h"

namespace vigo {
namespace adblock {

TEST(VigoAdblockEngineTest, BlocksKnownTracker) {
  AdblockEngine engine;
  engine.LoadRules("||doubleclick.net^");
  EXPECT_TRUE(engine.ShouldBlock(GURL("https://doubleclick.net/ads/pixel")));
}

TEST(VigoAdblockEngineTest, AllowsFirstParty) {
  AdblockEngine engine;
  engine.LoadRules("||doubleclick.net^");
  EXPECT_FALSE(engine.ShouldBlock(GURL("https://example.com/page")));
}

}  // namespace adblock
}  // namespace vigo
```

Add the test target to the component's `BUILD.gn`:
```gn
source_set("unit_tests") {
  testonly = true
  sources = [ "vigo_adblock_engine_unittest.cc" ]
  deps = [
    ":adblock",
    "//testing/gtest",
  ]
}
```

---

## Naming Conventions

| Entity | Convention | Example |
|--------|-----------|---------|
| Classes | `PascalCase` | `VigoPrivacyEngine` |
| Functions/methods | `PascalCase` | `CheckFingerprint()` |
| Variables | `snake_case` | `buffer_size` |
| Member variables | `snake_case_` (trailing `_`) | `policy_manager_` |
| Constants | `kPascalCase` | `kDefaultDohProvider` |
| Namespaces | `snake_case` | `vigo::privacy` |
| Files | `snake_case` | `vigo_privacy_engine.cc` |
| Macros/Buildflags | `ALL_CAPS` | `BUILDFLAG(VIGO_ENABLE_ADBLOCK)` |

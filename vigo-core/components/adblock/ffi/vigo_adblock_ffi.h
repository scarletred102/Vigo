// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

// C FFI header for the Rust adblock engine (vigo_adblock crate).
// This header must stay synchronised with the Rust FFI exports in
// vigo-core/rust/vigo_adblock/src/ffi.rs.

#ifndef VIGO_COMPONENTS_ADBLOCK_FFI_VIGO_ADBLOCK_FFI_H_
#define VIGO_COMPONENTS_ADBLOCK_FFI_VIGO_ADBLOCK_FFI_H_

#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// Create a new adblock engine instance. Returns an opaque pointer.
// Caller owns the returned pointer and must call vigo_adblock_destroy().
void* vigo_adblock_create(void);

// Destroy an adblock engine instance created by vigo_adblock_create().
void vigo_adblock_destroy(void* engine);

// Load a filter list from a UTF-8 encoded string buffer.
// |engine|: opaque engine pointer from vigo_adblock_create().
// |rules|: null-terminated UTF-8 filter list content.
// Returns true on success.
bool vigo_adblock_load_rules(void* engine, const char* rules);

// Check whether |url| should be blocked given |source_url|.
// Both are null-terminated UTF-8 C strings.
// Returns true if the URL should be blocked.
bool vigo_adblock_check_url(void* engine,
                             const char* url,
                             const char* source_url);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif  // VIGO_COMPONENTS_ADBLOCK_FFI_VIGO_ADBLOCK_FFI_H_

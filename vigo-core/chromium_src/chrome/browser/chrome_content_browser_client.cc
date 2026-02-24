// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.
//
// Vigo override: Replace ChromeContentBrowserClient construction to use
// VigoContentBrowserClient. This is the master injection point that
// connects all Vigo browser-process customisations to Chromium.
//
// Shadow pattern: #define ChromeContentBrowserClient to our subclass so
// that construction sites in chrome_browser_main.cc produce a Vigo
// instance instead.

#define ChromeContentBrowserClient VigoContentBrowserClient

#include "chrome/browser/chrome_content_browser_client.cc"  // NOLINT

#undef ChromeContentBrowserClient

// Pull in Vigo's implementation so the linker has our overridden vtable.
#include "vigo/browser/vigo_content_browser_client.h"

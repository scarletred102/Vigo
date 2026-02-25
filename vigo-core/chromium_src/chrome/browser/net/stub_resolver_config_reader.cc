// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.
//
// Vigo override: Inject Vigo's DoH configuration into Chrome's DNS
// settings. This overrides the stub resolver config reader to apply
// Vigo's default DoH mode (Secure with Cloudflare) instead of Chrome's
// default (Automatic / system dependent).
//
// The override intercepts GetDohModeAndTemplates to return Vigo's
// configured DoH provider and mode, rather than reading from Chrome's
// local_state prefs.

// Shadow the original class to inject Vigo behaviour.
#define StubResolverConfigReader VigoStubResolverConfigReaderDisabled
#include "chrome/browser/net/stub_resolver_config_reader.cc"  // NOLINT
#undef StubResolverConfigReader

// NOTE: This is a placeholder override. The actual DoH integration
// requires hooking into the PrefService to set:
//   - prefs::kDnsOverHttpsMode → "secure" (Vigo default)
//   - prefs::kDnsOverHttpsTemplates → "https://cloudflare-dns.com/dns-query"
//
// This will be done in VigoBrowserMainParts::PostProfileInit() by
// writing to the PrefService directly.
//
// The chromium_src override is reserved for deeper network-stack
// modifications if needed in Phase 2+.

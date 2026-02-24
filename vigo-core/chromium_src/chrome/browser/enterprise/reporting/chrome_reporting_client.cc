// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.
//
// Vigo override: disable Chrome reporting / feedback uploads.
// Chrome sends crash reports, performance data, and usage stats to
// Google. Vigo strips this entirely — no telemetry without explicit
// user opt-in.

#define ChromeReportingClient VigoDisabledChromeReportingClient

#include "chrome/browser/enterprise/reporting/chrome_reporting_client.cc"  // NOLINT

#undef ChromeReportingClient

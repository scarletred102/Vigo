// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.
//
// Vigo override: disable Chrome UMA metrics reporting.
// This prevents any telemetry from being sent to Google servers.
// The shadow pattern renames the original class so it is never
// instantiated, and Vigo's stub implementation is used instead.

// Rename the original class to prevent instantiation.
#define ChromeMetricsServiceClient VigoDisabledChromeMetricsServiceClient

// Include the original source — the #define above renames the class
// throughout the translation unit.
#include "chrome/browser/metrics/chrome_metrics_service_client.cc"  // NOLINT

#undef ChromeMetricsServiceClient

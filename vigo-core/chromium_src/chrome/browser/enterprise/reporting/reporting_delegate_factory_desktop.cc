// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.
//
// Vigo override: disable Chrome enterprise reporting to Google servers.
// Chrome sends crash reports, performance data, and usage stats to
// Google via the enterprise reporting pipeline. Vigo strips this
// entirely — no telemetry without explicit user opt-in.
//
// In M147, the enterprise reporting entry point on desktop is
// ReportingDelegateFactoryDesktop (the old ChromeReportingClient
// was removed). This shadow overrides the factory to prevent
// reporting delegate instantiation.

#define ReportingDelegateFactoryDesktop VigoDisabledReportingDelegateFactoryDesktop

#include "chrome/browser/enterprise/reporting/reporting_delegate_factory_desktop.cc"  // NOLINT

#undef ReportingDelegateFactoryDesktop

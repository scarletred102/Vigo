// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.
//
// Vigo override: disable Google Cloud Messaging (GCM) / Firebase Cloud
// Messaging (FCM) push notifications via Google infrastructure.
// Vigo does not route any push traffic through Google servers.

#define GCMDriver VigoDisabledGCMDriver

#include "components/gcm_driver/gcm_driver.cc"  // NOLINT

#undef GCMDriver

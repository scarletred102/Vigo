// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.
//
// Vigo override: disable Chrome Variations (Finch) service.
// Chrome Variations is Google's A/B experiment framework that phones
// home to Google servers. Vigo removes it to prevent tracking and
// ensure consistent browser behaviour across all users.

#define VariationsService VigoDisabledVariationsService

#include "components/variations/service/variations_service.cc"  // NOLINT

#undef VariationsService

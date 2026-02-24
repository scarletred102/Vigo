// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.
//
// Vigo override: disable Promotion / What's New promo tabs.
// Chrome opens Google promotional tabs on certain triggers (first run,
// updates, etc.). Vigo removes this to avoid phoning home.

#define PromoService VigoDisabledPromoService

#include "chrome/browser/promos/promo_service.cc"  // NOLINT

#undef PromoService

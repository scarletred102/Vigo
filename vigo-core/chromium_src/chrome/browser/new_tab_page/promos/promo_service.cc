// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.
//
// Vigo override: disable Promotion / What's New promo tabs.
// Chrome opens Google promotional tabs on certain triggers (first run,
// updates, etc.). Vigo removes this to avoid phoning home.
//
// In M147, PromoService moved from chrome/browser/promos/ to
// chrome/browser/new_tab_page/promos/. This shadow override targets
// the correct upstream path.

#define PromoService VigoDisabledPromoService

#include "chrome/browser/new_tab_page/promos/promo_service.cc"  // NOLINT

#undef PromoService

// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.
//
// Vigo override: strip Google RLZ tracking from the omnibox.
// RLZ is a Google promotional tag system that embeds tracking
// identifiers into search queries. Vigo removes it entirely.

#define RLZTracker VigoDisabledRLZTracker

#include "chrome/browser/rlz/chrome_rlz_tracker_delegate.cc"  // NOLINT

#undef RLZTracker

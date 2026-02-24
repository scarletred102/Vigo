// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.
//
// Vigo override: disable Chrome sign-in and Google account integration.
// Vigo uses its own self-hosted sync system. Google account sign-in
// in the browser chrome (not web pages) is completely removed.

#define SigninManager VigoDisabledSigninManager

#include "chrome/browser/signin/signin_manager.cc"  // NOLINT

#undef SigninManager

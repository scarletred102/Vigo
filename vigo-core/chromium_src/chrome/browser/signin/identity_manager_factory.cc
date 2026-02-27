// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.
//
// Vigo override: disable Chrome sign-in and Google account integration.
// Vigo uses its own self-hosted sync system. Google account sign-in
// in the browser chrome (not web pages) is completely removed.
//
// In M147, the desktop SigninManager was removed; sign-in is now
// managed via IdentityManagerFactory which creates the
// signin::IdentityManager. This shadow overrides the factory to
// prevent Google sign-in infrastructure from being wired up.

#define IdentityManagerFactory VigoDisabledIdentityManagerFactory

#include "chrome/browser/signin/identity_manager_factory.cc"  // NOLINT

#undef IdentityManagerFactory

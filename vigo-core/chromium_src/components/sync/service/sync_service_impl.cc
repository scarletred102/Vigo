// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.
//
// Vigo override: disable Chrome's built-in sync service (Google Sync).
// Vigo replaces Chrome Sync with its own E2E encrypted, self-hosted
// sync system (Phase 3). Google's sync infrastructure is fully removed.

#define SyncServiceImpl VigoDisabledSyncServiceImpl

#include "components/sync/service/sync_service_impl.cc"  // NOLINT

#undef SyncServiceImpl

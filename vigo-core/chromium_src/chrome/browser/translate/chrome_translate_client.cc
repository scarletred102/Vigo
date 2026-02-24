// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.
//
// Vigo override: disable Google Translate integration.
// Chrome's translate service sends page content to Google servers for
// translation. Vigo removes server-side translation. Local translation
// (if implemented later) will use on-device models.

#define TranslateRankerFactory VigoDisabledTranslateRankerFactory

#include "chrome/browser/translate/chrome_translate_client.cc"  // NOLINT

#undef TranslateRankerFactory

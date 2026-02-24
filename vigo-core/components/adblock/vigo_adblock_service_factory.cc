// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/adblock/vigo_adblock_service_factory.h"

#include <memory>
#include <utility>

#include "base/no_destructor.h"
#include "base/path_service.h"
#include "chrome/browser/profiles/profile.h"
#include "chrome/common/chrome_paths.h"
#include "components/keyed_service/content/browser_context_dependency_manager.h"
#include "vigo/components/adblock/vigo_adblock_service.h"
#include "vigo/components/adblock/vigo_filter_list_manager.h"

namespace vigo {
namespace adblock {

// static
VigoAdblockServiceFactory* VigoAdblockServiceFactory::GetInstance() {
  static base::NoDestructor<VigoAdblockServiceFactory> instance;
  return instance.get();
}

// static
VigoAdblockService* VigoAdblockServiceFactory::GetForBrowserContext(
    content::BrowserContext* context) {
  return static_cast<VigoAdblockService*>(
      GetInstance()->GetServiceForBrowserContext(context, /*create=*/true));
}

VigoAdblockServiceFactory::VigoAdblockServiceFactory()
    : BrowserContextKeyedServiceFactory(
          "VigoAdblockService",
          BrowserContextDependencyManager::GetInstance()) {}

VigoAdblockServiceFactory::~VigoAdblockServiceFactory() = default;

std::unique_ptr<KeyedService>
VigoAdblockServiceFactory::BuildServiceInstanceForBrowserContext(
    content::BrowserContext* context) const {
  VLOG(1) << "VigoAdblockServiceFactory: Building service for context";

  auto service = std::make_unique<VigoAdblockService>();

  // Determine the user data directory for filter list caching.
  Profile* profile = Profile::FromBrowserContext(context);
  base::FilePath user_data_dir = profile->GetPath();

  // Create and initialise the filter list manager.
  auto filter_list_manager = std::make_unique<VigoFilterListManager>();
  filter_list_manager->Init(user_data_dir);

  // Get the enabled filter list cache paths and initialise the Rust engine.
  std::vector<base::FilePath> filter_paths =
      filter_list_manager->GetEnabledFilterListPaths();
  service->Init(filter_paths);

  // Start auto-updating filter lists.
  filter_list_manager->StartAutoUpdate();

  // Transfer ownership of the filter list manager to the service.
  service->SetFilterListManager(std::move(filter_list_manager));

  return service;
}

content::BrowserContext* VigoAdblockServiceFactory::GetBrowserContextToUse(
    content::BrowserContext* context) const {
  // Share the adblock service between regular and incognito profiles.
  // Adblock filter lists are not secrets — they should work everywhere.
  // Use the original profile so we don't create duplicate engines.
  Profile* profile = Profile::FromBrowserContext(context);
  return profile->GetOriginalProfile();
}

}  // namespace adblock
}  // namespace vigo

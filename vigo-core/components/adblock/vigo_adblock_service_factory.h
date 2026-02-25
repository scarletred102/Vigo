// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_ADBLOCK_VIGO_ADBLOCK_SERVICE_FACTORY_H_
#define VIGO_COMPONENTS_ADBLOCK_VIGO_ADBLOCK_SERVICE_FACTORY_H_

#include "base/no_destructor.h"
#include "components/keyed_service/content/browser_context_keyed_service_factory.h"

namespace vigo {
namespace adblock {

class VigoAdblockService;

// Per-profile factory for VigoAdblockService. Each browser profile gets its
// own adblock service instance, so that per-profile enable/disable and custom
// filter lists are isolated.
//
// Usage:
//   auto* service =
//       VigoAdblockServiceFactory::GetForBrowserContext(browser_context);
//   if (service && service->IsReady())
//     service->ShouldBlock(url, source_url);
class VigoAdblockServiceFactory
    : public BrowserContextKeyedServiceFactory {
 public:
  // Returns the singleton factory instance.
  static VigoAdblockServiceFactory* GetInstance();

  // Returns the VigoAdblockService for the given |context|, creating one
  // if it does not yet exist. Returns nullptr if adblock is disabled.
  static VigoAdblockService* GetForBrowserContext(
      content::BrowserContext* context);

  VigoAdblockServiceFactory(const VigoAdblockServiceFactory&) = delete;
  VigoAdblockServiceFactory& operator=(const VigoAdblockServiceFactory&) =
      delete;

 private:
  friend class base::NoDestructor<VigoAdblockServiceFactory>;

  VigoAdblockServiceFactory();
  ~VigoAdblockServiceFactory() override;

  // BrowserContextKeyedServiceFactory overrides:
  std::unique_ptr<KeyedService> BuildServiceInstanceForBrowserContext(
      content::BrowserContext* context) const override;

  // Use the same service for incognito profiles (adblock should work
  // everywhere; filter lists are not per-profile secrets).
  content::BrowserContext* GetBrowserContextToUse(
      content::BrowserContext* context) const override;
};

}  // namespace adblock
}  // namespace vigo

#endif  // VIGO_COMPONENTS_ADBLOCK_VIGO_ADBLOCK_SERVICE_FACTORY_H_

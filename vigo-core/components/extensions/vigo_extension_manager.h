// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#ifndef VIGO_COMPONENTS_EXTENSIONS_VIGO_EXTENSION_MANAGER_H_
#define VIGO_COMPONENTS_EXTENSIONS_VIGO_EXTENSION_MANAGER_H_

#include <memory>
#include <string>
#include <unordered_map>
#include <vector>

#include "base/sequence_checker.h"

namespace vigo {
namespace extensions {

// Extension installation source.
enum class ExtensionSource {
  kChromeWebStore,   // Installed from Chrome Web Store.
  kVigoStore,        // Installed from Vigo's own store (future).
  kSideloaded,       // Loaded from disk (developer mode).
  kBuiltIn,          // Bundled with the browser.
};

// Extension state.
enum class ExtensionState {
  kEnabled,
  kDisabled,
  kBlocklisted,      // Blocked by Vigo for security reasons.
};

// Manifest V3 permission.
struct ExtensionPermission {
  std::string api_name;           // e.g., "storage", "tabs", "alarms"
  std::vector<std::string> hosts;  // Host permissions (if applicable).
  bool is_optional = false;
};

// A registered extension.
struct ExtensionInfo {
  // Extension ID (Chrome Web Store style hash or generated).
  std::string id;

  // Display name from manifest.
  std::string name;

  // Version from manifest.
  std::string version;

  // Description from manifest.
  std::string description;

  // Manifest version (must be 3 for Vigo).
  int manifest_version = 3;

  // Installation source.
  ExtensionSource source = ExtensionSource::kChromeWebStore;

  // Current state.
  ExtensionState state = ExtensionState::kEnabled;

  // Declared permissions.
  std::vector<ExtensionPermission> permissions;

  // Path to the unpacked extension on disk.
  std::string install_path;

  // Install timestamp.
  int64_t installed_at_ms = 0;

  // Whether the extension uses declarativeNetRequest.
  bool uses_dnr = false;

  // Whether the extension has a background service worker.
  bool has_service_worker = false;
};

// VigoExtensionManager manages the extension lifecycle with Vigo-specific
// security policies:
//
// Policies:
//   1. Only Manifest V3 extensions are allowed (MV2 blocked).
//   2. Extensions cannot read Vigo internal pages (vigo://*).
//   3. Extensions are sandboxed from the credential vault.
//   4. Extensions using remote code (eval, wasm from network) are blocked.
//   5. Background pages are migrated to service workers.
//   6. Content scripts are subject to the privacy engine's rules.
//   7. declarativeNetRequest rules cannot override Vigo's built-in
//      adblock rules (Vigo rules have higher priority).
//
// Thread safety: all public methods on UI sequence.
class VigoExtensionManager {
 public:
  VigoExtensionManager();
  ~VigoExtensionManager();

  VigoExtensionManager(const VigoExtensionManager&) = delete;
  VigoExtensionManager& operator=(const VigoExtensionManager&) = delete;

  // ─── Lifecycle ─────────────────────────────────────────────────

  // Initialise the extension system.
  bool Init();

  // ─── Installation ──────────────────────────────────────────────

  // Install an extension from a CRX file or unpacked directory.
  // Returns the extension ID on success, or empty string on failure.
  std::string Install(const std::string& path, ExtensionSource source);

  // Uninstall an extension by ID.
  bool Uninstall(const std::string& extension_id);

  // ─── State Management ─────────────────────────────────────────

  // Enable/disable an extension.
  bool Enable(const std::string& extension_id);
  bool Disable(const std::string& extension_id);

  // Get information about a specific extension.
  const ExtensionInfo* GetExtension(const std::string& extension_id) const;

  // Get all installed extensions.
  std::vector<ExtensionInfo> GetAll() const;

  // Get count of installed extensions.
  size_t Count() const;

  // ─── Permissions ──────────────────────────────────────────────

  // Check if an extension has a specific permission.
  bool HasPermission(const std::string& extension_id,
                     const std::string& api_name) const;

  // Grant an optional permission to an extension.
  bool GrantPermission(const std::string& extension_id,
                       const ExtensionPermission& permission);

  // Revoke an optional permission from an extension.
  bool RevokePermission(const std::string& extension_id,
                        const std::string& api_name);

  // ─── Security ─────────────────────────────────────────────────

  // Check if an extension passes Vigo's security requirements.
  // Returns true if the extension is safe to install.
  bool ValidateExtension(const ExtensionInfo& ext) const;

  // Check if a host pattern is allowed (blocks vigo://* etc.).
  bool IsHostAllowed(const std::string& host_pattern) const;

  // ─── Blocklist ────────────────────────────────────────────────

  // Add an extension to the local blocklist.
  void BlockExtension(const std::string& extension_id,
                      const std::string& reason);

  // Check if an extension is blocklisted.
  bool IsBlocked(const std::string& extension_id) const;

 private:
  bool initialized_ = false;

  // Extension registry: extension_id → ExtensionInfo.
  std::unordered_map<std::string, ExtensionInfo> extensions_;

  // Blocklist: extension_id → reason.
  std::unordered_map<std::string, std::string> blocklist_;

  SEQUENCE_CHECKER(sequence_checker_);
};

}  // namespace extensions
}  // namespace vigo

#endif  // VIGO_COMPONENTS_EXTENSIONS_VIGO_EXTENSION_MANAGER_H_

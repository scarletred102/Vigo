// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

#include "vigo/components/extensions/vigo_extension_manager.h"

#include <algorithm>

#include "base/logging.h"
#include "base/time/time.h"

namespace vigo {
namespace extensions {

namespace {

// Blocked host patterns — extensions cannot access these.
const char* kBlockedHosts[] = {
    "vigo://*",
    "vigo-internal://*",
    "chrome-extension://*",
};

// Generate a simple extension ID from the install path.
std::string GenerateExtensionId(const std::string& path) {
  // Simplified: hash the path to produce a 32-char hex string.
  // In production this would use SHA256 of the CRX public key.
  uint32_t hash = 0;
  for (char c : path) {
    hash = hash * 31 + static_cast<uint32_t>(c);
  }

  static const char kChars[] = "abcdefghijklmnop";
  std::string id;
  id.reserve(32);
  for (int i = 0; i < 32; ++i) {
    id.push_back(kChars[(hash >> (i % 16)) & 0xF]);
    hash = hash * 7 + i;
  }
  return id;
}

}  // namespace

VigoExtensionManager::VigoExtensionManager() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

VigoExtensionManager::~VigoExtensionManager() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
}

bool VigoExtensionManager::Init() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (initialized_) {
    return true;
  }

  VLOG(1) << "VigoExtensionManager: Initialising extension system";
  initialized_ = true;
  return true;
}

// ─── Installation ───────────────────────────────────────────────────────────

std::string VigoExtensionManager::Install(const std::string& path,
                                          ExtensionSource source) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  if (!initialized_) {
    LOG(ERROR) << "VigoExtensionManager: Not initialised";
    return {};
  }

  if (path.empty()) {
    LOG(ERROR) << "VigoExtensionManager: Empty install path";
    return {};
  }

  // Create extension info (in production, parse the manifest.json).
  ExtensionInfo ext;
  ext.id = GenerateExtensionId(path);
  ext.install_path = path;
  ext.source = source;
  ext.state = ExtensionState::kEnabled;
  ext.manifest_version = 3;
  ext.installed_at_ms =
      base::Time::Now().InMillisecondsSinceUnixEpoch();

  // Validate the extension meets Vigo's security requirements.
  if (!ValidateExtension(ext)) {
    LOG(ERROR) << "VigoExtensionManager: Extension failed validation";
    return {};
  }

  // Check blocklist.
  if (IsBlocked(ext.id)) {
    LOG(ERROR) << "VigoExtensionManager: Extension is blocklisted: "
               << ext.id;
    return {};
  }

  extensions_[ext.id] = std::move(ext);
  VLOG(1) << "VigoExtensionManager: Installed extension " << ext.id;
  return extensions_.find(ext.id) != extensions_.end()
             ? extensions_[ext.id].id
             : std::string();
}

bool VigoExtensionManager::Uninstall(const std::string& extension_id) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  auto it = extensions_.find(extension_id);
  if (it == extensions_.end()) {
    LOG(ERROR) << "VigoExtensionManager: Extension not found: "
               << extension_id;
    return false;
  }

  VLOG(1) << "VigoExtensionManager: Uninstalling " << extension_id;
  extensions_.erase(it);
  return true;
}

// ─── State Management ───────────────────────────────────────────────────────

bool VigoExtensionManager::Enable(const std::string& extension_id) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  auto it = extensions_.find(extension_id);
  if (it == extensions_.end()) {
    return false;
  }

  if (it->second.state == ExtensionState::kBlocklisted) {
    LOG(ERROR) << "VigoExtensionManager: Cannot enable blocklisted extension";
    return false;
  }

  it->second.state = ExtensionState::kEnabled;
  return true;
}

bool VigoExtensionManager::Disable(const std::string& extension_id) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  auto it = extensions_.find(extension_id);
  if (it == extensions_.end()) {
    return false;
  }

  it->second.state = ExtensionState::kDisabled;
  return true;
}

const ExtensionInfo* VigoExtensionManager::GetExtension(
    const std::string& extension_id) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  auto it = extensions_.find(extension_id);
  if (it == extensions_.end()) {
    return nullptr;
  }
  return &it->second;
}

std::vector<ExtensionInfo> VigoExtensionManager::GetAll() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  std::vector<ExtensionInfo> result;
  result.reserve(extensions_.size());
  for (const auto& [id, ext] : extensions_) {
    result.push_back(ext);
  }
  return result;
}

size_t VigoExtensionManager::Count() const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return extensions_.size();
}

// ─── Permissions ────────────────────────────────────────────────────────────

bool VigoExtensionManager::HasPermission(
    const std::string& extension_id,
    const std::string& api_name) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  auto it = extensions_.find(extension_id);
  if (it == extensions_.end()) {
    return false;
  }

  for (const auto& perm : it->second.permissions) {
    if (perm.api_name == api_name) {
      return true;
    }
  }
  return false;
}

bool VigoExtensionManager::GrantPermission(
    const std::string& extension_id,
    const ExtensionPermission& permission) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  auto it = extensions_.find(extension_id);
  if (it == extensions_.end()) {
    return false;
  }

  // Check host permissions against blocked hosts.
  for (const auto& host : permission.hosts) {
    if (!IsHostAllowed(host)) {
      LOG(ERROR) << "VigoExtensionManager: Host blocked: " << host;
      return false;
    }
  }

  it->second.permissions.push_back(permission);
  return true;
}

bool VigoExtensionManager::RevokePermission(
    const std::string& extension_id,
    const std::string& api_name) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  auto it = extensions_.find(extension_id);
  if (it == extensions_.end()) {
    return false;
  }

  auto& perms = it->second.permissions;
  auto perm_it = std::remove_if(
      perms.begin(), perms.end(),
      [&](const ExtensionPermission& p) { return p.api_name == api_name; });

  if (perm_it == perms.end()) {
    return false;
  }

  perms.erase(perm_it, perms.end());
  return true;
}

// ─── Security ───────────────────────────────────────────────────────────────

bool VigoExtensionManager::ValidateExtension(const ExtensionInfo& ext) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  // Rule 1: Only MV3 allowed.
  if (ext.manifest_version != 3) {
    LOG(ERROR) << "VigoExtensionManager: Manifest V"
               << ext.manifest_version
               << " not allowed (only V3)";
    return false;
  }

  // Rule 2: Check host permissions.
  for (const auto& perm : ext.permissions) {
    for (const auto& host : perm.hosts) {
      if (!IsHostAllowed(host)) {
        LOG(ERROR) << "VigoExtensionManager: Blocked host pattern: "
                   << host;
        return false;
      }
    }
  }

  return true;
}

bool VigoExtensionManager::IsHostAllowed(
    const std::string& host_pattern) const {
  for (const char* blocked : kBlockedHosts) {
    // Simple prefix match — in production, use proper pattern matching.
    if (host_pattern.find(blocked) != std::string::npos) {
      return false;
    }
    // Check if it starts with the scheme from blocked patterns.
    std::string scheme = std::string(blocked);
    auto colon = scheme.find(':');
    if (colon != std::string::npos) {
      scheme = scheme.substr(0, colon + 3);  // "vigo://"
      if (host_pattern.substr(0, scheme.size()) == scheme) {
        return false;
      }
    }
  }
  return true;
}

// ─── Blocklist ──────────────────────────────────────────────────────────────

void VigoExtensionManager::BlockExtension(const std::string& extension_id,
                                          const std::string& reason) {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);

  blocklist_[extension_id] = reason;

  // If the extension is installed, mark it as blocklisted.
  auto it = extensions_.find(extension_id);
  if (it != extensions_.end()) {
    it->second.state = ExtensionState::kBlocklisted;
  }

  VLOG(1) << "VigoExtensionManager: Blocked extension " << extension_id
          << ": " << reason;
}

bool VigoExtensionManager::IsBlocked(const std::string& extension_id) const {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  return blocklist_.find(extension_id) != blocklist_.end();
}

}  // namespace extensions
}  // namespace vigo

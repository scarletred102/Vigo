# Vigo Extension API

> Extension platform for the Vigo browser engine (Vex).

## Overview

Vigo extensions follow a manifest-based model inspired by the W3C Browser
Extensions specification (WebExtensions). Extensions are loaded from the
user's `~/.vigo/extensions/` directory. Each extension is a folder containing
a `manifest.json` and the referenced scripts / resources.

## Manifest Format

```json
{
  "manifest_version": 1,
  "name": "My Extension",
  "version": "1.0.0",
  "description": "A short description of the extension.",
  "permissions": ["tabs", "storage", "notifications"],
  "content_scripts": [
    {
      "matches": ["*://*.example.com/*"],
      "js": ["content.js"],
      "run_at": "document_idle"
    }
  ],
  "background": {
    "service_worker": "background.js"
  },
  "browser_action": {
    "default_popup": "popup.html",
    "default_icon": "icon.png",
    "default_title": "My Extension"
  }
}
```

### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `manifest_version` | `u32` | Must be `1`. |
| `name` | `string` | Human-readable extension name (1–60 chars). |
| `version` | `string` | Semver version string (e.g. "1.0.0"). |

### Optional Fields

| Field | Type | Description |
|-------|------|-------------|
| `description` | `string` | Short description (max 200 chars). |
| `permissions` | `string[]` | List of permission keys (see below). |
| `content_scripts` | `object[]` | Scripts injected into matching pages. |
| `background` | `object` | Background service worker configuration. |
| `browser_action` | `object` | Toolbar button + popup. |

## Permissions

| Key | Description |
|-----|-------------|
| `tabs` | Query and create tabs. |
| `storage` | Read/write extension-local storage. |
| `notifications` | Show desktop notifications. |
| `activeTab` | Access the currently active tab's DOM. |
| `webRequest` | Observe network requests. |
| `cookies` | Read/write cookies. |
| `history` | Access browser history. |
| `bookmarks` | Access bookmarks. |

## Content Scripts

Content scripts run in an **isolated world**: they share DOM access with the
page but have a separate JavaScript global object. They cannot access the
page's JS variables or functions.

### Match Patterns

Match patterns follow the format: `<scheme>://<host>/<path>`

| Pattern | Matches |
|---------|---------|
| `*://*.example.com/*` | Any page on example.com or subdomains. |
| `https://specific.com/page` | Only that exact URL. |
| `<all_urls>` | Every HTTP/HTTPS URL. |

### `run_at` Values

| Value | When |
|-------|------|
| `document_start` | Before any page script runs. |
| `document_end` | After DOM is ready (DOMContentLoaded). |
| `document_idle` | After page load (default). |

## Background Scripts

Each extension can have one background service worker. It runs in its own
`JsRuntime` (not tied to any tab). Available APIs:

- `vigo.tabs.query(filter)` — query open tabs.
- `vigo.tabs.create(opts)` — create a new tab.
- `vigo.storage.local.get(keys)` — read extension storage.
- `vigo.storage.local.set(items)` — write extension storage.
- `vigo.notifications.create(opts)` — show a notification.
- `vigo.runtime.sendMessage(msg)` — send message to popup/content.
- `vigo.runtime.onMessage.addListener(fn)` — listen for messages.

## Browser Action

An extension can show an icon in the browser's toolbar. Clicking the icon
opens a small popup (an HTML page rendered by the Vex engine).

The popup communicates with the background script via
`vigo.runtime.sendMessage()` / `vigo.runtime.onMessage`.

## Extension Lifecycle

1. **Load**: Vigo scans `~/.vigo/extensions/` on startup.
2. **Parse**: `manifest.json` is parsed and validated.
3. **Permission prompt**: On first install, the user is shown a dialog
   listing the requested permissions.
4. **Register**: Content scripts are registered for URL matching.
   Background worker is started.
5. **Page load**: When a page matches a content script pattern, the
   script is injected at the specified `run_at` time.
6. **Unload**: On browser exit, background workers are stopped.

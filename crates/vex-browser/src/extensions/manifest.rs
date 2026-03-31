// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Extension manifest parsing and validation.
//!
//! Each extension has a `manifest.json` that describes its name, version,
//! permissions, content scripts, background worker, and browser action.

use std::path::PathBuf;

use serde::Deserialize;

use super::content::ContentScriptDef;
use super::permissions::Permission;

/// Parsed and validated extension manifest.
#[derive(Debug, Clone)]
pub struct ExtensionManifest {
    /// Unique extension ID (derived from directory name).
    pub id: String,
    /// Manifest version (supported: 1, 2, 3).
    pub manifest_version: u32,
    /// Human-readable name.
    pub name: String,
    /// Semver version string.
    pub version: String,
    /// Short description.
    pub description: Option<String>,
    /// Requested permissions.
    pub permissions: Vec<Permission>,
    /// Content script definitions.
    pub content_scripts: Vec<ContentScriptDef>,
    /// Background service worker path (relative to extension root).
    pub background_worker: Option<PathBuf>,
    /// Browser action configuration.
    pub browser_action: Option<BrowserActionDef>,
    /// Root directory of the extension on disk.
    pub root_dir: PathBuf,
}

/// Raw manifest as deserialized from JSON.
#[derive(Debug, Deserialize)]
pub struct RawManifest {
    pub manifest_version: u32,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub content_scripts: Vec<RawContentScript>,
    #[serde(default)]
    pub background: Option<RawBackground>,
    #[serde(default)]
    pub browser_action: Option<RawBrowserAction>,
}

/// Raw content script entry from manifest JSON.
#[derive(Debug, Deserialize)]
pub struct RawContentScript {
    pub matches: Vec<String>,
    #[serde(default)]
    pub js: Vec<String>,
    #[serde(default)]
    pub run_at: Option<String>,
}

/// Raw background configuration from manifest JSON.
#[derive(Debug, Deserialize)]
pub struct RawBackground {
    #[serde(default)]
    pub service_worker: Option<String>,
    #[serde(default)]
    pub scripts: Vec<String>,
}

/// Raw browser action configuration from manifest JSON.
#[derive(Debug, Deserialize)]
pub struct RawBrowserAction {
    #[serde(default)]
    pub default_popup: Option<String>,
    #[serde(default)]
    pub default_icon: Option<String>,
    #[serde(default)]
    pub default_title: Option<String>,
}

/// Browser action definition within the manifest.
#[derive(Debug, Clone)]
pub struct BrowserActionDef {
    /// Path to the popup HTML (relative to extension root).
    pub popup: Option<PathBuf>,
    /// Path to the icon (relative to extension root).
    pub icon: Option<PathBuf>,
    /// Tooltip title.
    pub title: Option<String>,
}

/// Errors that can occur during manifest parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    /// JSON parse error.
    InvalidJson(String),
    /// Unsupported manifest version.
    UnsupportedVersion(u32),
    /// Missing required field.
    MissingField(&'static str),
    /// Invalid field value.
    InvalidField { field: &'static str, reason: String },
    /// Unknown permission key.
    UnknownPermission(String),
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJson(e) => write!(f, "invalid manifest JSON: {e}"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported manifest_version: {v}"),
            Self::MissingField(field) => write!(f, "missing required field: {field}"),
            Self::InvalidField { field, reason } => write!(f, "invalid {field}: {reason}"),
            Self::UnknownPermission(p) => write!(f, "unknown permission: {p}"),
        }
    }
}

/// Parse a raw manifest JSON string into a validated [`ExtensionManifest`].
pub fn parse_manifest(
    json: &str,
    extension_id: &str,
    root_dir: PathBuf,
) -> Result<ExtensionManifest, ManifestError> {
    let raw: RawManifest =
        serde_json::from_str(json).map_err(|e| ManifestError::InvalidJson(e.to_string()))?;

    // Validate manifest version.
    if !matches!(raw.manifest_version, 1..=3) {
        return Err(ManifestError::UnsupportedVersion(raw.manifest_version));
    }

    // Validate name.
    if raw.name.is_empty() || raw.name.len() > 60 {
        return Err(ManifestError::InvalidField {
            field: "name",
            reason: "must be 1–60 characters".to_owned(),
        });
    }

    // Validate version.
    if raw.version.is_empty() {
        return Err(ManifestError::MissingField("version"));
    }

    // Parse permissions.
    let permissions: Vec<Permission> = raw
        .permissions
        .iter()
        .map(|s| Permission::from_key(s).ok_or_else(|| ManifestError::UnknownPermission(s.clone())))
        .collect::<Result<Vec<_>, _>>()?;

    // Parse content scripts.
    let content_scripts: Vec<ContentScriptDef> = raw
        .content_scripts
        .into_iter()
        .map(|cs| ContentScriptDef {
            matches: cs.matches,
            js_files: cs.js.into_iter().map(PathBuf::from).collect(),
            run_at: cs
                .run_at
                .as_deref()
                .map(super::content::InjectionTime::parse_str)
                .unwrap_or(super::content::InjectionTime::DocumentIdle),
        })
        .collect();

    // Parse background.
    // MV3: `background.service_worker`
    // MV2: `background.scripts` (we use the first script as entrypoint)
    let background_worker = raw.background.and_then(|bg| {
        if let Some(sw) = bg.service_worker {
            Some(PathBuf::from(sw))
        } else {
            bg.scripts.into_iter().next().map(PathBuf::from)
        }
    });

    // Parse browser action.
    let browser_action = raw.browser_action.map(|ba| BrowserActionDef {
        popup: ba.default_popup.map(PathBuf::from),
        icon: ba.default_icon.map(PathBuf::from),
        title: ba.default_title,
    });

    Ok(ExtensionManifest {
        id: extension_id.to_owned(),
        manifest_version: raw.manifest_version,
        name: raw.name,
        version: raw.version,
        description: raw.description,
        permissions,
        content_scripts,
        background_worker,
        browser_action,
        root_dir,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest_json() -> &'static str {
        r#"{
            "manifest_version": 1,
            "name": "Test Extension",
            "version": "1.0.0",
            "description": "A test extension",
            "permissions": ["tabs", "storage"],
            "content_scripts": [{
                "matches": ["*://*.example.com/*"],
                "js": ["content.js"],
                "run_at": "document_idle"
            }],
            "background": {
                "service_worker": "background.js"
            },
            "browser_action": {
                "default_popup": "popup.html",
                "default_icon": "icon.png",
                "default_title": "Test"
            }
        }"#
    }

    #[test]
    fn parse_valid_manifest() {
        let manifest = parse_manifest(
            sample_manifest_json(),
            "test-ext",
            PathBuf::from("/ext/test"),
        )
        .unwrap();
        assert_eq!(manifest.name, "Test Extension");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.permissions.len(), 2);
        assert_eq!(manifest.content_scripts.len(), 1);
        assert!(manifest.background_worker.is_some());
        assert!(manifest.browser_action.is_some());
    }

    #[test]
    fn parse_minimal_manifest() {
        let json = r#"{ "manifest_version": 1, "name": "Min", "version": "0.1" }"#;
        let manifest = parse_manifest(json, "min", PathBuf::from("/ext/min")).unwrap();
        assert_eq!(manifest.name, "Min");
        assert!(manifest.permissions.is_empty());
        assert!(manifest.content_scripts.is_empty());
    }

    #[test]
    fn parse_mv3_service_worker_manifest() {
        let json = r#"{
            "manifest_version": 3,
            "name": "MV3 Ext",
            "version": "1.0.0",
            "background": { "service_worker": "sw.js" }
        }"#;
        let manifest = parse_manifest(json, "mv3-ext", PathBuf::from("/ext/mv3")).unwrap();
        assert_eq!(manifest.manifest_version, 3);
        assert_eq!(manifest.background_worker, Some(PathBuf::from("sw.js")));
    }

    #[test]
    fn parse_mv2_background_scripts_manifest() {
        let json = r#"{
            "manifest_version": 2,
            "name": "MV2 Ext",
            "version": "1.0.0",
            "background": { "scripts": ["bg1.js", "bg2.js"] }
        }"#;
        let manifest = parse_manifest(json, "mv2-ext", PathBuf::from("/ext/mv2")).unwrap();
        assert_eq!(manifest.manifest_version, 2);
        // First script is used as the entrypoint in current runtime model.
        assert_eq!(manifest.background_worker, Some(PathBuf::from("bg1.js")));
    }

    #[test]
    fn reject_invalid_version() {
        let json = r#"{ "manifest_version": 99, "name": "Bad", "version": "1.0" }"#;
        let err = parse_manifest(json, "bad", PathBuf::from("/")).unwrap_err();
        assert!(matches!(err, ManifestError::UnsupportedVersion(99)));
    }

    #[test]
    fn reject_empty_name() {
        let json = r#"{ "manifest_version": 1, "name": "", "version": "1.0" }"#;
        let err = parse_manifest(json, "bad", PathBuf::from("/")).unwrap_err();
        assert!(matches!(
            err,
            ManifestError::InvalidField { field: "name", .. }
        ));
    }

    #[test]
    fn reject_unknown_permission() {
        let json = r#"{ "manifest_version": 1, "name": "Bad", "version": "1.0", "permissions": ["unknown_perm"] }"#;
        let err = parse_manifest(json, "bad", PathBuf::from("/")).unwrap_err();
        assert!(matches!(err, ManifestError::UnknownPermission(_)));
    }

    #[test]
    fn reject_invalid_json() {
        let err = parse_manifest("not json", "bad", PathBuf::from("/")).unwrap_err();
        assert!(matches!(err, ManifestError::InvalidJson(_)));
    }

    #[test]
    fn manifest_error_display() {
        let err = ManifestError::UnsupportedVersion(5);
        assert!(err.to_string().contains("5"));
    }
}

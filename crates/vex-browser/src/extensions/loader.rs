// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Extension loader — discovers, loads, and manages extensions from disk.
//!
//! Each extension lives in its own subdirectory under the extensions root,
//! containing a `manifest.json` plus the JS/CSS/HTML files it references.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::content::{ContentScript, ContentScriptDef, MatchPattern};
use super::manifest::{parse_manifest, ExtensionManifest, ManifestError};
use super::messaging::{MessageTarget, MessagingHub, RuntimeMessage};
use super::permissions::PermissionSet;

/// State of a loaded extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionState {
    /// Installed but not active.
    Disabled,
    /// Active and running.
    Active,
    /// Failed to load — see error message.
    Error,
}

/// A loaded extension.
#[derive(Debug, Clone)]
pub struct Extension {
    /// Parsed manifest.
    pub manifest: ExtensionManifest,
    /// Current state.
    pub state: ExtensionState,
    /// Granted permissions (starts empty, populated on user approval).
    pub permissions: PermissionSet,
}

impl Extension {
    /// Create a new extension in disabled state.
    fn new(manifest: ExtensionManifest) -> Self {
        Self {
            manifest,
            state: ExtensionState::Disabled,
            permissions: PermissionSet::new(),
        }
    }

    /// The unique extension ID.
    pub fn id(&self) -> &str {
        &self.manifest.id
    }

    /// Whether the extension is active.
    pub fn is_active(&self) -> bool {
        self.state == ExtensionState::Active
    }
}

/// Error variants for extension loading.
#[derive(Debug, Clone)]
pub enum LoadError {
    /// manifest.json not found or not readable.
    ManifestNotFound(PathBuf),
    /// Manifest parsing failed.
    ManifestInvalid(ManifestError),
    /// Extension ID already loaded.
    DuplicateId(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ManifestNotFound(p) => write!(f, "manifest.json not found: {}", p.display()),
            Self::ManifestInvalid(e) => write!(f, "invalid manifest: {e}"),
            Self::DuplicateId(id) => write!(f, "duplicate extension id: {id}"),
        }
    }
}

/// Manages all loaded extensions.
pub struct ExtensionLoader {
    /// Root directory containing extension subdirectories.
    extensions_dir: PathBuf,
    /// Loaded extensions keyed by ID.
    extensions: HashMap<String, Extension>,
    /// Compiled content scripts from all active extensions.
    content_scripts: Vec<ContentScript>,
    /// Runtime messaging bus (`vigo.runtime.sendMessage` / `onMessage`).
    messaging_hub: MessagingHub,
}

impl ExtensionLoader {
    /// Create a new loader pointing at the given extensions directory.
    pub fn new(extensions_dir: PathBuf) -> Self {
        Self {
            extensions_dir,
            extensions: HashMap::new(),
            content_scripts: Vec::new(),
            messaging_hub: MessagingHub::new(),
        }
    }

    /// Load a single extension from a directory.
    ///
    /// The directory must contain a `manifest.json`.
    pub fn load_extension(&mut self, ext_dir: &Path) -> Result<String, LoadError> {
        let manifest_path = ext_dir.join("manifest.json");
        let json = std::fs::read_to_string(&manifest_path)
            .map_err(|_| LoadError::ManifestNotFound(manifest_path))?;

        // Derive ID from directory name.
        let id = ext_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_owned();

        if self.extensions.contains_key(&id) {
            return Err(LoadError::DuplicateId(id));
        }

        let manifest =
            parse_manifest(&json, &id, ext_dir.to_owned()).map_err(LoadError::ManifestInvalid)?;

        let extension = Extension::new(manifest);
        self.extensions.insert(id.clone(), extension);
        Ok(id)
    }

    /// Discover and load all extensions in the extensions directory.
    ///
    /// Returns the IDs of successfully loaded extensions and any errors.
    pub fn load_all(&mut self) -> (Vec<String>, Vec<LoadError>) {
        let mut loaded = Vec::new();
        let mut errors = Vec::new();

        let entries = match std::fs::read_dir(&self.extensions_dir) {
            Ok(entries) => entries,
            Err(_) => return (loaded, errors),
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                match self.load_extension(&path) {
                    Ok(id) => loaded.push(id),
                    Err(e) => errors.push(e),
                }
            }
        }

        (loaded, errors)
    }

    /// Enable an extension (set state to Active, compile content scripts).
    pub fn enable(&mut self, id: &str) -> bool {
        let Some(ext) = self.extensions.get_mut(id) else {
            return false;
        };
        ext.state = ExtensionState::Active;
        self.messaging_hub.register_extension(id);
        self.rebuild_content_scripts();
        true
    }

    /// Disable an extension and remove its content scripts.
    pub fn disable(&mut self, id: &str) -> bool {
        let Some(ext) = self.extensions.get_mut(id) else {
            return false;
        };
        ext.state = ExtensionState::Disabled;
        self.messaging_hub.unregister_extension(id);
        self.rebuild_content_scripts();
        true
    }

    /// Grant all requested permissions for an extension (auto-approve).
    pub fn approve_permissions(&mut self, id: &str) {
        if let Some(ext) = self.extensions.get_mut(id) {
            ext.permissions.grant_all(&ext.manifest.permissions);
        }
    }

    /// Get an extension by ID.
    pub fn get(&self, id: &str) -> Option<&Extension> {
        self.extensions.get(id)
    }

    /// All loaded extension IDs.
    pub fn loaded_ids(&self) -> Vec<String> {
        self.extensions.keys().cloned().collect()
    }

    /// All active extension IDs.
    pub fn active_ids(&self) -> Vec<String> {
        self.extensions
            .iter()
            .filter(|(_, e)| e.is_active())
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Number of loaded extensions.
    pub fn count(&self) -> usize {
        self.extensions.len()
    }

    /// Get all compiled content scripts for active extensions.
    pub fn content_scripts(&self) -> &[ContentScript] {
        &self.content_scripts
    }

    /// Get content scripts that match a given URL.
    pub fn content_scripts_for_url(&self, url: &str) -> Vec<&ContentScript> {
        self.content_scripts
            .iter()
            .filter(|cs| cs.matches_url(url))
            .collect()
    }

    /// Rebuild compiled content scripts from all active extensions.
    ///
    /// This reads JS files from disk. Content scripts whose files
    /// cannot be read are silently skipped.
    fn rebuild_content_scripts(&mut self) {
        self.content_scripts.clear();

        for ext in self.extensions.values() {
            if ext.state != ExtensionState::Active {
                continue;
            }
            for cs_def in &ext.manifest.content_scripts {
                if let Some(cs) = self.compile_content_script(ext.id(), cs_def) {
                    self.content_scripts.push(cs);
                }
            }
        }
    }

    /// Compile a content script definition into a ready-to-inject script.
    fn compile_content_script(
        &self,
        extension_id: &str,
        def: &ContentScriptDef,
    ) -> Option<ContentScript> {
        // Parse match patterns.
        let patterns: Vec<MatchPattern> = def
            .matches
            .iter()
            .filter_map(|p| MatchPattern::parse(p))
            .collect();

        if patterns.is_empty() {
            return None;
        }

        // Read and concatenate all JS files.
        let ext = self.extensions.get(extension_id)?;
        let mut source = String::new();
        for js_path in &def.js_files {
            let full_path = ext.manifest.root_dir.join(js_path);
            if let Ok(js) = std::fs::read_to_string(&full_path) {
                if !source.is_empty() {
                    source.push('\n');
                }
                source.push_str(&js);
            }
        }

        if source.is_empty() {
            return None;
        }

        Some(ContentScript {
            extension_id: extension_id.to_owned(),
            patterns,
            source,
            run_at: def.run_at,
        })
    }

    /// The extensions root directory.
    pub fn extensions_dir(&self) -> &Path {
        &self.extensions_dir
    }

    /// Collect browser actions from all active extensions (Task 60).
    pub fn browser_actions(&self) -> Vec<super::action::BrowserAction> {
        let mut actions = Vec::new();
        for ext in self.extensions.values() {
            if !ext.is_active() {
                continue;
            }
            if let Some(ref ba_def) = ext.manifest.browser_action {
                let title = ba_def
                    .title
                    .clone()
                    .unwrap_or_else(|| ext.manifest.name.clone());
                let has_popup = ba_def.popup.is_some();
                actions.push(super::action::BrowserAction::new(
                    ext.id().to_owned(),
                    title,
                    has_popup,
                ));
            }
        }
        actions
    }

    /// Read background worker source for an extension (Task 59).
    ///
    /// Returns `None` if no background worker is configured.
    pub fn background_source(&self, id: &str) -> Option<String> {
        let ext = self.extensions.get(id)?;
        let worker_path = ext.manifest.background_worker.as_ref()?;
        let full_path = ext.manifest.root_dir.join(worker_path);
        std::fs::read_to_string(full_path).ok()
    }

    /// Route a runtime message from one extension to another (or broadcast).
    ///
    /// Returns number of recipients that received the message.
    pub fn send_runtime_message(
        &mut self,
        from_extension_id: &str,
        target_extension_id: Option<&str>,
        payload: String,
    ) -> usize {
        let Some(sender) = self.extensions.get(from_extension_id) else {
            return 0;
        };
        if !sender.is_active() {
            return 0;
        }

        let target = match target_extension_id {
            Some(id) => {
                let Some(ext) = self.extensions.get(id) else {
                    return 0;
                };
                if !ext.is_active() {
                    return 0;
                }
                MessageTarget::Extension(id.to_owned())
            }
            None => MessageTarget::Broadcast,
        };

        self.messaging_hub
            .send_message(from_extension_id, target, payload)
    }

    /// Poll the next pending runtime message for an active extension.
    pub fn poll_runtime_message(&mut self, extension_id: &str) -> Option<RuntimeMessage> {
        if !self
            .extensions
            .get(extension_id)
            .is_some_and(Extension::is_active)
        {
            return None;
        }
        self.messaging_hub.poll_message(extension_id)
    }

    /// Number of pending runtime messages for an extension inbox.
    pub fn pending_runtime_messages(&self, extension_id: &str) -> usize {
        self.messaging_hub.pending_count(extension_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Create a temp extension directory with a manifest and content script.
    fn create_test_extension(base: &Path, id: &str) -> PathBuf {
        let ext_dir = base.join(id);
        fs::create_dir_all(&ext_dir).unwrap();

        let manifest = format!(
            r#"{{
            "manifest_version": 1,
            "name": "{id}",
            "version": "1.0.0",
            "permissions": ["tabs"],
            "content_scripts": [{{
                "matches": ["*://*.example.com/*"],
                "js": ["content.js"]
            }}]
        }}"#
        );
        fs::write(ext_dir.join("manifest.json"), manifest).unwrap();
        fs::write(ext_dir.join("content.js"), "console.log('hello');").unwrap();
        ext_dir
    }

    #[test]
    fn load_extension_from_dir() {
        let tmp = std::env::temp_dir().join("vex_ext_test_load");
        let _ = fs::remove_dir_all(&tmp);
        let ext_dir = create_test_extension(&tmp, "test-ext");

        let mut loader = ExtensionLoader::new(tmp.clone());
        let id = loader.load_extension(&ext_dir).unwrap();
        assert_eq!(id, "test-ext");
        assert_eq!(loader.count(), 1);

        let ext = loader.get("test-ext").unwrap();
        assert_eq!(ext.manifest.name, "test-ext");
        assert_eq!(ext.state, ExtensionState::Disabled);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn reject_missing_manifest() {
        let tmp = std::env::temp_dir().join("vex_ext_test_missing");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let empty_dir = tmp.join("empty");
        fs::create_dir_all(&empty_dir).unwrap();

        let mut loader = ExtensionLoader::new(tmp.clone());
        let err = loader.load_extension(&empty_dir).unwrap_err();
        assert!(matches!(err, LoadError::ManifestNotFound(_)));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn reject_duplicate_id() {
        let tmp = std::env::temp_dir().join("vex_ext_test_dup");
        let _ = fs::remove_dir_all(&tmp);
        let ext_dir = create_test_extension(&tmp, "dup-ext");

        let mut loader = ExtensionLoader::new(tmp.clone());
        loader.load_extension(&ext_dir).unwrap();
        let err = loader.load_extension(&ext_dir).unwrap_err();
        assert!(matches!(err, LoadError::DuplicateId(_)));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn enable_compiles_content_scripts() {
        let tmp = std::env::temp_dir().join("vex_ext_test_enable");
        let _ = fs::remove_dir_all(&tmp);
        create_test_extension(&tmp, "cs-ext");

        let mut loader = ExtensionLoader::new(tmp.clone());
        loader.load_extension(&tmp.join("cs-ext")).unwrap();
        assert!(loader.content_scripts().is_empty());

        loader.enable("cs-ext");
        assert_eq!(loader.content_scripts().len(), 1);
        assert!(loader.content_scripts()[0].source.contains("console.log"));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn disable_removes_content_scripts() {
        let tmp = std::env::temp_dir().join("vex_ext_test_disable");
        let _ = fs::remove_dir_all(&tmp);
        create_test_extension(&tmp, "dis-ext");

        let mut loader = ExtensionLoader::new(tmp.clone());
        loader.load_extension(&tmp.join("dis-ext")).unwrap();
        loader.enable("dis-ext");
        assert_eq!(loader.content_scripts().len(), 1);

        loader.disable("dis-ext");
        assert!(loader.content_scripts().is_empty());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn runtime_messaging_routes_between_active_extensions() {
        let tmp = std::env::temp_dir().join("vex_ext_test_rt_msg");
        let _ = fs::remove_dir_all(&tmp);
        create_test_extension(&tmp, "ext-a");
        create_test_extension(&tmp, "ext-b");

        let mut loader = ExtensionLoader::new(tmp.clone());
        loader.load_extension(&tmp.join("ext-a")).unwrap();
        loader.load_extension(&tmp.join("ext-b")).unwrap();
        loader.enable("ext-a");
        loader.enable("ext-b");

        let delivered =
            loader.send_runtime_message("ext-a", Some("ext-b"), "{\"kind\":\"ping\"}".to_owned());
        assert_eq!(delivered, 1);
        assert_eq!(loader.pending_runtime_messages("ext-b"), 1);

        let msg = loader
            .poll_runtime_message("ext-b")
            .expect("message should be queued");
        assert_eq!(msg.from_extension_id, "ext-a");
        assert_eq!(msg.to_extension_id.as_deref(), Some("ext-b"));
        assert_eq!(msg.payload, "{\"kind\":\"ping\"}");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn runtime_messaging_blocks_disabled_sender() {
        let tmp = std::env::temp_dir().join("vex_ext_test_rt_msg_disabled");
        let _ = fs::remove_dir_all(&tmp);
        create_test_extension(&tmp, "ext-a");
        create_test_extension(&tmp, "ext-b");

        let mut loader = ExtensionLoader::new(tmp.clone());
        loader.load_extension(&tmp.join("ext-a")).unwrap();
        loader.load_extension(&tmp.join("ext-b")).unwrap();
        loader.enable("ext-b");

        let delivered = loader.send_runtime_message("ext-a", Some("ext-b"), "x".to_owned());
        assert_eq!(delivered, 0);
        assert_eq!(loader.pending_runtime_messages("ext-b"), 0);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn content_scripts_for_url_filtering() {
        let tmp = std::env::temp_dir().join("vex_ext_test_url");
        let _ = fs::remove_dir_all(&tmp);
        create_test_extension(&tmp, "url-ext");

        let mut loader = ExtensionLoader::new(tmp.clone());
        loader.load_extension(&tmp.join("url-ext")).unwrap();
        loader.enable("url-ext");

        let matches = loader.content_scripts_for_url("https://www.example.com/page");
        assert_eq!(matches.len(), 1);

        let no_matches = loader.content_scripts_for_url("https://other.test/");
        assert!(no_matches.is_empty());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn load_all_discovers_extensions() {
        let tmp = std::env::temp_dir().join("vex_ext_test_all");
        let _ = fs::remove_dir_all(&tmp);
        create_test_extension(&tmp, "ext-a");
        create_test_extension(&tmp, "ext-b");

        let mut loader = ExtensionLoader::new(tmp.clone());
        let (loaded, errors) = loader.load_all();
        assert_eq!(loaded.len(), 2);
        assert!(errors.is_empty());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn approve_permissions() {
        let tmp = std::env::temp_dir().join("vex_ext_test_perms");
        let _ = fs::remove_dir_all(&tmp);
        create_test_extension(&tmp, "perm-ext");

        let mut loader = ExtensionLoader::new(tmp.clone());
        loader.load_extension(&tmp.join("perm-ext")).unwrap();

        {
            let ext = loader.get("perm-ext").unwrap();
            assert_eq!(ext.permissions.count(), 0);
        }

        loader.approve_permissions("perm-ext");
        let ext = loader.get("perm-ext").unwrap();
        assert!(ext
            .permissions
            .has(super::super::permissions::Permission::Tabs));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn load_error_display() {
        let err = LoadError::DuplicateId("test".to_owned());
        assert!(err.to_string().contains("test"));
    }

    #[test]
    fn extension_state_transitions() {
        let tmp = std::env::temp_dir().join("vex_ext_test_state");
        let _ = fs::remove_dir_all(&tmp);
        create_test_extension(&tmp, "state-ext");

        let mut loader = ExtensionLoader::new(tmp.clone());
        loader.load_extension(&tmp.join("state-ext")).unwrap();

        // Starts disabled.
        assert!(!loader.get("state-ext").unwrap().is_active());
        assert!(loader.active_ids().is_empty());

        // Enable.
        assert!(loader.enable("state-ext"));
        assert!(loader.get("state-ext").unwrap().is_active());
        assert_eq!(loader.active_ids().len(), 1);

        // Disable.
        assert!(loader.disable("state-ext"));
        assert!(!loader.get("state-ext").unwrap().is_active());

        // Enable/disable nonexistent returns false.
        assert!(!loader.enable("does-not-exist"));

        let _ = fs::remove_dir_all(&tmp);
    }
}

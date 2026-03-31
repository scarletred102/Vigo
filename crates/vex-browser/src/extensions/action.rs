// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Browser action — toolbar icon, popup, badge for extensions.
//!
//! Each extension may define a `browser_action` in its manifest to add
//! a toolbar icon that can show a popup or trigger an action on click.

/// A browser action instance for a loaded extension.
#[derive(Debug, Clone)]
pub struct BrowserAction {
    /// Owning extension ID.
    pub extension_id: String,
    /// Tooltip text shown on hover.
    pub title: String,
    /// Optional badge text (e.g., "3" for blocked ads).
    pub badge_text: String,
    /// Badge background color as CSS hex (e.g., "#ff0000").
    pub badge_color: String,
    /// Whether the action is enabled (clickable).
    pub enabled: bool,
    /// True if a popup is configured.
    pub has_popup: bool,
    /// Popup HTML source (loaded from disk when needed).
    popup_source: Option<String>,
}

impl BrowserAction {
    /// Create a new browser action with default state.
    pub fn new(extension_id: String, title: String, has_popup: bool) -> Self {
        Self {
            extension_id,
            title,
            badge_text: String::new(),
            badge_color: "#4285f4".to_owned(),
            enabled: true,
            has_popup,
            popup_source: None,
        }
    }

    /// Set the badge text (short, e.g., "99+").
    pub fn set_badge_text(&mut self, text: &str) {
        // Truncate badge text to 4 characters.
        self.badge_text = text.chars().take(4).collect();
    }

    /// Set the badge background color.
    pub fn set_badge_color(&mut self, color: &str) {
        self.badge_color = color.to_owned();
    }

    /// Set the tooltip title.
    pub fn set_title(&mut self, title: &str) {
        self.title = title.to_owned();
    }

    /// Enable or disable the action.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Set the popup HTML source.
    pub fn set_popup_source(&mut self, source: String) {
        self.popup_source = Some(source);
        self.has_popup = true;
    }

    /// Get the popup HTML source, if available.
    pub fn popup_source(&self) -> Option<&str> {
        self.popup_source.as_deref()
    }

    /// Clear the popup — clicks will fire an event instead.
    pub fn remove_popup(&mut self) {
        self.popup_source = None;
        self.has_popup = false;
    }

    /// Whether this action has a non-empty badge.
    pub fn has_badge(&self) -> bool {
        !self.badge_text.is_empty()
    }
}

/// Collects browser actions from all active extensions.
#[derive(Debug, Default)]
pub struct ActionBar {
    actions: Vec<BrowserAction>,
}

impl ActionBar {
    /// Create an empty action bar.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a browser action.
    pub fn add(&mut self, action: BrowserAction) {
        self.actions.push(action);
    }

    /// Remove all actions for an extension.
    pub fn remove(&mut self, extension_id: &str) {
        self.actions.retain(|a| a.extension_id != extension_id);
    }

    /// Get all actions.
    pub fn actions(&self) -> &[BrowserAction] {
        &self.actions
    }

    /// Get a mutable reference to an action by extension ID.
    pub fn get_mut(&mut self, extension_id: &str) -> Option<&mut BrowserAction> {
        self.actions
            .iter_mut()
            .find(|a| a.extension_id == extension_id)
    }

    /// Number of visible actions.
    pub fn count(&self) -> usize {
        self.actions.len()
    }

    /// Handle a click on the action at the given index.
    ///
    /// Returns the extension ID and whether a popup should be shown.
    pub fn click(&self, index: usize) -> Option<ActionClick> {
        let action = self.actions.get(index)?;
        if !action.enabled {
            return None;
        }
        Some(ActionClick {
            extension_id: action.extension_id.clone(),
            show_popup: action.has_popup,
        })
    }
}

/// Result of clicking a browser action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionClick {
    /// Extension that owns this action.
    pub extension_id: String,
    /// Whether to show the popup (true) or fire a click event (false).
    pub show_popup: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_action_defaults() {
        let action = BrowserAction::new("ext1".into(), "My Ext".into(), true);
        assert_eq!(action.extension_id, "ext1");
        assert_eq!(action.title, "My Ext");
        assert!(action.enabled);
        assert!(action.has_popup);
        assert!(!action.has_badge());
        assert!(action.popup_source().is_none());
    }

    #[test]
    fn set_badge_text_truncates() {
        let mut action = BrowserAction::new("ext1".into(), "Ext".into(), false);
        action.set_badge_text("12345");
        assert_eq!(action.badge_text, "1234");
        assert!(action.has_badge());
    }

    #[test]
    fn set_badge_color() {
        let mut action = BrowserAction::new("ext1".into(), "Ext".into(), false);
        action.set_badge_color("#ff0000");
        assert_eq!(action.badge_color, "#ff0000");
    }

    #[test]
    fn toggle_popup() {
        let mut action = BrowserAction::new("ext1".into(), "Ext".into(), false);
        assert!(!action.has_popup);

        action.set_popup_source("<html>popup</html>".into());
        assert!(action.has_popup);
        assert_eq!(action.popup_source(), Some("<html>popup</html>"));

        action.remove_popup();
        assert!(!action.has_popup);
        assert!(action.popup_source().is_none());
    }

    #[test]
    fn action_bar_add_remove() {
        let mut bar = ActionBar::new();
        bar.add(BrowserAction::new("ext1".into(), "One".into(), false));
        bar.add(BrowserAction::new("ext2".into(), "Two".into(), true));
        assert_eq!(bar.count(), 2);

        bar.remove("ext1");
        assert_eq!(bar.count(), 1);
        assert_eq!(bar.actions()[0].extension_id, "ext2");
    }

    #[test]
    fn action_bar_get_mut() {
        let mut bar = ActionBar::new();
        bar.add(BrowserAction::new("ext1".into(), "One".into(), false));

        let action = bar.get_mut("ext1").unwrap();
        action.set_title("Updated");
        assert_eq!(bar.actions()[0].title, "Updated");
    }

    #[test]
    fn action_bar_click_with_popup() {
        let mut bar = ActionBar::new();
        bar.add(BrowserAction::new("ext1".into(), "One".into(), true));

        let click = bar.click(0).unwrap();
        assert_eq!(
            click,
            ActionClick {
                extension_id: "ext1".into(),
                show_popup: true,
            }
        );
    }

    #[test]
    fn action_bar_click_without_popup() {
        let mut bar = ActionBar::new();
        bar.add(BrowserAction::new("ext1".into(), "One".into(), false));

        let click = bar.click(0).unwrap();
        assert!(!click.show_popup);
    }

    #[test]
    fn action_bar_click_disabled() {
        let mut bar = ActionBar::new();
        let mut action = BrowserAction::new("ext1".into(), "One".into(), true);
        action.set_enabled(false);
        bar.add(action);

        assert!(bar.click(0).is_none());
    }

    #[test]
    fn action_bar_click_out_of_bounds() {
        let bar = ActionBar::new();
        assert!(bar.click(0).is_none());
    }

    #[test]
    fn set_title_and_enabled() {
        let mut action = BrowserAction::new("ext1".into(), "Old".into(), false);
        action.set_title("New");
        assert_eq!(action.title, "New");

        action.set_enabled(false);
        assert!(!action.enabled);
    }
}

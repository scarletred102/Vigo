// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Web Notifications API.
//!
//! System notification creation, permission management, and lifecycle.

use std::collections::VecDeque;

// ── NotificationPermission ───────────────────────────────────────────────────

/// Permission state for notifications (mirrors the Notification API spec).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NotificationPermission {
    #[default]
    Default,
    Granted,
    Denied,
}

impl NotificationPermission {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Granted => "granted",
            Self::Denied => "denied",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "granted" => Self::Granted,
            "denied" => Self::Denied,
            _ => Self::Default,
        }
    }
}

// ── NotificationDirection ────────────────────────────────────────────────────

/// Text direction for the notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NotificationDirection {
    #[default]
    Auto,
    Ltr,
    Rtl,
}

impl NotificationDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Ltr => "ltr",
            Self::Rtl => "rtl",
        }
    }
}

// ── NotificationOptions ──────────────────────────────────────────────────────

/// Options for creating a notification.
#[derive(Debug, Clone, Default)]
pub struct NotificationOptions {
    pub body: Option<String>,
    pub tag: Option<String>,
    pub icon: Option<String>,
    pub badge: Option<String>,
    pub image: Option<String>,
    pub dir: NotificationDirection,
    pub lang: Option<String>,
    pub require_interaction: bool,
    pub silent: Option<bool>,
    pub renotify: bool,
    pub data: Option<String>,
    pub actions: Vec<NotificationAction>,
}

/// An action button in a notification.
#[derive(Debug, Clone)]
pub struct NotificationAction {
    pub action: String,
    pub title: String,
    pub icon: Option<String>,
}

// ── Notification ─────────────────────────────────────────────────────────────

/// A notification instance.
#[derive(Debug, Clone)]
pub struct Notification {
    /// Unique ID for this notification.
    pub id: u64,
    /// The title.
    pub title: String,
    /// Options.
    pub options: NotificationOptions,
    /// Origin that created this notification.
    pub origin: String,
    /// When it was created (seconds since epoch, simplified).
    pub timestamp: u64,
    /// Whether it has been clicked.
    pub clicked: bool,
    /// Whether it has been closed.
    pub closed: bool,
}

impl Notification {
    pub fn new(id: u64, title: &str, options: NotificationOptions, origin: &str) -> Self {
        Self {
            id,
            title: title.to_string(),
            options,
            origin: origin.to_string(),
            timestamp: 0,
            clicked: false,
            closed: false,
        }
    }

    /// The notification body.
    pub fn body(&self) -> &str {
        self.options.body.as_deref().unwrap_or("")
    }

    /// The tag (for replacement).
    pub fn tag(&self) -> Option<&str> {
        self.options.tag.as_deref()
    }

    /// Close this notification.
    pub fn close(&mut self) {
        self.closed = true;
    }

    /// Mark as clicked.
    pub fn click(&mut self) {
        self.clicked = true;
    }
}

// ── NotificationEvent ────────────────────────────────────────────────────────

/// Events from the notification system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationEvent {
    /// A notification was shown.
    Show(u64),
    /// A notification was clicked.
    Click(u64),
    /// A notification action was clicked.
    ActionClick(u64, String),
    /// A notification was closed.
    Close(u64),
    /// A notification errored.
    Error(u64),
}

// ── NotificationCenter ───────────────────────────────────────────────────────

/// Manages notifications for the browser.
#[derive(Debug)]
pub struct NotificationCenter {
    /// Next notification ID.
    next_id: u64,
    /// Active notifications.
    active: Vec<Notification>,
    /// Queue of events to be processed.
    events: VecDeque<NotificationEvent>,
    /// Maximum active notifications before oldest is auto-closed.
    pub max_active: usize,
}

impl Default for NotificationCenter {
    fn default() -> Self {
        Self::new()
    }
}

impl NotificationCenter {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            active: Vec::new(),
            events: VecDeque::new(),
            max_active: 10,
        }
    }

    /// Show a new notification. Returns its ID.
    pub fn show(&mut self, title: &str, options: NotificationOptions, origin: &str) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        // If a tag is provided, replace existing notification with same tag+origin
        if let Some(tag) = &options.tag {
            if let Some(existing) = self
                .active
                .iter_mut()
                .find(|n| n.origin == origin && n.tag() == Some(tag.as_str()))
            {
                let old_id = existing.id;
                existing.id = id;
                existing.title = title.to_string();
                existing.options = options;
                self.events.push_back(NotificationEvent::Close(old_id));
                self.events.push_back(NotificationEvent::Show(id));
                return id;
            }
        }

        // Enforce max active limit
        if self.active.len() >= self.max_active {
            if let Some(oldest) = self.active.first() {
                let oldest_id = oldest.id;
                self.events
                    .push_back(NotificationEvent::Close(oldest_id));
            }
            self.active.remove(0);
        }

        let notification = Notification::new(id, title, options, origin);
        self.active.push(notification);
        self.events.push_back(NotificationEvent::Show(id));
        id
    }

    /// Close a notification by ID.
    pub fn close(&mut self, id: u64) -> bool {
        if let Some(n) = self.active.iter_mut().find(|n| n.id == id) {
            n.close();
            self.events.push_back(NotificationEvent::Close(id));
            self.active.retain(|n| n.id != id);
            true
        } else {
            false
        }
    }

    /// Handle a click on a notification.
    pub fn click(&mut self, id: u64) -> bool {
        if let Some(n) = self.active.iter_mut().find(|n| n.id == id) {
            n.click();
            self.events.push_back(NotificationEvent::Click(id));
            true
        } else {
            false
        }
    }

    /// Handle an action click on a notification.
    pub fn action_click(&mut self, id: u64, action: &str) -> bool {
        if self.active.iter().any(|n| n.id == id) {
            self.events
                .push_back(NotificationEvent::ActionClick(id, action.to_string()));
            true
        } else {
            false
        }
    }

    /// Poll the next pending event.
    pub fn poll_event(&mut self) -> Option<NotificationEvent> {
        self.events.pop_front()
    }

    /// Get all active notifications.
    pub fn active_notifications(&self) -> &[Notification] {
        &self.active
    }

    /// Get a specific notification by ID.
    pub fn get(&self, id: u64) -> Option<&Notification> {
        self.active.iter().find(|n| n.id == id)
    }

    /// Number of active notifications.
    pub fn active_count(&self) -> usize {
        self.active.len()
    }

    /// Close all notifications from a specific origin.
    pub fn close_all_for_origin(&mut self, origin: &str) {
        let ids: Vec<u64> = self
            .active
            .iter()
            .filter(|n| n.origin == origin)
            .map(|n| n.id)
            .collect();
        for id in &ids {
            self.events.push_back(NotificationEvent::Close(*id));
        }
        self.active.retain(|n| n.origin != origin);
    }

    /// Close all notifications.
    pub fn close_all(&mut self) {
        for n in &self.active {
            self.events.push_back(NotificationEvent::Close(n.id));
        }
        self.active.clear();
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const ORIGIN: &str = "https://example.com";

    #[test]
    fn permission_roundtrip() {
        assert_eq!(
            NotificationPermission::parse("granted"),
            NotificationPermission::Granted
        );
        assert_eq!(NotificationPermission::Granted.as_str(), "granted");
        assert_eq!(NotificationPermission::parse("unknown"), NotificationPermission::Default);
    }

    #[test]
    fn show_notification() {
        let mut center = NotificationCenter::new();
        let id = center.show("Hello", NotificationOptions::default(), ORIGIN);
        assert_eq!(center.active_count(), 1);
        let n = center.get(id).unwrap();
        assert_eq!(n.title, "Hello");
        assert_eq!(n.origin, ORIGIN);
    }

    #[test]
    fn notification_body() {
        let opts = NotificationOptions {
            body: Some("World".to_string()),
            ..Default::default()
        };
        let mut center = NotificationCenter::new();
        let id = center.show("Hello", opts, ORIGIN);
        assert_eq!(center.get(id).unwrap().body(), "World");
    }

    #[test]
    fn close_notification() {
        let mut center = NotificationCenter::new();
        let id = center.show("Test", NotificationOptions::default(), ORIGIN);
        assert!(center.close(id));
        assert_eq!(center.active_count(), 0);
        assert!(!center.close(id)); // Already closed
    }

    #[test]
    fn click_notification() {
        let mut center = NotificationCenter::new();
        let id = center.show("Test", NotificationOptions::default(), ORIGIN);
        assert!(center.click(id));
        let n = center.get(id).unwrap();
        assert!(n.clicked);
    }

    #[test]
    fn action_click() {
        let mut center = NotificationCenter::new();
        let id = center.show("Test", NotificationOptions::default(), ORIGIN);
        assert!(center.action_click(id, "reply"));
        assert!(!center.action_click(999, "nope"));
    }

    #[test]
    fn tag_replacement() {
        let mut center = NotificationCenter::new();
        let opts1 = NotificationOptions {
            tag: Some("chat".to_string()),
            body: Some("Message 1".to_string()),
            ..Default::default()
        };
        let id1 = center.show("Chat", opts1, ORIGIN);

        let opts2 = NotificationOptions {
            tag: Some("chat".to_string()),
            body: Some("Message 2".to_string()),
            ..Default::default()
        };
        let id2 = center.show("Chat", opts2, ORIGIN);

        assert_ne!(id1, id2);
        assert_eq!(center.active_count(), 1);
        let n = center.get(id2).unwrap();
        assert_eq!(n.body(), "Message 2");
    }

    #[test]
    fn max_active_limit() {
        let mut center = NotificationCenter::new();
        center.max_active = 3;
        for i in 0..5 {
            center.show(&format!("N{i}"), NotificationOptions::default(), ORIGIN);
        }
        assert_eq!(center.active_count(), 3);
    }

    #[test]
    fn events_queue() {
        let mut center = NotificationCenter::new();
        let id = center.show("Test", NotificationOptions::default(), ORIGIN);
        let event = center.poll_event().unwrap();
        assert_eq!(event, NotificationEvent::Show(id));
    }

    #[test]
    fn close_all_for_origin() {
        let mut center = NotificationCenter::new();
        center.show("A", NotificationOptions::default(), ORIGIN);
        center.show("B", NotificationOptions::default(), ORIGIN);
        center.show("C", NotificationOptions::default(), "https://other.com");

        center.close_all_for_origin(ORIGIN);
        assert_eq!(center.active_count(), 1);
        assert_eq!(center.active_notifications()[0].origin, "https://other.com");
    }

    #[test]
    fn close_all() {
        let mut center = NotificationCenter::new();
        center.show("A", NotificationOptions::default(), ORIGIN);
        center.show("B", NotificationOptions::default(), ORIGIN);
        center.close_all();
        assert_eq!(center.active_count(), 0);
    }

    #[test]
    fn notification_actions() {
        let opts = NotificationOptions {
            actions: vec![
                NotificationAction {
                    action: "reply".to_string(),
                    title: "Reply".to_string(),
                    icon: None,
                },
                NotificationAction {
                    action: "dismiss".to_string(),
                    title: "Dismiss".to_string(),
                    icon: None,
                },
            ],
            ..Default::default()
        };
        let mut center = NotificationCenter::new();
        let id = center.show("Chat", opts, ORIGIN);
        let n = center.get(id).unwrap();
        assert_eq!(n.options.actions.len(), 2);
    }
}

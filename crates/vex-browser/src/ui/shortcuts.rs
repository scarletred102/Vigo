// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Keyboard shortcuts — maps key combinations to browser actions.

/// A keyboard shortcut: modifier flags + key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Shortcut {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub key: Key,
}

/// Keys relevant for browser shortcuts.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    /// A letter key (uppercase).
    Char(char),
    /// A function key.
    F(u8),
    /// Tab key.
    Tab,
    /// Escape.
    Escape,
    /// Enter / Return.
    Enter,
    /// Backspace.
    Backspace,
    /// Left arrow.
    Left,
    /// Right arrow.
    Right,
    /// Plus/equals (for zoom in).
    Plus,
    /// Minus (for zoom out).
    Minus,
    /// Zero (for zoom reset).
    Zero,
}

/// Browser action triggered by a shortcut.
#[derive(Debug, Clone, PartialEq)]
pub enum BrowserAction {
    /// Create a new tab.
    NewTab,
    /// Close the active tab.
    CloseTab,
    /// Reopen the last closed tab.
    ReopenTab,
    /// Focus the address bar.
    FocusAddressBar,
    /// Reload the current page.
    Reload,
    /// Hard reload (bypass cache).
    HardReload,
    /// Go back in history.
    Back,
    /// Go forward in history.
    Forward,
    /// Switch to the next tab.
    NextTab,
    /// Switch to the previous tab.
    PrevTab,
    /// Open find-in-page.
    FindInPage,
    /// Zoom in.
    ZoomIn,
    /// Zoom out.
    ZoomOut,
    /// Reset zoom to 100%.
    ZoomReset,
    /// Open dev tools (placeholder).
    DevTools,
    /// Toggle fullscreen.
    Fullscreen,
    /// Stop loading.
    Stop,
    /// Open bookmarks.
    Bookmarks,
    /// Open history.
    History,
    /// Open downloads.
    Downloads,
    /// Open settings.
    Settings,
}

/// Match a key event to a browser action using standard browser shortcuts.
pub fn match_shortcut(shortcut: &Shortcut) -> Option<BrowserAction> {
    use BrowserAction::*;
    use Key::*;

    match shortcut {
        // Ctrl+T — new tab.
        Shortcut {
            ctrl: true,
            shift: false,
            alt: false,
            key: Char('T'),
        } => Some(NewTab),
        // Ctrl+W — close tab.
        Shortcut {
            ctrl: true,
            shift: false,
            alt: false,
            key: Char('W'),
        } => Some(CloseTab),
        // Ctrl+Shift+T — reopen closed tab.
        Shortcut {
            ctrl: true,
            shift: true,
            alt: false,
            key: Char('T'),
        } => Some(ReopenTab),
        // Ctrl+L / F6 — focus address bar.
        Shortcut {
            ctrl: true,
            shift: false,
            alt: false,
            key: Char('L'),
        }
        | Shortcut {
            ctrl: false,
            shift: false,
            alt: false,
            key: F(6),
        } => Some(FocusAddressBar),
        // Ctrl+R / F5 — reload.
        Shortcut {
            ctrl: true,
            shift: false,
            alt: false,
            key: Char('R'),
        }
        | Shortcut {
            ctrl: false,
            shift: false,
            alt: false,
            key: F(5),
        } => Some(Reload),
        // Ctrl+Shift+R — hard reload.
        Shortcut {
            ctrl: true,
            shift: true,
            alt: false,
            key: Char('R'),
        } => Some(HardReload),
        // Alt+Left — back.
        Shortcut {
            ctrl: false,
            shift: false,
            alt: true,
            key: Left,
        } => Some(Back),
        // Alt+Right — forward.
        Shortcut {
            ctrl: false,
            shift: false,
            alt: true,
            key: Right,
        } => Some(Forward),
        // Ctrl+Tab — next tab.
        Shortcut {
            ctrl: true,
            shift: false,
            alt: false,
            key: Tab,
        } => Some(NextTab),
        // Ctrl+Shift+Tab — previous tab.
        Shortcut {
            ctrl: true,
            shift: true,
            alt: false,
            key: Tab,
        } => Some(PrevTab),
        // Ctrl+F — find in page.
        Shortcut {
            ctrl: true,
            shift: false,
            alt: false,
            key: Char('F'),
        } => Some(FindInPage),
        // Ctrl+Plus — zoom in.
        Shortcut {
            ctrl: true,
            shift: false,
            alt: false,
            key: Plus,
        } => Some(ZoomIn),
        // Ctrl+Minus — zoom out.
        Shortcut {
            ctrl: true,
            shift: false,
            alt: false,
            key: Minus,
        } => Some(ZoomOut),
        // Ctrl+0 — zoom reset.
        Shortcut {
            ctrl: true,
            shift: false,
            alt: false,
            key: Zero,
        } => Some(ZoomReset),
        // F12 — dev tools.
        Shortcut {
            ctrl: false,
            shift: false,
            alt: false,
            key: F(12),
        } => Some(DevTools),
        // F11 — fullscreen.
        Shortcut {
            ctrl: false,
            shift: false,
            alt: false,
            key: F(11),
        } => Some(Fullscreen),
        // Escape — stop loading.
        Shortcut {
            ctrl: false,
            shift: false,
            alt: false,
            key: Escape,
        } => Some(Stop),
        // Ctrl+B — bookmarks.
        Shortcut {
            ctrl: true,
            shift: false,
            alt: false,
            key: Char('B'),
        } => Some(Bookmarks),
        // Ctrl+H — history.
        Shortcut {
            ctrl: true,
            shift: false,
            alt: false,
            key: Char('H'),
        } => Some(History),
        // Ctrl+J — downloads.
        Shortcut {
            ctrl: true,
            shift: false,
            alt: false,
            key: Char('J'),
        } => Some(Downloads),
        // Ctrl+, — settings.
        Shortcut {
            ctrl: true,
            shift: false,
            alt: false,
            key: Char(','),
        } => Some(Settings),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctrl(c: char) -> Shortcut {
        Shortcut {
            ctrl: true,
            shift: false,
            alt: false,
            key: Key::Char(c),
        }
    }

    fn ctrl_shift(c: char) -> Shortcut {
        Shortcut {
            ctrl: true,
            shift: true,
            alt: false,
            key: Key::Char(c),
        }
    }

    fn fkey(n: u8) -> Shortcut {
        Shortcut {
            ctrl: false,
            shift: false,
            alt: false,
            key: Key::F(n),
        }
    }

    fn alt_key(key: Key) -> Shortcut {
        Shortcut {
            ctrl: false,
            shift: false,
            alt: true,
            key,
        }
    }

    #[test]
    fn new_tab_shortcut() {
        assert_eq!(match_shortcut(&ctrl('T')), Some(BrowserAction::NewTab));
    }

    #[test]
    fn close_tab_shortcut() {
        assert_eq!(match_shortcut(&ctrl('W')), Some(BrowserAction::CloseTab));
    }

    #[test]
    fn reopen_tab_shortcut() {
        assert_eq!(
            match_shortcut(&ctrl_shift('T')),
            Some(BrowserAction::ReopenTab)
        );
    }

    #[test]
    fn focus_address_bar() {
        assert_eq!(
            match_shortcut(&ctrl('L')),
            Some(BrowserAction::FocusAddressBar)
        );
        assert_eq!(
            match_shortcut(&fkey(6)),
            Some(BrowserAction::FocusAddressBar)
        );
    }

    #[test]
    fn reload_shortcuts() {
        assert_eq!(match_shortcut(&ctrl('R')), Some(BrowserAction::Reload));
        assert_eq!(match_shortcut(&fkey(5)), Some(BrowserAction::Reload));
        assert_eq!(
            match_shortcut(&ctrl_shift('R')),
            Some(BrowserAction::HardReload)
        );
    }

    #[test]
    fn navigation_shortcuts() {
        assert_eq!(
            match_shortcut(&alt_key(Key::Left)),
            Some(BrowserAction::Back)
        );
        assert_eq!(
            match_shortcut(&alt_key(Key::Right)),
            Some(BrowserAction::Forward)
        );
    }

    #[test]
    fn tab_switching() {
        let next = Shortcut {
            ctrl: true,
            shift: false,
            alt: false,
            key: Key::Tab,
        };
        let prev = Shortcut {
            ctrl: true,
            shift: true,
            alt: false,
            key: Key::Tab,
        };
        assert_eq!(match_shortcut(&next), Some(BrowserAction::NextTab));
        assert_eq!(match_shortcut(&prev), Some(BrowserAction::PrevTab));
    }

    #[test]
    fn find_in_page() {
        assert_eq!(match_shortcut(&ctrl('F')), Some(BrowserAction::FindInPage));
    }

    #[test]
    fn zoom_shortcuts() {
        let zoom_in = Shortcut {
            ctrl: true,
            shift: false,
            alt: false,
            key: Key::Plus,
        };
        let zoom_out = Shortcut {
            ctrl: true,
            shift: false,
            alt: false,
            key: Key::Minus,
        };
        let zoom_reset = Shortcut {
            ctrl: true,
            shift: false,
            alt: false,
            key: Key::Zero,
        };
        assert_eq!(match_shortcut(&zoom_in), Some(BrowserAction::ZoomIn));
        assert_eq!(match_shortcut(&zoom_out), Some(BrowserAction::ZoomOut));
        assert_eq!(match_shortcut(&zoom_reset), Some(BrowserAction::ZoomReset));
    }

    #[test]
    fn unrecognized_shortcut_returns_none() {
        let unknown = Shortcut {
            ctrl: false,
            shift: false,
            alt: false,
            key: Key::Char('X'),
        };
        assert_eq!(match_shortcut(&unknown), None);
    }
}

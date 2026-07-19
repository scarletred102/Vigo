// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Media Session API.
//!
//! Allows web pages to provide metadata (title, artist, album, artwork)
//! and handle media playback actions (play, pause, seek, etc.)
//! for integration with OS media controls.

use std::collections::HashMap;

// ── Types ────────────────────────────────────────────────────────────────────

/// Playback state for the media session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MediaSessionPlaybackState {
    #[default]
    None,
    Paused,
    Playing,
}

/// Media session action type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MediaSessionAction {
    Play,
    Pause,
    SeekBackward,
    SeekForward,
    PreviousTrack,
    NextTrack,
    SkipAd,
    Stop,
    SeekTo,
}

/// Artwork metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaImage {
    pub src: String,
    pub sizes: Option<String>,
    pub image_type: Option<String>,
}

/// Media metadata.
#[derive(Debug, Clone, Default)]
pub struct MediaMetadata {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub artwork: Vec<MediaImage>,
}

impl MediaMetadata {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            ..Default::default()
        }
    }

    pub fn with_artist(mut self, artist: &str) -> Self {
        self.artist = artist.to_string();
        self
    }

    pub fn with_album(mut self, album: &str) -> Self {
        self.album = album.to_string();
        self
    }

    pub fn add_artwork(mut self, src: &str, sizes: Option<&str>, image_type: Option<&str>) -> Self {
        self.artwork.push(MediaImage {
            src: src.to_string(),
            sizes: sizes.map(|s| s.to_string()),
            image_type: image_type.map(|s| s.to_string()),
        });
        self
    }
}

/// Action details for seek-type actions.
#[derive(Debug, Clone)]
pub struct MediaSessionActionDetails {
    pub action: MediaSessionAction,
    pub seek_offset: Option<f64>,
    pub seek_time: Option<f64>,
    pub fast_seek: bool,
}

/// Action handler callback.
pub type ActionHandler = Box<dyn Fn(&MediaSessionActionDetails) + Send>;

/// Position state for the media session.
#[derive(Debug, Clone, Copy)]
pub struct MediaPositionState {
    pub duration: f64,
    pub playback_rate: f64,
    pub position: f64,
}

impl Default for MediaPositionState {
    fn default() -> Self {
        Self {
            duration: 0.0,
            playback_rate: 1.0,
            position: 0.0,
        }
    }
}

// ── MediaSession ─────────────────────────────────────────────────────────────

/// The MediaSession API implementation.
pub struct MediaSession {
    metadata: Option<MediaMetadata>,
    playback_state: MediaSessionPlaybackState,
    position_state: MediaPositionState,
    action_handlers: HashMap<MediaSessionAction, ActionHandler>,
}

impl Default for MediaSession {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for MediaSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MediaSession")
            .field("metadata", &self.metadata)
            .field("playback_state", &self.playback_state)
            .field("position_state", &self.position_state)
            .field("handler_count", &self.action_handlers.len())
            .finish()
    }
}

impl MediaSession {
    pub fn new() -> Self {
        Self {
            metadata: None,
            playback_state: MediaSessionPlaybackState::None,
            position_state: MediaPositionState::default(),
            action_handlers: HashMap::new(),
        }
    }

    pub fn metadata(&self) -> Option<&MediaMetadata> {
        self.metadata.as_ref()
    }

    pub fn set_metadata(&mut self, metadata: Option<MediaMetadata>) {
        self.metadata = metadata;
    }

    pub fn playback_state(&self) -> MediaSessionPlaybackState {
        self.playback_state
    }

    pub fn set_playback_state(&mut self, state: MediaSessionPlaybackState) {
        self.playback_state = state;
    }

    pub fn position_state(&self) -> &MediaPositionState {
        &self.position_state
    }

    pub fn set_position_state(&mut self, state: MediaPositionState) {
        self.position_state = state;
    }

    /// Register an action handler.
    pub fn set_action_handler(&mut self, action: MediaSessionAction, handler: ActionHandler) {
        self.action_handlers.insert(action, handler);
    }

    /// Remove an action handler.
    pub fn remove_action_handler(&mut self, action: MediaSessionAction) {
        self.action_handlers.remove(&action);
    }

    /// Whether an action has a handler.
    pub fn has_handler(&self, action: MediaSessionAction) -> bool {
        self.action_handlers.contains_key(&action)
    }

    /// Dispatch an action (invoke the handler if registered).
    pub fn dispatch_action(&self, details: &MediaSessionActionDetails) -> bool {
        if let Some(handler) = self.action_handlers.get(&details.action) {
            handler(details);
            true
        } else {
            false
        }
    }
}

// ── Web Share API ────────────────────────────────────────────────────────────

/// Share data for the Web Share API.
#[derive(Debug, Clone, Default)]
pub struct ShareData {
    pub title: Option<String>,
    pub text: Option<String>,
    pub url: Option<String>,
    pub files: Vec<ShareFile>,
}

impl ShareData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn with_text(mut self, text: &str) -> Self {
        self.text = Some(text.to_string());
        self
    }

    pub fn with_url(mut self, url: &str) -> Self {
        self.url = Some(url.to_string());
        self
    }

    /// Whether this share data has any content.
    pub fn is_empty(&self) -> bool {
        self.title.is_none() && self.text.is_none() && self.url.is_none() && self.files.is_empty()
    }
}

/// A file to share.
#[derive(Debug, Clone)]
pub struct ShareFile {
    pub name: String,
    pub mime_type: String,
    pub data: Vec<u8>,
}

/// Share error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShareError {
    /// Empty share data.
    NoData,
    /// Not triggered by user action.
    NotAllowed,
    /// Share was aborted by user.
    Aborted,
    /// Internal error.
    Internal(String),
}

impl std::fmt::Display for ShareError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoData => write!(f, "Share data is empty"),
            Self::NotAllowed => write!(f, "Not triggered by user activation"),
            Self::Aborted => write!(f, "Share was aborted"),
            Self::Internal(msg) => write!(f, "Share error: {msg}"),
        }
    }
}

/// Web Share API manager.
#[derive(Debug, Default)]
pub struct WebShare {
    /// Whether sharing is supported.
    supported: bool,
    /// Whether file sharing is supported.
    file_sharing: bool,
    /// Allowed MIME types for file sharing.
    allowed_file_types: Vec<String>,
}

impl WebShare {
    pub fn new(supported: bool) -> Self {
        Self {
            supported,
            file_sharing: false,
            allowed_file_types: vec![
                "image/png".to_string(),
                "image/jpeg".to_string(),
                "image/gif".to_string(),
                "image/webp".to_string(),
                "text/plain".to_string(),
                "text/html".to_string(),
                "application/pdf".to_string(),
            ],
        }
    }

    pub fn with_file_sharing(mut self, enabled: bool) -> Self {
        self.file_sharing = enabled;
        self
    }

    /// Whether sharing is supported.
    pub fn can_share(&self) -> bool {
        self.supported
    }

    /// Whether specific share data can be shared.
    pub fn can_share_data(&self, data: &ShareData) -> bool {
        if !self.supported || data.is_empty() {
            return false;
        }

        if !data.files.is_empty() {
            if !self.file_sharing {
                return false;
            }
            for file in &data.files {
                if !self.allowed_file_types.iter().any(|t| t == &file.mime_type) {
                    return false;
                }
            }
        }

        true
    }

    /// Share data. In a real browser, this would invoke the OS share sheet.
    pub fn share(&self, data: &ShareData, user_activated: bool) -> Result<(), ShareError> {
        if !self.supported {
            return Err(ShareError::Internal("Sharing not supported".to_string()));
        }
        if !user_activated {
            return Err(ShareError::NotAllowed);
        }
        if data.is_empty() {
            return Err(ShareError::NoData);
        }
        if !self.can_share_data(data) {
            return Err(ShareError::Internal(
                "Data contains unsupported file types".to_string(),
            ));
        }

        // In a real implementation, this would invoke the OS share sheet.
        // For now, we validate and return success.
        Ok(())
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    // ── MediaSession ────────────────────────────────────────────

    #[test]
    fn media_session_initial() {
        let session = MediaSession::new();
        assert_eq!(session.playback_state(), MediaSessionPlaybackState::None);
        assert!(session.metadata().is_none());
    }

    #[test]
    fn set_metadata() {
        let mut session = MediaSession::new();
        let meta = MediaMetadata::new("Test Song")
            .with_artist("Artist")
            .with_album("Album");
        session.set_metadata(Some(meta));

        let m = session.metadata().unwrap();
        assert_eq!(m.title, "Test Song");
        assert_eq!(m.artist, "Artist");
        assert_eq!(m.album, "Album");
    }

    #[test]
    fn metadata_artwork() {
        let meta = MediaMetadata::new("Song").add_artwork(
            "https://example.com/art.png",
            Some("256x256"),
            Some("image/png"),
        );
        assert_eq!(meta.artwork.len(), 1);
        assert_eq!(meta.artwork[0].src, "https://example.com/art.png");
    }

    #[test]
    fn playback_state() {
        let mut session = MediaSession::new();
        session.set_playback_state(MediaSessionPlaybackState::Playing);
        assert_eq!(session.playback_state(), MediaSessionPlaybackState::Playing);
    }

    #[test]
    fn position_state() {
        let mut session = MediaSession::new();
        session.set_position_state(MediaPositionState {
            duration: 300.0,
            playback_rate: 1.0,
            position: 42.0,
        });
        assert_eq!(session.position_state().position, 42.0);
        assert_eq!(session.position_state().duration, 300.0);
    }

    #[test]
    fn action_handler() {
        let mut session = MediaSession::new();
        let counter = Arc::new(AtomicU32::new(0));
        let c = counter.clone();

        session.set_action_handler(
            MediaSessionAction::Play,
            Box::new(move |_| {
                c.fetch_add(1, Ordering::Relaxed);
            }),
        );

        assert!(session.has_handler(MediaSessionAction::Play));
        assert!(!session.has_handler(MediaSessionAction::Pause));

        let details = MediaSessionActionDetails {
            action: MediaSessionAction::Play,
            seek_offset: None,
            seek_time: None,
            fast_seek: false,
        };
        assert!(session.dispatch_action(&details));
        assert_eq!(counter.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn dispatch_unhandled_action() {
        let session = MediaSession::new();
        let details = MediaSessionActionDetails {
            action: MediaSessionAction::Stop,
            seek_offset: None,
            seek_time: None,
            fast_seek: false,
        };
        assert!(!session.dispatch_action(&details));
    }

    #[test]
    fn remove_action_handler() {
        let mut session = MediaSession::new();
        session.set_action_handler(MediaSessionAction::Play, Box::new(|_| {}));
        assert!(session.has_handler(MediaSessionAction::Play));
        session.remove_action_handler(MediaSessionAction::Play);
        assert!(!session.has_handler(MediaSessionAction::Play));
    }

    // ── Web Share ───────────────────────────────────────────────

    #[test]
    fn can_share_basic() {
        let share = WebShare::new(true);
        assert!(share.can_share());

        let data = ShareData::new()
            .with_title("Test")
            .with_url("https://example.com");
        assert!(share.can_share_data(&data));
    }

    #[test]
    fn cannot_share_empty() {
        let share = WebShare::new(true);
        let data = ShareData::new();
        assert!(!share.can_share_data(&data));
    }

    #[test]
    fn share_requires_user_activation() {
        let share = WebShare::new(true);
        let data = ShareData::new().with_title("Test");
        let err = share.share(&data, false).unwrap_err();
        assert_eq!(err, ShareError::NotAllowed);
    }

    #[test]
    fn share_success() {
        let share = WebShare::new(true);
        let data = ShareData::new().with_text("Hello world");
        share.share(&data, true).unwrap();
    }

    #[test]
    fn share_not_supported() {
        let share = WebShare::new(false);
        let data = ShareData::new().with_title("Test");
        assert!(share.share(&data, true).is_err());
    }

    #[test]
    fn file_sharing() {
        let share = WebShare::new(true).with_file_sharing(true);
        let mut data = ShareData::new().with_title("Photo");
        data.files.push(ShareFile {
            name: "photo.png".to_string(),
            mime_type: "image/png".to_string(),
            data: vec![0; 10],
        });
        assert!(share.can_share_data(&data));
        share.share(&data, true).unwrap();
    }

    #[test]
    fn file_sharing_unsupported_type() {
        let share = WebShare::new(true).with_file_sharing(true);
        let mut data = ShareData::new().with_title("Binary");
        data.files.push(ShareFile {
            name: "test.exe".to_string(),
            mime_type: "application/x-executable".to_string(),
            data: vec![0; 10],
        });
        assert!(!share.can_share_data(&data));
    }
}

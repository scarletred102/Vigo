// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Download manager — tracks in-progress and completed downloads.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Unique identifier for a download.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DownloadId(pub u64);

/// Current state of a download.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DownloadState {
    /// Queued, not yet started.
    Pending,
    /// Currently downloading.
    InProgress {
        /// Bytes received so far.
        received: u64,
        /// Total size (if known).
        total: Option<u64>,
    },
    /// Download completed successfully.
    Complete,
    /// Download failed with an error message.
    Failed(String),
    /// Download was cancelled by the user.
    Cancelled,
}

/// A single download entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Download {
    pub id: DownloadId,
    /// Source URL.
    pub url: String,
    /// Suggested filename.
    pub filename: String,
    /// Local file path (destination).
    pub path: PathBuf,
    /// Current state.
    pub state: DownloadState,
    /// Unix timestamp when the download was started.
    pub started_at: u64,
    /// MIME type (if known).
    pub mime_type: Option<String>,
}

/// Download manager — tracks all downloads.
#[derive(Debug, Default)]
pub struct DownloadManager {
    downloads: Vec<Download>,
    next_id: u64,
}

impl DownloadManager {
    /// Create a new empty download manager.
    pub fn new() -> Self {
        Self {
            downloads: Vec::new(),
            next_id: 1,
        }
    }

    /// Start a new download. Returns the download ID.
    pub fn start_download(
        &mut self,
        url: &str,
        filename: &str,
        download_dir: &std::path::Path,
    ) -> DownloadId {
        let id = DownloadId(self.next_id);
        self.next_id += 1;

        let path = download_dir.join(filename);
        self.downloads.push(Download {
            id,
            url: url.to_string(),
            filename: filename.to_string(),
            path,
            state: DownloadState::Pending,
            started_at: current_timestamp(),
            mime_type: None,
        });
        id
    }

    /// Update download progress.
    pub fn update_progress(&mut self, id: DownloadId, received: u64, total: Option<u64>) {
        if let Some(dl) = self.downloads.iter_mut().find(|d| d.id == id) {
            dl.state = DownloadState::InProgress { received, total };
        }
    }

    /// Mark a download as complete.
    pub fn mark_complete(&mut self, id: DownloadId) {
        if let Some(dl) = self.downloads.iter_mut().find(|d| d.id == id) {
            dl.state = DownloadState::Complete;
        }
    }

    /// Mark a download as failed.
    pub fn mark_failed(&mut self, id: DownloadId, error: &str) {
        if let Some(dl) = self.downloads.iter_mut().find(|d| d.id == id) {
            dl.state = DownloadState::Failed(error.to_string());
        }
    }

    /// Cancel a download.
    pub fn cancel(&mut self, id: DownloadId) {
        if let Some(dl) = self.downloads.iter_mut().find(|d| d.id == id) {
            dl.state = DownloadState::Cancelled;
        }
    }

    /// Remove a download from the list (doesn't delete the file).
    pub fn remove(&mut self, id: DownloadId) -> bool {
        let len_before = self.downloads.len();
        self.downloads.retain(|d| d.id != id);
        self.downloads.len() < len_before
    }

    /// Get a download by ID.
    pub fn get(&self, id: DownloadId) -> Option<&Download> {
        self.downloads.iter().find(|d| d.id == id)
    }

    /// Get all downloads, most recent first.
    pub fn all(&self) -> impl Iterator<Item = &Download> {
        self.downloads.iter().rev()
    }

    /// Get in-progress downloads.
    pub fn in_progress(&self) -> Vec<&Download> {
        self.downloads
            .iter()
            .filter(|d| {
                matches!(
                    d.state,
                    DownloadState::Pending | DownloadState::InProgress { .. }
                )
            })
            .collect()
    }

    /// Total download count.
    pub fn count(&self) -> usize {
        self.downloads.len()
    }

    /// Clear completed/failed/cancelled downloads.
    pub fn clear_finished(&mut self) {
        self.downloads.retain(|d| {
            matches!(
                d.state,
                DownloadState::Pending | DownloadState::InProgress { .. }
            )
        });
    }
}

/// Suggest a filename from a URL.
pub fn suggest_filename(url: &str) -> String {
    // Extract the last path segment.
    if let Some(path) = url.split('?').next() {
        if let Some(segment) = path.rsplit('/').next() {
            if !segment.is_empty() && segment.contains('.') {
                return segment.to_string();
            }
        }
    }
    "download".to_string()
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn start_and_get_download() {
        let mut mgr = DownloadManager::new();
        let id = mgr.start_download(
            "https://example.com/file.zip",
            "file.zip",
            Path::new("/tmp"),
        );
        let dl = mgr.get(id).unwrap();
        assert_eq!(dl.url, "https://example.com/file.zip");
        assert_eq!(dl.state, DownloadState::Pending);
    }

    #[test]
    fn update_progress() {
        let mut mgr = DownloadManager::new();
        let id = mgr.start_download(
            "https://example.com/file.zip",
            "file.zip",
            Path::new("/tmp"),
        );
        mgr.update_progress(id, 500, Some(1000));
        let dl = mgr.get(id).unwrap();
        assert_eq!(
            dl.state,
            DownloadState::InProgress {
                received: 500,
                total: Some(1000)
            }
        );
    }

    #[test]
    fn mark_complete() {
        let mut mgr = DownloadManager::new();
        let id = mgr.start_download("https://example.com/a.txt", "a.txt", Path::new("/tmp"));
        mgr.mark_complete(id);
        assert_eq!(mgr.get(id).unwrap().state, DownloadState::Complete);
    }

    #[test]
    fn cancel_download() {
        let mut mgr = DownloadManager::new();
        let id = mgr.start_download("https://example.com/a.txt", "a.txt", Path::new("/tmp"));
        mgr.cancel(id);
        assert_eq!(mgr.get(id).unwrap().state, DownloadState::Cancelled);
    }

    #[test]
    fn clear_finished() {
        let mut mgr = DownloadManager::new();
        let id1 = mgr.start_download("https://a.com/a", "a", Path::new("/tmp"));
        let _id2 = mgr.start_download("https://b.com/b", "b", Path::new("/tmp"));
        mgr.mark_complete(id1);
        mgr.clear_finished();
        assert_eq!(mgr.count(), 1);
    }

    #[test]
    fn suggest_filename_from_url() {
        assert_eq!(
            suggest_filename("https://example.com/path/file.pdf"),
            "file.pdf"
        );
        assert_eq!(
            suggest_filename("https://example.com/path/file.zip?token=abc"),
            "file.zip"
        );
        assert_eq!(suggest_filename("https://example.com/"), "download");
    }

    #[test]
    fn in_progress_filter() {
        let mut mgr = DownloadManager::new();
        let id1 = mgr.start_download("https://a.com/a", "a", Path::new("/tmp"));
        let _id2 = mgr.start_download("https://b.com/b", "b", Path::new("/tmp"));
        mgr.mark_complete(id1);
        let active = mgr.in_progress();
        assert_eq!(active.len(), 1);
    }
}

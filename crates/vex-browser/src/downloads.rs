// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Download manager — queues HTTP(S) transfers and exposes live download state.

use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

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

/// Errors returned while a download is being queued.
#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("could not create the download directory: {0}")]
    CreateDirectory(#[source] std::io::Error),
    #[error("could not reserve a destination file: {0}")]
    ReserveDestination(#[source] std::io::Error),
    #[error("could not start the download worker: {0}")]
    SpawnWorker(#[source] std::io::Error),
}

#[derive(Debug)]
struct DownloadStore {
    downloads: Vec<Download>,
    cancelled: HashSet<DownloadId>,
    next_id: u64,
}

/// Download manager — a cloneable, thread-safe handle for browser UI and
/// background workers. Transfers always write to a temporary `.part` file and
/// atomically rename it only after a successful response body has been read.
#[derive(Debug, Clone)]
pub struct DownloadManager {
    store: Arc<Mutex<DownloadStore>>,
    download_dir: PathBuf,
}

impl Default for DownloadManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DownloadManager {
    /// Create a manager using a local `downloads` directory as a conservative
    /// fallback. The application should use [`Self::with_download_dir`] for its
    /// profile-owned download directory.
    pub fn new() -> Self {
        Self::with_download_dir(PathBuf::from("downloads"))
    }

    /// Create a manager that stores completed files in `download_dir`.
    pub fn with_download_dir(download_dir: impl Into<PathBuf>) -> Self {
        Self {
            store: Arc::new(Mutex::new(DownloadStore {
                downloads: Vec::new(),
                cancelled: HashSet::new(),
                next_id: 1,
            })),
            download_dir: download_dir.into(),
        }
    }

    /// Queue an HTTP(S) transfer on a background thread.
    ///
    /// The URL is recorded immediately. Its final state can be observed via
    /// [`Self::all`] without blocking the browser UI.
    pub fn queue_download(&self, url: &str) -> Result<DownloadId, DownloadError> {
        fs::create_dir_all(&self.download_dir).map_err(DownloadError::CreateDirectory)?;
        let filename = suggest_filename(url);
        let path = reserve_destination(&self.download_dir, &filename)
            .map_err(DownloadError::ReserveDestination)?;
        let id = self.insert_download(url, &filename, path.clone(), None);
        let manager = self.clone();
        let url = url.to_owned();

        std::thread::Builder::new()
            .name(format!("vigo-download-{}", id.0))
            .spawn(move || manager.transfer(id, &url, &path))
            .map_err(DownloadError::SpawnWorker)?;

        Ok(id)
    }

    /// Queue bytes already received by a navigation response.
    ///
    /// Keeping the file write on a worker prevents a response that asks to be
    /// downloaded through `Content-Disposition: attachment` from blocking the
    /// browser chrome a second time or issuing a duplicate network request.
    pub fn queue_response(
        &self,
        url: &str,
        filename: Option<&str>,
        mime_type: Option<&str>,
        body: Vec<u8>,
    ) -> Result<DownloadId, DownloadError> {
        fs::create_dir_all(&self.download_dir).map_err(DownloadError::CreateDirectory)?;
        let filename = filename
            .map(safe_filename)
            .unwrap_or_else(|| suggest_filename(url));
        let path = reserve_destination(&self.download_dir, &filename)
            .map_err(DownloadError::ReserveDestination)?;
        let id = self.insert_download(
            url,
            &filename,
            path.clone(),
            mime_type.map(ToOwned::to_owned),
        );
        let manager = self.clone();

        std::thread::Builder::new()
            .name(format!("vigo-download-response-{}", id.0))
            .spawn(move || manager.write_response(id, body, &path))
            .map_err(DownloadError::SpawnWorker)?;

        Ok(id)
    }

    /// Record a download without starting a network transfer. This is useful
    /// for imported/download-restoration state and unit tests.
    pub fn start_download(&self, url: &str, filename: &str, download_dir: &Path) -> DownloadId {
        self.insert_download(
            url,
            filename,
            download_dir.join(safe_filename(filename)),
            None,
        )
    }

    fn insert_download(
        &self,
        url: &str,
        filename: &str,
        path: PathBuf,
        mime_type: Option<String>,
    ) -> DownloadId {
        let mut store = self.store.lock().expect("download store lock poisoned");
        let id = DownloadId(store.next_id);
        store.next_id += 1;
        store.downloads.push(Download {
            id,
            url: url.to_string(),
            filename: safe_filename(filename),
            path,
            state: DownloadState::Pending,
            started_at: current_timestamp(),
            mime_type,
        });
        id
    }

    fn transfer(&self, id: DownloadId, url: &str, destination: &Path) {
        let result = self.transfer_inner(id, url, destination);
        if let Err(error) = result {
            if !self.is_cancelled(id) {
                self.mark_failed(id, &error);
            }
        }
    }

    fn write_response(&self, id: DownloadId, body: Vec<u8>, destination: &Path) {
        let result = self.write_response_inner(id, &body, destination);
        if let Err(error) = result {
            if !self.is_cancelled(id) {
                self.mark_failed(id, &error);
            }
        }
    }

    fn write_response_inner(
        &self,
        id: DownloadId,
        body: &[u8],
        destination: &Path,
    ) -> Result<(), String> {
        let total = body.len() as u64;
        self.update_progress(id, 0, Some(total));
        let temporary = temporary_path(destination);
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| format!("could not create temporary file: {error}"))?;

        let mut received = 0usize;
        while received < body.len() {
            if self.is_cancelled(id) {
                let _ = fs::remove_file(&temporary);
                return Ok(());
            }
            let next = (received + 64 * 1024).min(body.len());
            output
                .write_all(&body[received..next])
                .map_err(|error| format!("could not write file: {error}"))?;
            received = next;
            self.update_progress(id, received as u64, Some(total));
        }
        output
            .sync_all()
            .map_err(|error| format!("could not finalize temporary file: {error}"))?;
        drop(output);

        if self.is_cancelled(id) {
            let _ = fs::remove_file(&temporary);
            return Ok(());
        }
        fs::rename(&temporary, destination)
            .map_err(|error| format!("could not finalize download: {error}"))?;
        self.mark_complete(id);
        Ok(())
    }

    fn transfer_inner(&self, id: DownloadId, url: &str, destination: &Path) -> Result<(), String> {
        let client = reqwest::blocking::Client::builder()
            .user_agent("Vigo/0.1 (download)")
            .build()
            .map_err(|error| format!("could not create HTTP client: {error}"))?;
        let mut response = client
            .get(url)
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .map_err(|error| format!("request failed: {error}"))?;
        let total = response.content_length();
        self.update_progress(id, 0, total);

        let temporary = temporary_path(destination);
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| format!("could not create temporary file: {error}"))?;

        let mut received = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            if self.is_cancelled(id) {
                let _ = fs::remove_file(&temporary);
                return Ok(());
            }

            let read = response
                .read(&mut buffer)
                .map_err(|error| format!("could not read response: {error}"))?;
            if read == 0 {
                break;
            }
            output
                .write_all(&buffer[..read])
                .map_err(|error| format!("could not write file: {error}"))?;
            received += read as u64;
            self.update_progress(id, received, total);
        }
        output
            .sync_all()
            .map_err(|error| format!("could not finalize temporary file: {error}"))?;
        drop(output);

        if self.is_cancelled(id) {
            let _ = fs::remove_file(&temporary);
            return Ok(());
        }
        fs::rename(&temporary, destination)
            .map_err(|error| format!("could not finalize download: {error}"))?;
        self.mark_complete(id);
        Ok(())
    }

    /// Update download progress.
    pub fn update_progress(&self, id: DownloadId, received: u64, total: Option<u64>) {
        let mut store = self.store.lock().expect("download store lock poisoned");
        if let Some(download) = store
            .downloads
            .iter_mut()
            .find(|download| download.id == id)
        {
            if !matches!(download.state, DownloadState::Cancelled) {
                download.state = DownloadState::InProgress { received, total };
            }
        }
    }

    /// Mark a download as complete.
    pub fn mark_complete(&self, id: DownloadId) {
        let mut store = self.store.lock().expect("download store lock poisoned");
        if let Some(download) = store
            .downloads
            .iter_mut()
            .find(|download| download.id == id)
        {
            if !matches!(download.state, DownloadState::Cancelled) {
                download.state = DownloadState::Complete;
            }
        }
    }

    /// Mark a download as failed.
    pub fn mark_failed(&self, id: DownloadId, error: &str) {
        let mut store = self.store.lock().expect("download store lock poisoned");
        if let Some(download) = store
            .downloads
            .iter_mut()
            .find(|download| download.id == id)
        {
            if !matches!(download.state, DownloadState::Cancelled) {
                download.state = DownloadState::Failed(error.to_string());
            }
        }
    }

    /// Cancel a download. The worker removes its temporary file on its next
    /// buffer boundary; completed files are never deleted by cancellation.
    pub fn cancel(&self, id: DownloadId) {
        let mut store = self.store.lock().expect("download store lock poisoned");
        store.cancelled.insert(id);
        if let Some(download) = store
            .downloads
            .iter_mut()
            .find(|download| download.id == id)
        {
            if !matches!(download.state, DownloadState::Complete) {
                download.state = DownloadState::Cancelled;
            }
        }
    }

    /// Remove a download from the list. This never deletes its completed file.
    pub fn remove(&self, id: DownloadId) -> bool {
        let mut store = self.store.lock().expect("download store lock poisoned");
        let before = store.downloads.len();
        store.downloads.retain(|download| download.id != id);
        store.cancelled.remove(&id);
        store.downloads.len() != before
    }

    /// Get a snapshot of a download by ID.
    pub fn get(&self, id: DownloadId) -> Option<Download> {
        self.store
            .lock()
            .expect("download store lock poisoned")
            .downloads
            .iter()
            .find(|download| download.id == id)
            .cloned()
    }

    /// Get all downloads, most recent first, as a stable snapshot.
    pub fn all(&self) -> Vec<Download> {
        self.store
            .lock()
            .expect("download store lock poisoned")
            .downloads
            .iter()
            .rev()
            .cloned()
            .collect()
    }

    /// Get in-progress downloads as snapshots.
    pub fn in_progress(&self) -> Vec<Download> {
        self.all()
            .into_iter()
            .filter(|download| {
                matches!(
                    download.state,
                    DownloadState::Pending | DownloadState::InProgress { .. }
                )
            })
            .collect()
    }

    /// Total download count.
    pub fn count(&self) -> usize {
        self.store
            .lock()
            .expect("download store lock poisoned")
            .downloads
            .len()
    }

    /// Clear completed, failed, and cancelled downloads from the UI list.
    pub fn clear_finished(&self) {
        let mut store = self.store.lock().expect("download store lock poisoned");
        store.downloads.retain(|download| {
            matches!(
                download.state,
                DownloadState::Pending | DownloadState::InProgress { .. }
            )
        });
    }

    fn is_cancelled(&self, id: DownloadId) -> bool {
        self.store
            .lock()
            .expect("download store lock poisoned")
            .cancelled
            .contains(&id)
    }
}

/// Return the suggested filename for an attachment response, if the server
/// explicitly requested a download with `Content-Disposition: attachment`.
///
/// `filename*` uses RFC 5987 percent encoding and takes precedence over the
/// legacy `filename` parameter when both are present.
pub fn attachment_filename(headers: &HashMap<String, String>, url: &str) -> Option<String> {
    let value = header_value(headers, "content-disposition")?;
    let (disposition, parameters) = split_content_disposition(value);
    if !disposition.eq_ignore_ascii_case("attachment") {
        return None;
    }

    let filename = parameters
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("filename*"))
        .and_then(|(_, value)| decode_extended_filename(value))
        .or_else(|| {
            parameters
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case("filename"))
                .map(|(_, value)| unquote_filename(value))
        })
        .filter(|filename| !filename.is_empty())
        .unwrap_or_else(|| suggest_filename(url));

    Some(safe_filename(&filename))
}

/// Extract a normalized MIME type without Content-Type parameters.
pub fn response_mime_type(headers: &HashMap<String, String>) -> Option<String> {
    header_value(headers, "content-type")
        .map(|value| {
            value
                .split(';')
                .next()
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase()
        })
        .filter(|value| !value.is_empty())
}

fn header_value<'a>(headers: &'a HashMap<String, String>, name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

fn split_content_disposition(value: &str) -> (&str, Vec<(String, String)>) {
    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    for (index, byte) in value.bytes().enumerate() {
        match byte {
            b'\\' if quoted => escaped = !escaped,
            b'"' if !escaped => quoted = !quoted,
            b';' if !quoted => {
                segments.push(&value[start..index]);
                start = index + 1;
                escaped = false;
            }
            _ => escaped = false,
        }
    }
    segments.push(&value[start..]);

    let disposition = segments.first().copied().unwrap_or_default().trim();
    let parameters = segments
        .into_iter()
        .skip(1)
        .filter_map(|segment| {
            let (name, value) = segment.split_once('=')?;
            Some((name.trim().to_owned(), value.trim().to_owned()))
        })
        .collect();
    (disposition, parameters)
}

fn unquote_filename(value: &str) -> String {
    let value = value.trim();
    let value = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(value);
    value.replace("\\\"", "\"").replace("\\\\", "\\")
}

fn decode_extended_filename(value: &str) -> Option<String> {
    let (_, encoded) = value.split_once("''")?;
    let mut bytes = Vec::with_capacity(encoded.len());
    let mut chars = encoded.as_bytes().iter().copied();
    while let Some(byte) = chars.next() {
        if byte == b'%' {
            let high = chars.next()?;
            let low = chars.next()?;
            let high = (high as char).to_digit(16)? as u8;
            let low = (low as char).to_digit(16)? as u8;
            bytes.push(high << 4 | low);
        } else {
            bytes.push(byte);
        }
    }
    String::from_utf8(bytes).ok()
}

/// Suggest a safe filename from a URL.
pub fn suggest_filename(url: &str) -> String {
    let candidate = url
        .split('?')
        .next()
        .and_then(|path| path.rsplit('/').next())
        .filter(|segment| !segment.is_empty())
        .unwrap_or("download");
    safe_filename(candidate)
}

fn safe_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' | '\0' => '_',
            ch if ch.is_control() => '_',
            ch => ch,
        })
        .collect();
    let cleaned = cleaned.trim_matches([' ', '.']);
    if cleaned.is_empty() || cleaned == "." || cleaned == ".." {
        "download".to_string()
    } else {
        cleaned.chars().take(180).collect()
    }
}

fn reserve_destination(dir: &Path, filename: &str) -> std::io::Result<PathBuf> {
    let original = safe_filename(filename);
    let stem = Path::new(&original)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("download");
    let extension = Path::new(&original)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| format!(".{extension}"))
        .unwrap_or_default();

    for suffix in 0..10_000 {
        let name = if suffix == 0 {
            original.clone()
        } else {
            format!("{stem} ({suffix}){extension}")
        };
        let candidate = dir.join(name);
        if !candidate.exists() && !temporary_path(&candidate).exists() {
            return Ok(candidate);
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "could not find an unused filename",
    ))
}

fn temporary_path(destination: &Path) -> PathBuf {
    let mut name = destination.as_os_str().to_os_string();
    name.push(".part");
    PathBuf::from(name)
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

    #[test]
    fn start_and_get_download() {
        let mgr = DownloadManager::new();
        let id = mgr.start_download(
            "https://example.com/file.zip",
            "file.zip",
            Path::new("/tmp"),
        );
        assert_eq!(mgr.get(id).unwrap().url, "https://example.com/file.zip");
        assert_eq!(mgr.get(id).unwrap().state, DownloadState::Pending);
    }

    #[test]
    fn update_progress() {
        let mgr = DownloadManager::new();
        let id = mgr.start_download(
            "https://example.com/file.zip",
            "file.zip",
            Path::new("/tmp"),
        );
        mgr.update_progress(id, 500, Some(1000));
        assert_eq!(
            mgr.get(id).unwrap().state,
            DownloadState::InProgress {
                received: 500,
                total: Some(1000)
            }
        );
    }

    #[test]
    fn cancel_prevents_completion() {
        let mgr = DownloadManager::new();
        let id = mgr.start_download("https://example.com/a.txt", "a.txt", Path::new("/tmp"));
        mgr.cancel(id);
        mgr.mark_complete(id);
        assert_eq!(mgr.get(id).unwrap().state, DownloadState::Cancelled);
    }

    #[test]
    fn reserve_destination_adds_a_suffix() {
        let dir = std::env::temp_dir().join(format!("vigo-download-test-{}", current_timestamp()));
        fs::create_dir_all(&dir).unwrap();
        std::fs::File::create(dir.join("report.pdf")).unwrap();
        assert_eq!(
            reserve_destination(&dir, "report.pdf").unwrap(),
            dir.join("report (1).pdf")
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn filename_is_sanitized() {
        assert_eq!(
            suggest_filename("https://example.test/../../evil?.zip"),
            "evil"
        );
        assert_eq!(safe_filename("..\\evil:thing?.txt"), "_evil_thing_.txt");
    }

    #[test]
    fn queue_download_transfers_a_local_http_response() {
        use std::net::TcpListener;
        use std::time::{Duration, Instant};

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request).unwrap();
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 18\r\nConnection: close\r\n\r\nVigo download test")
                .unwrap();
        });

        let dir = std::env::temp_dir().join(format!(
            "vigo-download-transfer-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let manager = DownloadManager::with_download_dir(&dir);
        let id = manager
            .queue_download(&format!("http://{address}/fixture.txt"))
            .unwrap();

        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            if matches!(manager.get(id).unwrap().state, DownloadState::Complete) {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        let download = manager.get(id).unwrap();
        assert_eq!(download.state, DownloadState::Complete);
        assert_eq!(fs::read(download.path).unwrap(), b"Vigo download test");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn queue_response_writes_navigation_bytes_without_refetching() {
        use std::time::{Duration, Instant};

        let dir = std::env::temp_dir().join(format!(
            "vigo-download-response-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let manager = DownloadManager::with_download_dir(&dir);
        let id = manager
            .queue_response(
                "https://example.test/report",
                Some("report.txt"),
                Some("text/plain"),
                b"downloaded through navigation".to_vec(),
            )
            .unwrap();

        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            if matches!(manager.get(id).unwrap().state, DownloadState::Complete) {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        let download = manager.get(id).unwrap();
        assert_eq!(download.state, DownloadState::Complete);
        assert_eq!(download.mime_type.as_deref(), Some("text/plain"));
        assert_eq!(
            fs::read(download.path).unwrap(),
            b"downloaded through navigation"
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn attachment_filename_prefers_rfc5987_and_sanitizes_it() {
        let mut headers = HashMap::new();
        headers.insert(
            "Content-Disposition".to_owned(),
            "attachment; filename=backup.zip; filename*=UTF-8''Vigo%20report%20%F0%9F%93%84.txt"
                .to_owned(),
        );
        headers.insert(
            "content-type".to_owned(),
            "text/plain; charset=utf-8".to_owned(),
        );

        assert_eq!(
            attachment_filename(&headers, "https://example.test/export"),
            Some("Vigo report 📄.txt".to_owned())
        );
        assert_eq!(response_mime_type(&headers).as_deref(), Some("text/plain"));
    }

    #[test]
    fn inline_content_disposition_does_not_trigger_download() {
        let mut headers = HashMap::new();
        headers.insert(
            "content-disposition".to_owned(),
            "inline; filename=manual.pdf".to_owned(),
        );
        assert_eq!(
            attachment_filename(&headers, "https://example.test/manual"),
            None
        );
    }

    #[test]
    fn clear_finished_keeps_active_downloads() {
        let mgr = DownloadManager::new();
        let finished = mgr.start_download("https://a.com/a", "a", Path::new("/tmp"));
        let _pending = mgr.start_download("https://b.com/b", "b", Path::new("/tmp"));
        mgr.mark_complete(finished);
        mgr.clear_finished();
        assert_eq!(mgr.count(), 1);
    }
}

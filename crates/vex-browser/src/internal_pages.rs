// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! `vex://` internal page generators.
//!
//! Each function returns an HTML string that gets loaded into a tab
//! via `Tab::load_html()`. Pages are styled consistently using the
//! Vigo purple-dark theme.

use crate::bookmarks::Bookmark;
use crate::downloads::Download;
use crate::history::HistoryRecord;

/// CSS shared by all internal pages.
const INTERNAL_CSS: &str = r#"
body {
    background: #0b0d14;
    color: #e8ebf6;
    font-family: system-ui, -apple-system, sans-serif;
    margin: 0;
    padding: 32px;
    line-height: 1.6;
}
h1 { color: #c7b8ff; margin-top: 0; font-size: 34px; line-height: 1.2; }
h2 { color: #a9b7ff; margin: 0 0 10px; }
a { color: #9aa8ff; text-decoration: none; }
a:hover { text-decoration: underline; }
table { border-collapse: collapse; width: 100%; margin: 16px 0; }
th, td { text-align: left; padding: 10px 12px; border-bottom: 1px solid #252a3a; }
th { color: #a9b7ff; }
.card {
    background: #141826;
    border-radius: 12px;
    padding: 20px 22px;
    margin: 14px 0;
}
.hero {
    background: #121624;
    border-radius: 14px;
    padding: 24px;
    margin-bottom: 18px;
}
.hero p {
    color: #b8bfd9;
    margin: 8px 0 0;
}
.grid {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
}
.grid-item {
    background: #171c2b;
    border-radius: 10px;
    padding: 14px;
    width: 220px;
    cursor: pointer;
}
.grid-item:hover { background: #202740; }
.grid-item h3 { margin: 0 0 4px; color: #d2d9ff; font-size: 14px; }
.grid-item p { margin: 0; color: #8f98bd; font-size: 12px; }
.empty { color: #7c84a5; font-style: italic; }
input[type="text"], input[type="search"] {
    background: #121624;
    color: #e8ebf6;
    border: 1px solid #2b3147;
    border-radius: 8px;
    padding: 8px 12px;
    font-size: 14px;
    width: 300px;
}
.btn {
    background: #6c63ff;
    color: #fff;
    border: none;
    border-radius: 8px;
    padding: 8px 16px;
    cursor: pointer;
    font-size: 14px;
}
.section { margin-bottom: 32px; }
.muted { color: #9ca4c8; }
"#;

/// Generate the new tab page.
#[must_use]
pub fn newtab_page() -> String {
    format!(
        r#"<!DOCTYPE html>
<html><head><title>New Tab</title><style>{INTERNAL_CSS}</style></head>
<body>
<div class="hero">
    <h1>Vigo Browser</h1>
    <p>Fast, private browsing powered by the Vex engine.</p>
    <p class="muted">Type an address above or open one of the internal pages below.</p>
</div>

<div class="section">
    <h2>Quick access</h2>
  <div class="grid">
        <div class="grid-item"><h3>History</h3><p>vex://history</p></div>
        <div class="grid-item"><h3>Bookmarks</h3><p>vex://bookmarks</p></div>
        <div class="grid-item"><h3>Downloads</h3><p>vex://downloads</p></div>
        <div class="grid-item"><h3>Settings</h3><p>vex://settings</p></div>
  </div>
</div>

<div class="card">
    <h2>About this build</h2>
    <p>Rust + Zig browser engine with native UI chrome and privacy defaults.</p>
    <p class="muted">If a page fails to load, check URL format or try another site.</p>
</div>
</body></html>"#
    )
}

/// Generate the settings page.
#[must_use]
pub fn settings_page() -> String {
    format!(
        r#"<!DOCTYPE html>
<html><head><title>Settings — Vigo</title><style>{INTERNAL_CSS}</style></head>
<body>
<h1>Settings</h1>

<div class="card">
  <h2>General</h2>
  <table>
    <tr><td>Search Engine</td><td>DuckDuckGo</td></tr>
    <tr><td>Default Zoom</td><td>100%</td></tr>
    <tr><td>Home Page</td><td>vex://newtab</td></tr>
  </table>
</div>

<div class="card">
  <h2>Privacy &amp; Security</h2>
  <table>
    <tr><td>Canvas Fingerprint Protection</td><td>Enabled</td></tr>
    <tr><td>WebGL Masking</td><td>Enabled</td></tr>
    <tr><td>Font Restriction</td><td>Web-safe only</td></tr>
    <tr><td>HTTPS-Only Mode</td><td>Enabled</td></tr>
    <tr><td>Tracking Parameter Stripping</td><td>Enabled</td></tr>
  </table>
</div>

<div class="card">
  <h2>About</h2>
  <table>
    <tr><td>Engine</td><td>Vex (Vigo Engine X)</td></tr>
    <tr><td>Version</td><td>0.1.0</td></tr>
    <tr><td>License</td><td>MPL-2.0</td></tr>
  </table>
</div>
</body></html>"#
    )
}

/// Generate the history page from a list of entries.
#[must_use]
pub fn history_page(records: &[HistoryRecord]) -> String {
    let mut rows = String::new();
    if records.is_empty() {
        rows.push_str(r#"<tr><td colspan="3" class="empty">No history yet.</td></tr>"#);
    } else {
        for entry in records.iter().rev().take(200) {
            rows.push_str(&format!(
                "<tr><td><a href=\"{}\">{}</a></td><td>{}</td><td>{}</td></tr>\n",
                entry.url,
                html_escape(&entry.title),
                html_escape(&entry.url),
                entry.visited_at,
            ));
        }
    }

    format!(
        r#"<!DOCTYPE html>
<html><head><title>History — Vigo</title><style>{INTERNAL_CSS}</style></head>
<body>
<h1>📖 History</h1>
<table>
  <thead><tr><th>Title</th><th>URL</th><th>Visited</th></tr></thead>
  <tbody>{rows}</tbody>
</table>
</body></html>"#
    )
}

/// Generate the bookmarks page.
#[must_use]
pub fn bookmarks_page(bookmarks: &[Bookmark]) -> String {
    let mut rows = String::new();
    if bookmarks.is_empty() {
        rows.push_str(r#"<tr><td colspan="2" class="empty">No bookmarks yet. Press Ctrl+D to add one.</td></tr>"#);
    } else {
        for b in bookmarks {
            rows.push_str(&format!(
                "<tr><td><a href=\"{}\">{}</a></td><td>{}</td></tr>\n",
                b.url,
                html_escape(&b.title),
                html_escape(&b.url),
            ));
        }
    }

    format!(
        r#"<!DOCTYPE html>
<html><head><title>Bookmarks — Vigo</title><style>{INTERNAL_CSS}</style></head>
<body>
<h1>🔖 Bookmarks</h1>
<table>
  <thead><tr><th>Title</th><th>URL</th></tr></thead>
  <tbody>{rows}</tbody>
</table>
</body></html>"#
    )
}

/// Generate the downloads page.
#[must_use]
pub fn downloads_page(downloads: &[Download]) -> String {
    let mut rows = String::new();
    if downloads.is_empty() {
        rows.push_str(r#"<tr><td colspan="3" class="empty">No downloads yet.</td></tr>"#);
    } else {
        for d in downloads {
            let status = format!("{:?}", d.state);
            rows.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                html_escape(&d.filename),
                html_escape(&d.url),
                status,
            ));
        }
    }

    format!(
        r#"<!DOCTYPE html>
<html><head><title>Downloads — Vigo</title><style>{INTERNAL_CSS}</style></head>
<body>
<h1>⬇️ Downloads</h1>
<table>
  <thead><tr><th>File</th><th>URL</th><th>Status</th></tr></thead>
  <tbody>{rows}</tbody>
</table>
</body></html>"#
    )
}

/// Minimal HTML entity escaping.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Resolve a `vex://` URL to its internal page name.
///
/// Returns `None` for unrecognized URLs.
#[must_use]
pub fn resolve_internal_url(url: &str) -> Option<&str> {
    let path = url.strip_prefix("vex://")?;
    match path {
        "newtab" | "welcome" | "" => Some("newtab"),
        "settings" => Some("settings"),
        "history" => Some("history"),
        "bookmarks" => Some("bookmarks"),
        "downloads" => Some("downloads"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newtab_contains_title() {
        let html = newtab_page();
        assert!(html.contains("<title>New Tab</title>"));
        assert!(html.contains("Vigo Browser"));
    }

    #[test]
    fn settings_contains_engine_info() {
        let html = settings_page();
        assert!(html.contains("Vex (Vigo Engine X)"));
        assert!(html.contains("MPL-2.0"));
    }

    #[test]
    fn history_empty() {
        let html = history_page(&[]);
        assert!(html.contains("No history yet"));
    }

    #[test]
    fn history_with_entries() {
        let records = vec![HistoryRecord {
            url: "https://example.com".to_string(),
            title: "Example".to_string(),
            visited_at: 1709290800,
            visit_count: 1,
        }];
        let html = history_page(&records);
        assert!(html.contains("example.com"));
        assert!(html.contains("Example"));
    }

    #[test]
    fn bookmarks_empty() {
        let html = bookmarks_page(&[]);
        assert!(html.contains("No bookmarks yet"));
    }

    #[test]
    fn downloads_empty() {
        let html = downloads_page(&[]);
        assert!(html.contains("No downloads yet"));
    }

    #[test]
    fn resolve_urls() {
        assert_eq!(resolve_internal_url("vex://newtab"), Some("newtab"));
        assert_eq!(resolve_internal_url("vex://settings"), Some("settings"));
        assert_eq!(resolve_internal_url("vex://history"), Some("history"));
        assert_eq!(resolve_internal_url("vex://bookmarks"), Some("bookmarks"));
        assert_eq!(resolve_internal_url("vex://downloads"), Some("downloads"));
        assert_eq!(resolve_internal_url("vex://welcome"), Some("newtab"));
        assert_eq!(resolve_internal_url("vex://"), Some("newtab"));
        assert_eq!(resolve_internal_url("vex://unknown"), None);
        assert_eq!(resolve_internal_url("https://example.com"), None);
    }

    #[test]
    fn html_escape_special_chars() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a&b"), "a&amp;b");
    }
}

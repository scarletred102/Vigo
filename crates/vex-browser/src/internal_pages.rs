// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! `vex://` internal page generators.
//!
//! Each function returns an HTML string that gets loaded into a tab
//! via `Tab::load_html()`. Pages are styled consistently using the
//! Vigo clean-light product theme.

use crate::bookmarks::Bookmark;
use crate::downloads::Download;
use crate::history::HistoryRecord;

/// CSS shared by all internal pages.
const INTERNAL_CSS: &str = r#"
body {
    background: #f4f7fb;
    color: #1d2433;
    font-family: system-ui, -apple-system, sans-serif;
    margin: 0;
    padding: 28px;
    line-height: 1.6;
}
.page {
    max-width: 1100px;
    margin: 0 auto;
}
h1 { color: #1f293d; margin: 0 0 8px; font-size: 34px; line-height: 1.2; }
h2 { color: #33425f; margin: 0 0 10px; }
a { color: #2c66e2; text-decoration: none; }
a:hover { text-decoration: underline; }
.card {
    background: #ffffff;
    border: 1px solid #d8e2f1;
    border-radius: 12px;
    padding: 20px 22px;
    margin: 14px 0;
}
.hero {
    background: linear-gradient(145deg, #ffffff 0%, #f6f9ff 100%);
    border: 1px solid #d5dff0;
    border-radius: 14px;
    padding: 24px;
    margin-bottom: 18px;
}
.hero p {
    color: #556380;
    margin: 8px 0 0;
}
.grid {
    display: block;
}
.grid-item {
    display: block;
    background: #ffffff;
    border: 1px solid #d7e1f0;
    border-radius: 10px;
    padding: 14px 16px;
    cursor: pointer;
    text-decoration: none;
    color: inherit;
    transition: transform .08s ease, box-shadow .08s ease, border-color .08s ease;
    margin: 0 0 12px;
}
.grid-item:hover {
    transform: translateY(-1px);
    box-shadow: 0 6px 14px rgba(36, 66, 120, 0.08);
    border-color: #b9cbed;
    text-decoration: none;
}
.grid-item h3 { margin: 0 0 4px; color: #28344f; font-size: 14px; }
.grid-item p { margin: 0; color: #65779a; font-size: 12px; }
.empty { color: #7a88a2; font-style: italic; }
input[type="text"], input[type="search"] {
    background: #ffffff;
    color: #1f2a3f;
    border: 1px solid #c8d7ed;
    border-radius: 8px;
    padding: 8px 12px;
    font-size: 14px;
    width: 300px;
}
.input-row { display: flex; align-items: center; gap: 10px; margin-top: 10px; }
.btn {
    background: #366ce5;
    color: #fff;
    border: none;
    border-radius: 8px;
    padding: 8px 16px;
    cursor: pointer;
    font-size: 14px;
}
.section { margin-bottom: 32px; }
.muted { color: #66778f; }
.record {
    background: #ffffff;
    border: 1px solid #d8e2f1;
    border-radius: 10px;
    padding: 14px 16px;
    margin: 10px 0;
}
.record-title { display: block; color: #2c66e2; font-size: 16px; }
.record-meta { color: #65779a; font-size: 13px; margin-top: 4px; }
.setting-row { border-top: 1px solid #dde5f2; padding: 12px 0; }
.setting-row:first-child { border-top: none; padding-top: 0; }
.setting-label { color: #33425f; font-weight: 600; }
.setting-value { color: #65779a; margin-top: 3px; }
.kpi {
    display: inline-block;
    font-size: 12px;
    font-weight: 600;
    background: #e8f0ff;
    color: #2d5fcc;
    border: 1px solid #c6d8fb;
    border-radius: 999px;
    padding: 2px 10px;
    margin-right: 6px;
}
"#;

/// Generate the new tab page.
#[must_use]
pub fn newtab_page() -> String {
    format!(
        r#"<!DOCTYPE html>
<html><head><title>New Tab</title><style>{INTERNAL_CSS}</style></head>
<body>
<div class="page">
<div class="hero">
    <h1>Vigo Browser</h1>
    <p>Fast, private browsing powered by the Vex engine.</p>
    <p class="muted">Type an address above or use quick actions below.</p>
    <div style="margin-top:10px">
        <span class="kpi">Privacy by default</span>
        <span class="kpi">Native Rust engine</span>
        <span class="kpi">Multi-layer isolation</span>
    </div>
</div>

<div class="section">
    <h2>Quick access</h2>
  <div class="grid">
        <a class="grid-item" href="vex://history"><h3>History</h3><p>vex://history</p></a>
        <a class="grid-item" href="vex://bookmarks"><h3>Bookmarks</h3><p>vex://bookmarks</p></a>
        <a class="grid-item" href="vex://downloads"><h3>Downloads</h3><p>vex://downloads</p></a>
        <a class="grid-item" href="vex://settings"><h3>Settings</h3><p>vex://settings</p></a>
  </div>
</div>

<div class="card">
    <h2>About this build</h2>
    <p>Rust + Zig browser engine with native UI chrome and privacy defaults.</p>
    <p class="muted">If a page fails to load, check URL format or try another site.</p>
</div>
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
<div class="page">
<h1>Settings</h1>

<div class="card">
  <h2>General</h2>
  <div class="setting-row"><div class="setting-label">Search Engine</div><div class="setting-value">DuckDuckGo</div></div>
  <div class="setting-row"><div class="setting-label">Default Zoom</div><div class="setting-value">100%</div></div>
  <div class="setting-row"><div class="setting-label">Home Page</div><div class="setting-value">vex://newtab</div></div>
</div>

<div class="card">
  <h2>Privacy &amp; Security</h2>
  <div class="setting-row"><div class="setting-label">Canvas Fingerprint Protection</div><div class="setting-value">Enabled</div></div>
  <div class="setting-row"><div class="setting-label">WebGL Masking</div><div class="setting-value">Enabled</div></div>
  <div class="setting-row"><div class="setting-label">Font Restriction</div><div class="setting-value">Web-safe only</div></div>
  <div class="setting-row"><div class="setting-label">HTTPS-Only Mode</div><div class="setting-value">Enabled</div></div>
  <div class="setting-row"><div class="setting-label">Tracking Parameter Stripping</div><div class="setting-value">Enabled</div></div>
</div>

<div class="card">
  <h2>About</h2>
  <div class="setting-row"><div class="setting-label">Engine</div><div class="setting-value">Vex (Vigo Engine X)</div></div>
  <div class="setting-row"><div class="setting-label">Version</div><div class="setting-value">0.1.0</div></div>
  <div class="setting-row"><div class="setting-label">License</div><div class="setting-value">MPL-2.0</div></div>
</div>
</div>
</body></html>"#
    )
}

/// Generate the history page from a list of entries.
#[must_use]
pub fn history_page(records: &[HistoryRecord]) -> String {
    let mut records_html = String::new();
    if records.is_empty() {
        records_html.push_str(r#"<div class="record empty">No history yet.</div>"#);
    } else {
        for entry in records.iter().take(200) {
            records_html.push_str(&format!(
                "<div class=\"record\"><a class=\"record-title\" href=\"{}\">{}</a><div class=\"record-meta\">{} · visited {}</div></div>\n",
                html_escape(&entry.url),
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
<div class="page">
<h1>History</h1>
{records_html}
</div>
</body></html>"#
    )
}

/// Generate the bookmarks page.
#[must_use]
pub fn bookmarks_page(bookmarks: &[Bookmark]) -> String {
    let mut records_html = String::new();
    if bookmarks.is_empty() {
        records_html.push_str(
            r#"<div class="record empty">No bookmarks yet. Press Ctrl+D to add one.</div>"#,
        );
    } else {
        for b in bookmarks {
            records_html.push_str(&format!(
                "<div class=\"record\"><a class=\"record-title\" href=\"{}\">{}</a><div class=\"record-meta\">{}</div></div>\n",
                html_escape(&b.url),
                html_escape(&b.title),
                html_escape(&b.url),
            ));
        }
    }

    format!(
        r#"<!DOCTYPE html>
<html><head><title>Bookmarks — Vigo</title><style>{INTERNAL_CSS}</style></head>
<body>
<div class="page">
<h1>Bookmarks</h1>
{records_html}
</div>
</body></html>"#
    )
}

/// Generate the downloads page.
#[must_use]
pub fn downloads_page(downloads: &[Download]) -> String {
    let mut records_html = String::new();
    if downloads.is_empty() {
        records_html.push_str(r#"<div class="record empty">No downloads yet.</div>"#);
    } else {
        for d in downloads {
            let status = format!("{:?}", d.state);
            records_html.push_str(&format!(
                "<div class=\"record\"><div class=\"setting-label\">{}</div><div class=\"record-meta\">{} · {}</div></div>\n",
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
<div class="page">
<h1>Downloads</h1>
{records_html}
</div>
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
        assert!(!html.contains("<table>"));
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
        assert!(!html.contains("<table>"));
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

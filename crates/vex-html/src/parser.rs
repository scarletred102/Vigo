// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Public parsing API.

use encoding_rs::Encoding;
use html5ever::{namespace_url, ns, LocalName, ParseOpts, QualName};
use tendril::TendrilSink;

use vex_dom::Document;

use crate::diagnostics::{diagnose_html, ParseDiagnostic};
use crate::sink::VexSink;

/// Parse a full HTML document and return the DOM tree.
///
/// ```
/// let doc = vex_html::parse_html("<p>Hello</p>");
/// assert!(doc.root_element().is_some());
/// ```
pub fn parse_html(input: &str) -> Document {
    let sink = VexSink::new();
    html5ever::parse_document(sink, ParseOpts::default()).one(input)
}

/// Parse HTML and return parser diagnostics collected from a heuristic pass.
pub fn parse_html_with_diagnostics(input: &str) -> (Document, Vec<ParseDiagnostic>) {
    let diagnostics = diagnose_html(input);
    (parse_html(input), diagnostics)
}

/// Parse raw HTML bytes into a DOM document.
///
/// Supports BOM-aware UTF-8/UTF-16 decoding. Unknown encodings fall back to
/// UTF-8 lossy decoding.
pub fn parse_html_bytes(input: &[u8]) -> Document {
    parse_html(&decode_html_bytes(input, None))
}

/// Parse raw HTML bytes with optional HTTP `Content-Type` header context.
///
/// Supports BOM-aware decoding plus charset sniffing from `Content-Type` and
/// `<meta charset=...>` declarations.
pub fn parse_html_bytes_with_content_type(input: &[u8], content_type: Option<&str>) -> Document {
    parse_html(&decode_html_bytes(input, content_type))
}

/// Parse an HTML fragment in the context of a given element tag.
///
/// Useful for `innerHTML`-style parsing.
pub fn parse_html_fragment(input: &str, context_tag: &str) -> Document {
    let sink = VexSink::new();
    let context = QualName::new(None, ns!(html), LocalName::from(context_tag));
    html5ever::parse_fragment(sink, ParseOpts::default(), context, vec![]).one(input)
}

fn decode_html_bytes(input: &[u8], content_type: Option<&str>) -> String {
    // UTF-8 BOM
    if let Some(rest) = input.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(rest).into_owned();
    }

    // UTF-16LE BOM
    if let Some(rest) = input.strip_prefix(&[0xFF, 0xFE]) {
        return decode_utf16(rest, true);
    }

    // UTF-16BE BOM
    if let Some(rest) = input.strip_prefix(&[0xFE, 0xFF]) {
        return decode_utf16(rest, false);
    }

    // HTTP header charset has precedence over meta charset.
    if let Some(label) = content_type.and_then(extract_charset_from_content_type) {
        if let Some(decoded) = decode_with_label(input, label.as_bytes()) {
            return decoded;
        }
    }

    // Sniff meta charset from first bytes.
    if let Some(label) = sniff_meta_charset(input) {
        if let Some(decoded) = decode_with_label(input, label.as_bytes()) {
            return decoded;
        }
    }

    String::from_utf8_lossy(input).into_owned()
}

fn decode_with_label(input: &[u8], label: &[u8]) -> Option<String> {
    let encoding = Encoding::for_label(label)?;
    let (text, _used_replacement, _had_errors) = encoding.decode(input);
    Some(text.into_owned())
}

fn extract_charset_from_content_type(content_type: &str) -> Option<String> {
    let lower = content_type.to_ascii_lowercase();
    let idx = lower.find("charset=")?;
    let mut value = lower[idx + "charset=".len()..].trim().to_string();
    // Trim optional ; suffix and quotes.
    if let Some(semi) = value.find(';') {
        value.truncate(semi);
    }
    let value = value.trim().trim_matches('"').trim_matches('\'');
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn sniff_meta_charset(input: &[u8]) -> Option<String> {
    let probe_len = input.len().min(4096);
    let probe = String::from_utf8_lossy(&input[..probe_len]).to_ascii_lowercase();

    // <meta charset="...">
    if let Some(idx) = probe.find("<meta") {
        let slice = &probe[idx..];
        if let Some(ch_idx) = slice.find("charset=") {
            let after = &slice[ch_idx + "charset=".len()..];
            return take_charset_token(after);
        }
    }

    // <meta http-equiv="content-type" content="text/html; charset=...">
    if let Some(idx) = probe.find("http-equiv=\"content-type\"") {
        let slice = &probe[idx..];
        if let Some(ch_idx) = slice.find("charset=") {
            let after = &slice[ch_idx + "charset=".len()..];
            return take_charset_token(after);
        }
    }

    None
}

fn take_charset_token(s: &str) -> Option<String> {
    let trimmed = s.trim_start();
    if trimmed.is_empty() {
        return None;
    }

    let token = if let Some(rest) = trimmed.strip_prefix('"') {
        rest.split('"').next().unwrap_or("")
    } else if let Some(rest) = trimmed.strip_prefix('\'') {
        rest.split('\'').next().unwrap_or("")
    } else {
        trimmed
            .split(|c: char| c.is_whitespace() || c == '>' || c == ';')
            .next()
            .unwrap_or("")
    };

    let token = token.trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

fn decode_utf16(bytes: &[u8], little_endian: bool) -> String {
    let mut units = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.chunks_exact(2) {
        let unit = if little_endian {
            u16::from_le_bytes([chunk[0], chunk[1]])
        } else {
            u16::from_be_bytes([chunk[0], chunk[1]])
        };
        units.push(unit);
    }

    std::char::decode_utf16(units)
        .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_html_bytes_utf8_bom() {
        let bytes = b"\xEF\xBB\xBF<html><body><p>Hello</p></body></html>";
        let doc = parse_html_bytes(bytes);
        let p = doc.get_elements_by_tag_name("p");
        assert_eq!(p.len(), 1);
        assert_eq!(doc.text_content(p[0]), "Hello");
    }

    #[test]
    fn parse_html_bytes_utf16le_bom() {
        // "<p>Hi</p>" in UTF-16LE with BOM.
        let mut bytes = vec![0xFF, 0xFE];
        for u in "<p>Hi</p>".encode_utf16() {
            bytes.extend_from_slice(&u.to_le_bytes());
        }

        let doc = parse_html_bytes(&bytes);
        let p = doc.get_elements_by_tag_name("p");
        assert_eq!(p.len(), 1);
        assert_eq!(doc.text_content(p[0]), "Hi");
    }

    #[test]
    fn parse_html_bytes_utf16be_bom() {
        // "<p>Hi</p>" in UTF-16BE with BOM.
        let mut bytes = vec![0xFE, 0xFF];
        for u in "<p>Hi</p>".encode_utf16() {
            bytes.extend_from_slice(&u.to_be_bytes());
        }

        let doc = parse_html_bytes(&bytes);
        let p = doc.get_elements_by_tag_name("p");
        assert_eq!(p.len(), 1);
        assert_eq!(doc.text_content(p[0]), "Hi");
    }

    #[test]
    fn parse_html_bytes_with_content_type_charset() {
        // café in windows-1252 uses 0xE9.
        let bytes = b"<html><body><p>caf\xE9</p></body></html>";
        let doc = parse_html_bytes_with_content_type(bytes, Some("text/html; charset=windows-1252"));
        let p = doc.get_elements_by_tag_name("p");
        assert_eq!(p.len(), 1);
        assert_eq!(doc.text_content(p[0]), "café");
    }

    #[test]
    fn parse_html_bytes_with_meta_charset() {
        let bytes = b"<meta charset=\"windows-1252\"><p>caf\xE9</p>";
        let doc = parse_html_bytes(bytes);
        let p = doc.get_elements_by_tag_name("p");
        assert_eq!(p.len(), 1);
        assert_eq!(doc.text_content(p[0]), "café");
    }
}

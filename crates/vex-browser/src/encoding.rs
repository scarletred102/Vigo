// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Encoding API (TextEncoder / TextDecoder).
//!
//! Provides UTF-8 encoding and decoding, plus support for
//! common legacy encodings.

// ── TextEncoder ──────────────────────────────────────────────────────────────

/// TextEncoder — always encodes to UTF-8 per the spec.
#[derive(Debug, Clone)]
pub struct TextEncoder {
    pub encoding: &'static str,
}

impl Default for TextEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl TextEncoder {
    pub fn new() -> Self {
        Self { encoding: "utf-8" }
    }

    /// Encode a string to UTF-8 bytes.
    pub fn encode(&self, input: &str) -> Vec<u8> {
        input.as_bytes().to_vec()
    }

    /// Encode into an existing buffer. Returns number of bytes written.
    pub fn encode_into(&self, input: &str, dest: &mut [u8]) -> EncodeResult {
        let bytes = input.as_bytes();
        let len = bytes.len().min(dest.len());
        dest[..len].copy_from_slice(&bytes[..len]);

        // Count how many complete UTF-8 chars fit
        let s = std::str::from_utf8(&bytes[..len]).unwrap_or("");
        EncodeResult {
            read: s.chars().count(),
            written: len,
        }
    }
}

/// Result of encode_into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodeResult {
    pub read: usize,
    pub written: usize,
}

// ── TextDecoder ──────────────────────────────────────────────────────────────

/// Supported encodings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Encoding {
    Utf8,
    Utf16Le,
    Utf16Be,
    Ascii,
    Latin1,
}

impl Encoding {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Utf8 => "utf-8",
            Self::Utf16Le => "utf-16le",
            Self::Utf16Be => "utf-16be",
            Self::Ascii => "us-ascii",
            Self::Latin1 => "iso-8859-1",
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        match label.to_ascii_lowercase().as_str() {
            "utf-8" | "utf8" => Some(Self::Utf8),
            "utf-16le" | "utf-16" => Some(Self::Utf16Le),
            "utf-16be" => Some(Self::Utf16Be),
            "ascii" | "us-ascii" => Some(Self::Ascii),
            "iso-8859-1" | "latin1" | "latin-1" => Some(Self::Latin1),
            _ => None,
        }
    }
}

/// Decode error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError {
    pub message: String,
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DecodeError: {}", self.message)
    }
}

/// TextDecoder — decodes bytes to strings.
#[derive(Debug, Clone)]
pub struct TextDecoder {
    pub encoding: Encoding,
    pub fatal: bool,
    pub ignore_bom: bool,
}

impl Default for TextDecoder {
    fn default() -> Self {
        Self::new(Encoding::Utf8)
    }
}

impl TextDecoder {
    pub fn new(encoding: Encoding) -> Self {
        Self {
            encoding,
            fatal: false,
            ignore_bom: false,
        }
    }

    /// Create from a label string.
    pub fn from_label(label: &str) -> Option<Self> {
        Encoding::from_label(label).map(Self::new)
    }

    pub fn with_fatal(mut self, fatal: bool) -> Self {
        self.fatal = fatal;
        self
    }

    /// Decode bytes to a string.
    pub fn decode(&self, input: &[u8]) -> Result<String, DecodeError> {
        match self.encoding {
            Encoding::Utf8 => {
                let input = if !self.ignore_bom && input.starts_with(&[0xEF, 0xBB, 0xBF]) {
                    &input[3..]
                } else {
                    input
                };

                if self.fatal {
                    std::str::from_utf8(input)
                        .map(|s| s.to_string())
                        .map_err(|e| DecodeError {
                            message: format!("Invalid UTF-8: {e}"),
                        })
                } else {
                    Ok(String::from_utf8_lossy(input).into_owned())
                }
            }
            Encoding::Latin1 => Ok(input.iter().map(|&b| b as char).collect()),
            Encoding::Ascii => {
                if self.fatal && input.iter().any(|&b| b > 127) {
                    Err(DecodeError {
                        message: "Invalid ASCII byte".to_string(),
                    })
                } else {
                    Ok(input
                        .iter()
                        .map(|&b| if b > 127 { '\u{FFFD}' } else { b as char })
                        .collect())
                }
            }
            Encoding::Utf16Le => decode_utf16(input, false, self.fatal),
            Encoding::Utf16Be => decode_utf16(input, true, self.fatal),
        }
    }
}

fn decode_utf16(input: &[u8], big_endian: bool, fatal: bool) -> Result<String, DecodeError> {
    if input.len() % 2 != 0 && fatal {
        return Err(DecodeError {
            message: "Odd number of bytes for UTF-16".to_string(),
        });
    }

    let mut result = String::new();
    let mut i = 0;
    while i + 1 < input.len() {
        let code_unit = if big_endian {
            u16::from_be_bytes([input[i], input[i + 1]])
        } else {
            u16::from_le_bytes([input[i], input[i + 1]])
        };

        if let Some(c) = char::from_u32(code_unit as u32) {
            result.push(c);
        } else if fatal {
            return Err(DecodeError {
                message: format!("Invalid UTF-16 code unit: {code_unit:#06X}"),
            });
        } else {
            result.push('\u{FFFD}');
        }
        i += 2;
    }
    Ok(result)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── TextEncoder ─────────────────────────────────────────────

    #[test]
    fn encode_ascii() {
        let enc = TextEncoder::new();
        assert_eq!(enc.encode("hello"), b"hello");
    }

    #[test]
    fn encode_unicode() {
        let enc = TextEncoder::new();
        let bytes = enc.encode("日本語");
        assert_eq!(bytes.len(), 9); // 3 chars × 3 bytes each
    }

    #[test]
    fn encode_into_buffer() {
        let enc = TextEncoder::new();
        let mut buf = [0u8; 5];
        let result = enc.encode_into("hello world", &mut buf);
        assert_eq!(result.written, 5);
        assert_eq!(&buf, b"hello");
    }

    #[test]
    fn encoder_encoding() {
        let enc = TextEncoder::new();
        assert_eq!(enc.encoding, "utf-8");
    }

    // ── TextDecoder ─────────────────────────────────────────────

    #[test]
    fn decode_utf8() {
        let dec = TextDecoder::new(Encoding::Utf8);
        assert_eq!(dec.decode(b"hello").unwrap(), "hello");
    }

    #[test]
    fn decode_utf8_with_bom() {
        let dec = TextDecoder::new(Encoding::Utf8);
        let mut input = vec![0xEF, 0xBB, 0xBF];
        input.extend_from_slice(b"hello");
        assert_eq!(dec.decode(&input).unwrap(), "hello");
    }

    #[test]
    fn decode_utf8_invalid_lossy() {
        let dec = TextDecoder::new(Encoding::Utf8);
        let input = vec![0xFF, 0xFE, b'a'];
        let result = dec.decode(&input).unwrap();
        assert!(result.contains('\u{FFFD}'));
    }

    #[test]
    fn decode_utf8_invalid_fatal() {
        let dec = TextDecoder::new(Encoding::Utf8).with_fatal(true);
        let input = vec![0xFF, 0xFE];
        assert!(dec.decode(&input).is_err());
    }

    #[test]
    fn decode_latin1() {
        let dec = TextDecoder::new(Encoding::Latin1);
        let input = vec![0xC9]; // É in Latin-1
        assert_eq!(dec.decode(&input).unwrap(), "É");
    }

    #[test]
    fn decode_ascii_valid() {
        let dec = TextDecoder::new(Encoding::Ascii);
        assert_eq!(dec.decode(b"hello").unwrap(), "hello");
    }

    #[test]
    fn decode_ascii_invalid_replacement() {
        let dec = TextDecoder::new(Encoding::Ascii);
        let result = dec.decode(&[0x80]).unwrap();
        assert_eq!(result, "\u{FFFD}");
    }

    #[test]
    fn decode_ascii_invalid_fatal() {
        let dec = TextDecoder::new(Encoding::Ascii).with_fatal(true);
        assert!(dec.decode(&[0x80]).is_err());
    }

    #[test]
    fn decode_utf16le() {
        let dec = TextDecoder::new(Encoding::Utf16Le);
        let input = vec![0x41, 0x00, 0x42, 0x00]; // "AB" in UTF-16LE
        assert_eq!(dec.decode(&input).unwrap(), "AB");
    }

    #[test]
    fn decode_utf16be() {
        let dec = TextDecoder::new(Encoding::Utf16Be);
        let input = vec![0x00, 0x41, 0x00, 0x42]; // "AB" in UTF-16BE
        assert_eq!(dec.decode(&input).unwrap(), "AB");
    }

    #[test]
    fn encoding_from_label() {
        assert_eq!(Encoding::from_label("utf-8"), Some(Encoding::Utf8));
        assert_eq!(Encoding::from_label("UTF8"), Some(Encoding::Utf8));
        assert_eq!(Encoding::from_label("latin1"), Some(Encoding::Latin1));
        assert_eq!(Encoding::from_label("unknown"), None);
    }

    #[test]
    fn decoder_from_label() {
        let dec = TextDecoder::from_label("utf-8").unwrap();
        assert_eq!(dec.encoding, Encoding::Utf8);
        assert!(TextDecoder::from_label("invalid").is_none());
    }

    #[test]
    fn encoding_label() {
        assert_eq!(Encoding::Utf8.label(), "utf-8");
        assert_eq!(Encoding::Latin1.label(), "iso-8859-1");
    }
}

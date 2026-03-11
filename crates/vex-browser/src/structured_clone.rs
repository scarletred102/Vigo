// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Structured Clone Algorithm implementation.
//!
//! The structured clone algorithm is used by `postMessage()`, `IndexedDB`,
//! `structuredClone()`, and other APIs to deep-copy JavaScript values.
//! <https://html.spec.whatwg.org/multipage/structured-data.html>

use std::collections::HashMap;

// ── StructuredValue ──────────────────────────────────────────────────────────

/// A value that can be structured-cloned.
///
/// This is a Rust enum representing the serializable subset of JavaScript values.
#[derive(Debug, Clone, PartialEq)]
pub enum StructuredValue {
    /// `undefined`
    Undefined,
    /// `null`
    Null,
    /// Boolean.
    Boolean(bool),
    /// Number (f64, includes NaN, Infinity).
    Number(f64),
    /// BigInt (stored as string representation).
    BigInt(String),
    /// String.
    String(String),
    /// Date (milliseconds since Unix epoch).
    Date(f64),
    /// RegExp (pattern, flags).
    RegExp { pattern: String, flags: String },
    /// ArrayBuffer (raw bytes).
    ArrayBuffer(Vec<u8>),
    /// A typed array view into a buffer.
    TypedArray {
        kind: TypedArrayKind,
        buffer: Vec<u8>,
        byte_offset: usize,
        length: usize,
    },
    /// DataView over a buffer.
    DataView {
        buffer: Vec<u8>,
        byte_offset: usize,
        byte_length: usize,
    },
    /// Map (ordered key-value pairs).
    Map(Vec<(StructuredValue, StructuredValue)>),
    /// Set (ordered unique values).
    Set(Vec<StructuredValue>),
    /// Array.
    Array(Vec<StructuredValue>),
    /// Object (string-keyed properties).
    Object(Vec<(String, StructuredValue)>),
    /// Blob.
    Blob { data: Vec<u8>, mime_type: String },
    /// File.
    File {
        data: Vec<u8>,
        name: String,
        mime_type: String,
        last_modified: f64,
    },
    /// ImageData (width, height, RGBA pixel data).
    ImageData {
        width: u32,
        height: u32,
        data: Vec<u8>,
    },
    /// Error (name, message, stack).
    Error {
        name: String,
        message: String,
        stack: Option<String>,
    },
}

/// Typed array kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypedArrayKind {
    Int8,
    Uint8,
    Uint8Clamped,
    Int16,
    Uint16,
    Int32,
    Uint32,
    Float32,
    Float64,
    BigInt64,
    BigUint64,
}

impl TypedArrayKind {
    pub fn element_size(&self) -> usize {
        match self {
            Self::Int8 | Self::Uint8 | Self::Uint8Clamped => 1,
            Self::Int16 | Self::Uint16 => 2,
            Self::Int32 | Self::Uint32 | Self::Float32 => 4,
            Self::Float64 | Self::BigInt64 | Self::BigUint64 => 8,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Int8 => "Int8Array",
            Self::Uint8 => "Uint8Array",
            Self::Uint8Clamped => "Uint8ClampedArray",
            Self::Int16 => "Int16Array",
            Self::Uint16 => "Uint16Array",
            Self::Int32 => "Int32Array",
            Self::Uint32 => "Uint32Array",
            Self::Float32 => "Float32Array",
            Self::Float64 => "Float64Array",
            Self::BigInt64 => "BigInt64Array",
            Self::BigUint64 => "BigUint64Array",
        }
    }
}

// ── Clone algorithm ──────────────────────────────────────────────────────────

/// Errors during structured cloning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloneError {
    /// Value cannot be cloned (e.g. functions, symbols, DOM nodes).
    NotCloneable(String),
    /// Circular reference detected.
    CircularReference,
    /// Transfer failed.
    TransferFailed(String),
}

impl std::fmt::Display for CloneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotCloneable(msg) => write!(f, "not cloneable: {msg}"),
            Self::CircularReference => write!(f, "circular reference detected"),
            Self::TransferFailed(msg) => write!(f, "transfer failed: {msg}"),
        }
    }
}

/// Perform a deep structured clone of a value.
pub fn structured_clone(value: &StructuredValue) -> Result<StructuredValue, CloneError> {
    clone_with_memo(value, &mut HashMap::new(), 0)
}

/// Maximum recursion depth to prevent stack overflow.
const MAX_DEPTH: usize = 256;

fn clone_with_memo(
    value: &StructuredValue,
    _memo: &mut HashMap<usize, StructuredValue>,
    depth: usize,
) -> Result<StructuredValue, CloneError> {
    if depth > MAX_DEPTH {
        return Err(CloneError::CircularReference);
    }

    match value {
        // Primitive types — trivial clone
        StructuredValue::Undefined => Ok(StructuredValue::Undefined),
        StructuredValue::Null => Ok(StructuredValue::Null),
        StructuredValue::Boolean(b) => Ok(StructuredValue::Boolean(*b)),
        StructuredValue::Number(n) => Ok(StructuredValue::Number(*n)),
        StructuredValue::BigInt(s) => Ok(StructuredValue::BigInt(s.clone())),
        StructuredValue::String(s) => Ok(StructuredValue::String(s.clone())),
        StructuredValue::Date(ms) => Ok(StructuredValue::Date(*ms)),
        StructuredValue::RegExp { pattern, flags } => Ok(StructuredValue::RegExp {
            pattern: pattern.clone(),
            flags: flags.clone(),
        }),

        // Buffer types — deep copy
        StructuredValue::ArrayBuffer(data) => Ok(StructuredValue::ArrayBuffer(data.clone())),
        StructuredValue::TypedArray {
            kind,
            buffer,
            byte_offset,
            length,
        } => Ok(StructuredValue::TypedArray {
            kind: *kind,
            buffer: buffer.clone(),
            byte_offset: *byte_offset,
            length: *length,
        }),
        StructuredValue::DataView {
            buffer,
            byte_offset,
            byte_length,
        } => Ok(StructuredValue::DataView {
            buffer: buffer.clone(),
            byte_offset: *byte_offset,
            byte_length: *byte_length,
        }),

        // Collections — recursive clone
        StructuredValue::Array(items) => {
            let cloned: Result<Vec<_>, _> = items
                .iter()
                .map(|v| clone_with_memo(v, _memo, depth + 1))
                .collect();
            Ok(StructuredValue::Array(cloned?))
        }
        StructuredValue::Object(props) => {
            let cloned: Result<Vec<_>, _> = props
                .iter()
                .map(|(k, v)| {
                    clone_with_memo(v, _memo, depth + 1).map(|cv| (k.clone(), cv))
                })
                .collect();
            Ok(StructuredValue::Object(cloned?))
        }
        StructuredValue::Map(entries) => {
            let cloned: Result<Vec<_>, _> = entries
                .iter()
                .map(|(k, v)| {
                    let ck = clone_with_memo(k, _memo, depth + 1)?;
                    let cv = clone_with_memo(v, _memo, depth + 1)?;
                    Ok((ck, cv))
                })
                .collect();
            Ok(StructuredValue::Map(cloned?))
        }
        StructuredValue::Set(items) => {
            let cloned: Result<Vec<_>, _> = items
                .iter()
                .map(|v| clone_with_memo(v, _memo, depth + 1))
                .collect();
            Ok(StructuredValue::Set(cloned?))
        }

        // Blob/File — deep copy
        StructuredValue::Blob { data, mime_type } => Ok(StructuredValue::Blob {
            data: data.clone(),
            mime_type: mime_type.clone(),
        }),
        StructuredValue::File {
            data,
            name,
            mime_type,
            last_modified,
        } => Ok(StructuredValue::File {
            data: data.clone(),
            name: name.clone(),
            mime_type: mime_type.clone(),
            last_modified: *last_modified,
        }),

        // ImageData — deep copy
        StructuredValue::ImageData {
            width,
            height,
            data,
        } => Ok(StructuredValue::ImageData {
            width: *width,
            height: *height,
            data: data.clone(),
        }),

        // Error — clone
        StructuredValue::Error {
            name,
            message,
            stack,
        } => Ok(StructuredValue::Error {
            name: name.clone(),
            message: message.clone(),
            stack: stack.clone(),
        }),
    }
}

// ── Transfer ─────────────────────────────────────────────────────────────────

/// Transferable types that can be moved (not copied) between contexts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferableType {
    ArrayBuffer,
    MessagePort,
    ReadableStream,
    WritableStream,
    TransformStream,
    ImageBitmap,
    OffscreenCanvas,
}

impl TransferableType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::ArrayBuffer => "ArrayBuffer",
            Self::MessagePort => "MessagePort",
            Self::ReadableStream => "ReadableStream",
            Self::WritableStream => "WritableStream",
            Self::TransformStream => "TransformStream",
            Self::ImageBitmap => "ImageBitmap",
            Self::OffscreenCanvas => "OffscreenCanvas",
        }
    }
}

/// A transferable value (neutered after transfer).
#[derive(Debug)]
pub struct Transferable {
    pub transfer_type: TransferableType,
    pub data: Vec<u8>,
    pub transferred: bool,
}

impl Transferable {
    pub fn new(transfer_type: TransferableType, data: Vec<u8>) -> Self {
        Self {
            transfer_type,
            data,
            transferred: false,
        }
    }

    /// Transfer the data (moves ownership, neutering the source).
    pub fn transfer(&mut self) -> Result<Vec<u8>, CloneError> {
        if self.transferred {
            return Err(CloneError::TransferFailed(
                "already transferred".to_string(),
            ));
        }
        self.transferred = true;
        Ok(std::mem::take(&mut self.data))
    }

    pub fn is_transferred(&self) -> bool {
        self.transferred
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clone_primitives() {
        assert_eq!(
            structured_clone(&StructuredValue::Undefined).unwrap(),
            StructuredValue::Undefined
        );
        assert_eq!(
            structured_clone(&StructuredValue::Null).unwrap(),
            StructuredValue::Null
        );
        assert_eq!(
            structured_clone(&StructuredValue::Boolean(true)).unwrap(),
            StructuredValue::Boolean(true)
        );
        assert_eq!(
            structured_clone(&StructuredValue::Number(42.5)).unwrap(),
            StructuredValue::Number(42.5)
        );
        assert_eq!(
            structured_clone(&StructuredValue::String("hello".into())).unwrap(),
            StructuredValue::String("hello".into())
        );
    }

    #[test]
    fn clone_date() {
        let val = StructuredValue::Date(1_700_000_000_000.0);
        let cloned = structured_clone(&val).unwrap();
        assert_eq!(cloned, val);
    }

    #[test]
    fn clone_regexp() {
        let val = StructuredValue::RegExp {
            pattern: "abc".into(),
            flags: "gi".into(),
        };
        let cloned = structured_clone(&val).unwrap();
        assert_eq!(cloned, val);
    }

    #[test]
    fn clone_array_buffer() {
        let val = StructuredValue::ArrayBuffer(vec![1, 2, 3, 4]);
        let cloned = structured_clone(&val).unwrap();
        assert_eq!(cloned, val);
    }

    #[test]
    fn clone_typed_array() {
        let val = StructuredValue::TypedArray {
            kind: TypedArrayKind::Uint8,
            buffer: vec![0, 1, 2, 3],
            byte_offset: 0,
            length: 4,
        };
        let cloned = structured_clone(&val).unwrap();
        assert_eq!(cloned, val);
    }

    #[test]
    fn clone_array() {
        let val = StructuredValue::Array(vec![
            StructuredValue::Number(1.0),
            StructuredValue::String("two".into()),
            StructuredValue::Null,
        ]);
        let cloned = structured_clone(&val).unwrap();
        assert_eq!(cloned, val);
    }

    #[test]
    fn clone_object() {
        let val = StructuredValue::Object(vec![
            ("name".into(), StructuredValue::String("Vigo".into())),
            ("version".into(), StructuredValue::Number(1.0)),
        ]);
        let cloned = structured_clone(&val).unwrap();
        assert_eq!(cloned, val);
    }

    #[test]
    fn clone_nested() {
        let val = StructuredValue::Object(vec![(
            "data".into(),
            StructuredValue::Array(vec![
                StructuredValue::Map(vec![(
                    StructuredValue::String("key".into()),
                    StructuredValue::Number(42.0),
                )]),
            ]),
        )]);
        let cloned = structured_clone(&val).unwrap();
        assert_eq!(cloned, val);
    }

    #[test]
    fn clone_blob() {
        let val = StructuredValue::Blob {
            data: vec![0xFF, 0xD8],
            mime_type: "image/jpeg".into(),
        };
        let cloned = structured_clone(&val).unwrap();
        assert_eq!(cloned, val);
    }

    #[test]
    fn clone_error() {
        let val = StructuredValue::Error {
            name: "TypeError".into(),
            message: "x is not a function".into(),
            stack: Some("at foo:1:1".into()),
        };
        let cloned = structured_clone(&val).unwrap();
        assert_eq!(cloned, val);
    }

    #[test]
    fn clone_image_data() {
        let val = StructuredValue::ImageData {
            width: 2,
            height: 2,
            data: vec![255; 16], // 2x2 RGBA
        };
        let cloned = structured_clone(&val).unwrap();
        assert_eq!(cloned, val);
    }

    #[test]
    fn deep_recursion_limit() {
        // Build a deeply nested structure
        let mut val = StructuredValue::Number(42.0);
        for _ in 0..300 {
            val = StructuredValue::Array(vec![val]);
        }
        let result = structured_clone(&val);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CloneError::CircularReference);
    }

    #[test]
    fn transferable_basic() {
        let mut t = Transferable::new(TransferableType::ArrayBuffer, vec![1, 2, 3]);
        assert!(!t.is_transferred());
        let data = t.transfer().unwrap();
        assert_eq!(data, vec![1, 2, 3]);
        assert!(t.is_transferred());

        // Second transfer fails
        let result = t.transfer();
        assert!(result.is_err());
    }

    #[test]
    fn typed_array_kind_element_size() {
        assert_eq!(TypedArrayKind::Uint8.element_size(), 1);
        assert_eq!(TypedArrayKind::Int16.element_size(), 2);
        assert_eq!(TypedArrayKind::Float32.element_size(), 4);
        assert_eq!(TypedArrayKind::Float64.element_size(), 8);
    }

    #[test]
    fn clone_map_and_set() {
        let map = StructuredValue::Map(vec![
            (StructuredValue::String("a".into()), StructuredValue::Number(1.0)),
        ]);
        assert_eq!(structured_clone(&map).unwrap(), map);

        let set = StructuredValue::Set(vec![
            StructuredValue::Number(1.0),
            StructuredValue::Number(2.0),
        ]);
        assert_eq!(structured_clone(&set).unwrap(), set);
    }
}

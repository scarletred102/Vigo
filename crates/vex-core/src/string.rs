// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Interned string type for cheap cloning and pointer-equality comparison.
//!
//! `VexString` stores strings in a global intern table, so that two strings
//! with the same content resolve to the same underlying allocation. Cloning
//! is just a reference-count bump (O(1)), and equality/hashing compare by
//! pointer (O(1)).

use std::collections::HashSet;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, OnceLock};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Global intern table. Strings are never removed — they live until exit.
fn intern_table() -> &'static Mutex<HashSet<Arc<str>>> {
    static TABLE: OnceLock<Mutex<HashSet<Arc<str>>>> = OnceLock::new();
    TABLE.get_or_init(|| Mutex::new(HashSet::new()))
}

/// An interned, reference-counted string.
///
/// Cloning is O(1) (ref-count bump). Equality and hashing compare by pointer.
#[derive(Clone)]
pub struct VexString(Arc<str>);

impl VexString {
    /// Intern a string. If the same content was already interned, returns
    /// a handle that shares the same allocation.
    pub fn new(s: &str) -> Self {
        let mut table = match intern_table().lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if let Some(existing) = table.get(s) {
            VexString(existing.clone())
        } else {
            let arc: Arc<str> = Arc::from(s);
            table.insert(arc.clone());
            VexString(arc)
        }
    }

    /// View as a plain string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// ── Trait impls ──────────────────────────────────────────────────────

impl PartialEq for VexString {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for VexString {}

impl Hash for VexString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.0).hash(state);
    }
}

impl fmt::Debug for VexString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VexString({:?})", self.as_str())
    }
}

impl fmt::Display for VexString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for VexString {
    fn from(s: &str) -> Self {
        VexString::new(s)
    }
}

impl From<String> for VexString {
    fn from(s: String) -> Self {
        VexString::new(&s)
    }
}

impl AsRef<str> for VexString {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Serialize for VexString {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.as_str().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for VexString {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(VexString::new(&s))
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn creation_and_access() {
        let s = VexString::new("hello");
        assert_eq!(s.as_str(), "hello");
    }

    #[test]
    fn equality_of_same_content() {
        let a = VexString::new("div");
        let b = VexString::new("div");
        assert_eq!(a, b);
    }

    #[test]
    fn inequality_of_different_content() {
        let a = VexString::new("div");
        let b = VexString::new("span");
        assert_ne!(a, b);
    }

    #[test]
    fn usable_in_hashset() {
        let mut set = HashSet::new();
        set.insert(VexString::new("a"));
        set.insert(VexString::new("b"));
        set.insert(VexString::new("a")); // duplicate
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn display_and_debug() {
        let s = VexString::new("world");
        assert_eq!(format!("{s}"), "world");
        assert_eq!(format!("{s:?}"), r#"VexString("world")"#);
    }

    #[test]
    fn clone_is_cheap_pointer_equality() {
        let a = VexString::new("tag");
        let b = a.clone();
        assert!(Arc::ptr_eq(&a.0, &b.0));
    }

    #[test]
    fn from_string() {
        let owned = String::from("body");
        let s = VexString::from(owned);
        assert_eq!(s.as_str(), "body");
    }

    #[test]
    fn serde_roundtrip() {
        let original = VexString::new("test");
        let json = serde_json::to_string(&original).unwrap();
        assert_eq!(json, "\"test\"");

        let restored: VexString = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.as_str(), "test");
    }
}

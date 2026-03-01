// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Same-Origin Policy enforcement.
//!
//! Provides checks for DOM access, fetch classification, and storage scoping
//! based on the [Same-Origin Policy](https://developer.mozilla.org/en-US/docs/Web/Security/Same-origin_policy).

use vex_core::VexUrl;

use crate::error::{SecurityError, SecurityResult};
use crate::origin::Origin;

/// Whether a fetch request is same-origin or cross-origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchType {
    /// Request target shares the same origin as the page.
    SameOrigin,
    /// Request target is on a different origin — CORS may be required.
    CrossOrigin,
}

/// Classify a fetch request as same-origin or cross-origin.
///
/// Returns [`FetchType::CrossOrigin`] if either origin is opaque or the
/// origins differ.
pub fn classify_fetch(page_origin: &Origin, request_url: &VexUrl) -> FetchType {
    let target = Origin::from_url(request_url);
    if Origin::same_origin(page_origin, &target) {
        FetchType::SameOrigin
    } else {
        FetchType::CrossOrigin
    }
}

/// Guard DOM access between two origins (e.g. cross-origin iframe).
///
/// Returns `Ok(())` if the origins match, or [`SecurityError::SopViolation`]
/// if they differ.
pub fn check_dom_access(accessor: &Origin, target: &Origin) -> SecurityResult<()> {
    if Origin::same_origin(accessor, target) {
        Ok(())
    } else {
        Err(SecurityError::SopViolation(format!(
            "cannot access DOM of {} from {}",
            target, accessor,
        )))
    }
}

/// Guard storage access: only the matching tuple origin may read/write.
///
/// Both origins must be non-opaque and identical.
pub fn check_storage_access(accessor: &Origin, storage_origin: &Origin) -> SecurityResult<()> {
    if accessor.is_opaque() || storage_origin.is_opaque() {
        return Err(SecurityError::SopViolation(
            "opaque origins cannot access storage".into(),
        ));
    }
    if Origin::same_origin(accessor, storage_origin) {
        Ok(())
    } else {
        Err(SecurityError::SopViolation(format!(
            "storage origin mismatch: {} vs {}",
            accessor, storage_origin,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_origin_fetch_classified() {
        let page = Origin::from_url(&VexUrl::parse("https://example.com/page").unwrap());
        let target = VexUrl::parse("https://example.com/api").unwrap();
        assert_eq!(classify_fetch(&page, &target), FetchType::SameOrigin);
    }

    #[test]
    fn cross_origin_fetch_classified() {
        let page = Origin::from_url(&VexUrl::parse("https://example.com").unwrap());
        let target = VexUrl::parse("https://api.other.com/data").unwrap();
        assert_eq!(classify_fetch(&page, &target), FetchType::CrossOrigin);
    }

    #[test]
    fn different_scheme_is_cross_origin() {
        let page = Origin::from_url(&VexUrl::parse("https://example.com").unwrap());
        let target = VexUrl::parse("http://example.com/api").unwrap();
        assert_eq!(classify_fetch(&page, &target), FetchType::CrossOrigin);
    }

    #[test]
    fn dom_access_same_origin_ok() {
        let a = Origin::from_url(&VexUrl::parse("https://example.com").unwrap());
        let b = Origin::from_url(&VexUrl::parse("https://example.com/other").unwrap());
        assert!(check_dom_access(&a, &b).is_ok());
    }

    #[test]
    fn dom_access_cross_origin_blocked() {
        let a = Origin::from_url(&VexUrl::parse("https://example.com").unwrap());
        let b = Origin::from_url(&VexUrl::parse("https://evil.com").unwrap());
        let err = check_dom_access(&a, &b).unwrap_err();
        assert!(matches!(err, SecurityError::SopViolation(_)));
    }

    #[test]
    fn storage_access_same_origin_ok() {
        let o = Origin::from_url(&VexUrl::parse("https://example.com").unwrap());
        assert!(check_storage_access(&o, &o).is_ok());
    }

    #[test]
    fn storage_access_cross_origin_blocked() {
        let a = Origin::from_url(&VexUrl::parse("https://a.com").unwrap());
        let b = Origin::from_url(&VexUrl::parse("https://b.com").unwrap());
        assert!(check_storage_access(&a, &b).is_err());
    }

    #[test]
    fn storage_access_opaque_origin_blocked() {
        let opaque = Origin::from_url(&VexUrl::parse("data:text/html,hi").unwrap());
        let tuple = Origin::from_url(&VexUrl::parse("https://example.com").unwrap());
        assert!(check_storage_access(&opaque, &tuple).is_err());
        assert!(check_storage_access(&tuple, &opaque).is_err());
    }
}

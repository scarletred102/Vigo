// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Tracking parameter stripping from URLs.
//!
//! Removes known analytics/ad tracking query parameters while preserving
//! legitimate parameters.

use vex_core::VexUrl;

/// Known tracking query parameters to strip.
const TRACKING_PARAMS: &[&str] = &[
    // Google Analytics / Ads
    "utm_source",
    "utm_medium",
    "utm_campaign",
    "utm_term",
    "utm_content",
    "utm_id",
    "gclid",
    "gclsrc",
    "dclid",
    "_ga",
    "_gl",
    "_gac",
    // Facebook / Meta
    "fbclid",
    "fb_action_ids",
    "fb_action_types",
    "fb_source",
    "fb_ref",
    // Microsoft
    "msclkid",
    // Twitter / X
    "twclid",
    // Mailchimp
    "mc_cid",
    "mc_eid",
    // HubSpot
    "hsa_cam",
    "hsa_grp",
    "hsa_mt",
    "hsa_src",
    "hsa_ad",
    "hsa_acc",
    "hsa_net",
    "hsa_ver",
    "hsa_la",
    "hsa_ol",
    "hsa_kw",
    // Adobe
    "s_cid",
    "ef_id",
    // Misc
    "yclid",
    "igshid",
    "ref_src",
    "ref_url",
];

/// Strip tracking parameters from a URL.
///
/// Returns a new URL with tracking params removed, or the original if none found.
pub fn strip_tracking_params(url: &VexUrl) -> VexUrl {
    let inner = url.inner();

    // Fast path: no query string at all
    if inner.query().is_none() {
        return url.clone();
    }

    let pairs: Vec<(String, String)> = inner
        .query_pairs()
        .filter(|(key, _)| {
            let k = key.as_ref();
            !TRACKING_PARAMS.contains(&k)
        })
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    let mut new_url = inner.clone();

    if pairs.is_empty() {
        new_url.set_query(None);
    } else {
        {
            let mut query_pairs = new_url.query_pairs_mut();
            query_pairs.clear();
            for (k, v) in &pairs {
                query_pairs.append_pair(k, v);
            }
        } // drop query_pairs to release the mutable borrow on new_url
    }

    // Re-parse to create a VexUrl. If reparsing somehow fails, keep the
    // original input URL rather than panicking in library code.
    match VexUrl::parse(new_url.as_str()) {
        Ok(parsed) => parsed,
        Err(_) => url.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> VexUrl {
        VexUrl::parse(s).unwrap()
    }

    #[test]
    fn strip_utm_params() {
        let u = url("https://example.com/page?q=hello&utm_source=twitter&utm_medium=social");
        let clean = strip_tracking_params(&u);
        assert_eq!(clean.inner().as_str(), "https://example.com/page?q=hello");
    }

    #[test]
    fn strip_fbclid() {
        let u = url("https://example.com/?fbclid=abc123&page=1");
        let clean = strip_tracking_params(&u);
        assert_eq!(clean.inner().as_str(), "https://example.com/?page=1");
    }

    #[test]
    fn strip_gclid() {
        let u = url("https://example.com/?gclid=xyz");
        let clean = strip_tracking_params(&u);
        assert_eq!(clean.inner().as_str(), "https://example.com/");
    }

    #[test]
    fn preserve_non_tracking() {
        let u = url("https://example.com/search?q=rust&page=2&sort=date");
        let clean = strip_tracking_params(&u);
        assert_eq!(
            clean.inner().as_str(),
            "https://example.com/search?q=rust&page=2&sort=date"
        );
    }

    #[test]
    fn no_query_string() {
        let u = url("https://example.com/page");
        let clean = strip_tracking_params(&u);
        assert_eq!(clean.inner().as_str(), "https://example.com/page");
    }

    #[test]
    fn strip_all_params_leaves_no_query() {
        let u = url("https://example.com/?utm_source=a&utm_medium=b&utm_campaign=c");
        let clean = strip_tracking_params(&u);
        // Should have no query string
        assert!(clean.inner().query().is_none());
    }

    #[test]
    fn strip_microsoft_and_twitter() {
        let u = url("https://example.com/?msclkid=m1&twclid=t1&real=yes");
        let clean = strip_tracking_params(&u);
        assert_eq!(clean.inner().as_str(), "https://example.com/?real=yes");
    }

    #[test]
    fn strip_ga_params() {
        let u = url("https://example.com/?_ga=1.2.3&_gl=4.5.6&q=test");
        let clean = strip_tracking_params(&u);
        assert_eq!(clean.inner().as_str(), "https://example.com/?q=test");
    }
}

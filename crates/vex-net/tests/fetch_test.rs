// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Integration test: fetch real HTTPS pages.
//!
//! Requires network access — run with `cargo test -p vex-net --test fetch_test`.

use vex_net::{HttpClient, Request};

#[tokio::test]
#[ignore = "requires network access"]
async fn fetch_https_html() {
    let client = HttpClient::new().expect("client should init");
    let request = Request::get("https://httpbin.org/html").expect("URL should parse");

    let response = client.fetch(request).await.expect("fetch should succeed");

    assert_eq!(response.status, 200);
    let body = response.text().expect("body should be UTF-8");
    assert!(body.contains("<html"), "expected HTML in body, got: {}", &body[..80.min(body.len())]);
}

#[tokio::test]
#[ignore = "requires network access"]
async fn fetch_follows_redirect() {
    let client = HttpClient::new().expect("client should init");
    // httpbin /redirect/1 → single 302 redirect → /get
    let request = Request::get("https://httpbin.org/redirect/1").expect("URL should parse");

    let response = client.fetch(request).await.expect("fetch should succeed");

    assert!(response.is_success(), "status was {}", response.status);
}

#[tokio::test]
#[ignore = "requires network access"]
async fn fetch_404_returns_status() {
    let client = HttpClient::new().expect("client should init");
    let request =
        Request::get("https://httpbin.org/status/404").expect("URL should parse");

    let response = client.fetch(request).await.expect("fetch should succeed");

    assert_eq!(response.status, 404);
    assert!(!response.is_success());
}

#[tokio::test]
#[ignore = "requires network access"]
async fn fetch_gzip_decompression() {
    let client = HttpClient::new().expect("client should init");
    let request = Request::get("https://httpbin.org/gzip").expect("URL should parse");

    let response = client.fetch(request).await.expect("fetch should succeed");

    assert_eq!(response.status, 200);
    let body = response.text().expect("body should be UTF-8");
    // httpbin /gzip returns a JSON with "gzipped": true
    assert!(body.contains("gzipped"), "expected gzip response: {}", &body[..100.min(body.len())]);
}

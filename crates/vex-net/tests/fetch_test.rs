// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Integration test: fetch real HTTPS pages.
//!
//! Requires network access — run with `cargo test -p vex-net --test fetch_test`.

use vex_net::{HttpClient, Request};
use vex_privacy::PrivacyLayer;

#[tokio::test]
#[ignore = "requires network access"]
async fn fetch_https_html() {
    let client = HttpClient::new().expect("client should init");
    let request = Request::get("https://httpbin.org/html").expect("URL should parse");

    let response = client.fetch(request).await.expect("fetch should succeed");

    assert_eq!(response.status, 200);
    let body = response.text().expect("body should be UTF-8");
    assert!(
        body.contains("<html"),
        "expected HTML in body, got: {}",
        &body[..80.min(body.len())]
    );
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
    let request = Request::get("https://httpbin.org/status/404").expect("URL should parse");

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
    assert!(
        body.contains("gzipped"),
        "expected gzip response: {}",
        &body[..100.min(body.len())]
    );
}

#[tokio::test]
#[ignore = "requires network access"]
async fn fetch_ten_https_sites() {
    let client = HttpClient::new().expect("client should init");

    let urls = [
        "https://example.com/",
        "https://httpbin.org/html",
        "https://www.rust-lang.org/",
        "https://www.mozilla.org/",
        "https://www.wikipedia.org/",
        "https://www.github.com/",
        "https://docs.rs/",
        "https://www.w3.org/",
        "https://www.cloudflare.com/",
        "https://www.gnu.org/",
    ];

    let mut failures = Vec::new();

    for url in urls {
        eprintln!("[fetch_ten_https_sites] fetching {url}");

        let request = Request::get(url).expect("URL should parse");
        match client.fetch(request).await {
            Ok(response) => {
                if !response.is_success() {
                    failures.push(format!("{url}: non-success status {}", response.status));
                    continue;
                }

                match response.text() {
                    Ok(body) => {
                        if !looks_like_html(body) {
                            failures.push(format!("{url}: response did not look like HTML"));
                        }
                    }
                    Err(e) => failures.push(format!("{url}: utf8 decode failed: {e}")),
                }
            }
            Err(e) => failures.push(format!("{url}: fetch error: {e}")),
        }
    }

    assert!(
        failures.is_empty(),
        "one or more fetches failed:\n{}",
        failures.join("\n")
    );
}

fn looks_like_html(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    lower.contains("<html")
        || lower.contains("<!doctype html")
        || lower.contains("<head")
        || lower.contains("<body")
}

#[tokio::test]
#[ignore = "requires network access"]
async fn fetch_filtered_strips_tracking_params() {
    let client = HttpClient::new().expect("client should init");
    let privacy = PrivacyLayer::new();

    let request =
        Request::get("https://httpbin.org/get?utm_source=test&q=hello").expect("URL should parse");

    let response = client
        .fetch_filtered(request, |req| privacy.process_request(req))
        .await
        .expect("fetch should succeed");

    assert_eq!(response.status, 200);
    let body = response.text().expect("body should be UTF-8");

    assert!(
        body.contains("\"q\": \"hello\""),
        "expected q param in echo body"
    );
    assert!(
        !body.contains("utm_source"),
        "expected utm_source to be stripped from request URL"
    );
}

#[tokio::test]
#[ignore = "benchmark: requires network and is timing-sensitive"]
async fn benchmark_100_sequential_fetches() {
    use std::time::{Duration, Instant};

    let client = HttpClient::new().expect("client should init");
    let request = Request::get("https://example.com/").expect("URL should parse");

    let start = Instant::now();
    for _ in 0..100 {
        let response = client
            .fetch(request.clone())
            .await
            .expect("fetch should succeed");
        assert!(response.is_success());
    }
    let elapsed = start.elapsed();

    println!("100 sequential fetches elapsed: {elapsed:?}");

    // Soft target from Phase 2 planning notes.
    let soft_target = Duration::from_millis(500);
    if elapsed > soft_target {
        println!("benchmark target missed (soft): elapsed={elapsed:?}, target={soft_target:?}");
    }
}

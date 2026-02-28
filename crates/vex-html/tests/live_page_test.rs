// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Integration test: fetch a real page, parse to DOM, query, serialize.
//!
//! Requires network access:
//! ```
//! cargo test -p vex-html --test live_page_test
//! ```

use vex_dom::serialize::serialize;
use vex_html::parse_html;
use vex_net::{HttpClient, Request};

#[tokio::test]
#[ignore] // Requires network. Run with: cargo test -p vex-html --test live_page_test -- --ignored
async fn fetch_parse_query_serialize_example_com() {
    // 1. Fetch the page
    let client = HttpClient::new().expect("client should init");
    let request = Request::get("https://example.com").expect("URL should parse");
    let response = client.fetch(request).await.expect("fetch should succeed");

    assert!(response.is_success(), "status was {}", response.status);
    let html = response.text().expect("body should be UTF-8");
    assert!(!html.is_empty(), "body should not be empty");

    // 2. Parse HTML into DOM
    let document = parse_html(html);

    // 3. querySelector("h1") → "Example Domain"
    let h1_id = document
        .query_selector("h1")
        .expect("selector should parse")
        .expect("should find an <h1>");
    let h1_text = document.text_content(h1_id);
    assert_eq!(h1_text.trim(), "Example Domain");

    // 4. querySelectorAll("p") → at least 1 paragraph
    let paragraphs = document
        .query_selector_all("p")
        .expect("selector should parse");
    assert!(!paragraphs.is_empty(), "should find at least one <p>");

    // 5. Serialize back to HTML → contains <h1>
    let root = document.root_element().expect("should have root element");
    let serialized = serialize(document.arena(), root);
    assert!(
        serialized.contains("<h1>"),
        "serialized HTML should contain <h1>, got: {}",
        &serialized[..200.min(serialized.len())]
    );
}

#[tokio::test]
#[ignore] // Requires network. Run with: cargo test -p vex-html --test live_page_test -- --ignored
async fn fetch_parse_complex_page() {
    // Fetch a page with more structure (httpbin.org/html has Moby Dick excerpt)
    let client = HttpClient::new().expect("client should init");
    let request = Request::get("https://httpbin.org/html").expect("URL should parse");
    let response = client.fetch(request).await.expect("fetch should succeed");

    assert_eq!(response.status, 200);
    let html = response.text().expect("body should be UTF-8");

    let document = parse_html(html);

    // Should have a root element
    let root = document.root_element().expect("should have root");
    let arena = document.arena();

    // Should be parseable and serializable without panic
    let serialized = serialize(arena, root);
    assert!(serialized.len() > 100, "serialized should be substantial");

    // The page has <h1> with "Herman Melville - Moby Dick"
    if let Ok(Some(h1)) = document.query_selector("h1") {
        let text = document.text_content(h1);
        assert!(
            text.contains("Herman Melville") || text.contains("Moby"),
            "h1 text: {text}"
        );
    }
}

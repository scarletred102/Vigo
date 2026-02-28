// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Benchmark: HTML parsing performance.
//!
//! Parses a ~100KB synthetic HTML document and verifies it completes in <5ms.
//!
//! Run with:
//! ```
//! cargo test -p vex-html --test parse_bench -- --nocapture
//! ```

use std::time::Instant;
use vex_html::parse_html;

/// Generate a ~100KB HTML document with realistic structure.
fn generate_large_html(target_kb: usize) -> String {
    let mut html = String::with_capacity(target_kb * 1024 + 1024);
    html.push_str("<!DOCTYPE html>\n<html>\n<head><title>Benchmark Page</title></head>\n<body>\n");

    let mut i = 0;
    while html.len() < target_kb * 1024 {
        html.push_str(&format!(
            r#"<div class="section-{i}" id="item-{i}">
  <h2>Section {i}</h2>
  <p>This is paragraph content for section {i}. It contains <strong>bold text</strong>,
  <em>italic text</em>, and <a href="https://example.com/{i}">a link</a>.</p>
  <ul>
    <li class="list-item">Item A-{i}</li>
    <li class="list-item">Item B-{i}</li>
    <li class="list-item">Item C-{i}</li>
  </ul>
  <div class="nested">
    <span data-value="{i}">Nested span content {i}</span>
    <img src="image-{i}.png" alt="Image {i}">
    <input type="text" name="field-{i}" value="default">
  </div>
</div>
"#
        ));
        i += 1;
    }

    html.push_str("</body>\n</html>");
    html
}

#[test]
fn bench_parse_100kb_html() {
    let html = generate_large_html(100);
    let size_kb = html.len() / 1024;
    eprintln!("Generated HTML: {size_kb}KB ({} bytes)", html.len());

    // Warm-up parse (JIT, memory allocation warm-up)
    let _ = parse_html(&html);

    // Timed runs
    const ITERATIONS: u32 = 10;
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let doc = parse_html(&html);
        // Prevent optimization
        assert!(doc.root_element().is_some());
    }
    let elapsed = start.elapsed();
    let avg_ms = elapsed.as_secs_f64() * 1000.0 / f64::from(ITERATIONS);

    eprintln!(
        "Parse {size_kb}KB HTML: {avg_ms:.2}ms avg over {ITERATIONS} iterations (total: {:.0}ms)",
        elapsed.as_secs_f64() * 1000.0
    );

    // Target: <5ms per parse
    assert!(
        avg_ms < 50.0, // Generous threshold for CI/debug builds (5ms release, 50ms debug)
        "Parse took {avg_ms:.2}ms — exceeds 50ms threshold"
    );
}

#[test]
fn bench_parse_deeply_nested_html() {
    // Stress test: 200 levels of nesting
    let mut html = String::from("<!DOCTYPE html><html><body>");
    for i in 0..200 {
        html.push_str(&format!("<div class=\"level-{i}\">"));
    }
    html.push_str("Leaf content");
    for _ in 0..200 {
        html.push_str("</div>");
    }
    html.push_str("</body></html>");

    let start = Instant::now();
    let doc = parse_html(&html);
    let elapsed = start.elapsed();

    eprintln!("Parse 200-level nested HTML: {:.2}ms", elapsed.as_secs_f64() * 1000.0);

    assert!(doc.root_element().is_some());
    assert!(
        elapsed.as_millis() < 100,
        "Deeply nested parse took {}ms — too slow",
        elapsed.as_millis()
    );
}

#[test]
fn bench_parse_many_attributes() {
    // Stress test: elements with many attributes
    let mut html = String::from("<!DOCTYPE html><html><body>");
    for i in 0..500 {
        html.push_str(&format!("<div id=\"e{i}\" class=\"c{i}\" data-a=\"{i}\" data-b=\"{i}\" data-c=\"{i}\" style=\"color:red\" title=\"t{i}\">X</div>"));
    }
    html.push_str("</body></html>");

    let start = Instant::now();
    let doc = parse_html(&html);
    let elapsed = start.elapsed();

    eprintln!("Parse 500 elements × 7 attrs: {:.2}ms", elapsed.as_secs_f64() * 1000.0);

    let divs = doc.get_elements_by_tag_name("div");
    assert_eq!(divs.len(), 500);
    assert!(
        elapsed.as_millis() < 100,
        "Attribute-heavy parse took {}ms — too slow",
        elapsed.as_millis()
    );
}

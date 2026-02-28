// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Benchmark: Layout engine performance.
//!
//! Quick benchmarks run normally; the large 5000-element benchmark is `#[ignore]`
//! and should be run in release mode:
//! ```
//! cargo test -p vex-layout --test layout_bench --release -- --ignored --nocapture
//! ```

use std::time::Instant;
use vex_core::geometry::Size;
use vex_css::{compute_styles, parse_stylesheet};
use vex_html::parse_html;
use vex_layout::layout_document;

const VIEWPORT: Size = Size {
    width: 1280.0,
    height: 720.0,
};

/// Generate an HTML document with `n` elements in a realistic page structure.
fn generate_page(element_count: usize) -> String {
    let mut html = String::with_capacity(element_count * 120);
    html.push_str("<!DOCTYPE html><html><head><title>Bench</title></head><body>\n");

    // Groups of 5: header div, 3 content divs, footer div
    let groups = element_count / 5;
    for g in 0..groups {
        html.push_str(&format!(
            r##"<section class="group">
  <h3>Section {g}</h3>
  <div class="card"><p>Card content {g}-a with <strong>bold</strong> text</p></div>
  <div class="card"><p>Card content {g}-b with <em>italic</em> text</p></div>
  <div class="card"><p>Card content {g}-c with <a href="#">a link</a></p></div>
</section>
"##
        ));
    }

    html.push_str("</body></html>");
    html
}

/// CSS that exercises multiple layout paths.
fn bench_css() -> &'static str {
    r#"
    body { margin: 8px; }
    section { margin-bottom: 16px; padding: 8px; }
    h3 { margin: 0 0 8px 0; }
    .card { padding: 12px; margin-bottom: 8px; }
    p { margin: 0; }
    strong, em { display: inline; }
    a { display: inline; }
    "#
}

#[test]
#[ignore] // Slow in debug. Run in release: cargo test -p vex-layout --test layout_bench --release -- --ignored --nocapture
fn bench_layout_5000_elements() {
    let html = generate_page(5000);
    let doc = parse_html(&html);
    let sheet = parse_stylesheet(bench_css());
    let styles = compute_styles(&doc, &[sheet], VIEWPORT);

    // Warm-up
    let _ = layout_document(&doc, &styles, VIEWPORT);

    // Timed runs
    const ITERATIONS: u32 = 5;
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let tree = layout_document(&doc, &styles, VIEWPORT);
        assert!(tree.dimensions.content.size.width > 0.0);
    }
    let elapsed = start.elapsed();
    let avg_ms = elapsed.as_secs_f64() * 1000.0 / f64::from(ITERATIONS);

    eprintln!(
        "Layout 5000-element page: {avg_ms:.2}ms avg over {ITERATIONS} iterations (total: {:.0}ms)",
        elapsed.as_secs_f64() * 1000.0
    );

    // Release target: <16ms (60fps frame budget)
    assert!(
        avg_ms < 5000.0,
        "Layout took {avg_ms:.2}ms — exceeds threshold"
    );
}

#[test]
fn bench_layout_200_elements_with_styles() {
    let html = generate_page(200);
    let doc = parse_html(&html);
    let sheet = parse_stylesheet(bench_css());

    // Benchmark: style computation + layout combined
    let start = Instant::now();
    let styles = compute_styles(&doc, &[sheet], VIEWPORT);
    let tree = layout_document(&doc, &styles, VIEWPORT);
    let elapsed = start.elapsed();

    eprintln!(
        "Style+Layout 200-element page: {:.2}ms",
        elapsed.as_secs_f64() * 1000.0
    );

    assert!(tree.dimensions.content.size.width > 0.0);
    assert!(
        elapsed.as_secs() < 60,
        "Style+Layout took {}s — too slow even for debug",
        elapsed.as_secs()
    );
}

#[test]
fn bench_deep_nesting_layout() {
    // 200 levels of nested divs — stress test for recursion/stack
    let mut html = String::from("<!DOCTYPE html><html><body>");
    for i in 0..200 {
        html.push_str(&format!("<div class=\"d{i}\">"));
    }
    html.push_str("Deep leaf");
    for _ in 0..200 {
        html.push_str("</div>");
    }
    html.push_str("</body></html>");

    let doc = parse_html(&html);
    let sheet = parse_stylesheet("div { padding: 2px; margin: 1px; }");
    let styles = compute_styles(&doc, &[sheet], VIEWPORT);

    let start = Instant::now();
    let tree = layout_document(&doc, &styles, VIEWPORT);
    let elapsed = start.elapsed();

    eprintln!(
        "Layout 200-nested divs: {:.2}ms",
        elapsed.as_secs_f64() * 1000.0
    );

    // Should still produce valid layout
    assert!(tree.dimensions.content.size.width > 0.0);
    assert!(
        elapsed.as_millis() < 200,
        "Deep nesting layout took {}ms — too slow",
        elapsed.as_millis()
    );
}

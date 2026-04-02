// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Compatibility smoke corpus inspired by common web-compat edge cases.

use vex_dom::serialize::serialize;
use vex_html::{parse_html, parse_html_with_diagnostics};

#[test]
fn compat_corpus_parses_without_panic() {
    let corpus = [
        "<!doctype html><html><head><title>x</title></head><body><p>a",
        "<table><tr><td>cell<p>paragraph</table>",
        "<form><input name=a value=1><input name=b value=2></form>",
        "<svg><foreignObject><div>html-in-svg</div></foreignObject></svg>",
        "<math><mrow><mi>x</mi></mrow></math>",
        "<script>if (a < b) { c = '</script-not-end>'; }</script>",
        "<template><div>inside-template</div></template>",
        "<meta charset='utf-8'><p>utf8 ✅</p>",
        "<a href=/x id=i class='a b c'>link</a>",
        "<ul><li>one<li>two<li>three</ul>",
        "<div><span></div></span>",
        "<body><noscript><p>x</p></noscript></body>",
        "<input type=hidden><input required value=''>",
        "<!--comment--><!doctype html><p>text",
        "<link rel='stylesheet preload' href='/a.css'>",
    ];

    for html in corpus {
        let doc = parse_html(html);
        assert!(doc.root_element().is_some());
    }
}

#[test]
fn compat_corpus_query_and_serialize_roundtrip() {
    let html = r#"
    <!doctype html>
    <html>
      <head><title>Compat</title></head>
      <body>
        <main id="main" class="content page">
          <section><h1>Heading</h1><p>Alpha <b>Beta</b></p></section>
          <img src="/a.png">
        </main>
      </body>
    </html>
    "#;

    let doc = parse_html(html);

    let main = doc
        .query_selector("#main.content")
        .expect("selector parse")
        .expect("match main");
    assert!(doc.text_content(main).contains("Heading"));

    let html_root = doc.root_element().expect("root html");
    let out = serialize(doc.arena(), html_root);
    assert!(out.contains("<h1>Heading</h1>"));
    assert!(out.contains("<img src=\"/a.png\">"));
}

#[test]
fn diagnostics_surface_malformed_inputs() {
    let (_doc, d) = parse_html_with_diagnostics("<div><span\0");
    assert!(!d.is_empty());
}

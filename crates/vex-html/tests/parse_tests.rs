// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Integration tests for the HTML parser → DOM pipeline.

use vex_dom::node::NodeData;
use vex_dom::serialize;
use vex_html::{parse_html, parse_html_fragment};

// ── Basic document structure ─────────────────────────────────────────

#[test]
fn empty_document() {
    let doc = parse_html("");
    // html5ever always creates <html><head></head><body></body></html>
    let html = doc.root_element().expect("root element");
    let arena = doc.arena();
    if let NodeData::Element(ref el) = arena.get(html).data {
        assert_eq!(el.tag_name, "html");
    } else {
        panic!("root_element is not an Element");
    }
}

#[test]
fn basic_html_structure() {
    let doc = parse_html("<html><head><title>Hi</title></head><body><p>Hello</p></body></html>");
    let titles = doc.get_elements_by_tag_name("title");
    assert_eq!(titles.len(), 1);
    assert_eq!(doc.text_content(titles[0]), "Hi");

    let ps = doc.get_elements_by_tag_name("p");
    assert_eq!(ps.len(), 1);
    assert_eq!(doc.text_content(ps[0]), "Hello");
}

#[test]
fn nested_divs() {
    let doc = parse_html("<div><div><div>deep</div></div></div>");
    let divs = doc.get_elements_by_tag_name("div");
    assert_eq!(divs.len(), 3);
    assert_eq!(doc.text_content(divs[2]), "deep");
}

// ── Attributes ───────────────────────────────────────────────────────

#[test]
fn attributes_preserved() {
    let doc = parse_html(r#"<a href="/foo" class="link" id="main-link">click</a>"#);
    let links = doc.get_elements_by_tag_name("a");
    assert_eq!(links.len(), 1);
    let arena = doc.arena();
    if let NodeData::Element(ref el) = arena.get(links[0]).data {
        assert_eq!(
            el.attributes.iter().find(|a| a.name == "href").unwrap().value,
            "/foo"
        );
        assert_eq!(
            el.attributes.iter().find(|a| a.name == "class").unwrap().value,
            "link"
        );
        assert_eq!(
            el.attributes.iter().find(|a| a.name == "id").unwrap().value,
            "main-link"
        );
    }
}

#[test]
fn get_element_by_id() {
    let doc = parse_html(r#"<div id="a"></div><div id="b"></div>"#);
    assert!(doc.get_element_by_id("a").is_some());
    assert!(doc.get_element_by_id("b").is_some());
    assert!(doc.get_element_by_id("c").is_none());
}

// ── Text, comments, entities ─────────────────────────────────────────

#[test]
fn text_nodes() {
    let doc = parse_html("<p>one<span>two</span>three</p>");
    let ps = doc.get_elements_by_tag_name("p");
    assert_eq!(doc.text_content(ps[0]), "onetwothree");
}

#[test]
fn comment_nodes() {
    let doc = parse_html("<!-- hello --><p>hi</p>");
    // Comments end up in the document
    let arena = doc.arena();
    let root = doc.root();
    let mut found_comment = false;
    let mut child = arena.get(root).first_child;
    while let Some(id) = child {
        if let NodeData::Comment(ref text) = arena.get(id).data {
            assert_eq!(text.trim(), "hello");
            found_comment = true;
        }
        child = arena.get(id).next_sibling;
    }
    assert!(found_comment, "comment node not found");
}

#[test]
fn html_entities() {
    let doc = parse_html("<p>&amp; &lt; &gt; &quot;</p>");
    let ps = doc.get_elements_by_tag_name("p");
    assert_eq!(doc.text_content(ps[0]), "& < > \"");
}

// ── Self-closing and void elements ───────────────────────────────────

#[test]
fn void_elements() {
    let doc = parse_html("<p>a<br>b</p><img src=\"x.png\"><hr>");
    let brs = doc.get_elements_by_tag_name("br");
    assert_eq!(brs.len(), 1);
    let imgs = doc.get_elements_by_tag_name("img");
    assert_eq!(imgs.len(), 1);
    let hrs = doc.get_elements_by_tag_name("hr");
    assert_eq!(hrs.len(), 1);
}

// ── Malformed HTML (auto-correction) ─────────────────────────────────

#[test]
fn missing_close_tags() {
    // html5ever should auto-close
    let doc = parse_html("<p>one<p>two<p>three");
    let ps = doc.get_elements_by_tag_name("p");
    assert_eq!(ps.len(), 3);
    assert_eq!(doc.text_content(ps[0]), "one");
    assert_eq!(doc.text_content(ps[1]), "two");
    assert_eq!(doc.text_content(ps[2]), "three");
}

#[test]
fn misnested_tags() {
    let doc = parse_html("<b><i>bold-italic</b>italic</i>");
    // html5ever fixes nesting — both tags should exist
    let bs = doc.get_elements_by_tag_name("b");
    let is = doc.get_elements_by_tag_name("i");
    assert!(!bs.is_empty());
    assert!(!is.is_empty());
}

// ── Script and style content ─────────────────────────────────────────

#[test]
fn script_content_not_parsed() {
    let doc = parse_html("<script>var x = '<div>fake</div>';</script>");
    let scripts = doc.get_elements_by_tag_name("script");
    assert_eq!(scripts.len(), 1);
    assert_eq!(doc.text_content(scripts[0]), "var x = '<div>fake</div>';");
    // The <div> inside <script> must NOT become a DOM element
    assert!(doc.get_elements_by_tag_name("div").is_empty());
}

// ── Multiple classes and complex attributes ──────────────────────────

#[test]
fn multiple_classes() {
    let doc = parse_html(r#"<div class="a b c">text</div>"#);
    let result = doc.get_elements_by_class_name("b");
    assert_eq!(result.len(), 1);
    assert!(doc.get_elements_by_class_name("d").is_empty());
}

// ── Doctype ──────────────────────────────────────────────────────────

#[test]
fn doctype_preserved() {
    let doc = parse_html("<!DOCTYPE html><html><body></body></html>");
    let arena = doc.arena();
    let root = doc.root();
    let first = arena.get(root).first_child.expect("first child");
    assert!(
        matches!(arena.get(first).data, NodeData::Doctype { ref name, .. } if name == "html")
    );
}

// ── Serialization round-trip ─────────────────────────────────────────

#[test]
fn serialize_round_trip() {
    let doc = parse_html("<div><p>Hello <b>world</b></p></div>");
    let html_el = doc.root_element().unwrap();
    let output = serialize::serialize(doc.arena(), html_el);
    assert!(output.contains("<p>Hello <b>world</b></p>"));
    assert!(output.contains("<div>"));
}

// ── Fragment parsing ─────────────────────────────────────────────────

#[test]
fn fragment_parsing() {
    let doc = parse_html_fragment("<li>one</li><li>two</li>", "ul");
    let lis = doc.get_elements_by_tag_name("li");
    assert_eq!(lis.len(), 2);
    assert_eq!(doc.text_content(lis[0]), "one");
    assert_eq!(doc.text_content(lis[1]), "two");
}

// ── Deeply nested structure ──────────────────────────────────────────

#[test]
fn deeply_nested() {
    // 50 levels of nesting
    let open: String = (0..50).map(|_| "<div>").collect();
    let text = "leaf";
    let close: String = (0..50).map(|_| "</div>").collect();
    let html = format!("{}{}{}", open, text, close);

    let doc = parse_html(&html);
    let divs = doc.get_elements_by_tag_name("div");
    assert_eq!(divs.len(), 50);
    // The innermost div has the text
    assert_eq!(doc.text_content(divs[49]), "leaf");
}

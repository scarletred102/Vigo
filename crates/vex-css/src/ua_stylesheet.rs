// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! User-agent stylesheet — minimal browser defaults for common HTML elements.

use crate::parser::{parse_stylesheet, Stylesheet};

/// Returns the built-in user-agent stylesheet.
pub fn ua_stylesheet() -> Stylesheet {
    parse_stylesheet(UA_CSS)
}

const UA_CSS: &str = r#"
/* ── Root & document ─────────────────────────────────────────────── */

html {
    display: block;
}

body {
    display: block;
    margin: 8px;
}

/* ── Sectioning & structural ─────────────────────────────────────── */

div, section, article, aside, nav, main, header, footer, figure, figcaption,
details, summary, dialog, address, hgroup, search {
    display: block;
}

/* ── Headings ────────────────────────────────────────────────────── */

p {
    display: block;
    margin-top: 1em;
    margin-bottom: 1em;
}

h1 {
    display: block;
    font-size: 2em;
    font-weight: bold;
    margin-top: 0.67em;
    margin-bottom: 0.67em;
}

h2 {
    display: block;
    font-size: 1.5em;
    font-weight: bold;
    margin-top: 0.83em;
    margin-bottom: 0.83em;
}

h3 {
    display: block;
    font-size: 1.17em;
    font-weight: bold;
    margin-top: 1em;
    margin-bottom: 1em;
}

h4 {
    display: block;
    font-weight: bold;
    margin-top: 1.33em;
    margin-bottom: 1.33em;
}

h5 {
    display: block;
    font-size: 0.83em;
    font-weight: bold;
    margin-top: 1.67em;
    margin-bottom: 1.67em;
}

h6 {
    display: block;
    font-size: 0.67em;
    font-weight: bold;
    margin-top: 2.33em;
    margin-bottom: 2.33em;
}

/* ── Links ───────────────────────────────────────────────────────── */

a {
    color: blue;
    text-decoration: underline;
    cursor: pointer;
}

/* ── Lists ───────────────────────────────────────────────────────── */

ul, ol, menu, dir {
    display: block;
    padding-left: 40px;
    margin-top: 1em;
    margin-bottom: 1em;
}

li {
    display: list-item;
}

dl {
    display: block;
    margin-top: 1em;
    margin-bottom: 1em;
}

dt {
    display: block;
    font-weight: bold;
}

dd {
    display: block;
    margin-left: 40px;
}

/* ── Block quotes & citation ─────────────────────────────────────── */

blockquote {
    display: block;
    margin-top: 1em;
    margin-bottom: 1em;
    margin-left: 40px;
    margin-right: 40px;
}

/* ── Preformatted & code ─────────────────────────────────────────── */

pre {
    display: block;
    font-family: monospace;
    white-space: pre;
    margin-top: 1em;
    margin-bottom: 1em;
}

code, kbd, samp, tt {
    font-family: monospace;
}

var {
    font-style: italic;
}

/* ── Phrasing (inline) ───────────────────────────────────────────── */

b, strong {
    font-weight: bold;
}

i, em, cite, dfn {
    font-style: italic;
}

u, ins {
    text-decoration: underline;
}

s, strike, del {
    text-decoration: line-through;
}

small {
    font-size: 0.83em;
}

big {
    font-size: 1.17em;
}

sub {
    vertical-align: sub;
    font-size: 0.83em;
}

sup {
    vertical-align: super;
    font-size: 0.83em;
}

abbr, acronym {
    text-decoration: underline;
}

mark {
    background-color: yellow;
    color: black;
}

/* ── Horizontal rule ─────────────────────────────────────────────── */

hr {
    display: block;
    margin-top: 0.5em;
    margin-bottom: 0.5em;
    border-top-style: solid;
    border-top-width: 1px;
    border-top-color: gray;
}

/* ── Table ───────────────────────────────────────────────────────── */

table {
    display: table;
}

caption {
    display: table-caption;
    text-align: center;
}

thead {
    display: table-header-group;
}

tbody {
    display: table-row-group;
}

tfoot {
    display: table-footer-group;
}

colgroup {
    display: table-column-group;
}

col {
    display: table-column;
}

tr {
    display: table-row;
}

td, th {
    display: table-cell;
    padding: 1px;
}

th {
    font-weight: bold;
    text-align: center;
}

/* ── Forms ───────────────────────────────────────────────────────── */

form {
    display: block;
    margin-top: 0;
    margin-bottom: 0;
}

fieldset {
    display: block;
    margin-left: 2px;
    margin-right: 2px;
    padding-top: 0.35em;
    padding-bottom: 0.625em;
    padding-left: 0.75em;
    padding-right: 0.75em;
    border-top-width: 2px;
    border-right-width: 2px;
    border-bottom-width: 2px;
    border-left-width: 2px;
    border-top-style: groove;
    border-right-style: groove;
    border-bottom-style: groove;
    border-left-style: groove;
}

legend {
    display: block;
    padding-left: 2px;
    padding-right: 2px;
}

input, textarea, select, button {
    font-family: inherit;
    font-size: inherit;
}

button {
    display: inline;
    text-align: center;
    cursor: pointer;
}

textarea {
    display: inline;
    white-space: pre;
    font-family: monospace;
}

label {
    cursor: pointer;
}

/* ── Embedded content ────────────────────────────────────────────── */

img, svg, video, audio, canvas, iframe, object, embed {
    display: inline;
}

/* ── Hidden elements ─────────────────────────────────────────────── */

head, title, meta, link, style, script, noscript, template {
    display: none;
}

[hidden] {
    display: none;
}

/* ── Misc block elements ─────────────────────────────────────────── */

center {
    display: block;
    text-align: center;
}

br {
    display: inline;
}

wbr {
    display: inline;
}

/* ── Ruby ────────────────────────────────────────────────────────── */

ruby {
    display: inline;
}

rt {
    font-size: 0.5em;
}

/* ── Output ──────────────────────────────────────────────────────── */

output {
    display: inline;
}

progress, meter {
    display: inline;
}
"#;

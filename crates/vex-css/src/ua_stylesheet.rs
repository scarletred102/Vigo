// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! User-agent stylesheet — minimal browser defaults for common HTML elements.

use crate::parser::{parse_stylesheet, Stylesheet};

/// Returns the built-in user-agent stylesheet.
pub fn ua_stylesheet() -> Stylesheet {
    parse_stylesheet(UA_CSS)
}

const UA_CSS: &str = r#"
html {
    display: block;
}

body {
    display: block;
    margin: 8px;
}

div, section, article, aside, nav, main, header, footer, figure, figcaption,
details, summary, dialog {
    display: block;
}

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

a {
    color: blue;
    text-decoration: underline;
    cursor: pointer;
}

ul, ol {
    display: block;
    padding-left: 40px;
    margin-top: 1em;
    margin-bottom: 1em;
}

li {
    display: list-item;
}

blockquote {
    display: block;
    margin-top: 1em;
    margin-bottom: 1em;
    margin-left: 40px;
    margin-right: 40px;
}

pre {
    display: block;
    font-family: monospace;
    white-space: pre;
    margin-top: 1em;
    margin-bottom: 1em;
}

code, kbd, samp {
    font-family: monospace;
}

b, strong {
    font-weight: bold;
}

i, em {
    font-style: italic;
}

u {
    text-decoration: underline;
}

s, strike, del {
    text-decoration: line-through;
}

small {
    font-size: 0.83em;
}

hr {
    display: block;
    margin-top: 0.5em;
    margin-bottom: 0.5em;
    border-top-style: solid;
    border-top-width: 1px;
}

table {
    display: table;
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

form {
    display: block;
    margin-top: 0;
    margin-bottom: 0;
}

input, textarea, select, button {
    font-family: inherit;
    font-size: inherit;
}

img {
    display: inline;
}
"#;

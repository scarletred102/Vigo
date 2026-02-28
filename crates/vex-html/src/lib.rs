// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! # vex-html
//!
//! HTML5-compliant parser for the Vex browser engine.
//!
//! Wraps `html5ever` with a custom `TreeSink` that builds a `vex-dom` tree.
//! Supports full document parsing, fragment parsing, and incremental (streaming) parsing.

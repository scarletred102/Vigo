// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! # vex-css
//!
//! CSS parser and style system for the Vex browser engine.
//!
//! Parses stylesheets via `cssparser`, resolves the cascade
//! (specificity, origin, inheritance), and produces `ComputedStyle`
//! for every DOM element. Supports media queries.

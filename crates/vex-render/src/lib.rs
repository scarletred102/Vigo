// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! # vex-render
//!
//! GPU rendering pipeline for the Vex browser engine.
//!
//! Builds display lists from layout trees and renders via wgpu.
//! Manages glyph atlas, image atlas, scrolling, and the Zig
//! platform layer bridge for windowing.

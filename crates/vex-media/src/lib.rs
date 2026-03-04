// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! # vex-media
//!
//! Media pipeline for the Vex browser engine.
//!
//! Audio/video decoding (via Zig/ffmpeg FFI), DASH/HLS streaming,
//! adaptive bitrate, A/V synchronization, picture-in-picture.

pub mod abr;
pub mod controls;
pub mod dash;
pub mod ffi;
pub mod hls;
pub mod media_element;
pub mod media_loading;
pub mod pip;
pub mod pipeline;
pub mod streaming;
pub mod sync;
pub mod video_render;

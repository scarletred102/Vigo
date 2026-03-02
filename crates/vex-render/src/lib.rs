// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! # vex-render
//!
//! GPU rendering pipeline and platform windowing for the Vex browser engine.

pub mod display_list;
pub mod event;
pub mod form_painter;
pub mod glyph_atlas;
pub mod gpu;
pub mod image_atlas;
pub mod image_decode;
pub mod painter;
#[cfg(target_os = "windows")]
pub mod platform;
#[cfg(target_os = "windows")]
mod platform_ffi;
pub mod privacy;
pub mod renderer;
pub mod screenshot;
pub mod scroll;

pub use display_list::DisplayList;
pub use event::Event;
#[cfg(target_os = "windows")]
pub use gpu::GpuContext;
pub use painter::build_display_list;
#[cfg(target_os = "windows")]
pub use platform::Window;
pub use privacy::RenderPrivacyConfig;

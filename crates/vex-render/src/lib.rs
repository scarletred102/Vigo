// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! # vex-render
//!
//! GPU rendering pipeline and platform windowing for the Vex browser engine.

pub mod event;
pub mod gpu;
#[cfg(target_os = "windows")]
pub mod platform;
#[cfg(target_os = "windows")]
mod platform_ffi;

pub use event::Event;
#[cfg(target_os = "windows")]
pub use gpu::GpuContext;
#[cfg(target_os = "windows")]
pub use platform::Window;

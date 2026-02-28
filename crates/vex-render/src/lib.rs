// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! # vex-render
//!
//! GPU rendering pipeline and platform windowing for the Vex browser engine.

pub mod event;
pub mod gpu;
pub mod platform;
mod platform_ffi;

pub use event::Event;
pub use gpu::GpuContext;
pub use platform::Window;

// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

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

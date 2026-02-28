// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! DOM events — types, listeners, and dispatch.

pub mod dispatch;
pub mod event;
pub mod event_type;
pub mod listeners;

pub use dispatch::dispatch_event;
pub use event::{Event, EventPhase};
pub use event_type::EventType;
pub use listeners::{EventListener, EventListenerMap};

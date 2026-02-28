// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! # vex-net
//!
//! Network stack for the Vex browser engine.
//!
//! Provides HTTP/1.1 + HTTP/2 client with TLS 1.3 (rustls),
//! DNS resolution with DoH support, caching, compression,
//! cookie handling, and redirect following.

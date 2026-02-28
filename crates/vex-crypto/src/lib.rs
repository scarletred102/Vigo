// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! # vex-crypto
//!
//! Cryptographic primitives for the Vex browser engine.
//!
//! - Symmetric encryption: ChaCha20-Poly1305
//! - Key derivation: Argon2id
//! - Signatures: Ed25519
//! - Key exchange: X25519
//! - All secrets `Zeroize` on drop

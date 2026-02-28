// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! # vex-crypto
//!
//! Cryptographic primitives for the Vex browser engine.
//!
//! - Symmetric encryption: ChaCha20-Poly1305
//! - Key derivation: Argon2id
//! - Signatures: Ed25519
//! - Key exchange: X25519
//! - All secrets `Zeroize` on drop

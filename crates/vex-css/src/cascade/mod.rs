// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Cascade resolution: specificity, origin ordering, matching, and final resolution.

pub mod specificity;
pub mod origin;
pub mod matching;
pub mod resolve;
pub mod inheritance;
pub mod value_resolution;
pub mod compute;

pub use specificity::Specificity;
pub use origin::Origin;
pub use matching::collect_matching_declarations;
pub use resolve::resolve_cascade;
pub use inheritance::apply_inheritance;
pub use value_resolution::resolve_value;
pub use compute::compute_styles;

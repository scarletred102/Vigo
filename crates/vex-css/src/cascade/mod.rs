// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Cascade resolution: specificity, origin ordering, matching, and final resolution.

pub mod compute;
pub mod inheritance;
pub mod matching;
pub mod origin;
pub mod resolve;
pub mod specificity;
pub mod value_resolution;

pub use compute::compute_styles;
pub use inheritance::apply_inheritance;
pub use matching::collect_matching_declarations;
pub use origin::Origin;
pub use resolve::resolve_cascade;
pub use specificity::Specificity;
pub use value_resolution::resolve_value;

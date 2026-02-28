// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! CSS cascade origin ordering.

/// The origin of a CSS declaration, ordered from lowest to highest priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Origin {
    /// Browser default styles.
    UserAgent = 0,
    /// Author stylesheets (external + `<style>`).
    Author = 1,
    /// Inline `style=""` attribute.
    Inline = 2,
    /// Author `!important` declarations.
    AuthorImportant = 3,
    /// Inline `!important` declarations.
    InlineImportant = 4,
}

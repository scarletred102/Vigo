// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! CSS value resolution: convert relative units (em, rem, %, vw, vh) to px.

use crate::properties::Property;
use crate::values::LengthValue;
use vex_core::Size;

/// Resolve a length property value to absolute pixels.
pub fn resolve_value(
    length: &LengthValue,
    parent_font_size: f32,
    root_font_size: f32,
    viewport: Size,
    containing: f32,
) -> f32 {
    length.resolve(parent_font_size, root_font_size, viewport, containing)
}

/// Resolve all length-based properties in a Property to pixel values.
pub fn resolve_property(
    property: &Property,
    parent_font_size: f32,
    root_font_size: f32,
    viewport: Size,
    containing_width: f32,
) -> Property {
    match property {
        Property::Width(l) => Property::Width(resolve_to_px(
            l,
            parent_font_size,
            root_font_size,
            viewport,
            containing_width,
        )),
        Property::Height(l) => Property::Height(resolve_to_px(
            l,
            parent_font_size,
            root_font_size,
            viewport,
            containing_width,
        )),
        Property::MinWidth(l) => Property::MinWidth(resolve_to_px(
            l,
            parent_font_size,
            root_font_size,
            viewport,
            containing_width,
        )),
        Property::MinHeight(l) => Property::MinHeight(resolve_to_px(
            l,
            parent_font_size,
            root_font_size,
            viewport,
            containing_width,
        )),
        Property::MaxWidth(l) => Property::MaxWidth(resolve_to_px(
            l,
            parent_font_size,
            root_font_size,
            viewport,
            containing_width,
        )),
        Property::MaxHeight(l) => Property::MaxHeight(resolve_to_px(
            l,
            parent_font_size,
            root_font_size,
            viewport,
            containing_width,
        )),

        Property::MarginTop(l) => Property::MarginTop(resolve_to_px(
            l,
            parent_font_size,
            root_font_size,
            viewport,
            containing_width,
        )),
        Property::MarginRight(l) => Property::MarginRight(resolve_to_px(
            l,
            parent_font_size,
            root_font_size,
            viewport,
            containing_width,
        )),
        Property::MarginBottom(l) => Property::MarginBottom(resolve_to_px(
            l,
            parent_font_size,
            root_font_size,
            viewport,
            containing_width,
        )),
        Property::MarginLeft(l) => Property::MarginLeft(resolve_to_px(
            l,
            parent_font_size,
            root_font_size,
            viewport,
            containing_width,
        )),

        Property::PaddingTop(l) => Property::PaddingTop(resolve_to_px(
            l,
            parent_font_size,
            root_font_size,
            viewport,
            containing_width,
        )),
        Property::PaddingRight(l) => Property::PaddingRight(resolve_to_px(
            l,
            parent_font_size,
            root_font_size,
            viewport,
            containing_width,
        )),
        Property::PaddingBottom(l) => Property::PaddingBottom(resolve_to_px(
            l,
            parent_font_size,
            root_font_size,
            viewport,
            containing_width,
        )),
        Property::PaddingLeft(l) => Property::PaddingLeft(resolve_to_px(
            l,
            parent_font_size,
            root_font_size,
            viewport,
            containing_width,
        )),

        Property::FontSize(l) => Property::FontSize(resolve_to_px(
            l,
            parent_font_size,
            root_font_size,
            viewport,
            containing_width,
        )),
        Property::LineHeight(l) => Property::LineHeight(resolve_to_px(
            l,
            parent_font_size,
            root_font_size,
            viewport,
            containing_width,
        )),

        // Non-length properties pass through unchanged
        other => other.clone(),
    }
}

fn resolve_to_px(
    length: &LengthValue,
    parent_font_size: f32,
    root_font_size: f32,
    viewport: Size,
    containing: f32,
) -> LengthValue {
    if length.is_auto() {
        return LengthValue::Auto;
    }
    LengthValue::Px(length.resolve(parent_font_size, root_font_size, viewport, containing))
}

#[cfg(test)]
mod tests {
    use super::*;

    const VP: Size = Size {
        width: 1920.0,
        height: 1080.0,
    };

    #[test]
    fn resolve_em_to_px() {
        let v = resolve_value(&LengthValue::Em(2.0), 16.0, 16.0, VP, 800.0);
        assert_eq!(v, 32.0);
    }

    #[test]
    fn resolve_rem_to_px() {
        let v = resolve_value(&LengthValue::Rem(1.5), 14.0, 16.0, VP, 800.0);
        assert_eq!(v, 24.0);
    }

    #[test]
    fn resolve_percent_to_px() {
        let v = resolve_value(&LengthValue::Percent(50.0), 16.0, 16.0, VP, 800.0);
        assert_eq!(v, 400.0);
    }

    #[test]
    fn resolve_vw_to_px() {
        let v = resolve_value(&LengthValue::Vw(10.0), 16.0, 16.0, VP, 800.0);
        assert_eq!(v, 192.0);
    }

    #[test]
    fn resolve_vh_to_px() {
        let v = resolve_value(&LengthValue::Vh(50.0), 16.0, 16.0, VP, 800.0);
        assert_eq!(v, 540.0);
    }

    #[test]
    fn resolve_px_passthrough() {
        let v = resolve_value(&LengthValue::Px(42.0), 16.0, 16.0, VP, 800.0);
        assert_eq!(v, 42.0);
    }

    #[test]
    fn resolve_auto() {
        let v = resolve_value(&LengthValue::Auto, 16.0, 16.0, VP, 800.0);
        assert_eq!(v, 0.0);
    }

    #[test]
    fn resolve_property_width() {
        let p = resolve_property(
            &Property::Width(LengthValue::Em(2.0)),
            16.0,
            16.0,
            VP,
            800.0,
        );
        assert_eq!(p, Property::Width(LengthValue::Px(32.0)));
    }
}

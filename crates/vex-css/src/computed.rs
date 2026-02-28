// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! `ComputedStyle` — fully resolved style for an element (all values in absolute px).

use vex_core::Color;

use crate::values::box_model::{BorderStyle, BoxSizing, Visibility};
use crate::values::display::Display;
use crate::values::flex::{AlignContent, AlignItems, AlignSelf, FlexDirection, FlexWrap, JustifyContent};
use crate::values::font::{FontFamily, FontStyle, FontWeight};
use crate::values::position::Position;
use crate::values::text::{Cursor, Overflow, TextAlign, TextDecoration, VerticalAlign, WhiteSpace};

/// Fully computed style for an element.
/// All lengths are in absolute pixels. All values are concrete.
#[derive(Debug, Clone, PartialEq)]
pub struct ComputedStyle {
    // Display & positioning
    pub display: Display,
    pub position: Position,

    // Dimensions (f32::NAN = auto)
    pub width: f32,
    pub height: f32,
    pub min_width: f32,
    pub min_height: f32,
    pub max_width: f32,
    pub max_height: f32,

    // Margin
    pub margin_top: f32,
    pub margin_right: f32,
    pub margin_bottom: f32,
    pub margin_left: f32,

    // Padding
    pub padding_top: f32,
    pub padding_right: f32,
    pub padding_bottom: f32,
    pub padding_left: f32,

    // Border width
    pub border_top_width: f32,
    pub border_right_width: f32,
    pub border_bottom_width: f32,
    pub border_left_width: f32,

    // Border style
    pub border_top_style: BorderStyle,
    pub border_right_style: BorderStyle,
    pub border_bottom_style: BorderStyle,
    pub border_left_style: BorderStyle,

    // Border color
    pub border_top_color: Color,
    pub border_right_color: Color,
    pub border_bottom_color: Color,
    pub border_left_color: Color,

    // Colors
    pub color: Color,
    pub background_color: Color,

    // Typography
    pub font_family: FontFamily,
    pub font_size: f32,
    pub font_weight: FontWeight,
    pub font_style: FontStyle,
    pub line_height: f32,
    pub text_align: TextAlign,
    pub text_decoration: TextDecoration,
    pub white_space: WhiteSpace,

    // Visual
    pub opacity: f32,
    pub overflow: Overflow,
    pub overflow_x: Overflow,
    pub overflow_y: Overflow,
    pub visibility: Visibility,
    pub cursor: Cursor,
    pub box_sizing: BoxSizing,
    pub z_index: i32,

    // Flexbox
    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
    pub align_self: AlignSelf,
    pub align_content: AlignContent,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: f32,

    // Position offsets (NAN = auto)
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,

    // Vertical align
    pub vertical_align: VerticalAlign,
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self {
            display: Display::Inline,
            position: Position::Static,

            width: f32::NAN,
            height: f32::NAN,
            min_width: 0.0,
            min_height: 0.0,
            max_width: f32::INFINITY,
            max_height: f32::INFINITY,

            margin_top: 0.0,
            margin_right: 0.0,
            margin_bottom: 0.0,
            margin_left: 0.0,

            padding_top: 0.0,
            padding_right: 0.0,
            padding_bottom: 0.0,
            padding_left: 0.0,

            border_top_width: 0.0,
            border_right_width: 0.0,
            border_bottom_width: 0.0,
            border_left_width: 0.0,

            border_top_style: BorderStyle::None,
            border_right_style: BorderStyle::None,
            border_bottom_style: BorderStyle::None,
            border_left_style: BorderStyle::None,

            border_top_color: Color::BLACK,
            border_right_color: Color::BLACK,
            border_bottom_color: Color::BLACK,
            border_left_color: Color::BLACK,

            color: Color::BLACK,
            background_color: Color::TRANSPARENT,

            font_family: FontFamily::default(),
            font_size: 16.0,
            font_weight: FontWeight::NORMAL,
            font_style: FontStyle::Normal,
            line_height: 19.2,  // 1.2 * 16px
            text_align: TextAlign::Left,
            text_decoration: TextDecoration::None,
            white_space: WhiteSpace::Normal,

            opacity: 1.0,
            overflow: Overflow::Visible,
            overflow_x: Overflow::Visible,
            overflow_y: Overflow::Visible,
            visibility: Visibility::Visible,
            cursor: Cursor::Auto,
            box_sizing: BoxSizing::ContentBox,
            z_index: 0,

            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::NoWrap,
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::Stretch,
            align_self: AlignSelf::Auto,
            align_content: AlignContent::Stretch,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: f32::NAN,

            top: f32::NAN,
            right: f32::NAN,
            bottom: f32::NAN,
            left: f32::NAN,

            vertical_align: VerticalAlign::Baseline,
        }
    }
}

impl ComputedStyle {
    /// Apply a resolved property value to this computed style.
    pub fn apply(&mut self, property: &crate::properties::Property) {
        use crate::properties::Property;
        use crate::values::LengthValue;

        fn px(l: &LengthValue) -> f32 {
            match l {
                LengthValue::Px(v) => *v,
                LengthValue::Auto => f32::NAN,
                LengthValue::Zero => 0.0,
                _ => 0.0, // Should already be resolved to px
            }
        }

        match property {
            Property::Display(v) => self.display = *v,
            Property::Position(v) => self.position = *v,

            Property::Width(l) => self.width = px(l),
            Property::Height(l) => self.height = px(l),
            Property::MinWidth(l) => self.min_width = px(l),
            Property::MinHeight(l) => self.min_height = px(l),
            Property::MaxWidth(l) => self.max_width = px(l),
            Property::MaxHeight(l) => self.max_height = px(l),

            Property::MarginTop(l) => self.margin_top = px(l),
            Property::MarginRight(l) => self.margin_right = px(l),
            Property::MarginBottom(l) => self.margin_bottom = px(l),
            Property::MarginLeft(l) => self.margin_left = px(l),

            Property::PaddingTop(l) => self.padding_top = px(l),
            Property::PaddingRight(l) => self.padding_right = px(l),
            Property::PaddingBottom(l) => self.padding_bottom = px(l),
            Property::PaddingLeft(l) => self.padding_left = px(l),

            Property::BorderTopWidth(l) => self.border_top_width = px(l),
            Property::BorderRightWidth(l) => self.border_right_width = px(l),
            Property::BorderBottomWidth(l) => self.border_bottom_width = px(l),
            Property::BorderLeftWidth(l) => self.border_left_width = px(l),

            Property::BorderTopStyle(v) => self.border_top_style = *v,
            Property::BorderRightStyle(v) => self.border_right_style = *v,
            Property::BorderBottomStyle(v) => self.border_bottom_style = *v,
            Property::BorderLeftStyle(v) => self.border_left_style = *v,

            Property::BorderTopColor(c) => self.border_top_color = c.resolve(self.color),
            Property::BorderRightColor(c) => self.border_right_color = c.resolve(self.color),
            Property::BorderBottomColor(c) => self.border_bottom_color = c.resolve(self.color),
            Property::BorderLeftColor(c) => self.border_left_color = c.resolve(self.color),

            Property::Color(c) => self.color = c.resolve(self.color),
            Property::BackgroundColor(c) => self.background_color = c.resolve(self.color),

            Property::FontFamily(v) => self.font_family = v.clone(),
            Property::FontSize(l) => self.font_size = px(l),
            Property::FontWeight(v) => self.font_weight = *v,
            Property::FontStyle(v) => self.font_style = *v,
            Property::LineHeight(l) => self.line_height = px(l),
            Property::TextAlign(v) => self.text_align = *v,
            Property::TextDecoration(v) => self.text_decoration = *v,
            Property::WhiteSpace(v) => self.white_space = *v,

            Property::Opacity(v) => self.opacity = *v,
            Property::Overflow(v) => self.overflow = *v,
            Property::OverflowX(v) => self.overflow_x = *v,
            Property::OverflowY(v) => self.overflow_y = *v,
            Property::Visibility(v) => self.visibility = *v,
            Property::Cursor(v) => self.cursor = *v,
            Property::BoxSizing(v) => self.box_sizing = *v,
            Property::ZIndex(v) => self.z_index = *v,

            Property::FlexDirection(v) => self.flex_direction = *v,
            Property::FlexWrap(v) => self.flex_wrap = *v,
            Property::JustifyContent(v) => self.justify_content = *v,
            Property::AlignItems(v) => self.align_items = *v,
            Property::AlignSelf(v) => self.align_self = *v,
            Property::AlignContent(v) => self.align_content = *v,
            Property::FlexGrow(v) => self.flex_grow = *v,
            Property::FlexShrink(v) => self.flex_shrink = *v,
            Property::FlexBasis(l) => self.flex_basis = px(l),

            Property::Top(l) => self.top = px(l),
            Property::Right(l) => self.right = px(l),
            Property::Bottom(l) => self.bottom = px(l),
            Property::Left(l) => self.left = px(l),

            Property::VerticalAlign(v) => self.vertical_align = *v,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::properties::Property;
    use crate::values::LengthValue;

    #[test]
    fn default_style() {
        let s = ComputedStyle::default();
        assert_eq!(s.display, Display::Inline);
        assert_eq!(s.font_size, 16.0);
        assert_eq!(s.color, Color::BLACK);
        assert_eq!(s.opacity, 1.0);
    }

    #[test]
    fn apply_property() {
        let mut s = ComputedStyle::default();
        s.apply(&Property::Display(Display::Block));
        assert_eq!(s.display, Display::Block);

        s.apply(&Property::Width(LengthValue::Px(100.0)));
        assert_eq!(s.width, 100.0);

        s.apply(&Property::Opacity(0.5));
        assert_eq!(s.opacity, 0.5);
    }
}

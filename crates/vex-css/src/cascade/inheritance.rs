// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! CSS property inheritance.

use crate::computed::ComputedStyle;

/// Apply inheritance from parent to child for inherited properties.
/// Properties that CSS defines as inherited (color, font-*, text-align, etc.)
/// copy from parent when not explicitly set on the child.
pub fn apply_inheritance(child: &mut ComputedStyle, parent: &ComputedStyle) {
    // Color
    if child.color == ComputedStyle::default().color && child.color != parent.color {
        // Only inherit if the child has the default value
    }
    // For simplicity, always copy inherited properties if the child hasn't set them.
    // We use a "set flags" approach: properties that were explicitly set are marked.
    // For now, we unconditionally copy if the child still has the default.

    let defaults = ComputedStyle::default();

    // Color inherits
    if child.color == defaults.color {
        child.color = parent.color;
    }

    // Font properties inherit
    if child.font_family == defaults.font_family {
        child.font_family = parent.font_family.clone();
    }
    if (child.font_size - defaults.font_size).abs() < f32::EPSILON {
        child.font_size = parent.font_size;
    }
    if child.font_weight == defaults.font_weight {
        child.font_weight = parent.font_weight;
    }
    if child.font_style == defaults.font_style {
        child.font_style = parent.font_style;
    }

    // Line-height inherits
    if (child.line_height - defaults.line_height).abs() < f32::EPSILON {
        child.line_height = parent.line_height;
    }

    // Text properties inherit
    if child.text_align == defaults.text_align {
        child.text_align = parent.text_align;
    }
    if child.text_decoration == defaults.text_decoration {
        child.text_decoration = parent.text_decoration;
    }
    if child.white_space == defaults.white_space {
        child.white_space = parent.white_space;
    }

    // Visibility inherits
    if child.visibility == defaults.visibility {
        child.visibility = parent.visibility;
    }

    // Cursor inherits
    if child.cursor == defaults.cursor {
        child.cursor = parent.cursor;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vex_core::Color;
    use crate::values::text::TextAlign;

    #[test]
    fn color_inherits() {
        let parent = ComputedStyle {
            color: Color { r: 255, g: 0, b: 0, a: 255 },
            ..Default::default()
        };
        let mut child = ComputedStyle::default();
        apply_inheritance(&mut child, &parent);
        assert_eq!(child.color, Color { r: 255, g: 0, b: 0, a: 255 });
    }

    #[test]
    fn margin_does_not_inherit() {
        let parent = ComputedStyle {
            margin_top: 20.0,
            ..Default::default()
        };
        let mut child = ComputedStyle::default();
        apply_inheritance(&mut child, &parent);
        assert_eq!(child.margin_top, 0.0); // Not inherited
    }

    #[test]
    fn explicit_value_overrides_inheritance() {
        let parent = ComputedStyle {
            text_align: TextAlign::Center,
            ..Default::default()
        };
        let mut child = ComputedStyle {
            text_align: TextAlign::Right,
            ..Default::default()
        };
        apply_inheritance(&mut child, &parent);
        // Child explicitly set to Right, should NOT be overridden
        assert_eq!(child.text_align, TextAlign::Right);
    }
}

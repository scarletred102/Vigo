// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! CSS property enum covering ~50 most common properties.

use crate::values::box_model::{BoxSizing, Visibility};
use crate::values::animation::{
    AnimationDirection, AnimationFillMode, AnimationIterationCount, AnimationPlayState,
    TimingFunction, TransitionProperty,
};
use crate::values::grid::{GridAutoFlow, GridLine, TrackList};
use crate::values::text::Cursor;
use crate::values::transform::{FilterList, TransformList, TransformOrigin};
use crate::values::*;

/// A single CSS declaration (property + value).
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    // Display & positioning
    Display(Display),
    Position(Position),

    // Dimensions
    Width(LengthValue),
    Height(LengthValue),
    MinWidth(LengthValue),
    MinHeight(LengthValue),
    MaxWidth(LengthValue),
    MaxHeight(LengthValue),

    // Margin (4 sides)
    MarginTop(LengthValue),
    MarginRight(LengthValue),
    MarginBottom(LengthValue),
    MarginLeft(LengthValue),

    // Padding (4 sides)
    PaddingTop(LengthValue),
    PaddingRight(LengthValue),
    PaddingBottom(LengthValue),
    PaddingLeft(LengthValue),

    // Border width (4 sides)
    BorderTopWidth(LengthValue),
    BorderRightWidth(LengthValue),
    BorderBottomWidth(LengthValue),
    BorderLeftWidth(LengthValue),

    // Border style (4 sides)
    BorderTopStyle(BorderStyle),
    BorderRightStyle(BorderStyle),
    BorderBottomStyle(BorderStyle),
    BorderLeftStyle(BorderStyle),

    // Border color (4 sides)
    BorderTopColor(ColorValue),
    BorderRightColor(ColorValue),
    BorderBottomColor(ColorValue),
    BorderLeftColor(ColorValue),

    // Colors
    Color(ColorValue),
    BackgroundColor(ColorValue),

    // Typography
    FontFamily(FontFamily),
    FontSize(LengthValue),
    FontWeight(FontWeight),
    FontStyle(FontStyle),
    LineHeight(LengthValue),
    TextAlign(TextAlign),
    TextDecoration(TextDecoration),
    WhiteSpace(WhiteSpace),

    // Visual
    Opacity(f32),
    Overflow(Overflow),
    OverflowX(Overflow),
    OverflowY(Overflow),
    Visibility(Visibility),
    Cursor(Cursor),
    BoxSizing(BoxSizing),
    ZIndex(i32),

    // Flexbox
    FlexDirection(FlexDirection),
    FlexWrap(FlexWrap),
    JustifyContent(JustifyContent),
    AlignItems(AlignItems),
    AlignSelf(AlignSelf),
    AlignContent(AlignContent),
    FlexGrow(f32),
    FlexShrink(f32),
    FlexBasis(LengthValue),

    // Position offsets
    Top(LengthValue),
    Right(LengthValue),
    Bottom(LengthValue),
    Left(LengthValue),

    // Vertical align
    VerticalAlign(VerticalAlign),

    // Float & clear
    Float(Float),
    Clear(Clear),

    // Grid
    GridTemplateColumns(TrackList),
    GridTemplateRows(TrackList),
    GridAutoFlow(GridAutoFlow),
    GridColumnStart(GridLine),
    GridColumnEnd(GridLine),
    GridRowStart(GridLine),
    GridRowEnd(GridLine),
    GridColumnGap(f32),
    GridRowGap(f32),

    // Transitions
    TransitionProperty(TransitionProperty),
    TransitionDuration(f32),       // seconds
    TransitionTimingFunction(TimingFunction),
    TransitionDelay(f32),          // seconds

    // Animations
    AnimationName(String),
    AnimationDuration(f32),        // seconds
    AnimationTimingFunction(TimingFunction),
    AnimationDelay(f32),           // seconds
    AnimationIterationCount(AnimationIterationCount),
    AnimationDirection(AnimationDirection),
    AnimationFillMode(AnimationFillMode),
    AnimationPlayState(AnimationPlayState),

    // Transform & related
    Transform(TransformList),
    TransformOrigin(TransformOrigin),
    Filter(FilterList),
    BackdropFilter(FilterList),
}

/// A declaration with importance flag.
#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    pub property: Property,
    pub important: bool,
}

impl Property {
    /// Returns the CSS property name string.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Display(_) => "display",
            Self::Position(_) => "position",
            Self::Width(_) => "width",
            Self::Height(_) => "height",
            Self::MinWidth(_) => "min-width",
            Self::MinHeight(_) => "min-height",
            Self::MaxWidth(_) => "max-width",
            Self::MaxHeight(_) => "max-height",
            Self::MarginTop(_) => "margin-top",
            Self::MarginRight(_) => "margin-right",
            Self::MarginBottom(_) => "margin-bottom",
            Self::MarginLeft(_) => "margin-left",
            Self::PaddingTop(_) => "padding-top",
            Self::PaddingRight(_) => "padding-right",
            Self::PaddingBottom(_) => "padding-bottom",
            Self::PaddingLeft(_) => "padding-left",
            Self::BorderTopWidth(_) => "border-top-width",
            Self::BorderRightWidth(_) => "border-right-width",
            Self::BorderBottomWidth(_) => "border-bottom-width",
            Self::BorderLeftWidth(_) => "border-left-width",
            Self::BorderTopStyle(_) => "border-top-style",
            Self::BorderRightStyle(_) => "border-right-style",
            Self::BorderBottomStyle(_) => "border-bottom-style",
            Self::BorderLeftStyle(_) => "border-left-style",
            Self::BorderTopColor(_) => "border-top-color",
            Self::BorderRightColor(_) => "border-right-color",
            Self::BorderBottomColor(_) => "border-bottom-color",
            Self::BorderLeftColor(_) => "border-left-color",
            Self::Color(_) => "color",
            Self::BackgroundColor(_) => "background-color",
            Self::FontFamily(_) => "font-family",
            Self::FontSize(_) => "font-size",
            Self::FontWeight(_) => "font-weight",
            Self::FontStyle(_) => "font-style",
            Self::LineHeight(_) => "line-height",
            Self::TextAlign(_) => "text-align",
            Self::TextDecoration(_) => "text-decoration",
            Self::WhiteSpace(_) => "white-space",
            Self::Opacity(_) => "opacity",
            Self::Overflow(_) => "overflow",
            Self::OverflowX(_) => "overflow-x",
            Self::OverflowY(_) => "overflow-y",
            Self::Visibility(_) => "visibility",
            Self::Cursor(_) => "cursor",
            Self::BoxSizing(_) => "box-sizing",
            Self::ZIndex(_) => "z-index",
            Self::FlexDirection(_) => "flex-direction",
            Self::FlexWrap(_) => "flex-wrap",
            Self::JustifyContent(_) => "justify-content",
            Self::AlignItems(_) => "align-items",
            Self::AlignSelf(_) => "align-self",
            Self::AlignContent(_) => "align-content",
            Self::FlexGrow(_) => "flex-grow",
            Self::FlexShrink(_) => "flex-shrink",
            Self::FlexBasis(_) => "flex-basis",
            Self::Top(_) => "top",
            Self::Right(_) => "right",
            Self::Bottom(_) => "bottom",
            Self::Left(_) => "left",
            Self::VerticalAlign(_) => "vertical-align",
            Self::Float(_) => "float",
            Self::Clear(_) => "clear",
            Self::GridTemplateColumns(_) => "grid-template-columns",
            Self::GridTemplateRows(_) => "grid-template-rows",
            Self::GridAutoFlow(_) => "grid-auto-flow",
            Self::GridColumnStart(_) => "grid-column-start",
            Self::GridColumnEnd(_) => "grid-column-end",
            Self::GridRowStart(_) => "grid-row-start",
            Self::GridRowEnd(_) => "grid-row-end",
            Self::GridColumnGap(_) => "column-gap",
            Self::GridRowGap(_) => "row-gap",
            Self::TransitionProperty(_) => "transition-property",
            Self::TransitionDuration(_) => "transition-duration",
            Self::TransitionTimingFunction(_) => "transition-timing-function",
            Self::TransitionDelay(_) => "transition-delay",
            Self::AnimationName(_) => "animation-name",
            Self::AnimationDuration(_) => "animation-duration",
            Self::AnimationTimingFunction(_) => "animation-timing-function",
            Self::AnimationDelay(_) => "animation-delay",
            Self::AnimationIterationCount(_) => "animation-iteration-count",
            Self::AnimationDirection(_) => "animation-direction",
            Self::AnimationFillMode(_) => "animation-fill-mode",
            Self::AnimationPlayState(_) => "animation-play-state",
            Self::Transform(_) => "transform",
            Self::TransformOrigin(_) => "transform-origin",
            Self::Filter(_) => "filter",
            Self::BackdropFilter(_) => "backdrop-filter",
        }
    }

    /// Whether this property inherits by default.
    pub fn inherits(&self) -> bool {
        matches!(
            self,
            Self::Color(_)
                | Self::FontFamily(_)
                | Self::FontSize(_)
                | Self::FontWeight(_)
                | Self::FontStyle(_)
                | Self::LineHeight(_)
                | Self::TextAlign(_)
                | Self::TextDecoration(_)
                | Self::WhiteSpace(_)
                | Self::Visibility(_)
                | Self::Cursor(_)
        )
    }
}

/// Parse a single CSS declaration from a property name + value string.
/// Returns the expanded list (shorthands expand into multiple properties).
pub fn parse_declaration(name: &str, value: &str) -> Vec<Property> {
    let name = name.trim().to_ascii_lowercase();
    let value = value.trim();

    match name.as_str() {
        // Shorthands
        "margin" => parse_shorthand_4(
            value,
            Property::MarginTop,
            Property::MarginRight,
            Property::MarginBottom,
            Property::MarginLeft,
        ),
        "padding" => parse_shorthand_4(
            value,
            Property::PaddingTop,
            Property::PaddingRight,
            Property::PaddingBottom,
            Property::PaddingLeft,
        ),
        "border-width" => parse_shorthand_4(
            value,
            Property::BorderTopWidth,
            Property::BorderRightWidth,
            Property::BorderBottomWidth,
            Property::BorderLeftWidth,
        ),
        "border-style" => parse_border_style_shorthand(value),
        "border-color" => parse_border_color_shorthand(value),
        "border" => parse_border_shorthand(value),

        // Longhand properties
        "display" => Display::parse(value)
            .map(Property::Display)
            .into_iter()
            .collect(),
        "position" => Position::parse(value)
            .map(Property::Position)
            .into_iter()
            .collect(),

        "width" => parse_length_prop(value, Property::Width),
        "height" => parse_length_prop(value, Property::Height),
        "min-width" => parse_length_prop(value, Property::MinWidth),
        "min-height" => parse_length_prop(value, Property::MinHeight),
        "max-width" => parse_length_prop(value, Property::MaxWidth),
        "max-height" => parse_length_prop(value, Property::MaxHeight),

        "margin-top" => parse_length_prop(value, Property::MarginTop),
        "margin-right" => parse_length_prop(value, Property::MarginRight),
        "margin-bottom" => parse_length_prop(value, Property::MarginBottom),
        "margin-left" => parse_length_prop(value, Property::MarginLeft),

        "padding-top" => parse_length_prop(value, Property::PaddingTop),
        "padding-right" => parse_length_prop(value, Property::PaddingRight),
        "padding-bottom" => parse_length_prop(value, Property::PaddingBottom),
        "padding-left" => parse_length_prop(value, Property::PaddingLeft),

        "border-top-width" => parse_length_prop(value, Property::BorderTopWidth),
        "border-right-width" => parse_length_prop(value, Property::BorderRightWidth),
        "border-bottom-width" => parse_length_prop(value, Property::BorderBottomWidth),
        "border-left-width" => parse_length_prop(value, Property::BorderLeftWidth),

        "border-top-style" => BorderStyle::parse(value)
            .map(Property::BorderTopStyle)
            .into_iter()
            .collect(),
        "border-right-style" => BorderStyle::parse(value)
            .map(Property::BorderRightStyle)
            .into_iter()
            .collect(),
        "border-bottom-style" => BorderStyle::parse(value)
            .map(Property::BorderBottomStyle)
            .into_iter()
            .collect(),
        "border-left-style" => BorderStyle::parse(value)
            .map(Property::BorderLeftStyle)
            .into_iter()
            .collect(),

        "border-top-color" => color::parse_color(value)
            .map(Property::BorderTopColor)
            .into_iter()
            .collect(),
        "border-right-color" => color::parse_color(value)
            .map(Property::BorderRightColor)
            .into_iter()
            .collect(),
        "border-bottom-color" => color::parse_color(value)
            .map(Property::BorderBottomColor)
            .into_iter()
            .collect(),
        "border-left-color" => color::parse_color(value)
            .map(Property::BorderLeftColor)
            .into_iter()
            .collect(),

        "color" => color::parse_color(value)
            .map(Property::Color)
            .into_iter()
            .collect(),
        "background-color" => color::parse_color(value)
            .map(Property::BackgroundColor)
            .into_iter()
            .collect(),

        "font-family" => vec![Property::FontFamily(FontFamily::parse(value))],
        "font-size" => parse_font_size(value),
        "font-weight" => FontWeight::parse(value)
            .map(Property::FontWeight)
            .into_iter()
            .collect(),
        "font-style" => FontStyle::parse(value)
            .map(Property::FontStyle)
            .into_iter()
            .collect(),
        "line-height" => parse_length_prop(value, Property::LineHeight),

        "text-align" => TextAlign::parse(value)
            .map(Property::TextAlign)
            .into_iter()
            .collect(),
        "text-decoration" => TextDecoration::parse(value)
            .map(Property::TextDecoration)
            .into_iter()
            .collect(),
        "white-space" => WhiteSpace::parse(value)
            .map(Property::WhiteSpace)
            .into_iter()
            .collect(),

        "opacity" => value
            .parse::<f32>()
            .ok()
            .map(|v| Property::Opacity(v.clamp(0.0, 1.0)))
            .into_iter()
            .collect(),
        "overflow" => Overflow::parse(value)
            .map(Property::Overflow)
            .into_iter()
            .collect(),
        "overflow-x" => Overflow::parse(value)
            .map(Property::OverflowX)
            .into_iter()
            .collect(),
        "overflow-y" => Overflow::parse(value)
            .map(Property::OverflowY)
            .into_iter()
            .collect(),
        "visibility" => Visibility::parse(value)
            .map(Property::Visibility)
            .into_iter()
            .collect(),
        "cursor" => Cursor::parse(value)
            .map(Property::Cursor)
            .into_iter()
            .collect(),
        "box-sizing" => BoxSizing::parse(value)
            .map(Property::BoxSizing)
            .into_iter()
            .collect(),
        "z-index" => value
            .parse::<i32>()
            .ok()
            .map(Property::ZIndex)
            .into_iter()
            .collect(),

        "flex-direction" => FlexDirection::parse(value)
            .map(Property::FlexDirection)
            .into_iter()
            .collect(),
        "flex-wrap" => FlexWrap::parse(value)
            .map(Property::FlexWrap)
            .into_iter()
            .collect(),
        "justify-content" => JustifyContent::parse(value)
            .map(Property::JustifyContent)
            .into_iter()
            .collect(),
        "align-items" => AlignItems::parse(value)
            .map(Property::AlignItems)
            .into_iter()
            .collect(),
        "align-self" => AlignSelf::parse(value)
            .map(Property::AlignSelf)
            .into_iter()
            .collect(),
        "align-content" => AlignContent::parse(value)
            .map(Property::AlignContent)
            .into_iter()
            .collect(),
        "flex-grow" => value
            .parse::<f32>()
            .ok()
            .map(Property::FlexGrow)
            .into_iter()
            .collect(),
        "flex-shrink" => value
            .parse::<f32>()
            .ok()
            .map(Property::FlexShrink)
            .into_iter()
            .collect(),
        "flex-basis" => parse_length_prop(value, Property::FlexBasis),

        "top" => parse_length_prop(value, Property::Top),
        "right" => parse_length_prop(value, Property::Right),
        "bottom" => parse_length_prop(value, Property::Bottom),
        "left" => parse_length_prop(value, Property::Left),

        "vertical-align" => VerticalAlign::parse(value)
            .map(Property::VerticalAlign)
            .into_iter()
            .collect(),

        "float" => Float::parse(value)
            .map(Property::Float)
            .into_iter()
            .collect(),
        "clear" => Clear::parse(value)
            .map(Property::Clear)
            .into_iter()
            .collect(),

        // Grid
        "grid-template-columns" => vec![Property::GridTemplateColumns(TrackList::parse(value))],
        "grid-template-rows" => vec![Property::GridTemplateRows(TrackList::parse(value))],
        "grid-auto-flow" => GridAutoFlow::parse(value)
            .map(Property::GridAutoFlow)
            .into_iter()
            .collect(),
        "grid-column-start" => vec![Property::GridColumnStart(GridLine::parse(value))],
        "grid-column-end" => vec![Property::GridColumnEnd(GridLine::parse(value))],
        "grid-row-start" => vec![Property::GridRowStart(GridLine::parse(value))],
        "grid-row-end" => vec![Property::GridRowEnd(GridLine::parse(value))],
        "grid-column" => parse_grid_line_shorthand(
            value,
            Property::GridColumnStart,
            Property::GridColumnEnd,
        ),
        "grid-row" => {
            parse_grid_line_shorthand(value, Property::GridRowStart, Property::GridRowEnd)
        }
        "column-gap" | "grid-column-gap" => value
            .strip_suffix("px")
            .and_then(|v| v.trim().parse::<f32>().ok())
            .map(Property::GridColumnGap)
            .into_iter()
            .collect(),
        "row-gap" | "grid-row-gap" => value
            .strip_suffix("px")
            .and_then(|v| v.trim().parse::<f32>().ok())
            .map(Property::GridRowGap)
            .into_iter()
            .collect(),
        "gap" | "grid-gap" => parse_gap_shorthand(value),

        // Transitions
        "transition-property" => {
            vec![Property::TransitionProperty(TransitionProperty::parse(value))]
        }
        "transition-duration" => crate::values::animation::parse_time(value)
            .map(Property::TransitionDuration)
            .into_iter()
            .collect(),
        "transition-timing-function" => TimingFunction::parse(value)
            .map(Property::TransitionTimingFunction)
            .into_iter()
            .collect(),
        "transition-delay" => crate::values::animation::parse_time(value)
            .map(Property::TransitionDelay)
            .into_iter()
            .collect(),
        "transition" => parse_transition_shorthand(value),

        // Animations
        "animation-name" => vec![Property::AnimationName(value.trim().to_string())],
        "animation-duration" => crate::values::animation::parse_time(value)
            .map(Property::AnimationDuration)
            .into_iter()
            .collect(),
        "animation-timing-function" => TimingFunction::parse(value)
            .map(Property::AnimationTimingFunction)
            .into_iter()
            .collect(),
        "animation-delay" => crate::values::animation::parse_time(value)
            .map(Property::AnimationDelay)
            .into_iter()
            .collect(),
        "animation-iteration-count" => AnimationIterationCount::parse(value)
            .map(Property::AnimationIterationCount)
            .into_iter()
            .collect(),
        "animation-direction" => AnimationDirection::parse(value)
            .map(Property::AnimationDirection)
            .into_iter()
            .collect(),
        "animation-fill-mode" => AnimationFillMode::parse(value)
            .map(Property::AnimationFillMode)
            .into_iter()
            .collect(),
        "animation-play-state" => AnimationPlayState::parse(value)
            .map(Property::AnimationPlayState)
            .into_iter()
            .collect(),

        // Transform & filters
        "transform" => vec![Property::Transform(TransformList::parse(value.trim()))],
        "transform-origin" => vec![Property::TransformOrigin(TransformOrigin::parse(value.trim()))],
        "filter" => vec![Property::Filter(FilterList::parse(value.trim()))],
        "backdrop-filter" => vec![Property::BackdropFilter(FilterList::parse(value.trim()))],

        _ => vec![], // Unknown property — ignore
    }
}

fn parse_length_prop(value: &str, constructor: fn(LengthValue) -> Property) -> Vec<Property> {
    length::parse_length(value)
        .map(constructor)
        .into_iter()
        .collect()
}

fn parse_font_size(value: &str) -> Vec<Property> {
    // Support keyword sizes
    let length = match value.trim() {
        "xx-small" => Some(LengthValue::Px(9.0)),
        "x-small" => Some(LengthValue::Px(10.0)),
        "small" => Some(LengthValue::Px(13.0)),
        "medium" => Some(LengthValue::Px(16.0)),
        "large" => Some(LengthValue::Px(18.0)),
        "x-large" => Some(LengthValue::Px(24.0)),
        "xx-large" => Some(LengthValue::Px(32.0)),
        "smaller" => Some(LengthValue::Em(0.833)),
        "larger" => Some(LengthValue::Em(1.2)),
        other => length::parse_length(other),
    };
    length.map(Property::FontSize).into_iter().collect()
}

/// Parse a 1-4 value shorthand (margin, padding, border-width).
fn parse_shorthand_4(
    value: &str,
    top: fn(LengthValue) -> Property,
    right: fn(LengthValue) -> Property,
    bottom: fn(LengthValue) -> Property,
    left: fn(LengthValue) -> Property,
) -> Vec<Property> {
    let parts: Vec<&str> = value.split_whitespace().collect();
    let lengths: Vec<LengthValue> = parts
        .iter()
        .filter_map(|p| length::parse_length(p))
        .collect();

    match lengths.len() {
        1 => vec![
            top(lengths[0].clone()),
            right(lengths[0].clone()),
            bottom(lengths[0].clone()),
            left(lengths[0].clone()),
        ],
        2 => vec![
            top(lengths[0].clone()),
            right(lengths[1].clone()),
            bottom(lengths[0].clone()),
            left(lengths[1].clone()),
        ],
        3 => vec![
            top(lengths[0].clone()),
            right(lengths[1].clone()),
            bottom(lengths[2].clone()),
            left(lengths[1].clone()),
        ],
        4 => vec![
            top(lengths[0].clone()),
            right(lengths[1].clone()),
            bottom(lengths[2].clone()),
            left(lengths[3].clone()),
        ],
        _ => vec![],
    }
}

fn parse_border_style_shorthand(value: &str) -> Vec<Property> {
    let parts: Vec<&str> = value.split_whitespace().collect();
    let styles: Vec<BorderStyle> = parts.iter().filter_map(|p| BorderStyle::parse(p)).collect();

    match styles.len() {
        1 => vec![
            Property::BorderTopStyle(styles[0]),
            Property::BorderRightStyle(styles[0]),
            Property::BorderBottomStyle(styles[0]),
            Property::BorderLeftStyle(styles[0]),
        ],
        4 => vec![
            Property::BorderTopStyle(styles[0]),
            Property::BorderRightStyle(styles[1]),
            Property::BorderBottomStyle(styles[2]),
            Property::BorderLeftStyle(styles[3]),
        ],
        _ => vec![],
    }
}

fn parse_border_color_shorthand(value: &str) -> Vec<Property> {
    // Just handle single value for now
    color::parse_color(value)
        .map(|c| {
            vec![
                Property::BorderTopColor(c.clone()),
                Property::BorderRightColor(c.clone()),
                Property::BorderBottomColor(c.clone()),
                Property::BorderLeftColor(c),
            ]
        })
        .unwrap_or_default()
}

/// Parse `border: 1px solid black` shorthand.
fn parse_border_shorthand(value: &str) -> Vec<Property> {
    let parts: Vec<&str> = value.split_whitespace().collect();
    let mut width = None;
    let mut style = None;
    let mut color_val = None;

    for part in &parts {
        if width.is_none() {
            if let Some(l) = length::parse_length(part) {
                width = Some(l);
                continue;
            }
        }
        if style.is_none() {
            if let Some(s) = BorderStyle::parse(part) {
                style = Some(s);
                continue;
            }
        }
        if color_val.is_none() {
            if let Some(c) = color::parse_color(part) {
                color_val = Some(c);
            }
        }
    }

    let mut result = Vec::new();
    if let Some(w) = width {
        result.push(Property::BorderTopWidth(w.clone()));
        result.push(Property::BorderRightWidth(w.clone()));
        result.push(Property::BorderBottomWidth(w.clone()));
        result.push(Property::BorderLeftWidth(w));
    }
    if let Some(s) = style {
        result.push(Property::BorderTopStyle(s));
        result.push(Property::BorderRightStyle(s));
        result.push(Property::BorderBottomStyle(s));
        result.push(Property::BorderLeftStyle(s));
    }
    if let Some(c) = color_val {
        result.push(Property::BorderTopColor(c.clone()));
        result.push(Property::BorderRightColor(c.clone()));
        result.push(Property::BorderBottomColor(c.clone()));
        result.push(Property::BorderLeftColor(c));
    }
    result
}

/// Parse `grid-column` / `grid-row` shorthand: `start / end`.
fn parse_grid_line_shorthand(
    value: &str,
    start_ctor: fn(GridLine) -> Property,
    end_ctor: fn(GridLine) -> Property,
) -> Vec<Property> {
    if let Some((s, e)) = value.split_once('/') {
        vec![
            start_ctor(GridLine::parse(s.trim())),
            end_ctor(GridLine::parse(e.trim())),
        ]
    } else {
        vec![start_ctor(GridLine::parse(value))]
    }
}

/// Parse `gap` shorthand: `row-gap column-gap` or single value for both.
fn parse_gap_shorthand(value: &str) -> Vec<Property> {
    let parts: Vec<&str> = value.split_whitespace().collect();
    let parse_px = |s: &str| -> Option<f32> {
        s.strip_suffix("px")
            .and_then(|v| v.trim().parse::<f32>().ok())
    };
    match parts.len() {
        1 => {
            if let Some(v) = parse_px(parts[0]) {
                vec![Property::GridRowGap(v), Property::GridColumnGap(v)]
            } else {
                vec![]
            }
        }
        2 => {
            let mut result = Vec::new();
            if let Some(r) = parse_px(parts[0]) {
                result.push(Property::GridRowGap(r));
            }
            if let Some(c) = parse_px(parts[1]) {
                result.push(Property::GridColumnGap(c));
            }
            result
        }
        _ => vec![],
    }
}

/// Parse the `transition` shorthand: `property duration timing-function delay`.
///
/// Example: `transition: opacity 0.3s ease 0s`
fn parse_transition_shorthand(value: &str) -> Vec<Property> {
    let parts: Vec<&str> = value.split_whitespace().collect();
    let mut result = Vec::new();

    // First non-time, non-timing-function token is the property
    // Time values: durations/delays
    // Timing function: keywords or cubic-bezier(...)
    let mut times_found: Vec<f32> = Vec::new();
    let mut timing_fn: Option<TimingFunction> = None;
    let mut prop: Option<TransitionProperty> = None;

    for part in &parts {
        // Try as timing function keyword first
        if timing_fn.is_none() {
            if let Some(tf) = TimingFunction::parse(part) {
                timing_fn = Some(tf);
                continue;
            }
        }
        // Try as time value
        if let Some(t) = crate::values::animation::parse_time(part) {
            times_found.push(t);
            continue;
        }
        // Must be the property name
        if prop.is_none() {
            prop = Some(TransitionProperty::parse(part));
        }
    }

    result.push(Property::TransitionProperty(
        prop.unwrap_or(TransitionProperty::All),
    ));
    if let Some(dur) = times_found.first() {
        result.push(Property::TransitionDuration(*dur));
    }
    if let Some(tf) = timing_fn {
        result.push(Property::TransitionTimingFunction(tf));
    }
    if let Some(delay) = times_found.get(1) {
        result.push(Property::TransitionDelay(*delay));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_display() {
        let result = parse_declaration("display", "flex");
        assert_eq!(result, vec![Property::Display(Display::Flex)]);
    }

    #[test]
    fn parse_margin_shorthand_1() {
        let result = parse_declaration("margin", "10px");
        assert_eq!(result.len(), 4);
        assert_eq!(result[0], Property::MarginTop(LengthValue::Px(10.0)));
    }

    #[test]
    fn parse_margin_shorthand_2() {
        let result = parse_declaration("margin", "10px 20px");
        assert_eq!(result[0], Property::MarginTop(LengthValue::Px(10.0)));
        assert_eq!(result[1], Property::MarginRight(LengthValue::Px(20.0)));
        assert_eq!(result[2], Property::MarginBottom(LengthValue::Px(10.0)));
        assert_eq!(result[3], Property::MarginLeft(LengthValue::Px(20.0)));
    }

    #[test]
    fn parse_border_shorthand() {
        let result = parse_declaration("border", "1px solid black");
        // Should produce 12 properties: 4 width + 4 style + 4 color
        assert_eq!(result.len(), 12);
    }

    #[test]
    fn parse_font_size_keyword() {
        let result = parse_declaration("font-size", "large");
        assert_eq!(result, vec![Property::FontSize(LengthValue::Px(18.0))]);
    }

    #[test]
    fn parse_color_property() {
        let result = parse_declaration("color", "red");
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn parse_opacity() {
        let result = parse_declaration("opacity", "0.5");
        assert_eq!(result, vec![Property::Opacity(0.5)]);
    }

    #[test]
    fn parse_z_index() {
        let result = parse_declaration("z-index", "10");
        assert_eq!(result, vec![Property::ZIndex(10)]);
    }

    #[test]
    fn unknown_property_returns_empty() {
        let result = parse_declaration("banana", "yellow");
        assert!(result.is_empty());
    }

    #[test]
    fn property_name_round_trip() {
        let p = Property::Display(Display::Block);
        assert_eq!(p.name(), "display");
        assert!(!p.inherits());

        let c = Property::Color(ColorValue::Inherit);
        assert!(c.inherits());
    }
}

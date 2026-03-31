// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! CSS Transform and Filter value types.
//!
//! Implements CSS `transform` functions and `filter` functions for
//! use in the style and layout pipeline.

// ── Transform ────────────────────────────────────────────────────────────────

/// A single CSS transform function.
#[derive(Debug, Clone, PartialEq)]
pub enum TransformFunction {
    /// `translate(x, y)` in pixels
    Translate(f32, f32),
    /// `translateX(x)` in pixels
    TranslateX(f32),
    /// `translateY(y)` in pixels
    TranslateY(f32),
    /// `scale(sx, sy)`
    Scale(f32, f32),
    /// `scaleX(sx)`
    ScaleX(f32),
    /// `scaleY(sy)`
    ScaleY(f32),
    /// `rotate(angle)` in degrees
    Rotate(f32),
    /// `skew(ax, ay)` in degrees
    Skew(f32, f32),
    /// `skewX(angle)` in degrees
    SkewX(f32),
    /// `skewY(angle)` in degrees
    SkewY(f32),
    /// `matrix(a, b, c, d, e, f)`
    Matrix(f32, f32, f32, f32, f32, f32),
}

impl TransformFunction {
    /// Parse a transform function from a string like `translate(10px, 20px)`.
    pub fn parse(input: &str) -> Option<Self> {
        let input = input.trim();
        let paren = input.find('(')?;
        let name = input[..paren].trim();
        let args_str = input[paren + 1..].strip_suffix(')')?.trim();

        match name {
            "translate" => {
                let parts = parse_values(args_str, 2)?;
                Some(Self::Translate(parts[0], parts.get(1).copied().unwrap_or(0.0)))
            }
            "translateX" => {
                let v = parse_single_value(args_str)?;
                Some(Self::TranslateX(v))
            }
            "translateY" => {
                let v = parse_single_value(args_str)?;
                Some(Self::TranslateY(v))
            }
            "scale" => {
                let parts = parse_values(args_str, 2)?;
                let sx = parts[0];
                let sy = parts.get(1).copied().unwrap_or(sx);
                Some(Self::Scale(sx, sy))
            }
            "scaleX" => {
                let v = parse_single_value(args_str)?;
                Some(Self::ScaleX(v))
            }
            "scaleY" => {
                let v = parse_single_value(args_str)?;
                Some(Self::ScaleY(v))
            }
            "rotate" => {
                let v = parse_angle(args_str)?;
                Some(Self::Rotate(v))
            }
            "skew" => {
                let parts = parse_angles(args_str, 2)?;
                Some(Self::Skew(parts[0], parts.get(1).copied().unwrap_or(0.0)))
            }
            "skewX" => {
                let v = parse_angle(args_str)?;
                Some(Self::SkewX(v))
            }
            "skewY" => {
                let v = parse_angle(args_str)?;
                Some(Self::SkewY(v))
            }
            "matrix" => {
                let parts = parse_values(args_str, 6)?;
                if parts.len() == 6 {
                    Some(Self::Matrix(
                        parts[0], parts[1], parts[2], parts[3], parts[4], parts[5],
                    ))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Convert to a 2D affine matrix [a, b, c, d, tx, ty].
    pub fn to_matrix(&self) -> [f32; 6] {
        match self {
            Self::Translate(x, y) => [1.0, 0.0, 0.0, 1.0, *x, *y],
            Self::TranslateX(x) => [1.0, 0.0, 0.0, 1.0, *x, 0.0],
            Self::TranslateY(y) => [1.0, 0.0, 0.0, 1.0, 0.0, *y],
            Self::Scale(sx, sy) => [*sx, 0.0, 0.0, *sy, 0.0, 0.0],
            Self::ScaleX(sx) => [*sx, 0.0, 0.0, 1.0, 0.0, 0.0],
            Self::ScaleY(sy) => [1.0, 0.0, 0.0, *sy, 0.0, 0.0],
            Self::Rotate(deg) => {
                let rad = deg.to_radians();
                let c = rad.cos();
                let s = rad.sin();
                [c, s, -s, c, 0.0, 0.0]
            }
            Self::Skew(ax, ay) => {
                let tx = ax.to_radians().tan();
                let ty = ay.to_radians().tan();
                [1.0, ty, tx, 1.0, 0.0, 0.0]
            }
            Self::SkewX(a) => [1.0, 0.0, a.to_radians().tan(), 1.0, 0.0, 0.0],
            Self::SkewY(a) => [1.0, a.to_radians().tan(), 0.0, 1.0, 0.0, 0.0],
            Self::Matrix(a, b, c, d, e, f) => [*a, *b, *c, *d, *e, *f],
        }
    }
}

/// A list of transform functions applied in order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TransformList {
    pub functions: Vec<TransformFunction>,
}

impl TransformList {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse a CSS transform value like `translate(10px) rotate(45deg) scale(2)`.
    pub fn parse(input: &str) -> Self {
        let input = input.trim();
        if input == "none" || input.is_empty() {
            return Self::new();
        }

        let mut functions = Vec::new();
        let mut rest = input;

        while !rest.is_empty() {
            // Find the next function
            if let Some(paren_open) = rest.find('(') {
                if let Some(paren_close) = rest[paren_open..].find(')') {
                    let end = paren_open + paren_close + 1;
                    let func_str = &rest[..end];
                    if let Some(f) = TransformFunction::parse(func_str) {
                        functions.push(f);
                    }
                    rest = rest[end..].trim_start();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        Self { functions }
    }

    /// Compose all transforms into a single 2D affine matrix.
    pub fn to_matrix(&self) -> [f32; 6] {
        let mut result = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        for f in &self.functions {
            let m = f.to_matrix();
            result = multiply_matrix(result, m);
        }
        result
    }

    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }
}

/// Multiply two 2D affine matrices: [a, b, c, d, tx, ty].
fn multiply_matrix(a: [f32; 6], b: [f32; 6]) -> [f32; 6] {
    [
        a[0] * b[0] + a[2] * b[1],
        a[1] * b[0] + a[3] * b[1],
        a[0] * b[2] + a[2] * b[3],
        a[1] * b[2] + a[3] * b[3],
        a[0] * b[4] + a[2] * b[5] + a[4],
        a[1] * b[4] + a[3] * b[5] + a[5],
    ]
}

// ── Filter ───────────────────────────────────────────────────────────────────

/// A single CSS filter function.
#[derive(Debug, Clone, PartialEq)]
pub enum FilterFunction {
    /// `blur(radius)` in pixels
    Blur(f32),
    /// `brightness(amount)` — 1.0 = no change
    Brightness(f32),
    /// `contrast(amount)` — 1.0 = no change
    Contrast(f32),
    /// `grayscale(amount)` — 0.0-1.0
    Grayscale(f32),
    /// `invert(amount)` — 0.0-1.0
    Invert(f32),
    /// `opacity(amount)` — 0.0-1.0
    Opacity(f32),
    /// `saturate(amount)` — 1.0 = no change
    Saturate(f32),
    /// `sepia(amount)` — 0.0-1.0
    Sepia(f32),
    /// `hue-rotate(angle)` in degrees
    HueRotate(f32),
    /// `drop-shadow(x, y, blur, color)`
    DropShadow(f32, f32, f32, String),
}

impl FilterFunction {
    /// Parse a filter function from a string.
    pub fn parse(input: &str) -> Option<Self> {
        let input = input.trim();
        let paren = input.find('(')?;
        let name = input[..paren].trim();
        let args_str = input[paren + 1..].strip_suffix(')')?.trim();

        match name {
            "blur" => {
                let v = parse_single_value(args_str)?;
                Some(Self::Blur(v))
            }
            "brightness" => {
                let v = parse_percent_or_number(args_str)?;
                Some(Self::Brightness(v))
            }
            "contrast" => {
                let v = parse_percent_or_number(args_str)?;
                Some(Self::Contrast(v))
            }
            "grayscale" => {
                let v = parse_percent_or_number(args_str)?;
                Some(Self::Grayscale(v.clamp(0.0, 1.0)))
            }
            "invert" => {
                let v = parse_percent_or_number(args_str)?;
                Some(Self::Invert(v.clamp(0.0, 1.0)))
            }
            "opacity" => {
                let v = parse_percent_or_number(args_str)?;
                Some(Self::Opacity(v.clamp(0.0, 1.0)))
            }
            "saturate" => {
                let v = parse_percent_or_number(args_str)?;
                Some(Self::Saturate(v))
            }
            "sepia" => {
                let v = parse_percent_or_number(args_str)?;
                Some(Self::Sepia(v.clamp(0.0, 1.0)))
            }
            "hue-rotate" => {
                let v = parse_angle(args_str)?;
                Some(Self::HueRotate(v))
            }
            "drop-shadow" => {
                let parts: Vec<&str> = args_str.split_whitespace().collect();
                if parts.len() >= 2 {
                    let x = parse_single_value(parts[0])?;
                    let y = parse_single_value(parts[1])?;
                    let blur = parts.get(2).and_then(|s| parse_single_value(s)).unwrap_or(0.0);
                    let color = parts.get(3).map(|s| s.to_string()).unwrap_or_default();
                    Some(Self::DropShadow(x, y, blur, color))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// A list of filter functions.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FilterList {
    pub functions: Vec<FilterFunction>,
}

impl FilterList {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse a CSS filter value like `blur(5px) brightness(1.2)`.
    pub fn parse(input: &str) -> Self {
        let input = input.trim();
        if input == "none" || input.is_empty() {
            return Self::new();
        }

        let mut functions = Vec::new();
        let mut rest = input;

        while !rest.is_empty() {
            if let Some(paren_open) = rest.find('(') {
                if let Some(paren_close) = rest[paren_open..].find(')') {
                    let end = paren_open + paren_close + 1;
                    let func_str = &rest[..end];
                    if let Some(f) = FilterFunction::parse(func_str) {
                        functions.push(f);
                    }
                    rest = rest[end..].trim_start();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        Self { functions }
    }

    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }
}

// ── TransformOrigin ──────────────────────────────────────────────────────────

/// CSS `transform-origin` value.
#[derive(Debug, Clone, PartialEq)]
pub struct TransformOrigin {
    pub x: f32, // percentage or pixel
    pub y: f32, // percentage or pixel
}

impl Default for TransformOrigin {
    fn default() -> Self {
        Self { x: 50.0, y: 50.0 } // center (percent)
    }
}

impl TransformOrigin {
    pub fn parse(input: &str) -> Self {
        let parts: Vec<&str> = input.split_whitespace().collect();
        let x = parts
            .first()
            .and_then(|p| parse_origin_value(p))
            .unwrap_or(50.0);
        let y = parts
            .get(1)
            .and_then(|p| parse_origin_value(p))
            .unwrap_or(50.0);
        Self { x, y }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn parse_single_value(s: &str) -> Option<f32> {
    let s = s.trim();
    s.strip_suffix("px")
        .or(Some(s))
        .and_then(|v| v.trim().parse::<f32>().ok())
}

fn parse_values(s: &str, max: usize) -> Option<Vec<f32>> {
    let parts: Vec<f32> = s
        .split(',')
        .take(max)
        .filter_map(|p| parse_single_value(p.trim()))
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts)
    }
}

fn parse_angle(s: &str) -> Option<f32> {
    let s = s.trim();
    if let Some(v) = s.strip_suffix("deg") {
        v.trim().parse().ok()
    } else if let Some(v) = s.strip_suffix("rad") {
        v.trim().parse::<f32>().ok().map(|r| r.to_degrees())
    } else if let Some(v) = s.strip_suffix("turn") {
        v.trim().parse::<f32>().ok().map(|t| t * 360.0)
    } else {
        s.parse().ok()
    }
}

fn parse_angles(s: &str, max: usize) -> Option<Vec<f32>> {
    let parts: Vec<f32> = s
        .split(',')
        .take(max)
        .filter_map(|p| parse_angle(p.trim()))
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts)
    }
}

fn parse_percent_or_number(s: &str) -> Option<f32> {
    let s = s.trim();
    if let Some(v) = s.strip_suffix('%') {
        v.trim().parse::<f32>().ok().map(|p| p / 100.0)
    } else {
        s.parse().ok()
    }
}

fn parse_origin_value(s: &str) -> Option<f32> {
    match s {
        "left" => Some(0.0),
        "center" => Some(50.0),
        "right" => Some(100.0),
        "top" => Some(0.0),
        "bottom" => Some(100.0),
        _ => {
            if let Some(v) = s.strip_suffix('%') {
                v.parse().ok()
            } else if let Some(v) = s.strip_suffix("px") {
                v.parse().ok()
            } else {
                s.parse().ok()
            }
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Transform parsing ───────────────────────────────────────

    #[test]
    fn parse_translate() {
        let f = TransformFunction::parse("translate(10px, 20px)").unwrap();
        assert_eq!(f, TransformFunction::Translate(10.0, 20.0));
    }

    #[test]
    fn parse_translate_single() {
        let f = TransformFunction::parse("translate(10px)").unwrap();
        assert_eq!(f, TransformFunction::Translate(10.0, 0.0));
    }

    #[test]
    fn parse_scale() {
        let f = TransformFunction::parse("scale(2)").unwrap();
        assert_eq!(f, TransformFunction::Scale(2.0, 2.0));
    }

    #[test]
    fn parse_scale_xy() {
        let f = TransformFunction::parse("scale(1.5, 2.0)").unwrap();
        assert_eq!(f, TransformFunction::Scale(1.5, 2.0));
    }

    #[test]
    fn parse_rotate() {
        let f = TransformFunction::parse("rotate(45deg)").unwrap();
        assert_eq!(f, TransformFunction::Rotate(45.0));
    }

    #[test]
    fn parse_rotate_rad() {
        let f = TransformFunction::parse("rotate(3.14159rad)").unwrap();
        assert!((f.to_matrix()[0] - (-1.0)).abs() < 0.01); // cos(π) ≈ -1
    }

    #[test]
    fn parse_skew() {
        let f = TransformFunction::parse("skew(30deg, 10deg)").unwrap();
        assert_eq!(f, TransformFunction::Skew(30.0, 10.0));
    }

    #[test]
    fn parse_matrix() {
        let f = TransformFunction::parse("matrix(1, 0, 0, 1, 50, 100)").unwrap();
        assert_eq!(f, TransformFunction::Matrix(1.0, 0.0, 0.0, 1.0, 50.0, 100.0));
    }

    #[test]
    fn parse_transform_list() {
        let t = TransformList::parse("translate(10px, 20px) rotate(45deg) scale(2)");
        assert_eq!(t.functions.len(), 3);
    }

    #[test]
    fn transform_list_none() {
        let t = TransformList::parse("none");
        assert!(t.is_empty());
    }

    #[test]
    fn identity_matrix() {
        let t = TransformList::new();
        let m = t.to_matrix();
        assert_eq!(m, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    }

    #[test]
    fn translate_matrix() {
        let f = TransformFunction::Translate(10.0, 20.0);
        let m = f.to_matrix();
        assert_eq!(m[4], 10.0);
        assert_eq!(m[5], 20.0);
    }

    #[test]
    fn scale_matrix() {
        let f = TransformFunction::Scale(2.0, 3.0);
        let m = f.to_matrix();
        assert_eq!(m[0], 2.0);
        assert_eq!(m[3], 3.0);
    }

    #[test]
    fn compose_transforms() {
        let t = TransformList::parse("translate(100px, 0px) scale(2)");
        let m = t.to_matrix();
        // After translate(100,0) then scale(2):
        // [2, 0, 0, 2, 100, 0]
        assert!((m[0] - 2.0).abs() < 0.01);
        assert!((m[4] - 100.0).abs() < 0.01);
    }

    // ── Filter parsing ──────────────────────────────────────────

    #[test]
    fn parse_blur() {
        let f = FilterFunction::parse("blur(5px)").unwrap();
        assert_eq!(f, FilterFunction::Blur(5.0));
    }

    #[test]
    fn parse_brightness() {
        let f = FilterFunction::parse("brightness(150%)").unwrap();
        assert_eq!(f, FilterFunction::Brightness(1.5));
    }

    #[test]
    fn parse_grayscale() {
        let f = FilterFunction::parse("grayscale(100%)").unwrap();
        assert_eq!(f, FilterFunction::Grayscale(1.0));
    }

    #[test]
    fn parse_hue_rotate() {
        let f = FilterFunction::parse("hue-rotate(90deg)").unwrap();
        assert_eq!(f, FilterFunction::HueRotate(90.0));
    }

    #[test]
    fn parse_drop_shadow() {
        let f = FilterFunction::parse("drop-shadow(2px 4px 6px black)").unwrap();
        assert_eq!(f, FilterFunction::DropShadow(2.0, 4.0, 6.0, "black".to_string()));
    }

    #[test]
    fn parse_filter_list() {
        let f = FilterList::parse("blur(5px) brightness(1.2) contrast(0.8)");
        assert_eq!(f.functions.len(), 3);
    }

    #[test]
    fn filter_list_none() {
        let f = FilterList::parse("none");
        assert!(f.is_empty());
    }

    #[test]
    fn filter_clamping() {
        let f = FilterFunction::parse("grayscale(200%)").unwrap();
        assert_eq!(f, FilterFunction::Grayscale(1.0)); // Clamped to 1.0
    }

    // ── TransformOrigin ─────────────────────────────────────────

    #[test]
    fn default_origin() {
        let o = TransformOrigin::default();
        assert_eq!(o.x, 50.0);
        assert_eq!(o.y, 50.0);
    }

    #[test]
    fn parse_origin_keywords() {
        let o = TransformOrigin::parse("left top");
        assert_eq!(o.x, 0.0);
        assert_eq!(o.y, 0.0);

        let o = TransformOrigin::parse("right bottom");
        assert_eq!(o.x, 100.0);
        assert_eq!(o.y, 100.0);
    }

    #[test]
    fn parse_origin_percent() {
        let o = TransformOrigin::parse("25% 75%");
        assert_eq!(o.x, 25.0);
        assert_eq!(o.y, 75.0);
    }

    #[test]
    fn parse_angle_turn() {
        let a = parse_angle("0.5turn").unwrap();
        assert!((a - 180.0).abs() < 0.01);
    }
}

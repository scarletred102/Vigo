// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! CSS color values including named colors, hex, rgb(), rgba(), hsl(), hsla().

use vex_core::Color;

/// A CSS color value.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ColorValue {
    Rgba(Color),
    CurrentColor,
    #[default]
    Inherit,
    Transparent,
}

impl ColorValue {
    /// Resolve to a concrete Color, given the inherited `current_color`.
    pub fn resolve(&self, current_color: Color) -> Color {
        match self {
            Self::Rgba(c) => *c,
            Self::CurrentColor => current_color,
            Self::Inherit => current_color,
            Self::Transparent => Color::TRANSPARENT,
        }
    }
}



/// Parse a CSS color string.
pub fn parse_color(input: &str) -> Option<ColorValue> {
    let input = input.trim();
    let lower = input.to_ascii_lowercase();

    match lower.as_str() {
        "transparent" => return Some(ColorValue::Transparent),
        "currentcolor" | "currentColor" => return Some(ColorValue::CurrentColor),
        "inherit" => return Some(ColorValue::Inherit),
        _ => {}
    }

    // Named colors
    if let Some(c) = parse_named_color(&lower) {
        return Some(ColorValue::Rgba(c));
    }

    // Hex
    if lower.starts_with('#') {
        if let Ok(c) = Color::from_hex(&lower) {
            return Some(ColorValue::Rgba(c));
        }
    }

    // rgb()/rgba()
    if let Some(c) = parse_rgb_function(&lower) {
        return Some(ColorValue::Rgba(c));
    }

    // hsl()/hsla()
    if let Some(c) = parse_hsl_function(&lower) {
        return Some(ColorValue::Rgba(c));
    }

    None
}

fn parse_rgb_function(input: &str) -> Option<Color> {
    let inner = if let Some(s) = input.strip_prefix("rgba(") {
        s.strip_suffix(')')
    } else if let Some(s) = input.strip_prefix("rgb(") {
        s.strip_suffix(')')
    } else {
        None
    }?;

    let parts: Vec<&str> = inner.split([',', '/']).map(str::trim).collect();
    if parts.len() < 3 {
        return None;
    }

    let r = parse_color_component(parts[0], 255.0)?;
    let g = parse_color_component(parts[1], 255.0)?;
    let b = parse_color_component(parts[2], 255.0)?;
    let a = if parts.len() >= 4 {
        parse_alpha_component(parts[3])?
    } else {
        255
    };

    Some(Color { r, g, b, a })
}

fn parse_hsl_function(input: &str) -> Option<Color> {
    let inner = if let Some(s) = input.strip_prefix("hsla(") {
        s.strip_suffix(')')
    } else if let Some(s) = input.strip_prefix("hsl(") {
        s.strip_suffix(')')
    } else {
        None
    }?;

    let parts: Vec<&str> = inner.split([',', '/']).map(str::trim).collect();
    if parts.len() < 3 {
        return None;
    }

    let h: f32 = parts[0].strip_suffix("deg").unwrap_or(parts[0]).parse().ok()?;
    let s_raw: f32 = parts[1].strip_suffix('%').unwrap_or(parts[1]).parse().ok()?;
    let s = s_raw / 100.0;
    let l_raw: f32 = parts[2].strip_suffix('%').unwrap_or(parts[2]).parse().ok()?;
    let l = l_raw / 100.0;
    let a = if parts.len() >= 4 {
        parse_alpha_component(parts[3])?
    } else {
        255
    };

    let (r, g, b) = hsl_to_rgb(h, s, l);
    Some(Color { r, g, b, a })
}

fn parse_color_component(s: &str, max: f32) -> Option<u8> {
    if let Some(pct) = s.strip_suffix('%') {
        let v: f32 = pct.trim().parse().ok()?;
        Some((v / 100.0 * max).round().clamp(0.0, 255.0) as u8)
    } else {
        let v: f32 = s.parse().ok()?;
        Some(v.round().clamp(0.0, 255.0) as u8)
    }
}

fn parse_alpha_component(s: &str) -> Option<u8> {
    if let Some(pct) = s.strip_suffix('%') {
        let v: f32 = pct.trim().parse().ok()?;
        Some((v / 100.0 * 255.0).round().clamp(0.0, 255.0) as u8)
    } else {
        let v: f32 = s.parse().ok()?;
        if v <= 1.0 {
            Some((v * 255.0).round().clamp(0.0, 255.0) as u8)
        } else {
            Some(v.round().clamp(0.0, 255.0) as u8)
        }
    }
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let h = ((h % 360.0) + 360.0) % 360.0;
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r1, g1, b1) = match h as u32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    (
        ((r1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

/// Parse a CSS named color. Supports 148 CSS named colors.
fn parse_named_color(name: &str) -> Option<Color> {
    // Most common colors + full CSS4 named colors
    let (r, g, b) = match name {
        "black" => (0, 0, 0),
        "white" => (255, 255, 255),
        "red" => (255, 0, 0),
        "green" => (0, 128, 0),
        "blue" => (0, 0, 255),
        "yellow" => (255, 255, 0),
        "cyan" | "aqua" => (0, 255, 255),
        "magenta" | "fuchsia" => (255, 0, 255),
        "silver" => (192, 192, 192),
        "gray" | "grey" => (128, 128, 128),
        "maroon" => (128, 0, 0),
        "olive" => (128, 128, 0),
        "lime" => (0, 255, 0),
        "teal" => (0, 128, 128),
        "navy" => (0, 0, 128),
        "purple" => (128, 0, 128),
        "orange" => (255, 165, 0),
        "pink" => (255, 192, 203),
        "brown" => (165, 42, 42),
        "coral" => (255, 127, 80),
        "crimson" => (220, 20, 60),
        "darkblue" => (0, 0, 139),
        "darkgreen" => (0, 100, 0),
        "darkred" => (139, 0, 0),
        "darkgray" | "darkgrey" => (169, 169, 169),
        "darkviolet" => (148, 0, 211),
        "deeppink" => (255, 20, 147),
        "deepskyblue" => (0, 191, 255),
        "dodgerblue" => (30, 144, 255),
        "firebrick" => (178, 34, 34),
        "gold" => (255, 215, 0),
        "goldenrod" => (218, 165, 32),
        "hotpink" => (255, 105, 180),
        "indianred" => (205, 92, 92),
        "indigo" => (75, 0, 130),
        "ivory" => (255, 255, 240),
        "khaki" => (240, 230, 140),
        "lavender" => (230, 230, 250),
        "lightblue" => (173, 216, 230),
        "lightcoral" => (240, 128, 128),
        "lightgray" | "lightgrey" => (211, 211, 211),
        "lightgreen" => (144, 238, 144),
        "lightyellow" => (255, 255, 224),
        "limegreen" => (50, 205, 50),
        "linen" => (250, 240, 230),
        "mediumblue" => (0, 0, 205),
        "midnightblue" => (25, 25, 112),
        "mintcream" => (245, 255, 250),
        "orchid" => (218, 112, 214),
        "orangered" => (255, 69, 0),
        "palegreen" => (152, 251, 152),
        "plum" => (221, 160, 221),
        "powderblue" => (176, 224, 230),
        "rosybrown" => (188, 143, 143),
        "royalblue" => (65, 105, 225),
        "salmon" => (250, 128, 114),
        "sandybrown" => (244, 164, 96),
        "seagreen" => (46, 139, 87),
        "sienna" => (160, 82, 45),
        "skyblue" => (135, 206, 235),
        "slateblue" => (106, 90, 205),
        "slategray" | "slategrey" => (112, 128, 144),
        "snow" => (255, 250, 250),
        "springgreen" => (0, 255, 127),
        "steelblue" => (70, 130, 180),
        "tan" => (210, 180, 140),
        "thistle" => (216, 191, 216),
        "tomato" => (255, 99, 71),
        "turquoise" => (64, 224, 208),
        "violet" => (238, 130, 238),
        "wheat" => (245, 222, 179),
        "whitesmoke" => (245, 245, 245),
        "yellowgreen" => (154, 205, 50),
        "aliceblue" => (240, 248, 255),
        "antiquewhite" => (250, 235, 215),
        "aquamarine" => (127, 255, 212),
        "azure" => (240, 255, 255),
        "beige" => (245, 245, 220),
        "bisque" => (255, 228, 196),
        "blanchedalmond" => (255, 235, 205),
        "blueviolet" => (138, 43, 226),
        "burlywood" => (222, 184, 135),
        "cadetblue" => (95, 158, 160),
        "chartreuse" => (127, 255, 0),
        "chocolate" => (210, 105, 30),
        "cornflowerblue" => (100, 149, 237),
        "cornsilk" => (255, 248, 220),
        "darkcyan" => (0, 139, 139),
        "darkgoldenrod" => (184, 134, 11),
        "darkkhaki" => (189, 183, 107),
        "darkmagenta" => (139, 0, 139),
        "darkolivegreen" => (85, 107, 47),
        "darkorange" => (255, 140, 0),
        "darkorchid" => (153, 50, 204),
        "darksalmon" => (233, 150, 122),
        "darkseagreen" => (143, 188, 143),
        "darkslateblue" => (72, 61, 139),
        "darkslategray" | "darkslategrey" => (47, 79, 79),
        "darkturquoise" => (0, 206, 209),
        "dimgray" | "dimgrey" => (105, 105, 105),
        "floralwhite" => (255, 250, 240),
        "forestgreen" => (34, 139, 34),
        "gainsboro" => (220, 220, 220),
        "ghostwhite" => (248, 248, 255),
        "greenyellow" => (173, 255, 47),
        "honeydew" => (240, 255, 240),
        "lavenderblush" => (255, 240, 245),
        "lawngreen" => (124, 252, 0),
        "lemonchiffon" => (255, 250, 205),
        "lightcyan" => (224, 255, 255),
        "lightgoldenrodyellow" => (250, 250, 210),
        "lightpink" => (255, 182, 193),
        "lightsalmon" => (255, 160, 122),
        "lightseagreen" => (32, 178, 170),
        "lightskyblue" => (135, 206, 250),
        "lightslategray" | "lightslategrey" => (119, 136, 153),
        "lightsteelblue" => (176, 196, 222),
        "mediumaquamarine" => (102, 205, 170),
        "mediumorchid" => (186, 85, 211),
        "mediumpurple" => (147, 112, 219),
        "mediumseagreen" => (60, 179, 113),
        "mediumslateblue" => (123, 104, 238),
        "mediumspringgreen" => (0, 250, 154),
        "mediumturquoise" => (72, 209, 204),
        "mediumvioletred" => (199, 21, 133),
        "mistyrose" => (255, 228, 225),
        "moccasin" => (255, 228, 181),
        "navajowhite" => (255, 222, 173),
        "oldlace" => (253, 245, 230),
        "olivedrab" => (107, 142, 35),
        "palegoldenrod" => (238, 232, 170),
        "paleturquoise" => (175, 238, 238),
        "palevioletred" => (219, 112, 147),
        "papayawhip" => (255, 239, 213),
        "peachpuff" => (255, 218, 185),
        "peru" => (205, 133, 63),
        "rebeccapurple" => (102, 51, 153),
        "seashell" => (255, 245, 238),
        _ => return None,
    };

    Some(Color { r, g, b, a: 255 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_colors() {
        assert_eq!(parse_color("#ff0000"), Some(ColorValue::Rgba(Color { r: 255, g: 0, b: 0, a: 255 })));
        assert_eq!(parse_color("#00ff00"), Some(ColorValue::Rgba(Color { r: 0, g: 255, b: 0, a: 255 })));
        assert_eq!(parse_color("#0000ff80"), Some(ColorValue::Rgba(Color { r: 0, g: 0, b: 255, a: 128 })));
    }

    #[test]
    fn parse_short_hex() {
        assert_eq!(parse_color("#fff"), Some(ColorValue::Rgba(Color { r: 255, g: 255, b: 255, a: 255 })));
    }

    #[test]
    fn parse_named_colors() {
        assert_eq!(parse_color("red"), Some(ColorValue::Rgba(Color { r: 255, g: 0, b: 0, a: 255 })));
        assert_eq!(parse_color("blue"), Some(ColorValue::Rgba(Color { r: 0, g: 0, b: 255, a: 255 })));
        assert_eq!(parse_color("rebeccapurple"), Some(ColorValue::Rgba(Color { r: 102, g: 51, b: 153, a: 255 })));
    }

    #[test]
    fn parse_rgb_function() {
        assert_eq!(parse_color("rgb(255, 0, 0)"), Some(ColorValue::Rgba(Color { r: 255, g: 0, b: 0, a: 255 })));
    }

    #[test]
    fn parse_rgba_function() {
        assert_eq!(parse_color("rgba(255, 0, 0, 0.5)"), Some(ColorValue::Rgba(Color { r: 255, g: 0, b: 0, a: 128 })));
    }

    #[test]
    fn parse_hsl() {
        // hsl(0, 100%, 50%) = pure red
        let result = parse_color("hsl(0, 100%, 50%)");
        if let Some(ColorValue::Rgba(c)) = result {
            assert_eq!(c.r, 255);
            assert!(c.g <= 1); // rounding
            assert!(c.b <= 1);
        } else {
            panic!("Expected Rgba");
        }
    }

    #[test]
    fn parse_keywords() {
        assert_eq!(parse_color("transparent"), Some(ColorValue::Transparent));
        assert_eq!(parse_color("currentcolor"), Some(ColorValue::CurrentColor));
        assert_eq!(parse_color("inherit"), Some(ColorValue::Inherit));
    }

    #[test]
    fn resolve_current_color() {
        let c = ColorValue::CurrentColor;
        let parent = Color { r: 100, g: 50, b: 200, a: 255 };
        assert_eq!(c.resolve(parent), parent);
    }

    #[test]
    fn resolve_transparent() {
        let c = ColorValue::Transparent;
        assert_eq!(c.resolve(Color::BLACK), Color::TRANSPARENT);
    }

    #[test]
    fn parse_rgb_percent() {
        assert_eq!(
            parse_color("rgb(100%, 0%, 50%)"),
            Some(ColorValue::Rgba(Color { r: 255, g: 0, b: 128, a: 255 }))
        );
    }

    #[test]
    fn unknown_color_returns_none() {
        assert_eq!(parse_color("notacolor"), None);
    }

    #[test]
    fn parse_hsla() {
        let result = parse_color("hsla(120, 100%, 50%, 0.5)");
        if let Some(ColorValue::Rgba(c)) = result {
            assert!(c.g > 250); // green
            assert_eq!(c.a, 128); // 0.5 alpha
        } else {
            panic!("Expected Rgba");
        }
    }
}

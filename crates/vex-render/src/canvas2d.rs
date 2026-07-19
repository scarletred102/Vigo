// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Canvas 2D rendering context.
//!
//! Implements a subset of the HTML Canvas 2D API:
//! - Rectangle drawing: `fillRect`, `strokeRect`, `clearRect`
//! - Path drawing: `beginPath`, `moveTo`, `lineTo`, `closePath`, `fill`, `stroke`
//! - Arc/circle: `arc`
//! - Style: `fillStyle`, `strokeStyle`, `lineWidth`, `globalAlpha`
//! - Text: `fillText`, `measureText`
//! - Transform: `save`, `restore`, `translate`, `scale`, `rotate`
//! - Pixel manipulation: `getImageData`, `putImageData`
//!
//! The context operates on a CPU-side RGBA pixel buffer that can be
//! uploaded to a GPU texture for compositing.

use vex_core::Color;

/// A 2D canvas rendering context backed by an RGBA pixel buffer.
#[derive(Debug)]
pub struct Canvas2D {
    /// Width in pixels.
    width: u32,
    /// Height in pixels.
    height: u32,
    /// RGBA pixel data (row-major, 4 bytes per pixel).
    pixels: Vec<u8>,
    /// Current fill style color.
    fill_style: Color,
    /// Current stroke style color.
    stroke_style: Color,
    /// Current line width.
    line_width: f32,
    /// Global alpha (0.0 – 1.0).
    global_alpha: f32,
    /// Current path segments.
    path: Vec<PathSegment>,
    /// Current pen position.
    pen: (f32, f32),
    /// Transform stack.
    state_stack: Vec<CanvasState>,
    /// Current transform matrix [a, b, c, d, e, f] (2D affine).
    transform: [f32; 6],
}

/// A saved canvas state (push/pop via `save`/`restore`).
#[derive(Debug, Clone)]
struct CanvasState {
    fill_style: Color,
    stroke_style: Color,
    line_width: f32,
    global_alpha: f32,
    transform: [f32; 6],
}

/// A segment in the current path.
#[derive(Debug, Clone)]
enum PathSegment {
    MoveTo(f32, f32),
    LineTo(f32, f32),
    ClosePath,
    Arc {
        cx: f32,
        cy: f32,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        counter_clockwise: bool,
    },
}

/// Result of `measureText`.
#[derive(Debug, Clone, Copy)]
pub struct TextMetrics {
    /// Approximate width of the text in pixels.
    pub width: f32,
}

/// RGBA image data extracted from the canvas.
#[derive(Debug, Clone)]
pub struct ImageData {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl Canvas2D {
    /// Create a new canvas with the given dimensions, filled with transparent black.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0u8; (width * height * 4) as usize],
            fill_style: Color::BLACK,
            stroke_style: Color::BLACK,
            line_width: 1.0,
            global_alpha: 1.0,
            path: Vec::new(),
            pen: (0.0, 0.0),
            state_stack: Vec::new(),
            transform: IDENTITY,
        }
    }

    /// Canvas width.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Canvas height.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Raw pixel data (RGBA, row-major).
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Mutable pixel data access.
    pub fn pixels_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }

    // ── Style properties ─────────────────────────────────────────────

    pub fn set_fill_style(&mut self, color: Color) {
        self.fill_style = color;
    }

    pub fn fill_style(&self) -> Color {
        self.fill_style
    }

    pub fn set_stroke_style(&mut self, color: Color) {
        self.stroke_style = color;
    }

    pub fn stroke_style(&self) -> Color {
        self.stroke_style
    }

    pub fn set_line_width(&mut self, width: f32) {
        self.line_width = width.max(0.0);
    }

    pub fn line_width(&self) -> f32 {
        self.line_width
    }

    pub fn set_global_alpha(&mut self, alpha: f32) {
        self.global_alpha = alpha.clamp(0.0, 1.0);
    }

    pub fn global_alpha(&self) -> f32 {
        self.global_alpha
    }

    // ── Rectangle operations ─────────────────────────────────────────

    /// Fill a rectangle with the current fill style.
    pub fn fill_rect(&mut self, x: f32, y: f32, width: f32, height: f32) {
        let (tx0, ty0) = self.transform_point(x, y);
        let (tx1, ty1) = self.transform_point(x + width, y + height);
        let color = self.fill_color_with_alpha();

        let x0 = tx0.round().min(tx1.round()) as i32;
        let y0 = ty0.round().min(ty1.round()) as i32;
        let x1 = tx0.round().max(tx1.round()) as i32;
        let y1 = ty0.round().max(ty1.round()) as i32;

        for py in y0..y1 {
            for px in x0..x1 {
                self.blend_pixel(px, py, color);
            }
        }
    }

    /// Stroke the outline of a rectangle.
    pub fn stroke_rect(&mut self, x: f32, y: f32, width: f32, height: f32) {
        let lw = self.line_width;
        let color = self.stroke_color_with_alpha();

        // Top edge
        self.fill_rect_raw(x, y, width, lw, color);
        // Bottom edge
        self.fill_rect_raw(x, y + height - lw, width, lw, color);
        // Left edge
        self.fill_rect_raw(x, y + lw, lw, height - 2.0 * lw, color);
        // Right edge
        self.fill_rect_raw(x + width - lw, y + lw, lw, height - 2.0 * lw, color);
    }

    /// Clear a rectangle to transparent black.
    pub fn clear_rect(&mut self, x: f32, y: f32, width: f32, height: f32) {
        let (tx, ty) = self.transform_point(x, y);
        let x0 = (tx.round() as i32).max(0) as u32;
        let y0 = (ty.round() as i32).max(0) as u32;
        let x1 = ((tx + width).round() as i32).min(self.width as i32) as u32;
        let y1 = ((ty + height).round() as i32).min(self.height as i32) as u32;

        for py in y0..y1 {
            for px in x0..x1 {
                let i = ((py * self.width + px) * 4) as usize;
                if i + 3 < self.pixels.len() {
                    self.pixels[i] = 0;
                    self.pixels[i + 1] = 0;
                    self.pixels[i + 2] = 0;
                    self.pixels[i + 3] = 0;
                }
            }
        }
    }

    // ── Path operations ──────────────────────────────────────────────

    pub fn begin_path(&mut self) {
        self.path.clear();
    }

    pub fn move_to(&mut self, x: f32, y: f32) {
        self.path.push(PathSegment::MoveTo(x, y));
        self.pen = (x, y);
    }

    pub fn line_to(&mut self, x: f32, y: f32) {
        self.path.push(PathSegment::LineTo(x, y));
        self.pen = (x, y);
    }

    pub fn close_path(&mut self) {
        self.path.push(PathSegment::ClosePath);
    }

    /// Add an arc to the path.
    pub fn arc(
        &mut self,
        cx: f32,
        cy: f32,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        counter_clockwise: bool,
    ) {
        self.path.push(PathSegment::Arc {
            cx,
            cy,
            radius,
            start_angle,
            end_angle,
            counter_clockwise,
        });
    }

    /// Fill the current path with the current fill style.
    pub fn fill(&mut self) {
        let color = self.fill_color_with_alpha();
        let segments = self.path.clone();
        self.fill_path(&segments, color);
    }

    /// Stroke the current path with the current stroke style.
    pub fn stroke(&mut self) {
        let color = self.stroke_color_with_alpha();
        let segments = self.path.clone();
        let line_width = self.line_width;
        self.stroke_path(&segments, color, line_width);
    }

    // ── Text operations ──────────────────────────────────────────────

    /// Draw filled text at the given position.
    ///
    /// Uses a simple 8px-wide monospace approximation.
    pub fn fill_text(&mut self, text: &str, x: f32, y: f32) {
        let color = self.fill_color_with_alpha();
        let (tx, ty) = self.transform_point(x, y);

        // Simple monospace approximation: each char is 8px wide.
        let char_width = 8.0_f32;
        let char_height = 16.0_f32;

        for (i, _ch) in text.chars().enumerate() {
            let cx = tx + i as f32 * char_width;
            // Draw a simple glyph placeholder rectangle.
            let x0 = cx.round() as i32;
            let y0 = (ty - char_height * 0.75).round() as i32;
            let x1 = (cx + char_width * 0.8).round() as i32;
            let y1 = (ty + char_height * 0.25).round() as i32;

            for py in y0..y1 {
                for px in x0..x1 {
                    self.blend_pixel(px, py, color);
                }
            }
        }
    }

    /// Measure text width (monospace approximation).
    pub fn measure_text(&self, text: &str) -> TextMetrics {
        TextMetrics {
            width: text.chars().count() as f32 * 8.0,
        }
    }

    // ── Transform operations ─────────────────────────────────────────

    /// Save the current state (style + transform) onto the stack.
    pub fn save(&mut self) {
        self.state_stack.push(CanvasState {
            fill_style: self.fill_style,
            stroke_style: self.stroke_style,
            line_width: self.line_width,
            global_alpha: self.global_alpha,
            transform: self.transform,
        });
    }

    /// Restore the most recently saved state.
    pub fn restore(&mut self) {
        if let Some(state) = self.state_stack.pop() {
            self.fill_style = state.fill_style;
            self.stroke_style = state.stroke_style;
            self.line_width = state.line_width;
            self.global_alpha = state.global_alpha;
            self.transform = state.transform;
        }
    }

    /// Translate the canvas origin.
    pub fn translate(&mut self, tx: f32, ty: f32) {
        // Multiply current transform by translation matrix.
        self.transform[4] += self.transform[0] * tx + self.transform[2] * ty;
        self.transform[5] += self.transform[1] * tx + self.transform[3] * ty;
    }

    /// Scale the canvas.
    pub fn scale(&mut self, sx: f32, sy: f32) {
        self.transform[0] *= sx;
        self.transform[1] *= sx;
        self.transform[2] *= sy;
        self.transform[3] *= sy;
    }

    /// Rotate the canvas by the given angle (radians).
    pub fn rotate(&mut self, angle: f32) {
        let cos = angle.cos();
        let sin = angle.sin();
        let [a, b, c, d, e, f] = self.transform;
        self.transform = [
            a * cos + c * sin,
            b * cos + d * sin,
            c * cos - a * sin,
            d * cos - b * sin,
            e,
            f,
        ];
    }

    /// Reset the transform to identity.
    pub fn reset_transform(&mut self) {
        self.transform = IDENTITY;
    }

    // ── Pixel manipulation ───────────────────────────────────────────

    /// Get image data from a rectangle of the canvas.
    pub fn get_image_data(&self, x: u32, y: u32, width: u32, height: u32) -> ImageData {
        let mut data = vec![0u8; (width * height * 4) as usize];

        for row in 0..height {
            for col in 0..width {
                let src_x = x + col;
                let src_y = y + row;
                if src_x < self.width && src_y < self.height {
                    let si = ((src_y * self.width + src_x) * 4) as usize;
                    let di = ((row * width + col) * 4) as usize;
                    data[di..di + 4].copy_from_slice(&self.pixels[si..si + 4]);
                }
            }
        }

        ImageData {
            width,
            height,
            data,
        }
    }

    /// Put image data onto the canvas at the given position.
    pub fn put_image_data(&mut self, image_data: &ImageData, x: u32, y: u32) {
        for row in 0..image_data.height {
            for col in 0..image_data.width {
                let dst_x = x + col;
                let dst_y = y + row;
                if dst_x < self.width && dst_y < self.height {
                    let si = ((row * image_data.width + col) * 4) as usize;
                    let di = ((dst_y * self.width + dst_x) * 4) as usize;
                    self.pixels[di..di + 4].copy_from_slice(&image_data.data[si..si + 4]);
                }
            }
        }
    }

    // ── Internal helpers ─────────────────────────────────────────────

    /// Apply the current transform to a point.
    fn transform_point(&self, x: f32, y: f32) -> (f32, f32) {
        let [a, b, c, d, e, f] = self.transform;
        (a * x + c * y + e, b * x + d * y + f)
    }

    /// Get fill color with global alpha applied.
    fn fill_color_with_alpha(&self) -> [u8; 4] {
        color_with_alpha(self.fill_style, self.global_alpha)
    }

    /// Get stroke color with global alpha applied.
    fn stroke_color_with_alpha(&self) -> [u8; 4] {
        color_with_alpha(self.stroke_style, self.global_alpha)
    }

    /// Blend a single pixel using source-over compositing.
    fn blend_pixel(&mut self, x: i32, y: i32, color: [u8; 4]) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let i = ((y as u32 * self.width + x as u32) * 4) as usize;
        if i + 3 >= self.pixels.len() {
            return;
        }

        let sa = color[3] as f32 / 255.0;
        if sa <= 0.0 {
            return;
        }
        if sa >= 1.0 {
            self.pixels[i] = color[0];
            self.pixels[i + 1] = color[1];
            self.pixels[i + 2] = color[2];
            self.pixels[i + 3] = color[3];
            return;
        }

        // Source-over: out = src * sa + dst * (1 - sa)
        let da = 1.0 - sa;
        self.pixels[i] = (color[0] as f32 * sa + self.pixels[i] as f32 * da) as u8;
        self.pixels[i + 1] = (color[1] as f32 * sa + self.pixels[i + 1] as f32 * da) as u8;
        self.pixels[i + 2] = (color[2] as f32 * sa + self.pixels[i + 2] as f32 * da) as u8;
        self.pixels[i + 3] = ((color[3] as f32 + self.pixels[i + 3] as f32 * da).min(255.0)) as u8;
    }

    /// Fill a raw rectangle (before transform) with the given color.
    fn fill_rect_raw(&mut self, x: f32, y: f32, width: f32, height: f32, color: [u8; 4]) {
        let (tx, ty) = self.transform_point(x, y);
        let x0 = tx.round() as i32;
        let y0 = ty.round() as i32;
        let x1 = (tx + width).round() as i32;
        let y1 = (ty + height).round() as i32;

        for py in y0..y1 {
            for px in x0..x1 {
                self.blend_pixel(px, py, color);
            }
        }
    }

    /// Fill a path using a simple scanline algorithm.
    fn fill_path(&mut self, segments: &[PathSegment], color: [u8; 4]) {
        let lines = self.path_to_lines(segments);
        if lines.is_empty() {
            return;
        }

        // Find bounding box.
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        for &(_, y1, _, y2) in &lines {
            min_y = min_y.min(y1).min(y2);
            max_y = max_y.max(y1).max(y2);
        }

        let y_start = min_y.floor() as i32;
        let y_end = max_y.ceil() as i32;

        for scanline in y_start..y_end {
            let y = scanline as f32 + 0.5;
            let mut intersections = Vec::new();

            for &(x1, y1, x2, y2) in &lines {
                if (y1 <= y && y2 > y) || (y2 <= y && y1 > y) {
                    let t = (y - y1) / (y2 - y1);
                    intersections.push(x1 + t * (x2 - x1));
                }
            }

            intersections.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            for pair in intersections.chunks(2) {
                if pair.len() == 2 {
                    let x_start = pair[0].ceil() as i32;
                    let x_end = pair[1].floor() as i32;
                    for px in x_start..=x_end {
                        self.blend_pixel(px, scanline, color);
                    }
                }
            }
        }
    }

    /// Stroke a path by drawing thick lines along each segment.
    fn stroke_path(&mut self, segments: &[PathSegment], color: [u8; 4], line_width: f32) {
        let lines = self.path_to_lines(segments);
        let half = line_width / 2.0;

        for &(x1, y1, x2, y2) in &lines {
            self.draw_thick_line(x1, y1, x2, y2, half, color);
        }
    }

    /// Convert path segments to line segments (applying transform).
    fn path_to_lines(&self, segments: &[PathSegment]) -> Vec<(f32, f32, f32, f32)> {
        let mut lines = Vec::new();
        let mut first = (0.0_f32, 0.0_f32);
        let mut current = (0.0_f32, 0.0_f32);

        for seg in segments {
            match seg {
                PathSegment::MoveTo(x, y) => {
                    let p = self.transform_point(*x, *y);
                    first = p;
                    current = p;
                }
                PathSegment::LineTo(x, y) => {
                    let p = self.transform_point(*x, *y);
                    lines.push((current.0, current.1, p.0, p.1));
                    current = p;
                }
                PathSegment::ClosePath => {
                    if current != first {
                        lines.push((current.0, current.1, first.0, first.1));
                    }
                    current = first;
                }
                PathSegment::Arc {
                    cx,
                    cy,
                    radius,
                    start_angle,
                    end_angle,
                    counter_clockwise,
                } => {
                    // Approximate arc with line segments.
                    let steps = 32;
                    let mut angle_span = end_angle - start_angle;
                    if *counter_clockwise && angle_span > 0.0 {
                        angle_span -= std::f32::consts::TAU;
                    } else if !counter_clockwise && angle_span < 0.0 {
                        angle_span += std::f32::consts::TAU;
                    }

                    let mut prev = self.transform_point(
                        cx + radius * start_angle.cos(),
                        cy + radius * start_angle.sin(),
                    );

                    // First point: connect current pen to arc start.
                    if current != prev {
                        lines.push((current.0, current.1, prev.0, prev.1));
                    }

                    for i in 1..=steps {
                        let t = i as f32 / steps as f32;
                        let angle = start_angle + angle_span * t;
                        let p = self
                            .transform_point(cx + radius * angle.cos(), cy + radius * angle.sin());
                        lines.push((prev.0, prev.1, p.0, p.1));
                        prev = p;
                    }
                    current = prev;
                }
            }
        }

        lines
    }

    /// Draw a thick line using axis-aligned rectangle approximation.
    fn draw_thick_line(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        half_width: f32,
        color: [u8; 4],
    ) {
        let dx = x2 - x1;
        let dy = y2 - y1;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.001 {
            return;
        }

        // Normal perpendicular to line direction.
        let nx = -dy / len * half_width;
        let ny = dx / len * half_width;

        // Four corners of the thick line quad.
        let corners = [
            (x1 + nx, y1 + ny),
            (x1 - nx, y1 - ny),
            (x2 - nx, y2 - ny),
            (x2 + nx, y2 + ny),
        ];

        // Bounding box.
        let min_x = corners.iter().map(|c| c.0).reduce(f32::min).unwrap_or(0.0);
        let max_x = corners.iter().map(|c| c.0).reduce(f32::max).unwrap_or(0.0);
        let min_y = corners.iter().map(|c| c.1).reduce(f32::min).unwrap_or(0.0);
        let max_y = corners.iter().map(|c| c.1).reduce(f32::max).unwrap_or(0.0);

        let y_start = min_y.floor() as i32;
        let y_end = max_y.ceil() as i32;
        let x_start = min_x.floor() as i32;
        let x_end = max_x.ceil() as i32;

        // Point-in-quad test via cross products.
        for py in y_start..=y_end {
            for px in x_start..=x_end {
                let p = (px as f32 + 0.5, py as f32 + 0.5);
                if point_in_quad(p, &corners) {
                    self.blend_pixel(px, py, color);
                }
            }
        }
    }
}

// ── Free functions ───────────────────────────────────────────────────

const IDENTITY: [f32; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

fn color_with_alpha(color: Color, alpha: f32) -> [u8; 4] {
    [
        color.r,
        color.g,
        color.b,
        (color.a as f32 * alpha).round() as u8,
    ]
}

/// Check if a point is inside a convex quad (4 corners, CCW or CW).
fn point_in_quad(p: (f32, f32), corners: &[(f32, f32); 4]) -> bool {
    let mut positive = 0;
    let mut negative = 0;

    for i in 0..4 {
        let j = (i + 1) % 4;
        let cross = (corners[j].0 - corners[i].0) * (p.1 - corners[i].1)
            - (corners[j].1 - corners[i].1) * (p.0 - corners[i].0);
        if cross > 0.0 {
            positive += 1;
        } else if cross < 0.0 {
            negative += 1;
        }
    }

    positive == 0 || negative == 0
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_canvas_is_transparent() {
        let ctx = Canvas2D::new(10, 10);
        assert_eq!(ctx.width(), 10);
        assert_eq!(ctx.height(), 10);
        assert!(ctx.pixels().iter().all(|&b| b == 0));
    }

    #[test]
    fn fill_rect_basic() {
        let mut ctx = Canvas2D::new(100, 100);
        ctx.set_fill_style(Color::rgba(255, 0, 0, 255));
        ctx.fill_rect(10.0, 10.0, 5.0, 5.0);

        // Pixel at (12, 12) should be red.
        let i = ((12 * 100 + 12) * 4) as usize;
        assert_eq!(ctx.pixels()[i], 255); // R
        assert_eq!(ctx.pixels()[i + 1], 0); // G
        assert_eq!(ctx.pixels()[i + 2], 0); // B
        assert_eq!(ctx.pixels()[i + 3], 255); // A

        // Pixel at (0, 0) should still be transparent.
        assert_eq!(ctx.pixels()[0], 0);
    }

    #[test]
    fn clear_rect_erases_pixels() {
        let mut ctx = Canvas2D::new(50, 50);
        ctx.set_fill_style(Color::rgba(0, 255, 0, 255));
        ctx.fill_rect(0.0, 0.0, 50.0, 50.0);

        // Now clear a region.
        ctx.clear_rect(10.0, 10.0, 10.0, 10.0);

        // Inside cleared region: transparent.
        let i = ((15 * 50 + 15) * 4) as usize;
        assert_eq!(ctx.pixels()[i + 3], 0);

        // Outside cleared region: still green.
        let j = ((5 * 50 + 5) * 4) as usize;
        assert_eq!(ctx.pixels()[j], 0);
        assert_eq!(ctx.pixels()[j + 1], 255);
    }

    #[test]
    fn stroke_rect_draws_border() {
        let mut ctx = Canvas2D::new(100, 100);
        ctx.set_stroke_style(Color::rgba(0, 0, 255, 255));
        ctx.set_line_width(2.0);
        ctx.stroke_rect(20.0, 20.0, 40.0, 30.0);

        // Top edge: pixel at (40, 20) should be blue.
        let i = ((20 * 100 + 40) * 4) as usize;
        assert_eq!(ctx.pixels()[i + 2], 255); // B
        assert_eq!(ctx.pixels()[i + 3], 255); // A

        // Interior: pixel at (40, 35) should be transparent.
        let j = ((35 * 100 + 40) * 4) as usize;
        assert_eq!(ctx.pixels()[j + 3], 0);
    }

    #[test]
    fn global_alpha_affects_fill() {
        let mut ctx = Canvas2D::new(20, 20);
        ctx.set_fill_style(Color::rgba(255, 0, 0, 255));
        ctx.set_global_alpha(0.5);
        ctx.fill_rect(0.0, 0.0, 20.0, 20.0);

        // Alpha should be around 128 (0.5 * 255).
        let i = ((10 * 20 + 10) * 4) as usize;
        let alpha = ctx.pixels()[i + 3];
        assert!(alpha > 120 && alpha < 135, "alpha: {alpha}");
    }

    #[test]
    fn save_restore_preserves_state() {
        let mut ctx = Canvas2D::new(10, 10);
        ctx.set_fill_style(Color::rgba(255, 0, 0, 255));
        ctx.set_global_alpha(0.5);

        ctx.save();
        ctx.set_fill_style(Color::rgba(0, 255, 0, 255));
        ctx.set_global_alpha(1.0);
        assert_eq!(ctx.fill_style(), Color::rgba(0, 255, 0, 255));
        assert_eq!(ctx.global_alpha(), 1.0);

        ctx.restore();
        assert_eq!(ctx.fill_style(), Color::rgba(255, 0, 0, 255));
        assert!((ctx.global_alpha() - 0.5).abs() < 0.01);
    }

    #[test]
    fn translate_moves_origin() {
        let mut ctx = Canvas2D::new(100, 100);
        ctx.translate(50.0, 50.0);
        ctx.set_fill_style(Color::rgba(255, 0, 0, 255));
        ctx.fill_rect(0.0, 0.0, 5.0, 5.0);

        // Should be drawn at (50, 50).
        let i = ((52 * 100 + 52) * 4) as usize;
        assert_eq!(ctx.pixels()[i], 255);

        // Origin should be untouched.
        assert_eq!(ctx.pixels()[0], 0);
    }

    #[test]
    fn get_put_image_data_roundtrip() {
        let mut ctx = Canvas2D::new(50, 50);
        ctx.set_fill_style(Color::rgba(100, 200, 50, 255));
        ctx.fill_rect(5.0, 5.0, 10.0, 10.0);

        let data = ctx.get_image_data(5, 5, 10, 10);
        assert_eq!(data.width, 10);
        assert_eq!(data.height, 10);

        // Clear and put back.
        ctx.clear_rect(0.0, 0.0, 50.0, 50.0);
        ctx.put_image_data(&data, 20, 20);

        let i = ((25 * 50 + 25) * 4) as usize;
        assert_eq!(ctx.pixels()[i], 100);
        assert_eq!(ctx.pixels()[i + 1], 200);
    }

    #[test]
    fn measure_text_returns_width() {
        let ctx = Canvas2D::new(100, 100);
        let metrics = ctx.measure_text("Hello");
        assert!((metrics.width - 40.0).abs() < 0.01); // 5 chars * 8px
    }

    #[test]
    fn path_fill_triangle() {
        let mut ctx = Canvas2D::new(100, 100);
        ctx.set_fill_style(Color::rgba(0, 255, 0, 255));

        ctx.begin_path();
        ctx.move_to(50.0, 10.0);
        ctx.line_to(90.0, 90.0);
        ctx.line_to(10.0, 90.0);
        ctx.close_path();
        ctx.fill();

        // Center of triangle should be filled.
        let i = ((50 * 100 + 50) * 4) as usize;
        assert_eq!(ctx.pixels()[i + 1], 255); // Green
        assert_eq!(ctx.pixels()[i + 3], 255); // Alpha

        // Top-left corner should be empty.
        assert_eq!(ctx.pixels()[3], 0);
    }

    #[test]
    fn scale_affects_drawing() {
        let mut ctx = Canvas2D::new(100, 100);
        ctx.scale(2.0, 2.0);
        ctx.set_fill_style(Color::WHITE);
        ctx.fill_rect(0.0, 0.0, 10.0, 10.0);

        // At scale 2x, a 10x10 rect becomes 20x20.
        // Pixel at (15, 15) should be drawn.
        let i = ((15 * 100 + 15) * 4) as usize;
        assert_eq!(ctx.pixels()[i + 3], 255);

        // Pixel at (25, 25) should NOT be drawn.
        let j = ((25 * 100 + 25) * 4) as usize;
        assert_eq!(ctx.pixels()[j + 3], 0);
    }

    #[test]
    fn arc_creates_circle() {
        let mut ctx = Canvas2D::new(100, 100);
        ctx.set_fill_style(Color::rgba(255, 0, 0, 255));

        ctx.begin_path();
        ctx.arc(50.0, 50.0, 20.0, 0.0, std::f32::consts::TAU, false);
        ctx.fill();

        // Center should be filled.
        let center = ((50 * 100 + 50) * 4) as usize;
        assert_eq!(ctx.pixels()[center + 3], 255);

        // Far corner should be empty.
        assert_eq!(ctx.pixels()[3], 0);
    }
}

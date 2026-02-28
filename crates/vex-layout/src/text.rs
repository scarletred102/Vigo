// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Text measurement via `cosmic-text`.
//!
//! Provides a thin `TextEngine` wrapper around `cosmic_text::FontSystem`,
//! `Buffer`, and `Metrics` for measuring text extents and splitting lines.

use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping};

/// Wrapper around the cosmic-text font system for text measurement.
pub struct TextEngine {
    font_system: FontSystem,
}

impl TextEngine {
    /// Create a new text engine, loading system fonts.
    pub fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
        }
    }

    /// Measure the size of a text run at the given font size and max width.
    ///
    /// Returns `(width, height)` in pixels.
    pub fn measure(&mut self, text: &str, font_size: f32, line_height: f32, max_width: f32) -> (f32, f32) {
        let metrics = Metrics::new(font_size, line_height);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);

        buffer.set_size(&mut self.font_system, Some(max_width), None);
        buffer.set_text(&mut self.font_system, text, Attrs::new(), Shaping::Advanced);
        buffer.shape_until_scroll(&mut self.font_system, false);

        let mut width = 0.0_f32;
        let mut height = 0.0_f32;

        for run in buffer.layout_runs() {
            width = width.max(run.line_w);
            height += line_height;
        }

        // If no layout runs, at least one line tall
        if height == 0.0 && !text.is_empty() {
            height = line_height;
        }

        (width, height)
    }

    /// Break text into lines given a max width. Returns each line's text and width.
    pub fn break_lines(
        &mut self,
        text: &str,
        font_size: f32,
        line_height: f32,
        max_width: f32,
    ) -> Vec<LineMetrics> {
        let metrics = Metrics::new(font_size, line_height);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);

        buffer.set_size(&mut self.font_system, Some(max_width), None);
        buffer.set_text(&mut self.font_system, text, Attrs::new(), Shaping::Advanced);
        buffer.shape_until_scroll(&mut self.font_system, false);

        buffer
            .layout_runs()
            .map(|run| LineMetrics {
                width: run.line_w,
                height: line_height,
            })
            .collect()
    }
}

impl Default for TextEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Dimensions of a single line of laid-out text.
#[derive(Debug, Clone, Copy)]
pub struct LineMetrics {
    pub width: f32,
    pub height: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measure_empty_string() {
        let mut engine = TextEngine::new();
        let (w, _h) = engine.measure("", 16.0, 20.0, 500.0);
        assert_eq!(w, 0.0);
        // cosmic-text may still report line_height for empty buffers
    }

    #[test]
    fn measure_short_text_fits_one_line() {
        let mut engine = TextEngine::new();
        let (w, h) = engine.measure("Hello", 16.0, 20.0, 500.0);
        assert!(w > 0.0, "Width should be positive, got {w}");
        assert!(w <= 500.0, "Should fit in one line");
        assert!((h - 20.0).abs() < 1.0, "Should be one line tall, got {h}");
    }

    #[test]
    fn narrow_width_causes_line_wrap() {
        let mut engine = TextEngine::new();
        let (_, h_narrow) = engine.measure("Hello World Test", 16.0, 20.0, 50.0);
        let (_, h_wide) = engine.measure("Hello World Test", 16.0, 20.0, 500.0);
        assert!(h_narrow > h_wide, "Narrow width should produce more lines");
    }

    #[test]
    fn break_lines_produces_line_metrics() {
        let mut engine = TextEngine::new();
        let lines = engine.break_lines("Hello world", 16.0, 20.0, 500.0);
        assert!(!lines.is_empty());
        assert!(lines[0].width > 0.0);
    }
}

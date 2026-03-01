// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Display list — a flat list of GPU-ready drawing commands.
//!
//! Generated from a layout tree by the painter, then consumed by the renderer.
//! Commands are in paint order (back to front, respecting stacking contexts).

use vex_core::color::Color;
use vex_core::geometry::{Insets, Point, Rect};

/// Identifier for a decoded image stored in the image atlas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageId(pub u32);

/// A single glyph positioned for rendering.
#[derive(Debug, Clone, Copy)]
pub struct GlyphInstance {
    /// Glyph ID from the font.
    pub glyph_id: u32,
    /// Horizontal position (absolute px).
    pub x: f32,
    /// Vertical position (absolute px, baseline).
    pub y: f32,
}

/// Border style for rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderBorderStyle {
    #[default]
    None,
    Solid,
    Dashed,
    Dotted,
}

/// A single drawing command in the display list.
#[derive(Debug, Clone)]
pub enum DisplayCommand {
    /// Fill a rectangle with a solid color.
    FillRect { rect: Rect, color: Color },

    /// Draw borders around a rectangle.
    DrawBorder {
        rect: Rect,
        widths: Insets,
        colors: [Color; 4], // top, right, bottom, left
        styles: [RenderBorderStyle; 4],
    },

    /// Draw a run of text glyphs.
    DrawText {
        position: Point,
        text: String,
        color: Color,
        font_size: f32,
        line_height: f32,
    },

    /// Draw a decoded image.
    DrawImage { rect: Rect, image_id: ImageId },

    /// Push a clipping rectangle (overflow: hidden).
    PushClip { rect: Rect },

    /// Pop the most recent clipping rectangle.
    PopClip,

    /// Push an opacity layer.
    PushOpacity { opacity: f32 },

    /// Pop the most recent opacity layer.
    PopOpacity,
}

/// An ordered list of display commands ready for GPU rendering.
#[derive(Debug, Clone, Default)]
pub struct DisplayList {
    commands: Vec<DisplayCommand>,
}

impl DisplayList {
    /// Create an empty display list.
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    /// Create with pre-allocated capacity.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            commands: Vec::with_capacity(cap),
        }
    }

    /// Append a command.
    pub fn push(&mut self, cmd: DisplayCommand) {
        self.commands.push(cmd);
    }

    /// Number of commands.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Returns true if there are no commands.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Iterate over commands in paint order.
    pub fn commands(&self) -> &[DisplayCommand] {
        &self.commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_list_push_and_iterate() {
        let mut dl = DisplayList::new();
        dl.push(DisplayCommand::FillRect {
            rect: Rect::new(0.0, 0.0, 100.0, 50.0),
            color: Color::rgb(255, 0, 0),
        });
        dl.push(DisplayCommand::PushClip {
            rect: Rect::new(10.0, 10.0, 80.0, 30.0),
        });
        dl.push(DisplayCommand::PopClip);

        assert_eq!(dl.len(), 3);
        assert!(!dl.is_empty());

        match &dl.commands()[0] {
            DisplayCommand::FillRect { color, .. } => {
                assert_eq!(color.r, 255);
            }
            _ => panic!("expected FillRect"),
        }
    }

    #[test]
    fn display_list_with_capacity() {
        let dl = DisplayList::with_capacity(100);
        assert!(dl.is_empty());
        assert_eq!(dl.len(), 0);
    }

    #[test]
    fn draw_text_command() {
        let mut dl = DisplayList::new();
        dl.push(DisplayCommand::DrawText {
            position: Point::new(10.0, 20.0),
            text: "Hello".to_string(),
            color: Color::BLACK,
            font_size: 16.0,
            line_height: 20.0,
        });
        assert_eq!(dl.len(), 1);
    }

    #[test]
    fn opacity_commands_pair() {
        let mut dl = DisplayList::new();
        dl.push(DisplayCommand::PushOpacity { opacity: 0.5 });
        dl.push(DisplayCommand::FillRect {
            rect: Rect::new(0.0, 0.0, 50.0, 50.0),
            color: Color::WHITE,
        });
        dl.push(DisplayCommand::PopOpacity);
        assert_eq!(dl.len(), 3);
    }
}

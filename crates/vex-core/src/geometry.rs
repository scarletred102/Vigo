// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Geometry primitives used throughout layout and rendering.

use serde::{Deserialize, Serialize};

/// A 2D point.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

/// A 2D size.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

/// An axis-aligned rectangle defined by origin + size.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

/// Edge insets (margins, padding, borders).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Insets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl Size {
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            origin: Point { x, y },
            size: Size {
                width: w,
                height: h,
            },
        }
    }

    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.origin.x
            && p.y >= self.origin.y
            && p.x <= self.origin.x + self.size.width
            && p.y <= self.origin.y + self.size.height
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.origin.x < other.origin.x + other.size.width
            && self.origin.x + self.size.width > other.origin.x
            && self.origin.y < other.origin.y + other.size.height
            && self.origin.y + self.size.height > other.origin.y
    }

    /// Smallest rect enclosing both `self` and `other`.
    pub fn union(&self, other: &Rect) -> Rect {
        let x1 = self.origin.x.min(other.origin.x);
        let y1 = self.origin.y.min(other.origin.y);
        let x2 = (self.origin.x + self.size.width).max(other.origin.x + other.size.width);
        let y2 = (self.origin.y + self.size.height).max(other.origin.y + other.size.height);
        Rect::new(x1, y1, x2 - x1, y2 - y1)
    }

    pub fn offset(&self, dx: f32, dy: f32) -> Rect {
        Rect::new(
            self.origin.x + dx,
            self.origin.y + dy,
            self.size.width,
            self.size.height,
        )
    }

    /// Shrink rect by the given insets.
    pub fn inset(&self, i: Insets) -> Rect {
        Rect::new(
            self.origin.x + i.left,
            self.origin.y + i.top,
            self.size.width - i.left - i.right,
            self.size.height - i.top - i.bottom,
        )
    }
}

impl Insets {
    pub const fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    pub const fn uniform(v: f32) -> Self {
        Self::new(v, v, v, v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_inside() {
        let r = Rect::new(10.0, 10.0, 100.0, 50.0);
        assert!(r.contains(Point::new(50.0, 30.0)));
    }

    #[test]
    fn contains_outside() {
        let r = Rect::new(10.0, 10.0, 100.0, 50.0);
        assert!(!r.contains(Point::new(0.0, 0.0)));
    }

    #[test]
    fn intersects_overlap() {
        let a = Rect::new(0.0, 0.0, 50.0, 50.0);
        let b = Rect::new(25.0, 25.0, 50.0, 50.0);
        assert!(a.intersects(&b));
    }

    #[test]
    fn intersects_no_overlap() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(20.0, 20.0, 10.0, 10.0);
        assert!(!a.intersects(&b));
    }

    #[test]
    fn union_covers_both() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(5.0, 5.0, 20.0, 20.0);
        let u = a.union(&b);
        assert!(u.contains(Point::new(0.0, 0.0)));
        assert!(u.contains(Point::new(24.0, 24.0)));
    }

    #[test]
    fn offset_moves() {
        let r = Rect::new(0.0, 0.0, 10.0, 10.0).offset(5.0, 5.0);
        assert_eq!(r.origin, Point::new(5.0, 5.0));
        assert_eq!(r.size, Size::new(10.0, 10.0));
    }

    #[test]
    fn inset_shrinks() {
        let r = Rect::new(0.0, 0.0, 100.0, 100.0);
        let inner = r.inset(Insets::uniform(10.0));
        assert_eq!(inner, Rect::new(10.0, 10.0, 80.0, 80.0));
    }

    #[test]
    fn default_is_zero() {
        assert_eq!(Rect::default(), Rect::new(0.0, 0.0, 0.0, 0.0));
    }
}

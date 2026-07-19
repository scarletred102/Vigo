// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Damage tracking for partial repaint / compositing.
//!
//! Computes dirty rectangles from differences between display lists, then
//! merges overlapping/adjacent rectangles to produce a compact damage set.

use crate::display_list::{command_bounds, DisplayList};
use vex_core::geometry::Rect;

/// Compute damage rectangles between `previous` and `next` display lists.
///
/// The output is clipped to the viewport and merged for efficiency.
pub fn compute_damage(previous: &DisplayList, next: &DisplayList, viewport: Rect) -> Vec<Rect> {
    let mut rects = Vec::new();

    let prev_cmds = previous.commands();
    let next_cmds = next.commands();
    let common = prev_cmds.len().min(next_cmds.len());

    for i in 0..common {
        if prev_cmds[i] != next_cmds[i] {
            if let Some(r) = command_bounds(&prev_cmds[i]).and_then(|r| intersect_rect(r, viewport))
            {
                rects.push(r);
            }
            if let Some(r) = command_bounds(&next_cmds[i]).and_then(|r| intersect_rect(r, viewport))
            {
                rects.push(r);
            }
        }
    }

    for cmd in &prev_cmds[common..] {
        if let Some(r) = command_bounds(cmd).and_then(|r| intersect_rect(r, viewport)) {
            rects.push(r);
        }
    }
    for cmd in &next_cmds[common..] {
        if let Some(r) = command_bounds(cmd).and_then(|r| intersect_rect(r, viewport)) {
            rects.push(r);
        }
    }

    merge_damage(rects)
}

/// Merge overlapping or edge-touching damage rects.
pub fn merge_damage(mut rects: Vec<Rect>) -> Vec<Rect> {
    let mut changed = true;
    while changed {
        changed = false;
        let mut i = 0usize;
        while i < rects.len() {
            let mut j = i + 1;
            while j < rects.len() {
                if touches_or_overlaps(rects[i], rects[j]) {
                    let merged = rects[i].union(&rects[j]);
                    rects[i] = merged;
                    rects.swap_remove(j);
                    changed = true;
                } else {
                    j += 1;
                }
            }
            i += 1;
        }
    }
    rects
}

fn touches_or_overlaps(a: Rect, b: Rect) -> bool {
    let ax2 = a.origin.x + a.size.width;
    let ay2 = a.origin.y + a.size.height;
    let bx2 = b.origin.x + b.size.width;
    let by2 = b.origin.y + b.size.height;

    // Expanded overlap check where edge-touch counts as mergeable.
    a.origin.x <= bx2 && ax2 >= b.origin.x && a.origin.y <= by2 && ay2 >= b.origin.y
}

fn intersect_rect(a: Rect, b: Rect) -> Option<Rect> {
    let x1 = a.origin.x.max(b.origin.x);
    let y1 = a.origin.y.max(b.origin.y);
    let x2 = (a.origin.x + a.size.width).min(b.origin.x + b.size.width);
    let y2 = (a.origin.y + a.size.height).min(b.origin.y + b.size.height);

    if x2 <= x1 || y2 <= y1 {
        None
    } else {
        Some(Rect::new(x1, y1, x2 - x1, y2 - y1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display_list::DisplayCommand;
    use vex_core::color::Color;

    #[test]
    fn identical_lists_have_no_damage() {
        let mut dl = DisplayList::new();
        dl.push(DisplayCommand::FillRect {
            rect: Rect::new(0.0, 0.0, 100.0, 100.0),
            color: Color::BLACK,
            border_radius: 0.0,
        });
        let dmg = compute_damage(&dl, &dl, Rect::new(0.0, 0.0, 1000.0, 1000.0));
        assert!(dmg.is_empty());
    }

    #[test]
    fn changed_command_reports_damage() {
        let mut a = DisplayList::new();
        a.push(DisplayCommand::FillRect {
            rect: Rect::new(0.0, 0.0, 20.0, 20.0),
            color: Color::BLACK,
            border_radius: 0.0,
        });

        let mut b = DisplayList::new();
        b.push(DisplayCommand::FillRect {
            rect: Rect::new(10.0, 10.0, 20.0, 20.0),
            color: Color::BLACK,
            border_radius: 0.0,
        });

        let dmg = compute_damage(&a, &b, Rect::new(0.0, 0.0, 1000.0, 1000.0));
        assert!(!dmg.is_empty());
    }

    #[test]
    fn merge_damage_unions_overlaps() {
        let merged = merge_damage(vec![
            Rect::new(0.0, 0.0, 10.0, 10.0),
            Rect::new(8.0, 0.0, 10.0, 10.0),
        ]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0], Rect::new(0.0, 0.0, 18.0, 10.0));
    }
}

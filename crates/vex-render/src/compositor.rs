// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Compositor layerization heuristics inspired by modern browsers.
//!
//! Promotes layout boxes to composited layers when they have properties that
//! commonly benefit from independent raster/composite (opacity, transforms,
//! fixed/sticky positioning, filters, clipped scrolling regions, animations).

use std::collections::HashMap;

use vex_core::VexId;
use vex_css::values::position::Position;
use vex_css::ComputedStyle;
use vex_layout::LayoutBox;

/// Why a box was promoted to a composited layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerReason {
    Root,
    Positioned,
    Opacity,
    Transform,
    Filter,
    Clip,
    Animated,
}

/// A composited layer candidate.
#[derive(Debug, Clone)]
pub struct CompositorLayer {
    pub node_id: Option<VexId>,
    pub bounds: vex_core::geometry::Rect,
    pub z_index: i32,
    pub opaque: bool,
    pub reason: LayerReason,
}

/// Build a flat composited layer list from a layout tree + styles.
pub fn build_layers(
    root: &LayoutBox,
    styles: &HashMap<VexId, ComputedStyle>,
) -> Vec<CompositorLayer> {
    let mut layers = Vec::new();
    layers.push(CompositorLayer {
        node_id: root.node_id,
        bounds: root.border_box(),
        z_index: 0,
        opaque: false,
        reason: LayerReason::Root,
    });
    collect_layers(root, styles, &mut layers);
    layers.sort_by_key(|l| l.z_index);
    layers
}

/// Remove fully occluded layers (simple opaque-rect heuristic).
pub fn cull_fully_occluded(layers: Vec<CompositorLayer>) -> Vec<CompositorLayer> {
    if layers.len() <= 1 {
        return layers;
    }

    let mut visible = Vec::new();
    for i in 0..layers.len() {
        let current = &layers[i];
        let occluded = layers
            .iter()
            .skip(i + 1)
            .any(|top| top.opaque && contains_rect(top.bounds, current.bounds));
        if !occluded {
            visible.push(current.clone());
        }
    }
    visible
}

fn collect_layers(
    layout_box: &LayoutBox,
    styles: &HashMap<VexId, ComputedStyle>,
    out: &mut Vec<CompositorLayer>,
) {
    if let Some(id) = layout_box.node_id {
        if let Some(style) = styles.get(&id) {
            if let Some(reason) = promotion_reason(layout_box, style) {
                out.push(CompositorLayer {
                    node_id: Some(id),
                    bounds: layout_box.border_box(),
                    z_index: if style.position.is_positioned() {
                        style.z_index
                    } else {
                        0
                    },
                    opaque: style.opacity >= 1.0 && style.background_color.a == 255,
                    reason,
                });
            }
        }
    }

    for child in &layout_box.children {
        collect_layers(child, styles, out);
    }
}

fn promotion_reason(layout_box: &LayoutBox, style: &ComputedStyle) -> Option<LayerReason> {
    if matches!(
        style.position,
        Position::Fixed | Position::Sticky | Position::Absolute
    ) {
        return Some(LayerReason::Positioned);
    }
    if style.opacity < 1.0 {
        return Some(LayerReason::Opacity);
    }
    if !style.transform.is_empty() {
        return Some(LayerReason::Transform);
    }
    if !style.filter.is_empty() || !style.backdrop_filter.is_empty() {
        return Some(LayerReason::Filter);
    }
    if layout_box.clip_rect.is_some() {
        return Some(LayerReason::Clip);
    }
    if style.transition_duration > 0.0 || style.animation_duration > 0.0 {
        return Some(LayerReason::Animated);
    }
    None
}

fn contains_rect(outer: vex_core::geometry::Rect, inner: vex_core::geometry::Rect) -> bool {
    outer.origin.x <= inner.origin.x
        && outer.origin.y <= inner.origin.y
        && outer.origin.x + outer.size.width >= inner.origin.x + inner.size.width
        && outer.origin.y + outer.size.height >= inner.origin.y + inner.size.height
}

#[cfg(test)]
mod tests {
    use super::*;
    use vex_core::geometry::Rect;
    use vex_layout::{BoxType, LayoutBox};

    #[test]
    fn opacity_promotes_layer() {
        let id = VexId::new(1);
        let mut root = LayoutBox::new(Some(id), BoxType::Block);
        root.dimensions.content = Rect::new(0.0, 0.0, 100.0, 100.0);

        let mut styles = HashMap::new();
        let s = ComputedStyle {
            opacity: 0.5,
            ..Default::default()
        };
        styles.insert(id, s);

        let layers = build_layers(&root, &styles);
        assert!(layers.iter().any(|l| l.reason == LayerReason::Opacity));
    }

    #[test]
    fn opaque_top_layer_culls_lower() {
        let lower = CompositorLayer {
            node_id: Some(VexId::new(1)),
            bounds: Rect::new(0.0, 0.0, 100.0, 100.0),
            z_index: 0,
            opaque: false,
            reason: LayerReason::Root,
        };
        let upper = CompositorLayer {
            node_id: Some(VexId::new(2)),
            bounds: Rect::new(0.0, 0.0, 200.0, 200.0),
            z_index: 1,
            opaque: true,
            reason: LayerReason::Opacity,
        };

        let visible = cull_fully_occluded(vec![lower, upper]);
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].node_id, Some(VexId::new(2)));
    }
}

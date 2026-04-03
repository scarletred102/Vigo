// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! JS-side DOM mutation dirty-node queue.
//!
//! DOM/Style API setters push mutated node IDs here; the browser loop drains
//! and maps them to layout reflow invalidation.

use std::collections::BTreeSet;

use boa_engine::object::builtins::JsArray;
use boa_engine::{js_string, Context, JsValue};
use vex_core::VexId;

/// Record that a DOM node changed in JS and requires style/layout invalidation.
pub fn mark_dom_dirty_node(context: &mut Context, node_id: VexId) {
    let global = context.global_object();
    let key = js_string!("__vex_dom_dirty_nodes");

    let arr = match global.get(key.clone(), context) {
        Ok(v) if !v.is_undefined() && !v.is_null() => match v.to_object(context) {
            Ok(obj) => match JsArray::from_object(obj) {
                Ok(arr) => arr,
                Err(_) => JsArray::new(context),
            },
            Err(_) => JsArray::new(context),
        },
        _ => JsArray::new(context),
    };

    let _ = arr.push(JsValue::from(node_id.index() as i32), context);
    let _ = global.set(key, JsValue::from(arr), false, context);
}

/// Drain and clear all queued dirty node IDs from JS.
pub fn take_dom_dirty_nodes(context: &mut Context) -> Vec<VexId> {
    let global = context.global_object();
    let key = js_string!("__vex_dom_dirty_nodes");
    let mut out = BTreeSet::<u32>::new();

    if let Ok(v) = global.get(key.clone(), context) {
        if !v.is_undefined() && !v.is_null() {
            if let Ok(obj) = v.to_object(context) {
                if let Ok(len_val) = obj.get(js_string!("length"), context) {
                    if let Ok(len) = len_val.to_u32(context) {
                        for i in 0..len {
                            if let Ok(id_val) = obj.get(i, context) {
                                if let Ok(id) = id_val.to_u32(context) {
                                    out.insert(id);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let empty = JsArray::new(context);
    let _ = global.set(key, JsValue::from(empty), false, context);

    out.into_iter().map(VexId::new).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_and_drains_nodes() {
        let mut ctx = Context::default();
        mark_dom_dirty_node(&mut ctx, VexId::new(2));
        mark_dom_dirty_node(&mut ctx, VexId::new(2));
        mark_dom_dirty_node(&mut ctx, VexId::new(9));

        let dirty = take_dom_dirty_nodes(&mut ctx);
        assert_eq!(dirty, vec![VexId::new(2), VexId::new(9)]);

        // Draining again should be empty.
        assert!(take_dom_dirty_nodes(&mut ctx).is_empty());
    }
}

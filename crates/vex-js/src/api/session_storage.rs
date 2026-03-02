// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! `window.sessionStorage` binding — delegates to [`vex_storage::SessionStorage`].
//!
//! Exposes the Web Storage API: `getItem`, `setItem`, `removeItem`, `clear`,
//! `length`. Scoped per origin + tab ID.

use std::cell::RefCell;
use std::rc::Rc;

use boa_engine::object::ObjectInitializer;
use boa_engine::{js_string, Context, JsNativeError, JsValue, NativeFunction};
use vex_storage::SessionStorage;

/// Shared handle — single `SessionStorage` instance per browser.
pub type SharedSessionStorage = Rc<RefCell<SessionStorage>>;

/// Create a shared `SessionStorage`.
#[must_use]
pub fn shared_session_storage() -> SharedSessionStorage {
    Rc::new(RefCell::new(SessionStorage::new()))
}

/// Build a `sessionStorage` JS object scoped to the given origin + tab.
///
/// The object exposes: `getItem(key)`, `setItem(key, value)`,
/// `removeItem(key)`, `clear()`, `length` (function).
pub fn build_session_storage(
    store: &SharedSessionStorage,
    origin: &str,
    tab_id: u64,
    context: &mut Context,
) -> JsValue {
    let origin_owned = origin.to_owned();

    // getItem(key) → string | null
    let s = store.clone();
    let o = origin_owned.clone();
    // SAFETY: Closure captures Rc<RefCell<>> and String; runs on single JS thread.
    let get_item = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let key = args
                .first()
                .ok_or_else(|| {
                    JsNativeError::typ().with_message("getItem requires a key argument")
                })?
                .to_string(ctx)?
                .to_std_string_escaped();
            match s.borrow().get_item(&o, tab_id, &key) {
                Some(val) => Ok(JsValue::from(js_string!(val))),
                None => Ok(JsValue::null()),
            }
        })
    };

    // setItem(key, value)
    let s = store.clone();
    let o = origin_owned.clone();
    // SAFETY: Same single-thread guarantee.
    let set_item = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let key = args
                .first()
                .ok_or_else(|| {
                    JsNativeError::typ().with_message("setItem requires a key argument")
                })?
                .to_string(ctx)?
                .to_std_string_escaped();
            let value = args
                .get(1)
                .ok_or_else(|| {
                    JsNativeError::typ().with_message("setItem requires a value argument")
                })?
                .to_string(ctx)?
                .to_std_string_escaped();
            if let Err(e) = s.borrow_mut().set_item(&o, tab_id, &key, &value) {
                tracing::warn!(target: "vex_js::session_storage", error = %e, "setItem failed");
                return Err(JsNativeError::typ()
                    .with_message(format!("setItem failed: {e}"))
                    .into());
            }
            Ok(JsValue::undefined())
        })
    };

    // removeItem(key)
    let s = store.clone();
    let o = origin_owned.clone();
    // SAFETY: Same single-thread guarantee.
    let remove_item = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let key = args
                .first()
                .ok_or_else(|| {
                    JsNativeError::typ().with_message("removeItem requires a key argument")
                })?
                .to_string(ctx)?
                .to_std_string_escaped();
            s.borrow_mut().remove_item(&o, tab_id, &key);
            Ok(JsValue::undefined())
        })
    };

    // clear()
    let s = store.clone();
    let o = origin_owned.clone();
    // SAFETY: Same single-thread guarantee.
    let clear = unsafe {
        NativeFunction::from_closure(move |_this, _args, _ctx| {
            s.borrow_mut().clear(&o, tab_id);
            Ok(JsValue::undefined())
        })
    };

    // length()
    let s = store.clone();
    let o = origin_owned;
    // SAFETY: Same single-thread guarantee.
    let length_fn = unsafe {
        NativeFunction::from_closure(move |_this, _args, _ctx| {
            let len = s.borrow().length(&o, tab_id);
            Ok(JsValue::from(len as i32))
        })
    };

    ObjectInitializer::new(context)
        .function(get_item, js_string!("getItem"), 1)
        .function(set_item, js_string!("setItem"), 2)
        .function(remove_item, js_string!("removeItem"), 1)
        .function(clear, js_string!("clear"), 0)
        .function(length_fn, js_string!("length"), 0)
        .build()
        .into()
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use boa_engine::property::Attribute;
    use boa_engine::Source;

    fn setup() -> (Context, SharedSessionStorage) {
        let store = shared_session_storage();
        let mut ctx = Context::default();
        let ss = build_session_storage(&store, "https://example.com", 1, &mut ctx);
        ctx.register_global_property(
            js_string!("sessionStorage"),
            ss,
            Attribute::WRITABLE | Attribute::CONFIGURABLE,
        )
        .unwrap();
        (ctx, store)
    }

    #[test]
    fn get_item_returns_null_for_missing() {
        let (mut ctx, _) = setup();
        let result = ctx
            .eval(Source::from_bytes("sessionStorage.getItem('missing')"))
            .unwrap();
        assert!(result.is_null());
    }

    #[test]
    fn set_and_get_item() {
        let (mut ctx, _) = setup();
        ctx.eval(Source::from_bytes(
            "sessionStorage.setItem('key1', 'value1')",
        ))
        .unwrap();
        let result = ctx
            .eval(Source::from_bytes("sessionStorage.getItem('key1')"))
            .unwrap();
        assert_eq!(
            result.as_string().unwrap().to_std_string_escaped(),
            "value1"
        );
    }

    #[test]
    fn remove_item_deletes_key() {
        let (mut ctx, _) = setup();
        ctx.eval(Source::from_bytes(
            "sessionStorage.setItem('x', '1'); sessionStorage.removeItem('x')",
        ))
        .unwrap();
        let result = ctx
            .eval(Source::from_bytes("sessionStorage.getItem('x')"))
            .unwrap();
        assert!(result.is_null());
    }

    #[test]
    fn clear_removes_all() {
        let (mut ctx, _) = setup();
        ctx.eval(Source::from_bytes(
            "sessionStorage.setItem('a', '1'); \
             sessionStorage.setItem('b', '2'); \
             sessionStorage.clear()",
        ))
        .unwrap();
        let len = ctx
            .eval(Source::from_bytes("sessionStorage.length()"))
            .unwrap();
        assert_eq!(len.as_number().unwrap() as i32, 0);
    }

    #[test]
    fn tab_isolation() {
        let store = shared_session_storage();
        let mut ctx = Context::default();
        let ss1 = build_session_storage(&store, "https://example.com", 1, &mut ctx);
        let ss2 = build_session_storage(&store, "https://example.com", 2, &mut ctx);
        ctx.register_global_property(
            js_string!("ss1"),
            ss1,
            Attribute::WRITABLE | Attribute::CONFIGURABLE,
        )
        .unwrap();
        ctx.register_global_property(
            js_string!("ss2"),
            ss2,
            Attribute::WRITABLE | Attribute::CONFIGURABLE,
        )
        .unwrap();

        ctx.eval(Source::from_bytes("ss1.setItem('key', 'tab1')"))
            .unwrap();
        let from_tab2 = ctx.eval(Source::from_bytes("ss2.getItem('key')")).unwrap();
        assert!(from_tab2.is_null(), "tab 2 should not see tab 1's data");
    }
}

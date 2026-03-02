// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! `window.localStorage` binding — delegates to [`vex_storage::LocalStorage`].
//!
//! Exposes the Web Storage API: `getItem`, `setItem`, `removeItem`, `clear`,
//! `key`, `length`. Scoped per origin.

use std::cell::RefCell;
use std::rc::Rc;

use boa_engine::object::ObjectInitializer;
use boa_engine::{js_string, Context, JsNativeError, JsValue, NativeFunction};
use vex_storage::LocalStorage;

/// Shared handle so multiple closures can reference the same store.
pub type SharedLocalStorage = Rc<RefCell<LocalStorage>>;

/// Create a shared in-memory `LocalStorage` for testing.
pub fn shared_local_storage_in_memory() -> SharedLocalStorage {
    Rc::new(RefCell::new(
        LocalStorage::open_in_memory().expect("in-memory local storage"),
    ))
}

/// Create a shared file-backed `LocalStorage`.
///
/// # Errors
///
/// Returns an error if the SQLite database cannot be opened.
pub fn shared_local_storage(path: &str) -> Result<SharedLocalStorage, vex_storage::StorageError> {
    Ok(Rc::new(RefCell::new(LocalStorage::open(path)?)))
}

/// Build a `localStorage` JS object scoped to the given origin.
///
/// The object exposes: `getItem(key)`, `setItem(key, value)`,
/// `removeItem(key)`, `clear()`, `length` (getter).
pub fn build_local_storage(
    store: &SharedLocalStorage,
    origin: &str,
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
            match s.borrow().get_item(&o, &key) {
                Ok(Some(val)) => Ok(JsValue::from(js_string!(val))),
                Ok(None) => Ok(JsValue::null()),
                Err(e) => {
                    tracing::warn!(target: "vex_js::local_storage", error = %e, "getItem failed");
                    Ok(JsValue::null())
                }
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
            if let Err(e) = s.borrow().set_item(&o, &key, &value) {
                tracing::warn!(target: "vex_js::local_storage", error = %e, "setItem failed");
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
            if let Err(e) = s.borrow().remove_item(&o, &key) {
                tracing::warn!(target: "vex_js::local_storage", error = %e, "removeItem failed");
            }
            Ok(JsValue::undefined())
        })
    };

    // clear()
    let s = store.clone();
    let o = origin_owned.clone();
    // SAFETY: Same single-thread guarantee.
    let clear = unsafe {
        NativeFunction::from_closure(move |_this, _args, _ctx| {
            if let Err(e) = s.borrow().clear(&o) {
                tracing::warn!(target: "vex_js::local_storage", error = %e, "clear failed");
            }
            Ok(JsValue::undefined())
        })
    };

    // length (as a function — `localStorage.length` in real browsers is a getter,
    // but a function is acceptable for the initial wiring)
    let s = store.clone();
    let o = origin_owned;
    // SAFETY: Same single-thread guarantee.
    let length_fn = unsafe {
        NativeFunction::from_closure(move |_this, _args, _ctx| {
            let len = s.borrow().length(&o).unwrap_or(0);
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

    fn setup() -> (Context, SharedLocalStorage) {
        let store = shared_local_storage_in_memory();
        let mut ctx = Context::default();
        let ls = build_local_storage(&store, "https://example.com", &mut ctx);
        ctx.register_global_property(
            js_string!("localStorage"),
            ls,
            Attribute::WRITABLE | Attribute::CONFIGURABLE,
        )
        .unwrap();
        (ctx, store)
    }

    #[test]
    fn get_item_returns_null_for_missing() {
        let (mut ctx, _) = setup();
        let result = ctx
            .eval(Source::from_bytes("localStorage.getItem('missing')"))
            .unwrap();
        assert!(result.is_null());
    }

    #[test]
    fn set_and_get_item() {
        let (mut ctx, _) = setup();
        ctx.eval(Source::from_bytes("localStorage.setItem('key1', 'value1')"))
            .unwrap();
        let result = ctx
            .eval(Source::from_bytes("localStorage.getItem('key1')"))
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
            "localStorage.setItem('x', '1'); localStorage.removeItem('x')",
        ))
        .unwrap();
        let result = ctx
            .eval(Source::from_bytes("localStorage.getItem('x')"))
            .unwrap();
        assert!(result.is_null());
    }

    #[test]
    fn clear_removes_all() {
        let (mut ctx, _) = setup();
        ctx.eval(Source::from_bytes(
            "localStorage.setItem('a', '1'); localStorage.setItem('b', '2'); localStorage.clear()",
        ))
        .unwrap();
        let len = ctx
            .eval(Source::from_bytes("localStorage.length()"))
            .unwrap();
        assert_eq!(len.as_number().unwrap() as i32, 0);
    }

    #[test]
    fn origin_isolation() {
        let store = shared_local_storage_in_memory();
        let mut ctx = Context::default();
        let ls1 = build_local_storage(&store, "https://a.com", &mut ctx);
        let ls2 = build_local_storage(&store, "https://b.com", &mut ctx);
        ctx.register_global_property(
            js_string!("ls1"),
            ls1,
            Attribute::WRITABLE | Attribute::CONFIGURABLE,
        )
        .unwrap();
        ctx.register_global_property(
            js_string!("ls2"),
            ls2,
            Attribute::WRITABLE | Attribute::CONFIGURABLE,
        )
        .unwrap();

        ctx.eval(Source::from_bytes("ls1.setItem('key', 'from_a')"))
            .unwrap();
        let from_b = ctx.eval(Source::from_bytes("ls2.getItem('key')")).unwrap();
        assert!(from_b.is_null(), "ls2 should not see ls1's data");
    }
}

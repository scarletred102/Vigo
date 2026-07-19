// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! `window.indexedDB` binding — delegates to [`vex_storage::IdbDatabase`].
//!
//! Provides `indexedDB.open(name, version)` → request-like object.
//! Once opened, object stores support `put`, `get`, `delete`, `getAll`, `count`, `clear`.

use std::cell::RefCell;
use std::rc::Rc;

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsValue, NativeFunction};
use vex_storage::IdbDatabase;

/// Shared handle to an open IndexedDB instance.
type SharedIdb = Rc<RefCell<IdbDatabase>>;

/// Build the top-level `indexedDB` object exposed on `window`.
///
/// Currently supports `indexedDB.open(name, version)` which returns
/// a database object with store management and CRUD methods.
///
/// All databases are in-memory for now; the browser loop can provide
/// a file-backed path via a different constructor.
pub fn build_indexed_db(context: &mut Context) -> JsValue {
    // SAFETY: Closure captures nothing non-trivial; runs on single JS thread.
    let open_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let name = args
                .first()
                .ok_or_else(|| JsNativeError::typ().with_message("indexedDB.open requires a name"))?
                .to_string(ctx)?
                .to_std_string_escaped();
            let version = args.get(1).and_then(|v| v.as_number()).unwrap_or(1.0) as u32;

            match IdbDatabase::open_in_memory(&name, version) {
                Ok(db) => {
                    let shared: SharedIdb = Rc::new(RefCell::new(db));
                    let obj = build_db_object(&shared, ctx);
                    Ok(obj)
                }
                Err(e) => Err(JsNativeError::typ()
                    .with_message(format!("indexedDB.open failed: {e}"))
                    .into()),
            }
        })
    };

    ObjectInitializer::new(context)
        .function(open_fn, js_string!("open"), 2)
        .build()
        .into()
}

/// Build a persistent `indexedDB` object scoped to `origin` under `base_dir`.
///
/// Databases are materialized as SQLite files in `base_dir` using
/// `origin + db_name` file naming.
pub fn build_indexed_db_with_dir(base_dir: &str, origin: &str, context: &mut Context) -> JsValue {
    let base_dir = base_dir.to_owned();
    let origin = origin.to_owned();

    // SAFETY: Closure captures owned Strings; runs on single JS thread.
    let open_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let name = args
                .first()
                .ok_or_else(|| JsNativeError::typ().with_message("indexedDB.open requires a name"))?
                .to_string(ctx)?
                .to_std_string_escaped();
            let version = args.get(1).and_then(|v| v.as_number()).unwrap_or(1.0) as u32;

            if let Err(e) = std::fs::create_dir_all(&base_dir) {
                return Err(JsNativeError::typ()
                    .with_message(format!("failed to create IndexedDB dir: {e}"))
                    .into());
            }

            let db_path = format!(
                "{}/{}_{}.sqlite3",
                base_dir,
                sanitize_component(&origin),
                sanitize_component(&name)
            );

            match IdbDatabase::open(&db_path, &name, version) {
                Ok(db) => {
                    let shared: SharedIdb = Rc::new(RefCell::new(db));
                    let obj = build_db_object(&shared, ctx);
                    Ok(obj)
                }
                Err(e) => Err(JsNativeError::typ()
                    .with_message(format!("indexedDB.open failed: {e}"))
                    .into()),
            }
        })
    };

    ObjectInitializer::new(context)
        .function(open_fn, js_string!("open"), 2)
        .build()
        .into()
}

fn sanitize_component(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Build a database object with methods for object store management and CRUD.
fn build_db_object(db: &SharedIdb, context: &mut Context) -> JsValue {
    let db_ref = db.borrow();
    let name = js_string!(db_ref.name());
    let version = db_ref.version();
    drop(db_ref);

    // createObjectStore(name)
    let d = db.clone();
    // SAFETY: Closure captures Rc<RefCell<>>; runs on single JS thread.
    let create_store = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let store_name = args
                .first()
                .ok_or_else(|| {
                    JsNativeError::typ().with_message("createObjectStore requires a name")
                })?
                .to_string(ctx)?
                .to_std_string_escaped();
            d.borrow().create_object_store(&store_name).map_err(|e| {
                JsNativeError::typ().with_message(format!("createObjectStore failed: {e}"))
            })?;
            Ok(JsValue::undefined())
        })
    };

    // deleteObjectStore(name)
    let d = db.clone();
    // SAFETY: Same single-thread guarantee.
    let delete_store = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let store_name = args
                .first()
                .ok_or_else(|| {
                    JsNativeError::typ().with_message("deleteObjectStore requires a name")
                })?
                .to_string(ctx)?
                .to_std_string_escaped();
            d.borrow().delete_object_store(&store_name).map_err(|e| {
                JsNativeError::typ().with_message(format!("deleteObjectStore failed: {e}"))
            })?;
            Ok(JsValue::undefined())
        })
    };

    // put(store, key, value)
    let d = db.clone();
    // SAFETY: Same single-thread guarantee.
    let put_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let store = args
                .first()
                .ok_or_else(|| JsNativeError::typ().with_message("put requires store name"))?
                .to_string(ctx)?
                .to_std_string_escaped();
            let key = args
                .get(1)
                .ok_or_else(|| JsNativeError::typ().with_message("put requires key"))?
                .to_string(ctx)?
                .to_std_string_escaped();
            let js_val = args.get(2).cloned().unwrap_or(JsValue::undefined());
            let json_val = js_val.to_json(ctx).map_err(|e| {
                JsNativeError::typ().with_message(format!("value serialization failed: {e}"))
            })?;
            d.borrow()
                .put(&store, &key, &json_val)
                .map_err(|e| JsNativeError::typ().with_message(format!("put failed: {e}")))?;
            Ok(JsValue::undefined())
        })
    };

    // get(store, key) → value | null
    let d = db.clone();
    // SAFETY: Same single-thread guarantee.
    let get_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let store = args
                .first()
                .ok_or_else(|| JsNativeError::typ().with_message("get requires store name"))?
                .to_string(ctx)?
                .to_std_string_escaped();
            let key = args
                .get(1)
                .ok_or_else(|| JsNativeError::typ().with_message("get requires key"))?
                .to_string(ctx)?
                .to_std_string_escaped();
            match d.borrow().get(&store, &key) {
                Ok(Some(json_val)) => {
                    let js_val = JsValue::from_json(&json_val, ctx).map_err(|e| {
                        JsNativeError::typ()
                            .with_message(format!("value deserialization failed: {e}"))
                    })?;
                    Ok(js_val)
                }
                Ok(None) => Ok(JsValue::null()),
                Err(e) => {
                    tracing::warn!(target: "vex_js::indexed_db", error = %e, "get failed");
                    Ok(JsValue::null())
                }
            }
        })
    };

    // delete(store, key)
    let d = db.clone();
    // SAFETY: Same single-thread guarantee.
    let delete_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let store = args
                .first()
                .ok_or_else(|| JsNativeError::typ().with_message("delete requires store name"))?
                .to_string(ctx)?
                .to_std_string_escaped();
            let key = args
                .get(1)
                .ok_or_else(|| JsNativeError::typ().with_message("delete requires key"))?
                .to_string(ctx)?
                .to_std_string_escaped();
            if let Err(e) = d.borrow().delete(&store, &key) {
                tracing::warn!(target: "vex_js::indexed_db", error = %e, "delete failed");
            }
            Ok(JsValue::undefined())
        })
    };

    // getAllKeys(store) → string[]
    let d = db.clone();
    // SAFETY: Same single-thread guarantee.
    let get_all_keys = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let store = args
                .first()
                .ok_or_else(|| JsNativeError::typ().with_message("getAllKeys requires store name"))?
                .to_string(ctx)?
                .to_std_string_escaped();
            match d.borrow().get_all_keys(&store) {
                Ok(keys) => {
                    let arr: Vec<JsValue> = keys
                        .into_iter()
                        .map(|k| JsValue::from(js_string!(k)))
                        .collect();
                    let js_arr = boa_engine::object::builtins::JsArray::from_iter(arr, ctx);
                    Ok(JsValue::from(js_arr))
                }
                Err(e) => {
                    tracing::warn!(target: "vex_js::indexed_db", error = %e, "getAllKeys failed");
                    let empty = boa_engine::object::builtins::JsArray::from_iter(
                        Vec::<JsValue>::new(),
                        ctx,
                    );
                    Ok(JsValue::from(empty))
                }
            }
        })
    };

    // count(store) → number
    let d = db.clone();
    // SAFETY: Same single-thread guarantee.
    let count_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let store = args
                .first()
                .ok_or_else(|| JsNativeError::typ().with_message("count requires store name"))?
                .to_string(ctx)?
                .to_std_string_escaped();
            let cnt = d.borrow().count(&store).unwrap_or(0);
            Ok(JsValue::from(cnt as i32))
        })
    };

    // clear(store)
    let d = db.clone();
    // SAFETY: Same single-thread guarantee.
    let clear_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let store = args
                .first()
                .ok_or_else(|| JsNativeError::typ().with_message("clear requires store name"))?
                .to_string(ctx)?
                .to_std_string_escaped();
            if let Err(e) = d.borrow().clear(&store) {
                tracing::warn!(target: "vex_js::indexed_db", error = %e, "clear failed");
            }
            Ok(JsValue::undefined())
        })
    };

    ObjectInitializer::new(context)
        .property(js_string!("name"), name, Attribute::CONFIGURABLE)
        .property(
            js_string!("version"),
            JsValue::from(version),
            Attribute::CONFIGURABLE,
        )
        .function(create_store, js_string!("createObjectStore"), 1)
        .function(delete_store, js_string!("deleteObjectStore"), 1)
        .function(put_fn, js_string!("put"), 3)
        .function(get_fn, js_string!("get"), 2)
        .function(delete_fn, js_string!("delete"), 2)
        .function(get_all_keys, js_string!("getAllKeys"), 1)
        .function(count_fn, js_string!("count"), 1)
        .function(clear_fn, js_string!("clear"), 1)
        .build()
        .into()
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use boa_engine::Source;

    fn setup() -> Context {
        let mut ctx = Context::default();
        let idb = build_indexed_db(&mut ctx);
        ctx.register_global_property(
            js_string!("indexedDB"),
            idb,
            Attribute::WRITABLE | Attribute::CONFIGURABLE,
        )
        .unwrap();
        ctx
    }

    #[test]
    fn open_returns_db_object() {
        let mut ctx = setup();
        let result = ctx
            .eval(Source::from_bytes(
                "var db = indexedDB.open('test', 1); db.name",
            ))
            .unwrap();
        assert_eq!(result.as_string().unwrap().to_std_string_escaped(), "test");
    }

    #[test]
    fn create_store_and_put_get() {
        let mut ctx = setup();
        let result = ctx.eval(Source::from_bytes(
            "var db = indexedDB.open('test', 1); \
             db.createObjectStore('items'); \
             db.put('items', 'k1', {name: 'hello'}); \
             var v = db.get('items', 'k1'); \
             v.name",
        ));
        assert!(result.is_ok());
        let val = result.unwrap();
        assert_eq!(val.as_string().unwrap().to_std_string_escaped(), "hello");
    }

    #[test]
    fn get_missing_returns_null() {
        let mut ctx = setup();
        let result = ctx
            .eval(Source::from_bytes(
                "var db = indexedDB.open('test', 1); \
                 db.createObjectStore('store'); \
                 db.get('store', 'nope')",
            ))
            .unwrap();
        assert!(result.is_null());
    }

    #[test]
    fn delete_removes_entry() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            "var db = indexedDB.open('test', 1); \
             db.createObjectStore('s'); \
             db.put('s', 'k', 42); \
             db.delete('s', 'k');",
        ))
        .unwrap();
        let result = ctx.eval(Source::from_bytes("db.get('s', 'k')")).unwrap();
        assert!(result.is_null());
    }

    #[test]
    fn count_and_clear() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            "var db = indexedDB.open('test', 1); \
             db.createObjectStore('s'); \
             db.put('s', 'a', 1); \
             db.put('s', 'b', 2); \
             db.put('s', 'c', 3);",
        ))
        .unwrap();
        let count = ctx.eval(Source::from_bytes("db.count('s')")).unwrap();
        assert_eq!(count.as_number().unwrap() as i32, 3);

        ctx.eval(Source::from_bytes("db.clear('s')")).unwrap();
        let after = ctx.eval(Source::from_bytes("db.count('s')")).unwrap();
        assert_eq!(after.as_number().unwrap() as i32, 0);
    }

    #[test]
    fn get_all_keys_returns_array() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            "var db = indexedDB.open('test', 1); \
             db.createObjectStore('s'); \
             db.put('s', 'x', 1); \
             db.put('s', 'y', 2);",
        ))
        .unwrap();
        let result = ctx
            .eval(Source::from_bytes("db.getAllKeys('s').length"))
            .unwrap();
        assert_eq!(result.as_number().unwrap() as i32, 2);
    }
}

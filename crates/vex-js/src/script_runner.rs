// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Script execution lifecycle — discovery, classification, and ordered execution.
//!
//! After HTML parsing, scripts are classified into three categories:
//! 1. **Blocking** — inline scripts and external scripts without `defer`/`async`.
//!    Executed synchronously in document order.
//! 2. **Defer** — `<script defer>` external scripts. Fetched in parallel, executed
//!    in document order after the DOM is fully built.
//! 3. **Async** — `<script async>` external scripts. Fetched in parallel, executed
//!    as soon as available (no guaranteed order).
//!
//! ## Usage
//!
//! ```ignore
//! let scripts = extract_scripts(&doc); // from vex-html
//! let plan = ExecutionPlan::from_scripts(&scripts);
//! plan.execute_blocking(&mut runtime);
//! plan.execute_deferred(&mut runtime);
//! // async scripts would be fetched/executed via a task queue
//! ```

use vex_core::VexResult;
use vex_html::extract::ScriptInfo;

use crate::JsRuntime;

/// A classified script ready for execution.
#[derive(Debug, Clone)]
pub enum ScriptEntry {
    /// Inline script with source code.
    Inline {
        /// Source code to execute.
        source: String,
    },
    /// External script with a URL to fetch.
    External {
        /// URL of the script file.
        url: String,
    },
}

/// Classified and ordered script execution plan.
#[derive(Debug, Default)]
pub struct ExecutionPlan {
    /// Blocking scripts (inline + external without defer/async), in document order.
    pub blocking: Vec<ScriptEntry>,
    /// Deferred scripts (external with `defer`), in document order.
    pub deferred: Vec<ScriptEntry>,
    /// Async scripts (external with `async`), order doesn't matter.
    pub async_scripts: Vec<ScriptEntry>,
}

impl ExecutionPlan {
    /// Classify a list of [`ScriptInfo`] into an execution plan.
    pub fn from_scripts(scripts: &[ScriptInfo]) -> Self {
        let mut plan = ExecutionPlan::default();

        for script in scripts {
            // Skip module scripts and non-JS types for now.
            if let Some(ref t) = script.script_type {
                let t: &str = t.as_str();
                if t != "text/javascript" && t != "application/javascript" && !t.is_empty() {
                    continue;
                }
            }

            if script.is_async && script.src.is_some() {
                plan.async_scripts.push(ScriptEntry::External {
                    url: script.src.as_deref().unwrap_or_default().to_string(),
                });
            } else if script.is_defer && script.src.is_some() {
                plan.deferred.push(ScriptEntry::External {
                    url: script.src.as_deref().unwrap_or_default().to_string(),
                });
            } else if let Some(ref content) = script.inline_content {
                plan.blocking.push(ScriptEntry::Inline {
                    source: content.to_string(),
                });
            } else if let Some(ref url) = script.src {
                plan.blocking.push(ScriptEntry::External {
                    url: url.to_string(),
                });
            }
        }

        plan
    }

    /// Execute all blocking scripts in order.
    ///
    /// Inline scripts are executed directly. External scripts are fetched
    /// synchronously via the provided fetch function, then executed.
    pub fn execute_blocking<F>(
        &self,
        runtime: &mut JsRuntime,
        fetch_fn: &mut F,
    ) -> Vec<VexResult<()>>
    where
        F: FnMut(&str) -> VexResult<String>,
    {
        self.blocking
            .iter()
            .map(|entry| match entry {
                ScriptEntry::Inline { source } => runtime.execute(source),
                ScriptEntry::External { url } => {
                    let source = fetch_fn(url)?;
                    runtime.execute(&source)
                }
            })
            .collect()
    }

    /// Execute all deferred scripts in document order.
    ///
    /// Called after the DOM is fully built, before `DOMContentLoaded`.
    pub fn execute_deferred<F>(
        &self,
        runtime: &mut JsRuntime,
        fetch_fn: &mut F,
    ) -> Vec<VexResult<()>>
    where
        F: FnMut(&str) -> VexResult<String>,
    {
        self.deferred
            .iter()
            .map(|entry| match entry {
                ScriptEntry::Inline { source } => runtime.execute(source),
                ScriptEntry::External { url } => {
                    let source = fetch_fn(url)?;
                    runtime.execute(&source)
                }
            })
            .collect()
    }

    /// Execute a single async script.
    ///
    /// Called when the script has been fetched and is ready.
    pub fn execute_async_script(
        runtime: &mut JsRuntime,
        source: &str,
    ) -> VexResult<()> {
        runtime.execute(source)
    }

    /// Total number of scripts across all categories.
    pub fn total(&self) -> usize {
        self.blocking.len() + self.deferred.len() + self.async_scripts.len()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use vex_core::{VexError, VexId};
    use vex_html::extract::ScriptInfo;

    fn inline_script(content: &str) -> ScriptInfo {
        ScriptInfo {
            node_id: VexId::new(0),
            src: None,
            inline_content: Some(content.to_string()),
            script_type: None,
            is_async: false,
            is_defer: false,
        }
    }

    fn external_script(url: &str) -> ScriptInfo {
        ScriptInfo {
            node_id: VexId::new(0),
            src: Some(url.to_string()),
            inline_content: None,
            script_type: None,
            is_async: false,
            is_defer: false,
        }
    }

    fn defer_script(url: &str) -> ScriptInfo {
        ScriptInfo {
            node_id: VexId::new(0),
            src: Some(url.to_string()),
            inline_content: None,
            script_type: None,
            is_async: false,
            is_defer: true,
        }
    }

    fn async_script(url: &str) -> ScriptInfo {
        ScriptInfo {
            node_id: VexId::new(0),
            src: Some(url.to_string()),
            inline_content: None,
            script_type: None,
            is_async: true,
            is_defer: false,
        }
    }

    #[test]
    fn classify_scripts() {
        let scripts = vec![
            inline_script("alert(1)"),
            external_script("app.js"),
            defer_script("defer.js"),
            async_script("async.js"),
        ];

        let plan = ExecutionPlan::from_scripts(&scripts);
        assert_eq!(plan.blocking.len(), 2);     // inline + external
        assert_eq!(plan.deferred.len(), 1);      // defer
        assert_eq!(plan.async_scripts.len(), 1); // async
        assert_eq!(plan.total(), 4);
    }

    #[test]
    fn execute_inline_scripts() {
        let scripts = vec![inline_script("var x = 42;")];
        let plan = ExecutionPlan::from_scripts(&scripts);

        let mut runtime = JsRuntime::new();
        let mut no_fetch = |_url: &str| -> VexResult<String> {
            Err(VexError::Js("no fetch in test".into()))
        };

        let results = plan.execute_blocking(&mut runtime, &mut no_fetch);
        assert!(results[0].is_ok());

        let val = runtime.eval("x").unwrap();
        assert_eq!(val.as_number().unwrap() as i32, 42);
    }

    #[test]
    fn execute_external_scripts_with_fetch() {
        let scripts = vec![external_script("https://example.com/app.js")];
        let plan = ExecutionPlan::from_scripts(&scripts);

        let mut runtime = JsRuntime::new();
        let mut mock_fetch = |_url: &str| -> VexResult<String> {
            Ok("var fetched = true;".to_string())
        };

        let results = plan.execute_blocking(&mut runtime, &mut mock_fetch);
        assert!(results[0].is_ok());

        let val = runtime.eval("fetched").unwrap();
        assert!(val.as_boolean().unwrap());
    }

    #[test]
    fn skip_module_scripts() {
        let module = ScriptInfo {
            node_id: VexId::new(0),
            src: Some("module.mjs".to_string()),
            inline_content: None,
            script_type: Some("module".to_string()),
            is_async: false,
            is_defer: false,
        };

        let plan = ExecutionPlan::from_scripts(&[module]);
        assert_eq!(plan.total(), 0);
    }

    #[test]
    fn deferred_scripts_execute_in_order() {
        let scripts = vec![
            defer_script("first.js"),
            defer_script("second.js"),
        ];
        let plan = ExecutionPlan::from_scripts(&scripts);

        let mut runtime = JsRuntime::new();
        runtime.execute("var order = [];").unwrap();

        let mut call_count = 0;
        let mut mock_fetch = move |_url: &str| -> VexResult<String> {
            call_count += 1;
            Ok(format!("order.push({call_count});"))
        };

        let results = plan.execute_deferred(&mut runtime, &mut mock_fetch);
        assert!(results.iter().all(|r| r.is_ok()));

        let arr = runtime.eval("order.join(',')").unwrap();
        assert_eq!(arr.as_string().unwrap().to_std_string_escaped(), "1,2");
    }
}

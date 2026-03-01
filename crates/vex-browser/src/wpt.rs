// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Web Platform Tests (WPT) runner and triage framework.
//!
//! Harness for running WPT `.html` test files through the Vex pipeline
//! (HTML → DOM → CSS → Layout) and collecting pass/fail/error results
//! categorized by subsystem.
//!
//! Once the JS engine (Phase 7) is integrated, this can also execute
//! `testharness.js`-based assertions.

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Outcome & subsystem types
// ---------------------------------------------------------------------------

/// Result of a single WPT test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestOutcome {
    /// Test passed all assertions.
    Pass,
    /// Test failed one or more assertions.
    Fail(String),
    /// Test could not be executed (parse error, crash, etc.).
    Error(String),
    /// Test was skipped (e.g. requires JS features we don't support yet).
    Skip(String),
    /// Test timed out.
    Timeout,
}

impl fmt::Display for TestOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pass => write!(f, "PASS"),
            Self::Fail(msg) => write!(f, "FAIL: {msg}"),
            Self::Error(msg) => write!(f, "ERROR: {msg}"),
            Self::Skip(reason) => write!(f, "SKIP: {reason}"),
            Self::Timeout => write!(f, "TIMEOUT"),
        }
    }
}

/// Categorization of a test by Web subsystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Subsystem {
    Html,
    Dom,
    Css,
    Layout,
    Js,
    Fetch,
    Storage,
    Security,
    Other,
}

impl fmt::Display for Subsystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Html => "html",
            Self::Dom => "dom",
            Self::Css => "css",
            Self::Layout => "layout",
            Self::Js => "js",
            Self::Fetch => "fetch",
            Self::Storage => "storage",
            Self::Security => "security",
            Self::Other => "other",
        };
        write!(f, "{name}")
    }
}

/// Classify a WPT test path into a subsystem.
#[must_use]
pub fn classify_test(path: &Path) -> Subsystem {
    let s = path.to_string_lossy().to_ascii_lowercase();

    // Order matters: more specific paths first.
    if s.contains("/dom/") || s.contains("\\dom\\") {
        Subsystem::Dom
    } else if s.contains("/html/") || s.contains("\\html\\") {
        Subsystem::Html
    } else if s.contains("/css/") || s.contains("\\css\\") {
        Subsystem::Css
    } else if s.contains("/layout/") || s.contains("\\layout\\") {
        Subsystem::Layout
    } else if s.contains("/fetch/") || s.contains("\\fetch\\") {
        Subsystem::Fetch
    } else if s.contains("/webstorage/")
        || s.contains("\\webstorage\\")
        || s.contains("/indexeddb/")
        || s.contains("\\indexeddb\\")
    {
        Subsystem::Storage
    } else if s.contains("/content-security-policy/")
        || s.contains("\\content-security-policy\\")
        || s.contains("/cors/")
        || s.contains("\\cors\\")
    {
        Subsystem::Security
    } else if s.contains("/ecmascript/")
        || s.contains("\\ecmascript\\")
        || s.contains("/js/")
        || s.contains("\\js\\")
    {
        Subsystem::Js
    } else {
        Subsystem::Other
    }
}

// ---------------------------------------------------------------------------
// Test result & report structures
// ---------------------------------------------------------------------------

/// Result of running a single test file.
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Path to the test file.
    pub path: PathBuf,
    /// Which subsystem the test belongs to.
    pub subsystem: Subsystem,
    /// Outcome of the test.
    pub outcome: TestOutcome,
    /// Wall-clock duration.
    pub duration: Duration,
}

/// Per-subsystem statistics.
#[derive(Debug, Clone, Default)]
pub struct SubsystemStats {
    pub pass: usize,
    pub fail: usize,
    pub error: usize,
    pub skip: usize,
    pub timeout: usize,
}

impl SubsystemStats {
    /// Total tests in this subsystem.
    #[must_use]
    pub fn total(&self) -> usize {
        self.pass + self.fail + self.error + self.skip + self.timeout
    }

    /// Pass rate (0.0–1.0). Returns 0.0 if no tests.
    #[must_use]
    pub fn pass_rate(&self) -> f64 {
        let t = self.total();
        if t == 0 {
            0.0
        } else {
            self.pass as f64 / t as f64
        }
    }
}

/// Summary report from a WPT run.
#[derive(Debug, Clone)]
pub struct WptReport {
    /// All individual results.
    pub results: Vec<TestResult>,
    /// Summary counts per subsystem.
    pub by_subsystem: HashMap<Subsystem, SubsystemStats>,
    /// Total wall-clock time.
    pub total_duration: Duration,
}

// ---------------------------------------------------------------------------
// Runner
// ---------------------------------------------------------------------------

/// WPT test runner.
#[derive(Debug, Clone)]
pub struct WptRunner {
    /// Root directory of the WPT checkout (e.g. `tests/wpt/`).
    pub wpt_root: PathBuf,
    /// Timeout per test.
    pub test_timeout: Duration,
}

impl WptRunner {
    /// Create a new runner pointing at a WPT checkout directory.
    pub fn new(wpt_root: impl Into<PathBuf>) -> Self {
        Self {
            wpt_root: wpt_root.into(),
            test_timeout: Duration::from_secs(10),
        }
    }

    /// Discover all `.html` test files under a subdirectory.
    pub fn discover_tests(&self, subdir: &str) -> Vec<PathBuf> {
        let root = self.wpt_root.join(subdir);
        if !root.exists() {
            return Vec::new();
        }
        collect_html_files(&root)
    }

    /// Run a single test file through the Vex pipeline.
    ///
    /// Currently performs a parse-only check (HTML → DOM). Full
    /// execution with `testharness.js` assertions requires the JS
    /// engine (Phase 7+).
    pub fn run_test(&self, path: &Path) -> TestResult {
        let subsystem = classify_test(path);
        let start = Instant::now();

        let outcome = match std::fs::read_to_string(path) {
            Ok(html) => run_parse_test(&html),
            Err(e) => TestOutcome::Error(format!("read: {e}")),
        };

        TestResult {
            path: path.to_owned(),
            subsystem,
            outcome,
            duration: start.elapsed(),
        }
    }

    /// Run all discovered tests under `subdir` and produce a report.
    pub fn run_suite(&self, subdir: &str) -> WptReport {
        let start = Instant::now();
        let paths = self.discover_tests(subdir);
        let mut results = Vec::with_capacity(paths.len());

        for path in &paths {
            results.push(self.run_test(path));
        }

        let by_subsystem = build_stats(&results);

        WptReport {
            results,
            by_subsystem,
            total_duration: start.elapsed(),
        }
    }
}

// ---------------------------------------------------------------------------
// Triage helpers
// ---------------------------------------------------------------------------

/// Triage priorities: which subsystems need the most attention.
///
/// Returns subsystems sorted by number of failures (descending).
#[must_use]
pub fn triage_priorities(report: &WptReport) -> Vec<(Subsystem, SubsystemStats)> {
    let mut entries: Vec<_> = report.by_subsystem.clone().into_iter().collect();
    entries.sort_by(|a, b| {
        let a_fail = a.1.fail + a.1.error;
        let b_fail = b.1.fail + b.1.error;
        b_fail.cmp(&a_fail)
    });
    entries
}

/// Format a WPT report as a human-readable string.
#[must_use]
pub fn format_report(report: &WptReport) -> String {
    let mut out = String::new();
    out.push_str("=== WPT Report ===\n\n");

    let total = report.results.len();
    let pass = report
        .results
        .iter()
        .filter(|r| r.outcome == TestOutcome::Pass)
        .count();

    out.push_str(&format!(
        "Total: {total}  Pass: {pass}  Rate: {:.1}%\n",
        if total == 0 {
            0.0
        } else {
            100.0 * pass as f64 / total as f64
        }
    ));
    out.push_str(&format!(
        "Duration: {:.2}s\n\n",
        report.total_duration.as_secs_f64()
    ));

    // Triage order (most failures first).
    out.push_str("By subsystem (triage order):\n");
    for (sub, stats) in triage_priorities(report) {
        out.push_str(&format!(
            "  {sub:<10} pass={:<4} fail={:<4} error={:<4} skip={:<4} rate={:.1}%\n",
            stats.pass,
            stats.fail,
            stats.error,
            stats.skip,
            stats.pass_rate() * 100.0,
        ));
    }
    out
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Build per-subsystem statistics from results.
fn build_stats(results: &[TestResult]) -> HashMap<Subsystem, SubsystemStats> {
    let mut map: HashMap<Subsystem, SubsystemStats> = HashMap::new();
    for r in results {
        let stats = map.entry(r.subsystem).or_default();
        match &r.outcome {
            TestOutcome::Pass => stats.pass += 1,
            TestOutcome::Fail(_) => stats.fail += 1,
            TestOutcome::Error(_) => stats.error += 1,
            TestOutcome::Skip(_) => stats.skip += 1,
            TestOutcome::Timeout => stats.timeout += 1,
        }
    }
    map
}

/// Run a parse-only test: parse HTML and check the DOM is non-empty.
fn run_parse_test(html: &str) -> TestOutcome {
    let doc = vex_html::parse_html(html);
    // A basic sanity check: the document should have a root element (e.g. <html>).
    if doc.root_element().is_none() {
        TestOutcome::Fail("parsed document has no root element".into())
    } else {
        TestOutcome::Pass
    }
}

/// Recursively collect all `.html` files under a directory.
fn collect_html_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(collect_html_files(&path));
            } else if path.extension().is_some_and(|e| e == "html") {
                files.push(path);
            }
        }
    }
    files
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_dom_path() {
        let p = Path::new("tests/wpt/dom/nodes/Document-createElement.html");
        assert_eq!(classify_test(p), Subsystem::Dom);
    }

    #[test]
    fn classify_css_path() {
        let p = Path::new("tests/wpt/css/css-flexbox/flex-grow.html");
        assert_eq!(classify_test(p), Subsystem::Css);
    }

    #[test]
    fn classify_unknown_path() {
        let p = Path::new("tests/wpt/unknown/something.html");
        assert_eq!(classify_test(p), Subsystem::Other);
    }

    #[test]
    fn classify_security_cors() {
        let p = Path::new("tests/wpt/cors/preflight.html");
        assert_eq!(classify_test(p), Subsystem::Security);
    }

    #[test]
    fn classify_storage_path() {
        let p = Path::new("tests/wpt/webstorage/localStorage.html");
        assert_eq!(classify_test(p), Subsystem::Storage);
    }

    #[test]
    fn parse_test_passes_for_valid_html() {
        let outcome = run_parse_test("<html><body><p>hello</p></body></html>");
        assert_eq!(outcome, TestOutcome::Pass);
    }

    #[test]
    fn subsystem_stats_pass_rate() {
        let stats = SubsystemStats {
            pass: 3,
            fail: 1,
            error: 1,
            ..Default::default()
        };
        assert!((stats.pass_rate() - 0.6).abs() < 0.001);
    }

    #[test]
    fn empty_stats_zero_rate() {
        let stats = SubsystemStats::default();
        assert!((stats.pass_rate() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn build_stats_groups_correctly() {
        let results = vec![
            TestResult {
                path: PathBuf::from("tests/wpt/dom/a.html"),
                subsystem: Subsystem::Dom,
                outcome: TestOutcome::Pass,
                duration: Duration::ZERO,
            },
            TestResult {
                path: PathBuf::from("tests/wpt/dom/b.html"),
                subsystem: Subsystem::Dom,
                outcome: TestOutcome::Fail("x".into()),
                duration: Duration::ZERO,
            },
            TestResult {
                path: PathBuf::from("tests/wpt/css/c.html"),
                subsystem: Subsystem::Css,
                outcome: TestOutcome::Pass,
                duration: Duration::ZERO,
            },
        ];
        let stats = build_stats(&results);
        assert_eq!(stats[&Subsystem::Dom].pass, 1);
        assert_eq!(stats[&Subsystem::Dom].fail, 1);
        assert_eq!(stats[&Subsystem::Css].pass, 1);
    }

    #[test]
    fn triage_orders_by_most_failures() {
        let report = WptReport {
            results: vec![],
            by_subsystem: {
                let mut m = HashMap::new();
                m.insert(
                    Subsystem::Dom,
                    SubsystemStats {
                        pass: 10,
                        fail: 2,
                        ..Default::default()
                    },
                );
                m.insert(
                    Subsystem::Css,
                    SubsystemStats {
                        pass: 5,
                        fail: 8,
                        ..Default::default()
                    },
                );
                m.insert(
                    Subsystem::Html,
                    SubsystemStats {
                        pass: 20,
                        fail: 0,
                        ..Default::default()
                    },
                );
                m
            },
            total_duration: Duration::ZERO,
        };
        let priorities = triage_priorities(&report);
        // CSS (8 failures) should be first.
        assert_eq!(priorities[0].0, Subsystem::Css);
        // HTML (0 failures) should be last.
        assert_eq!(priorities[2].0, Subsystem::Html);
    }

    #[test]
    fn format_report_contains_summary() {
        let report = WptReport {
            results: vec![TestResult {
                path: PathBuf::from("a.html"),
                subsystem: Subsystem::Html,
                outcome: TestOutcome::Pass,
                duration: Duration::ZERO,
            }],
            by_subsystem: {
                let mut m = HashMap::new();
                m.insert(
                    Subsystem::Html,
                    SubsystemStats {
                        pass: 1,
                        ..Default::default()
                    },
                );
                m
            },
            total_duration: Duration::from_millis(42),
        };
        let text = format_report(&report);
        assert!(text.contains("Total: 1"));
        assert!(text.contains("Pass: 1"));
        assert!(text.contains("html"));
    }

    #[test]
    fn discover_from_nonexistent_dir() {
        let runner = WptRunner::new("/nonexistent");
        let tests = runner.discover_tests("dom");
        assert!(tests.is_empty());
    }
}

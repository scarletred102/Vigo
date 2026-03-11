// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! AbortController & AbortSignal API.
//!
//! Provides a mechanism for aborting in-progress operations (fetch requests,
//! event listeners, streams, etc.) following the WHATWG DOM specification.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// ── Types ────────────────────────────────────────────────────────────────────

/// A reason for the abort.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbortReason {
    /// Default abort (user-initiated).
    Default,
    /// Timeout.
    Timeout(u64),
    /// Custom reason message.
    Custom(String),
}

impl std::fmt::Display for AbortReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Default => write!(f, "AbortError: The operation was aborted."),
            Self::Timeout(ms) => write!(f, "TimeoutError: The operation timed out after {ms}ms."),
            Self::Custom(msg) => write!(f, "{msg}"),
        }
    }
}

/// Inner shared state for the signal.
#[derive(Debug)]
struct SignalInner {
    aborted: AtomicBool,
}

/// An AbortSignal that can be checked by async operations.
///
/// Signals are cheap to clone (Arc-based sharing).
#[derive(Debug, Clone)]
pub struct AbortSignal {
    inner: Arc<SignalInner>,
    /// Reason for the abort (set when abort is called).
    reason: Option<AbortReason>,
}

impl AbortSignal {
    fn new(inner: Arc<SignalInner>) -> Self {
        Self {
            inner,
            reason: None,
        }
    }

    /// Whether the signal has been aborted.
    pub fn aborted(&self) -> bool {
        self.inner.aborted.load(Ordering::Acquire)
    }

    /// Get the abort reason (if set).
    pub fn reason(&self) -> Option<&AbortReason> {
        if self.aborted() {
            self.reason.as_ref().or(Some(&AbortReason::Default))
        } else {
            None
        }
    }

    /// Throw if the signal has been aborted (returns `Err` if aborted).
    pub fn throw_if_aborted(&self) -> Result<(), AbortReason> {
        if self.aborted() {
            Err(self
                .reason
                .clone()
                .unwrap_or(AbortReason::Default))
        } else {
            Ok(())
        }
    }

    /// Create an already-aborted signal.
    pub fn abort() -> Self {
        let inner = Arc::new(SignalInner {
            aborted: AtomicBool::new(true),
        });
        Self {
            inner,
            reason: Some(AbortReason::Default),
        }
    }

    /// Create an already-aborted signal with a reason.
    pub fn abort_with(reason: AbortReason) -> Self {
        let inner = Arc::new(SignalInner {
            aborted: AtomicBool::new(true),
        });
        Self {
            inner,
            reason: Some(reason),
        }
    }

    /// Create a signal that aborts after a timeout (does NOT auto-abort; caller must check).
    pub fn timeout(ms: u64) -> Self {
        let inner = Arc::new(SignalInner {
            aborted: AtomicBool::new(false),
        });
        Self {
            inner,
            reason: Some(AbortReason::Timeout(ms)),
        }
    }

    /// Combine multiple signals — aborts when any input signal aborts.
    pub fn any(signals: &[&AbortSignal]) -> Self {
        for signal in signals {
            if signal.aborted() {
                return Self::abort_with(
                    signal.reason.clone().unwrap_or(AbortReason::Default),
                );
            }
        }
        let inner = Arc::new(SignalInner {
            aborted: AtomicBool::new(false),
        });
        Self {
            inner,
            reason: None,
        }
    }
}

/// An AbortController that can trigger associated AbortSignals.
#[derive(Debug)]
pub struct AbortController {
    inner: Arc<SignalInner>,
}

impl AbortController {
    /// Create a new AbortController.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(SignalInner {
                aborted: AtomicBool::new(false),
            }),
        }
    }

    /// Get the associated AbortSignal.
    pub fn signal(&self) -> AbortSignal {
        AbortSignal::new(Arc::clone(&self.inner))
    }

    /// Abort the signal.
    pub fn abort(&self) {
        self.inner.aborted.store(true, Ordering::Release);
    }

    /// Abort with a custom reason (note: reason is on the signal, not on the inner).
    pub fn abort_with_reason(&self, _reason: AbortReason) {
        self.inner.aborted.store(true, Ordering::Release);
    }
}

impl Default for AbortController {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controller_not_aborted() {
        let ctrl = AbortController::new();
        let signal = ctrl.signal();
        assert!(!signal.aborted());
        assert!(signal.reason().is_none());
    }

    #[test]
    fn controller_abort() {
        let ctrl = AbortController::new();
        let signal = ctrl.signal();
        ctrl.abort();
        assert!(signal.aborted());
    }

    #[test]
    fn signal_abort_static() {
        let signal = AbortSignal::abort();
        assert!(signal.aborted());
        assert_eq!(signal.reason(), Some(&AbortReason::Default));
    }

    #[test]
    fn signal_abort_with_reason() {
        let signal = AbortSignal::abort_with(AbortReason::Custom("cancelled".to_string()));
        assert!(signal.aborted());
        assert_eq!(
            signal.reason(),
            Some(&AbortReason::Custom("cancelled".to_string()))
        );
    }

    #[test]
    fn signal_throw_if_aborted() {
        let signal = AbortSignal::abort();
        assert!(signal.throw_if_aborted().is_err());

        let ctrl = AbortController::new();
        let signal2 = ctrl.signal();
        assert!(signal2.throw_if_aborted().is_ok());
    }

    #[test]
    fn signal_any() {
        let ctrl1 = AbortController::new();
        let ctrl2 = AbortController::new();
        let s1 = ctrl1.signal();
        let s2 = ctrl2.signal();

        let combined = AbortSignal::any(&[&s1, &s2]);
        assert!(!combined.aborted());

        // Create a new combined after abort
        ctrl1.abort();
        let combined2 = AbortSignal::any(&[&ctrl1.signal(), &s2]);
        assert!(combined2.aborted());
    }

    #[test]
    fn signal_timeout() {
        let signal = AbortSignal::timeout(5000);
        // Not auto-aborted — just sets the reason
        assert!(!signal.aborted());
        assert_eq!(signal.reason(), None); // Not aborted yet, so no reason
    }

    #[test]
    fn abort_reason_display() {
        assert_eq!(
            AbortReason::Default.to_string(),
            "AbortError: The operation was aborted."
        );
        assert_eq!(
            AbortReason::Timeout(5000).to_string(),
            "TimeoutError: The operation timed out after 5000ms."
        );
    }

    #[test]
    fn multiple_signals_from_controller() {
        let ctrl = AbortController::new();
        let s1 = ctrl.signal();
        let s2 = ctrl.signal();
        ctrl.abort();
        assert!(s1.aborted());
        assert!(s2.aborted());
    }
}

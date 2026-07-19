// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Dedicated Web Workers.
//!
//! Implements the Worker API for running JavaScript in background threads.
//! Workers communicate with the main thread via `postMessage`.

use std::collections::{HashMap, VecDeque};
use std::fmt;

// ── WorkerId ─────────────────────────────────────────────────────────────────

/// Unique identifier for a worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorkerId(u64);

impl WorkerId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn raw(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for WorkerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Worker({})", self.0)
    }
}

// ── WorkerState ──────────────────────────────────────────────────────────────

/// Lifecycle state of a worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerState {
    /// Worker script is being loaded.
    Loading,
    /// Worker is running.
    Running,
    /// Worker has been terminated.
    Terminated,
    /// Worker encountered an error.
    Error,
}

// ── WorkerMessage ────────────────────────────────────────────────────────────

/// A message sent between worker and main thread.
#[derive(Debug, Clone)]
pub struct WorkerMessage {
    /// Serialized message data (JSON string).
    pub data: String,
    /// Whether this message has transferable objects.
    pub has_transferables: bool,
}

impl WorkerMessage {
    pub fn new(data: &str) -> Self {
        Self {
            data: data.to_string(),
            has_transferables: false,
        }
    }

    pub fn with_transferables(data: &str) -> Self {
        Self {
            data: data.to_string(),
            has_transferables: true,
        }
    }
}

// ── WorkerType ───────────────────────────────────────────────────────────────

/// Type of worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkerType {
    /// Classic script worker.
    #[default]
    Classic,
    /// Module script worker.
    Module,
}

impl WorkerType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::Module => "module",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "module" => Self::Module,
            _ => Self::Classic,
        }
    }
}

// ── WorkerOptions ────────────────────────────────────────────────────────────

/// Options for creating a worker.
#[derive(Debug, Clone, Default)]
pub struct WorkerOptions {
    pub worker_type: WorkerType,
    pub name: Option<String>,
    pub credentials: WorkerCredentials,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkerCredentials {
    #[default]
    SameOrigin,
    Omit,
    Include,
}

// ── WorkerError ──────────────────────────────────────────────────────────────

/// An error that occurred in a worker.
#[derive(Debug, Clone)]
pub struct WorkerError {
    pub message: String,
    pub filename: String,
    pub lineno: u32,
    pub colno: u32,
}

impl WorkerError {
    pub fn new(message: &str, filename: &str, lineno: u32, colno: u32) -> Self {
        Self {
            message: message.to_string(),
            filename: filename.to_string(),
            lineno,
            colno,
        }
    }
}

impl fmt::Display for WorkerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at {}:{}:{}",
            self.message, self.filename, self.lineno, self.colno
        )
    }
}

// ── Worker ───────────────────────────────────────────────────────────────────

/// Represents a single dedicated worker.
#[derive(Debug)]
pub struct Worker {
    pub id: WorkerId,
    pub script_url: String,
    pub state: WorkerState,
    pub options: WorkerOptions,
    /// Messages from main thread to worker.
    pub inbox: VecDeque<WorkerMessage>,
    /// Messages from worker to main thread.
    pub outbox: VecDeque<WorkerMessage>,
    /// Errors from the worker.
    pub errors: Vec<WorkerError>,
}

impl Worker {
    pub fn new(id: WorkerId, script_url: &str, options: WorkerOptions) -> Self {
        Self {
            id,
            script_url: script_url.to_string(),
            state: WorkerState::Loading,
            options,
            inbox: VecDeque::new(),
            outbox: VecDeque::new(),
            errors: Vec::new(),
        }
    }

    /// Mark the worker as running (script loaded successfully).
    pub fn mark_running(&mut self) {
        if self.state == WorkerState::Loading {
            self.state = WorkerState::Running;
        }
    }

    /// Post a message to this worker (from main thread).
    pub fn post_message(&mut self, msg: WorkerMessage) -> bool {
        if self.state == WorkerState::Terminated {
            return false;
        }
        self.inbox.push_back(msg);
        true
    }

    /// Receive a message from the worker (in main thread).
    pub fn receive_message(&mut self) -> Option<WorkerMessage> {
        self.outbox.pop_front()
    }

    /// Enqueue a message from the worker to the main thread.
    pub fn send_to_main(&mut self, msg: WorkerMessage) {
        self.outbox.push_back(msg);
    }

    /// Report an error from the worker.
    pub fn report_error(&mut self, error: WorkerError) {
        self.state = WorkerState::Error;
        self.errors.push(error);
    }

    /// Terminate the worker.
    pub fn terminate(&mut self) {
        self.state = WorkerState::Terminated;
        self.inbox.clear();
    }

    /// Whether the worker is active (loading or running).
    pub fn is_active(&self) -> bool {
        matches!(self.state, WorkerState::Loading | WorkerState::Running)
    }

    /// The worker name (from options or default).
    pub fn name(&self) -> &str {
        self.options.name.as_deref().unwrap_or("")
    }
}

// ── WorkerManager ────────────────────────────────────────────────────────────

/// Manages all dedicated workers for the browser.
#[derive(Debug)]
pub struct WorkerManager {
    workers: HashMap<WorkerId, Worker>,
    next_id: u64,
}

impl Default for WorkerManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkerManager {
    pub fn new() -> Self {
        Self {
            workers: HashMap::new(),
            next_id: 1,
        }
    }

    /// Create a new worker. Returns its ID.
    pub fn create(&mut self, script_url: &str, options: WorkerOptions) -> WorkerId {
        let id = WorkerId::new(self.next_id);
        self.next_id += 1;
        let worker = Worker::new(id, script_url, options);
        self.workers.insert(id, worker);
        id
    }

    /// Get a worker by ID.
    pub fn get(&self, id: WorkerId) -> Option<&Worker> {
        self.workers.get(&id)
    }

    /// Get a mutable reference to a worker.
    pub fn get_mut(&mut self, id: WorkerId) -> Option<&mut Worker> {
        self.workers.get_mut(&id)
    }

    /// Post a message to a worker.
    pub fn post_message(&mut self, id: WorkerId, msg: WorkerMessage) -> bool {
        self.workers
            .get_mut(&id)
            .map(|w| w.post_message(msg))
            .unwrap_or(false)
    }

    /// Receive a message from a worker.
    pub fn receive_message(&mut self, id: WorkerId) -> Option<WorkerMessage> {
        self.workers.get_mut(&id).and_then(|w| w.receive_message())
    }

    /// Terminate a worker.
    pub fn terminate(&mut self, id: WorkerId) -> bool {
        if let Some(w) = self.workers.get_mut(&id) {
            w.terminate();
            true
        } else {
            false
        }
    }

    /// Remove terminated workers from the registry.
    pub fn cleanup(&mut self) {
        self.workers
            .retain(|_, w| w.state != WorkerState::Terminated);
    }

    /// Number of active workers.
    pub fn active_count(&self) -> usize {
        self.workers.values().filter(|w| w.is_active()).count()
    }

    /// Total number of workers (including terminated).
    pub fn total_count(&self) -> usize {
        self.workers.len()
    }

    /// Terminate all workers.
    pub fn terminate_all(&mut self) {
        for w in self.workers.values_mut() {
            w.terminate();
        }
    }

    /// Get all worker IDs.
    pub fn worker_ids(&self) -> Vec<WorkerId> {
        self.workers.keys().copied().collect()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_lifecycle() {
        let id = WorkerId::new(1);
        let mut worker = Worker::new(id, "worker.js", WorkerOptions::default());
        assert_eq!(worker.state, WorkerState::Loading);
        assert!(worker.is_active());

        worker.mark_running();
        assert_eq!(worker.state, WorkerState::Running);

        worker.terminate();
        assert_eq!(worker.state, WorkerState::Terminated);
        assert!(!worker.is_active());
    }

    #[test]
    fn worker_messaging() {
        let id = WorkerId::new(1);
        let mut worker = Worker::new(id, "worker.js", WorkerOptions::default());
        worker.mark_running();

        // Main → Worker
        assert!(worker.post_message(WorkerMessage::new(r#"{"hello":"world"}"#)));
        let msg = worker.inbox.pop_front().unwrap();
        assert_eq!(msg.data, r#"{"hello":"world"}"#);

        // Worker → Main
        worker.send_to_main(WorkerMessage::new(r#"{"reply":"ok"}"#));
        let reply = worker.receive_message().unwrap();
        assert_eq!(reply.data, r#"{"reply":"ok"}"#);
    }

    #[test]
    fn terminated_worker_rejects_messages() {
        let id = WorkerId::new(1);
        let mut worker = Worker::new(id, "worker.js", WorkerOptions::default());
        worker.terminate();
        assert!(!worker.post_message(WorkerMessage::new("data")));
    }

    #[test]
    fn worker_error() {
        let id = WorkerId::new(1);
        let mut worker = Worker::new(id, "worker.js", WorkerOptions::default());
        worker.mark_running();
        worker.report_error(WorkerError::new("ReferenceError", "worker.js", 10, 5));
        assert_eq!(worker.state, WorkerState::Error);
        assert_eq!(worker.errors.len(), 1);
        assert_eq!(worker.errors[0].lineno, 10);
    }

    #[test]
    fn worker_name() {
        let opts = WorkerOptions {
            name: Some("my-worker".to_string()),
            ..Default::default()
        };
        let worker = Worker::new(WorkerId::new(1), "w.js", opts);
        assert_eq!(worker.name(), "my-worker");

        let unnamed = Worker::new(WorkerId::new(2), "w.js", WorkerOptions::default());
        assert_eq!(unnamed.name(), "");
    }

    #[test]
    fn worker_type() {
        assert_eq!(WorkerType::parse("module"), WorkerType::Module);
        assert_eq!(WorkerType::parse("classic"), WorkerType::Classic);
        assert_eq!(WorkerType::Module.as_str(), "module");
    }

    #[test]
    fn worker_transferables() {
        let msg = WorkerMessage::with_transferables(r#"[1,2,3]"#);
        assert!(msg.has_transferables);
    }

    // ── WorkerManager ───────────────────────────────────────────

    #[test]
    fn manager_create_and_get() {
        let mut mgr = WorkerManager::new();
        let id = mgr.create("worker.js", WorkerOptions::default());
        assert!(mgr.get(id).is_some());
        assert_eq!(mgr.get(id).unwrap().script_url, "worker.js");
    }

    #[test]
    fn manager_post_and_receive() {
        let mut mgr = WorkerManager::new();
        let id = mgr.create("worker.js", WorkerOptions::default());

        // Post to worker
        assert!(mgr.post_message(id, WorkerMessage::new("hello")));

        // Simulate worker reply
        mgr.get_mut(id)
            .unwrap()
            .send_to_main(WorkerMessage::new("reply"));
        let reply = mgr.receive_message(id).unwrap();
        assert_eq!(reply.data, "reply");
    }

    #[test]
    fn manager_terminate() {
        let mut mgr = WorkerManager::new();
        let id = mgr.create("worker.js", WorkerOptions::default());
        assert_eq!(mgr.active_count(), 1);
        assert!(mgr.terminate(id));
        assert_eq!(mgr.active_count(), 0);
    }

    #[test]
    fn manager_cleanup() {
        let mut mgr = WorkerManager::new();
        let id1 = mgr.create("a.js", WorkerOptions::default());
        let _id2 = mgr.create("b.js", WorkerOptions::default());
        mgr.terminate(id1);
        assert_eq!(mgr.total_count(), 2);
        mgr.cleanup();
        assert_eq!(mgr.total_count(), 1);
    }

    #[test]
    fn manager_terminate_all() {
        let mut mgr = WorkerManager::new();
        mgr.create("a.js", WorkerOptions::default());
        mgr.create("b.js", WorkerOptions::default());
        mgr.create("c.js", WorkerOptions::default());
        assert_eq!(mgr.active_count(), 3);
        mgr.terminate_all();
        assert_eq!(mgr.active_count(), 0);
    }

    #[test]
    fn manager_worker_ids() {
        let mut mgr = WorkerManager::new();
        let id1 = mgr.create("a.js", WorkerOptions::default());
        let id2 = mgr.create("b.js", WorkerOptions::default());
        let ids = mgr.worker_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
    }

    #[test]
    fn manager_post_to_nonexistent() {
        let mut mgr = WorkerManager::new();
        assert!(!mgr.post_message(WorkerId::new(999), WorkerMessage::new("nope")));
        assert!(!mgr.terminate(WorkerId::new(999)));
    }

    #[test]
    fn worker_error_display() {
        let err = WorkerError::new("TypeError: x is not a function", "worker.js", 42, 7);
        let s = format!("{err}");
        assert!(s.contains("TypeError"));
        assert!(s.contains("42"));
        assert!(s.contains("7"));
    }
}

// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Windows process sandboxing for renderer processes.
//!
//! Defines [`SandboxConfig`] and [`SandboxPolicy`] for restricting renderer
//! processes via Windows Job Objects, restricted tokens, and UI limitations.
//!
//! The actual Win32 calls (`CreateJobObjectW`, `AssignProcessToJobObject`,
//! `CreateRestrictedToken`) will be wired once multi-process mode is active.
//! This module provides the policy configuration and a builder API.

use std::fmt;

// ── Errors ─────────────────────────────────────────────────────────────

/// Errors that can occur when applying a sandbox policy.
#[derive(Debug, Clone)]
pub enum SandboxError {
    /// Invalid configuration (e.g., CPU > 100%).
    InvalidConfig(String),
    /// Win32 API call failed.
    OsError(String),
}

impl fmt::Display for SandboxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(msg) => write!(f, "invalid sandbox config: {msg}"),
            Self::OsError(msg) => write!(f, "sandbox OS error: {msg}"),
        }
    }
}

impl std::error::Error for SandboxError {}

// ── Configuration ──────────────────────────────────────────────────────

/// Resource limits for a sandboxed renderer process.
#[derive(Debug, Clone)]
pub struct SandboxLimits {
    /// Maximum working-set memory in bytes (0 = unlimited).
    pub max_memory_bytes: u64,
    /// Maximum CPU usage as a percentage (1–100, 0 = unlimited).
    pub max_cpu_percent: u8,
    /// Maximum number of open handles (0 = unlimited).
    pub max_handles: u32,
}

impl Default for SandboxLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: 512 * 1024 * 1024, // 512 MB
            max_cpu_percent: 60,
            max_handles: 256,
        }
    }
}

/// UI restrictions applied to the renderer's desktop session.
#[derive(Debug, Clone)]
pub struct UiRestrictions {
    /// Prevent access to the system clipboard.
    pub no_clipboard: bool,
    /// Prevent creating or switching desktops.
    pub no_desktop_access: bool,
    /// Prevent reading/writing to the global atom table.
    pub no_global_atoms: bool,
    /// Prevent accessing USER handles from other processes.
    pub no_cross_process_handles: bool,
}

impl Default for UiRestrictions {
    fn default() -> Self {
        Self {
            no_clipboard: true,
            no_desktop_access: true,
            no_global_atoms: true,
            no_cross_process_handles: true,
        }
    }
}

/// Token restrictions for the renderer process.
#[derive(Debug, Clone)]
pub struct TokenRestrictions {
    /// Remove all admin SIDs from the token.
    pub remove_admin_sids: bool,
    /// Set the token integrity level (low / medium / untrusted).
    pub integrity_level: IntegrityLevel,
}

impl Default for TokenRestrictions {
    fn default() -> Self {
        Self {
            remove_admin_sids: true,
            integrity_level: IntegrityLevel::Low,
        }
    }
}

/// Windows integrity levels for process tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrityLevel {
    /// Untrusted — most restrictive.
    Untrusted,
    /// Low integrity (matches IE Protected Mode).
    Low,
    /// Medium integrity (default for standard users).
    Medium,
}

impl fmt::Display for IntegrityLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Untrusted => f.write_str("untrusted"),
            Self::Low => f.write_str("low"),
            Self::Medium => f.write_str("medium"),
        }
    }
}

// ── Sandbox policy ─────────────────────────────────────────────────────

/// Complete sandbox policy for a renderer process.
#[derive(Debug, Clone)]
pub struct SandboxPolicy {
    pub limits: SandboxLimits,
    pub ui: UiRestrictions,
    pub token: TokenRestrictions,
    /// Whether the sandbox is actually enforced (can be disabled for debugging).
    pub enabled: bool,
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        Self {
            limits: SandboxLimits::default(),
            ui: UiRestrictions::default(),
            token: TokenRestrictions::default(),
            enabled: true,
        }
    }
}

impl SandboxPolicy {
    /// Create a policy with the recommended production defaults.
    pub fn production() -> Self {
        Self::default()
    }

    /// Create a relaxed policy for debugging (sandbox disabled).
    pub fn debug() -> Self {
        Self {
            enabled: false,
            ..Self::default()
        }
    }

    /// Builder: set memory limit.
    #[must_use]
    pub fn with_max_memory(mut self, bytes: u64) -> Self {
        self.limits.max_memory_bytes = bytes;
        self
    }

    /// Builder: set CPU limit.
    #[must_use]
    pub fn with_max_cpu(mut self, percent: u8) -> Self {
        self.limits.max_cpu_percent = percent;
        self
    }

    /// Builder: set integrity level.
    #[must_use]
    pub fn with_integrity(mut self, level: IntegrityLevel) -> Self {
        self.token.integrity_level = level;
        self
    }

    /// Apply the sandbox policy to a process identified by its handle.
    ///
    /// **Current status:** Validates the policy configuration and returns
    /// `Ok(())` without calling Win32 APIs. The browser currently runs in
    /// single-process mode, so no real sandboxing is needed.
    ///
    /// When multi-process mode is implemented, this will:
    /// 1. Create a Windows Job Object via `CreateJobObjectW`
    /// 2. Set memory/CPU limits via `SetInformationJobObject`
    /// 3. Apply UI restrictions via `JOB_OBJECT_UILIMIT_*` flags
    /// 4. Create a restricted token via `CreateRestrictedToken`
    /// 5. Set the integrity level via `SetTokenInformation`
    /// 6. Assign the process to the Job Object via `AssignProcessToJobObject`
    ///
    /// # Errors
    ///
    /// Returns `Err` if the policy configuration is invalid (e.g., CPU > 100).
    pub fn apply(&self, _process_handle: *mut std::ffi::c_void) -> Result<(), SandboxError> {
        if !self.enabled {
            tracing::debug!("Sandbox disabled — skipping enforcement");
            return Ok(());
        }

        // Validate configuration.
        if self.limits.max_cpu_percent > 100 {
            return Err(SandboxError::InvalidConfig(
                "CPU percent must be 0–100".into(),
            ));
        }

        tracing::info!(
            memory_mb = self.limits.max_memory_bytes / (1024 * 1024),
            cpu = self.limits.max_cpu_percent,
            integrity = %self.token.integrity_level,
            "Sandbox policy would be applied (stub — single-process mode)"
        );

        Ok(())
    }

    /// Create a restricted token for the sandbox configuration.
    ///
    /// **Current status:** Returns `Ok(())`. When multi-process mode is active,
    /// this will call `CreateRestrictedToken` to strip admin SIDs and set the
    /// integrity level.
    pub fn create_restricted_token(&self) -> Result<(), SandboxError> {
        if !self.enabled {
            return Ok(());
        }
        tracing::debug!(
            remove_admin = self.token.remove_admin_sids,
            integrity = %self.token.integrity_level,
            "Would create restricted token (stub)"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_limits() {
        let limits = SandboxLimits::default();
        assert_eq!(limits.max_memory_bytes, 512 * 1024 * 1024);
        assert_eq!(limits.max_cpu_percent, 60);
        assert_eq!(limits.max_handles, 256);
    }

    #[test]
    fn default_ui_restrictions() {
        let ui = UiRestrictions::default();
        assert!(ui.no_clipboard);
        assert!(ui.no_desktop_access);
    }

    #[test]
    fn production_policy_is_enabled() {
        let policy = SandboxPolicy::production();
        assert!(policy.enabled);
        assert_eq!(policy.token.integrity_level, IntegrityLevel::Low);
        assert!(policy.token.remove_admin_sids);
    }

    #[test]
    fn debug_policy_is_disabled() {
        let policy = SandboxPolicy::debug();
        assert!(!policy.enabled);
    }

    #[test]
    fn builder_chain() {
        let policy = SandboxPolicy::production()
            .with_max_memory(256 * 1024 * 1024)
            .with_max_cpu(80)
            .with_integrity(IntegrityLevel::Untrusted);
        assert_eq!(policy.limits.max_memory_bytes, 256 * 1024 * 1024);
        assert_eq!(policy.limits.max_cpu_percent, 80);
        assert_eq!(policy.token.integrity_level, IntegrityLevel::Untrusted);
    }

    #[test]
    fn integrity_level_display() {
        assert_eq!(IntegrityLevel::Low.to_string(), "low");
        assert_eq!(IntegrityLevel::Untrusted.to_string(), "untrusted");
        assert_eq!(IntegrityLevel::Medium.to_string(), "medium");
    }

    #[test]
    fn apply_disabled_policy_succeeds() {
        let policy = SandboxPolicy::debug();
        assert!(policy.apply(std::ptr::null_mut()).is_ok());
    }

    #[test]
    fn apply_production_policy_succeeds() {
        let policy = SandboxPolicy::production();
        assert!(policy.apply(std::ptr::null_mut()).is_ok());
    }

    #[test]
    fn apply_invalid_cpu_fails() {
        let policy = SandboxPolicy::production().with_max_cpu(200);
        let err = policy.apply(std::ptr::null_mut()).unwrap_err();
        assert!(err.to_string().contains("CPU"));
    }

    #[test]
    fn create_restricted_token_succeeds() {
        let policy = SandboxPolicy::production();
        assert!(policy.create_restricted_token().is_ok());
    }

    #[test]
    fn create_restricted_token_disabled() {
        let policy = SandboxPolicy::debug();
        assert!(policy.create_restricted_token().is_ok());
    }

    #[test]
    fn sandbox_error_display() {
        let err = SandboxError::InvalidConfig("test".into());
        assert!(err.to_string().contains("invalid sandbox config"));
        let err = SandboxError::OsError("access denied".into());
        assert!(err.to_string().contains("sandbox OS error"));
    }
}

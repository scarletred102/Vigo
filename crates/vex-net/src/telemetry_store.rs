// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Persistent telemetry storage (JSONL records + stats snapshot).

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use vex_core::{VexError, VexResult};

use crate::telemetry::{NetworkRecord, NetworkStats};

#[derive(Debug, Clone)]
pub struct TelemetryStore {
    records_path: PathBuf,
    stats_path: PathBuf,
}

impl TelemetryStore {
    pub fn new(base_dir: impl AsRef<Path>) -> VexResult<Self> {
        let base_dir = base_dir.as_ref();
        fs::create_dir_all(base_dir)
            .map_err(|e| VexError::Storage(format!("failed to create telemetry dir: {e}")))?;

        Ok(Self {
            records_path: base_dir.join("network-records.jsonl"),
            stats_path: base_dir.join("network-stats.json"),
        })
    }

    pub fn append_record(&self, record: &NetworkRecord) -> VexResult<()> {
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.records_path)
            .map_err(|e| VexError::Storage(format!("open telemetry records failed: {e}")))?;

        let line = serde_json::to_string(record)
            .map_err(|e| VexError::Storage(format!("serialize telemetry record failed: {e}")))?;
        f.write_all(line.as_bytes())
            .and_then(|_| f.write_all(b"\n"))
            .map_err(|e| VexError::Storage(format!("write telemetry record failed: {e}")))
    }

    pub fn write_stats(&self, stats: &NetworkStats) -> VexResult<()> {
        let bytes = serde_json::to_vec_pretty(stats)
            .map_err(|e| VexError::Storage(format!("serialize telemetry stats failed: {e}")))?;
        fs::write(&self.stats_path, bytes)
            .map_err(|e| VexError::Storage(format!("write telemetry stats failed: {e}")))
    }

    pub fn records_path(&self) -> &Path {
        &self.records_path
    }

    pub fn stats_path(&self) -> &Path {
        &self.stats_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::{CacheOutcome, NetworkTimings};

    #[test]
    fn telemetry_store_writes_record_and_stats() {
        let temp = std::env::temp_dir().join(format!(
            "vigo-telemetry-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&temp);

        let store = TelemetryStore::new(&temp).expect("create store");
        let record = NetworkRecord {
            request_id: 1,
            method: "GET".to_string(),
            url: "https://example.com".to_string(),
            status: Some(200),
            cache_outcome: CacheOutcome::Miss,
            was_cached: false,
            timings: NetworkTimings {
                dns_ms: Some(1),
                ttfb_ms: Some(2),
                body_read_ms: Some(3),
                total_ms: 6,
            },
            error: None,
        };

        store.append_record(&record).expect("append record");
        store
            .write_stats(&NetworkStats::default())
            .expect("write stats");

        assert!(store.records_path().exists());
        assert!(store.stats_path().exists());

        let _ = fs::remove_dir_all(&temp);
    }
}

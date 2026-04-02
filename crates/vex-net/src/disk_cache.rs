// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Persistent HTTP cache with tiered eviction.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use vex_core::{VexError, VexResult, VexUrl};

use crate::cache::{
    parse_cache_control, CachedResponse, CacheDirectives, INTERNAL_PARTITION_HEADER,
};

#[derive(Debug, Clone)]
pub struct DiskCacheConfig {
    pub dir: PathBuf,
    pub max_bytes: u64,
}

impl Default for DiskCacheConfig {
    fn default() -> Self {
        Self {
            dir: std::env::temp_dir().join("vigo-http-cache"),
            max_bytes: 512 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiskEntryMeta {
    key: String,
    url: String,
    partition: String,
    status: u16,
    headers: HashMap<String, String>,
    etag: Option<String>,
    last_modified: Option<String>,
    directives: CacheDirectives,
    freshness_lifetime_secs: Option<u64>,
    stored_unix_secs: u64,
    last_access_unix_secs: u64,
    access_count: u64,
    body_len: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct DiskIndex {
    entries: HashMap<String, DiskEntryMeta>,
}

pub struct DiskCache {
    cfg: DiskCacheConfig,
    index: DiskIndex,
}

impl DiskCache {
    pub fn new(cfg: DiskCacheConfig) -> VexResult<Self> {
        fs::create_dir_all(&cfg.dir)
            .map_err(|e| VexError::Storage(format!("create disk cache dir failed: {e}")))?;

        let mut this = Self {
            cfg,
            index: DiskIndex::default(),
        };
        this.load_index()?;
        Ok(this)
    }

    pub fn get(
        &mut self,
        url: &VexUrl,
        request_headers: &HashMap<String, String>,
    ) -> Option<CachedResponse> {
        let key = cache_key(url, request_headers);
        let snapshot = {
            let meta = self.index.entries.get_mut(&key)?;

            if let Some(ttl) = meta.freshness_lifetime_secs {
                let age = now_unix_secs().saturating_sub(meta.stored_unix_secs);
                if age >= ttl {
                    return None;
                }
            } else {
                return None;
            }

            meta.access_count = meta.access_count.saturating_add(1);
            meta.last_access_unix_secs = now_unix_secs();
            meta.clone()
        };

        let body_path = self.body_path(&snapshot.key);
        let body = fs::read(&body_path).ok()?;
        let _ = self.persist_index();

        let age_secs = now_unix_secs().saturating_sub(snapshot.stored_unix_secs);
        let stored_at = Instant::now() - Duration::from_secs(age_secs);

        Some(CachedResponse {
            body,
            headers: snapshot.headers,
            status: snapshot.status,
            etag: snapshot.etag,
            last_modified: snapshot.last_modified,
            directives: snapshot.directives,
            freshness_lifetime_secs: snapshot.freshness_lifetime_secs,
            vary_on: vec![INTERNAL_PARTITION_HEADER.to_string()],
            vary_values: HashMap::from([(
                INTERNAL_PARTITION_HEADER.to_string(),
                snapshot.partition,
            )]),
            stored_at,
        })
    }

    pub fn store(
        &mut self,
        url: &VexUrl,
        request_headers: &HashMap<String, String>,
        status: u16,
        headers: &HashMap<String, String>,
        body: &[u8],
    ) -> VexResult<()> {
        let directives = headers
            .get("cache-control")
            .map(|v| parse_cache_control(v))
            .unwrap_or_default();
        if directives.no_store {
            return Ok(());
        }

        let key = cache_key(url, request_headers);
        let partition = partition_value(request_headers);
        let freshness_lifetime_secs = compute_freshness_lifetime_secs(headers, &directives);

        let body_path = self.body_path(&key);
        fs::write(&body_path, body)
            .map_err(|e| VexError::Storage(format!("write disk cache body failed: {e}")))?;

        let now = now_unix_secs();
        self.index.entries.insert(
            key.clone(),
            DiskEntryMeta {
                key,
                url: url.inner().as_str().to_string(),
                partition,
                status,
                headers: headers.clone(),
                etag: headers.get("etag").cloned(),
                last_modified: headers.get("last-modified").cloned(),
                directives,
                freshness_lifetime_secs,
                stored_unix_secs: now,
                last_access_unix_secs: now,
                access_count: 1,
                body_len: body.len() as u64,
            },
        );

        self.evict_tiered_if_needed()?;
        self.persist_index()?;
        Ok(())
    }

    pub fn refresh_from_not_modified(
        &mut self,
        url: &VexUrl,
        request_headers: &HashMap<String, String>,
        response_headers: &HashMap<String, String>,
    ) -> VexResult<bool> {
        let key = cache_key(url, request_headers);
        let Some(meta) = self.index.entries.get_mut(&key) else {
            return Ok(false);
        };

        for (k, v) in response_headers {
            meta.headers.insert(k.clone(), v.clone());
        }
        if let Some(etag) = response_headers.get("etag") {
            meta.etag = Some(etag.clone());
        }
        if let Some(lm) = response_headers.get("last-modified") {
            meta.last_modified = Some(lm.clone());
        }
        if let Some(cc) = response_headers.get("cache-control") {
            meta.directives = parse_cache_control(cc);
        }
        meta.freshness_lifetime_secs = compute_freshness_lifetime_secs(&meta.headers, &meta.directives);
        meta.stored_unix_secs = now_unix_secs();

        self.persist_index()?;
        Ok(true)
    }

    fn load_index(&mut self) -> VexResult<()> {
        let path = self.index_path();
        if !path.exists() {
            return Ok(());
        }

        let bytes = fs::read(&path)
            .map_err(|e| VexError::Storage(format!("read disk cache index failed: {e}")))?;
        self.index = serde_json::from_slice(&bytes)
            .map_err(|e| VexError::Storage(format!("parse disk cache index failed: {e}")))?;
        Ok(())
    }

    fn persist_index(&self) -> VexResult<()> {
        let bytes = serde_json::to_vec_pretty(&self.index)
            .map_err(|e| VexError::Storage(format!("serialize disk cache index failed: {e}")))?;
        fs::write(self.index_path(), bytes)
            .map_err(|e| VexError::Storage(format!("write disk cache index failed: {e}")))
    }

    fn evict_tiered_if_needed(&mut self) -> VexResult<()> {
        let mut total = self
            .index
            .entries
            .values()
            .map(|m| m.body_len)
            .sum::<u64>();
        if total <= self.cfg.max_bytes {
            return Ok(());
        }

        let mut keys: Vec<(String, u8, u64)> = self
            .index
            .entries
            .iter()
            .map(|(k, m)| {
                let tier = if m.access_count <= 1 {
                    0 // cold
                } else if m.access_count <= 5 {
                    1 // warm
                } else {
                    2 // hot
                };
                (k.clone(), tier, m.last_access_unix_secs)
            })
            .collect();

        // cold first, then warm, then hot; oldest first inside each tier
        keys.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.2.cmp(&b.2)));

        for (key, _, _) in keys {
            if total <= self.cfg.max_bytes {
                break;
            }
            if let Some(meta) = self.index.entries.remove(&key) {
                total = total.saturating_sub(meta.body_len);
                let _ = fs::remove_file(self.body_path(&meta.key));
            }
        }

        Ok(())
    }

    fn index_path(&self) -> PathBuf {
        self.cfg.dir.join("index.json")
    }

    fn body_path(&self, key: &str) -> PathBuf {
        self.cfg.dir.join(format!("{key}.bin"))
    }
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn partition_value(headers: &HashMap<String, String>) -> String {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(INTERNAL_PARTITION_HEADER))
        .map(|(_, v)| v.clone())
        .unwrap_or_default()
}

fn cache_key(url: &VexUrl, request_headers: &HashMap<String, String>) -> String {
    let partition = partition_value(request_headers);

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(&url.inner().as_str(), &mut hasher);
    std::hash::Hash::hash(&partition, &mut hasher);
    format!("{:016x}", std::hash::Hasher::finish(&hasher))
}

fn compute_freshness_lifetime_secs(
    headers: &HashMap<String, String>,
    directives: &CacheDirectives,
) -> Option<u64> {
    let age = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("age"))
        .and_then(|(_, v)| v.trim().parse::<u64>().ok())
        .unwrap_or(0);

    if let Some(max_age) = directives.max_age {
        return Some(max_age.saturating_sub(age));
    }

    let expires = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("expires"))
        .and_then(|(_, v)| httpdate::parse_http_date(v).ok())?;
    let now = SystemTime::now();
    let ttl = expires.duration_since(now).ok()?.as_secs();
    Some(ttl.saturating_sub(age))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disk_cache_roundtrip() {
        let dir = std::env::temp_dir().join(format!("vigo-disk-cache-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);

        let mut cache = DiskCache::new(DiskCacheConfig {
            dir: dir.clone(),
            max_bytes: 32 * 1024,
        })
        .expect("create cache");

        let url = VexUrl::parse("https://example.com/").unwrap();
        let req = HashMap::new();
        let headers = HashMap::from([("cache-control".to_string(), "max-age=60".to_string())]);

        cache
            .store(&url, &req, 200, &headers, b"hello")
            .expect("store");
        let hit = cache.get(&url, &req).expect("cache hit");
        assert_eq!(hit.status, 200);
        assert_eq!(hit.body, b"hello");

        let _ = fs::remove_dir_all(&dir);
    }
}

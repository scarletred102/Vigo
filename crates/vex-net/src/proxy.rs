// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Proxy and PAC policy configuration.

use vex_core::VexUrl;

#[derive(Debug, Clone, Default)]
pub struct ProxyConfig {
    pub http_proxy: Option<String>,
    pub https_proxy: Option<String>,
    pub no_proxy: Vec<String>,
    pub pac_url: Option<String>,
}

impl ProxyConfig {
    pub fn from_env() -> Self {
        let http_proxy = std::env::var("HTTP_PROXY")
            .ok()
            .or_else(|| std::env::var("http_proxy").ok());
        let https_proxy = std::env::var("HTTPS_PROXY")
            .ok()
            .or_else(|| std::env::var("https_proxy").ok());
        let no_proxy = std::env::var("NO_PROXY")
            .ok()
            .or_else(|| std::env::var("no_proxy").ok())
            .map(|v| {
                v.split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(ToString::to_string)
                    .collect()
            })
            .unwrap_or_default();
        let pac_url = std::env::var("VIGO_PROXY_PAC").ok();

        Self {
            http_proxy,
            https_proxy,
            no_proxy,
            pac_url,
        }
    }

    pub fn proxy_for(&self, url: &VexUrl) -> Option<&str> {
        let host = url.host().unwrap_or_default();
        if self
            .no_proxy
            .iter()
            .any(|entry| host == entry || host.ends_with(&format!(".{entry}")))
        {
            return None;
        }

        match url.scheme() {
            "https" => self.https_proxy.as_deref().or(self.http_proxy.as_deref()),
            "http" => self.http_proxy.as_deref(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_proxy_bypasses_proxy() {
        let cfg = ProxyConfig {
            http_proxy: Some("http://proxy.local:8080".to_string()),
            https_proxy: None,
            no_proxy: vec!["example.com".to_string()],
            pac_url: None,
        };

        let url = VexUrl::parse("http://www.example.com/").unwrap();
        assert!(cfg.proxy_for(&url).is_none());
    }
}

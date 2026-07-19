// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Proxy and PAC policy configuration.

use std::fs;

use boa_engine::{Context, Source};
use vex_core::VexUrl;

#[derive(Debug, Clone, Default)]
pub struct ProxyConfig {
    pub http_proxy: Option<String>,
    pub https_proxy: Option<String>,
    pub no_proxy: Vec<String>,
    pub pac_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyDirective {
    Direct,
    Proxy(String),
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

    /// Resolve proxy directive using no_proxy + PAC + static proxy settings.
    pub fn resolve_proxy(&self, url: &VexUrl) -> ProxyDirective {
        let host = url.host().unwrap_or_default();
        if self
            .no_proxy
            .iter()
            .any(|entry| host == entry || host.ends_with(&format!(".{entry}")))
        {
            return ProxyDirective::Direct;
        }

        if let Some(ref pac_url) = self.pac_url {
            if let Some(script) = load_pac_script(pac_url) {
                if let Some(result) = evaluate_pac(&script, url) {
                    return result;
                }
            }
        }

        self.proxy_for(url)
            .map(|p| ProxyDirective::Proxy(p.to_string()))
            .unwrap_or(ProxyDirective::Direct)
    }
}

fn load_pac_script(pac_url: &str) -> Option<String> {
    if let Some(path) = pac_url.strip_prefix("file://") {
        return fs::read_to_string(path).ok();
    }

    if pac_url.starts_with("http://") || pac_url.starts_with("https://") {
        let resp = reqwest::blocking::get(pac_url).ok()?;
        return resp.text().ok();
    }

    fs::read_to_string(pac_url).ok()
}

fn evaluate_pac(script: &str, url: &VexUrl) -> Option<ProxyDirective> {
    let host = url.host().unwrap_or_default();

    let mut ctx = Context::default();
    ctx.eval(Source::from_bytes(script)).ok()?;

    let js = format!(
        "typeof FindProxyForURL === 'function' ? FindProxyForURL({:?}, {:?}) : 'DIRECT'",
        url.inner().as_str(),
        host
    );

    let result = ctx.eval(Source::from_bytes(js.as_bytes())).ok()?;
    let result = result.to_string(&mut ctx).ok()?.to_std_string_escaped();
    parse_pac_result(&result)
}

fn parse_pac_result(result: &str) -> Option<ProxyDirective> {
    // Typical values: "PROXY host:port; DIRECT"
    for part in result.split(';').map(str::trim) {
        let upper = part.to_ascii_uppercase();
        if upper == "DIRECT" {
            return Some(ProxyDirective::Direct);
        }
        if let Some(proxy) = part.strip_prefix("PROXY ") {
            let proxy = proxy.trim();
            if !proxy.is_empty() {
                return Some(ProxyDirective::Proxy(format!("http://{proxy}")));
            }
        }
        if let Some(proxy) = part.strip_prefix("HTTPS ") {
            let proxy = proxy.trim();
            if !proxy.is_empty() {
                return Some(ProxyDirective::Proxy(format!("https://{proxy}")));
            }
        }
        if let Some(proxy) = part.strip_prefix("SOCKS5 ") {
            let proxy = proxy.trim();
            if !proxy.is_empty() {
                return Some(ProxyDirective::Proxy(format!("socks5://{proxy}")));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

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

    #[test]
    fn parse_pac_directive() {
        let r = parse_pac_result("PROXY proxy.local:8080; DIRECT");
        assert_eq!(
            r,
            Some(ProxyDirective::Proxy("http://proxy.local:8080".to_string()))
        );

        let r = parse_pac_result("DIRECT");
        assert_eq!(r, Some(ProxyDirective::Direct));
    }

    #[test]
    fn pac_file_is_evaluated() {
        let tmp = std::env::temp_dir().join(format!("vigo-pac-{}.pac", std::process::id()));
        fs::write(
            &tmp,
            r#"
            function FindProxyForURL(url, host) {
                if (host.indexOf("example.com") >= 0) return "PROXY pac.local:8899; DIRECT";
                return "DIRECT";
            }
            "#,
        )
        .expect("write pac");

        let cfg = ProxyConfig {
            http_proxy: None,
            https_proxy: None,
            no_proxy: vec![],
            pac_url: Some(format!("file://{}", tmp.display())),
        };

        let url = VexUrl::parse("https://www.example.com/").unwrap();
        let directive = cfg.resolve_proxy(&url);
        assert_eq!(
            directive,
            ProxyDirective::Proxy("http://pac.local:8899".to_string())
        );

        let _ = fs::remove_file(tmp);
    }
}

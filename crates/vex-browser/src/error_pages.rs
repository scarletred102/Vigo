// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Error page templates — simple HTML strings rendered in-tab when navigation fails.

/// Build an error page HTML document.
pub fn build_error_page(title: &str, message: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head><title>{title}</title>
<style>
body {{
    background: #1a1a24;
    color: #d0d0e0;
    font-family: sans-serif;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 80vh;
    margin: 0;
    padding: 40px;
}}
h1 {{ color: #c44; font-size: 28px; margin-bottom: 12px; }}
p {{ font-size: 16px; color: #999; max-width: 600px; text-align: center; }}
.code {{ font-family: monospace; background: #2a2a36; padding: 12px 20px; border-radius: 6px; margin-top: 20px; font-size: 13px; color: #888; word-break: break-all; max-width: 600px; }}
</style>
</head>
<body>
<h1>{title}</h1>
<p>{message}</p>
</body>
</html>"#,
        title = html_escape(title),
        message = html_escape(message),
    )
}

/// Build a DNS error page.
pub fn dns_error_page(host: &str) -> String {
    build_error_page(
        "Can't Find That Page",
        &format!(
            "The DNS lookup for <b>{}</b> failed. Check the URL or your network connection.",
            html_escape(host)
        ),
    )
}

/// Build a connection-refused error page.
pub fn connection_refused_page(url: &str) -> String {
    build_error_page(
        "Connection Refused",
        &format!(
            "The server at <b>{}</b> refused the connection.",
            html_escape(url)
        ),
    )
}

/// Build a TLS error page.
pub fn tls_error_page(host: &str) -> String {
    build_error_page(
        "Your Connection Is Not Private",
        &format!(
            "The certificate for <b>{}</b> is not trusted. Proceed with caution.",
            html_escape(host)
        ),
    )
}

/// Build a generic HTTP error page (404, 500, etc.).
pub fn http_error_page(status: u16, url: &str) -> String {
    let title = match status {
        404 => "Page Not Found (404)",
        500 => "Internal Server Error (500)",
        502 => "Bad Gateway (502)",
        503 => "Service Unavailable (503)",
        _ => "Server Error",
    };
    build_error_page(
        title,
        &format!("The server returned an error for {}", html_escape(url)),
    )
}

/// Minimal HTML entity escaping.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_page_contains_title_and_message() {
        let page = build_error_page("Test Error", "Something went wrong");
        assert!(page.contains("<title>Test Error</title>"));
        assert!(page.contains("Something went wrong"));
    }

    #[test]
    fn dns_error_page_mentions_host() {
        let page = dns_error_page("example.com");
        assert!(page.contains("example.com"));
        assert!(page.contains("DNS"));
    }

    #[test]
    fn http_error_page_for_404() {
        let page = http_error_page(404, "https://example.com/missing");
        assert!(page.contains("404"));
        assert!(page.contains("example.com"));
    }

    #[test]
    fn html_escape_works() {
        assert_eq!(
            html_escape("<script>&\"test"),
            "&lt;script&gt;&amp;&quot;test"
        );
    }
}

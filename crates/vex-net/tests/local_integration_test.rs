// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Deterministic local integration tests for vex-net.

use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use vex_net::{HttpClient, Method, Request};

struct LocalServer {
    addr: std::net::SocketAddr,
    requests: Arc<StdMutex<Vec<String>>>,
}

impl LocalServer {
    async fn spawn(responses: Vec<Vec<u8>>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind localhost");
        let addr = listener.local_addr().expect("local addr");
        let requests = Arc::new(StdMutex::new(Vec::new()));
        let requests_bg = Arc::clone(&requests);

        tokio::spawn(async move {
            for response in responses {
                let (mut socket, _) = listener.accept().await.expect("accept connection");

                let mut buf = vec![0u8; 16 * 1024];
                let n = socket.read(&mut buf).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                if let Ok(mut lock) = requests_bg.lock() {
                    lock.push(req);
                }

                socket
                    .write_all(&response)
                    .await
                    .expect("write response bytes");
                let _ = socket.shutdown().await;
            }
        });

        Self { addr, requests }
    }

    fn url(&self, path: &str) -> String {
        format!("http://{}:{}{path}", self.addr.ip(), self.addr.port())
    }

    fn requests(&self) -> Vec<String> {
        self.requests.lock().map(|v| v.clone()).unwrap_or_default()
    }
}

fn http_response(status_line: &str, headers: &[(&str, &str)], body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(format!("HTTP/1.1 {status_line}\r\n").as_bytes());
    for (k, v) in headers {
        out.extend_from_slice(format!("{k}: {v}\r\n").as_bytes());
    }
    out.extend_from_slice(format!("Content-Length: {}\r\n", body.len()).as_bytes());
    out.extend_from_slice(b"Connection: close\r\n\r\n");
    out.extend_from_slice(body);
    out
}

#[tokio::test]
async fn local_redirect_following_works() {
    let first = http_response("302 Found", &[("Location", "/final")], b"");
    let second = http_response("200 OK", &[], b"hello");
    let server = LocalServer::spawn(vec![first, second]).await;

    let client = HttpClient::new().expect("client init");
    let request = Request::get(&server.url("/start")).expect("request build");

    let response = client.fetch(request).await.expect("fetch success");
    assert_eq!(response.status, 200);
    assert_eq!(response.body, b"hello");

    let requests = server.requests();
    assert!(requests.len() >= 2);
    assert!(requests[0].starts_with("GET /start"));
    assert!(requests[1].starts_with("GET /final"));
}

#[tokio::test]
async fn local_cache_revalidation_sends_if_none_match() {
    let first = http_response(
        "200 OK",
        &[("Cache-Control", "max-age=0"), ("ETag", "\"v1\"")],
        b"cached-body",
    );
    let second = http_response("304 Not Modified", &[("Cache-Control", "max-age=120")], b"");

    let server = LocalServer::spawn(vec![first, second]).await;
    let url = server.url("/etag");

    let client = HttpClient::new().expect("client init");
    let request = Request::get(&url).expect("request build");

    let first_resp = client.fetch(request.clone()).await.expect("first fetch");
    assert_eq!(first_resp.status, 200);

    let second_resp = client.fetch(request).await.expect("second fetch");
    assert!(second_resp.was_cached);
    assert_eq!(second_resp.body, b"cached-body");

    let requests = server.requests();
    assert!(requests.len() >= 2);
    assert!(
        requests[1]
            .to_ascii_lowercase()
            .contains("if-none-match: \"v1\""),
        "expected conditional revalidation request, got:\n{}",
        requests[1]
    );
}

#[tokio::test]
async fn local_gzip_response_is_decompressed() {
    use std::io::Write;

    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder
        .write_all(b"compressed-local")
        .expect("write compressed payload");
    let gz = encoder.finish().expect("finish gzip");

    let response = http_response("200 OK", &[("Content-Encoding", "gzip")], &gz);
    let server = LocalServer::spawn(vec![response]).await;

    let client = HttpClient::new().expect("client init");
    let request = Request::get(&server.url("/gzip")).expect("request build");

    let resp = client.fetch(request).await.expect("fetch success");
    assert_eq!(resp.status, 200);
    assert_eq!(resp.body, b"compressed-local");
}

#[tokio::test]
async fn local_stale_if_error_uses_cached_copy_when_origin_down() {
    let first = http_response(
        "200 OK",
        &[("Cache-Control", "max-age=0, stale-if-error=60")],
        b"stale-ok",
    );

    let server = LocalServer::spawn(vec![first]).await;
    let url = server.url("/stale");

    let client = HttpClient::new().expect("client init");
    let req = Request::get(&url).expect("request build");

    let initial = client.fetch(req.clone()).await.expect("first fetch");
    assert_eq!(initial.status, 200);
    assert!(!initial.was_cached);

    // The server was configured with one response and is now down.
    // The second fetch should fail network and fall back to stale-if-error cache.
    let fallback = client.fetch(req).await.expect("stale-if-error fallback");
    assert!(fallback.was_cached);
    assert_eq!(fallback.body, b"stale-ok");
}

#[tokio::test]
async fn local_vary_cache_keeps_variants_separate() {
    let en = http_response(
        "200 OK",
        &[
            ("Cache-Control", "max-age=3600"),
            ("Vary", "Accept-Language"),
        ],
        b"hello",
    );
    let fr = http_response(
        "200 OK",
        &[
            ("Cache-Control", "max-age=3600"),
            ("Vary", "Accept-Language"),
        ],
        b"bonjour",
    );

    let server = LocalServer::spawn(vec![en, fr]).await;
    let url = server.url("/vary");

    let client = HttpClient::new().expect("client init");

    let mut en_req = Request::get(&url).expect("request build");
    en_req
        .headers
        .insert("Accept-Language".to_string(), "en".to_string());

    let mut fr_req = Request {
        url: en_req.url.clone(),
        method: Method::Get,
        headers: HashMap::new(),
        body: None,
    };
    fr_req
        .headers
        .insert("Accept-Language".to_string(), "fr".to_string());

    let first_en = client.fetch(en_req.clone()).await.expect("en fetch");
    let first_fr = client.fetch(fr_req.clone()).await.expect("fr fetch");

    assert_eq!(first_en.body, b"hello");
    assert_eq!(first_fr.body, b"bonjour");

    // Server has no more programmed responses. These should come from cache
    // with the correct Vary variant selected.
    let cached_en = client.fetch(en_req).await.expect("en cached fetch");
    let cached_fr = client.fetch(fr_req).await.expect("fr cached fetch");

    assert!(cached_en.was_cached);
    assert!(cached_fr.was_cached);
    assert_eq!(cached_en.body, b"hello");
    assert_eq!(cached_fr.body, b"bonjour");
}

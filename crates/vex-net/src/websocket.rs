// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! WebSocket client (RFC 6455) built on `tokio-tungstenite`.
//!
//! Provides a high-level API for connecting to WebSocket endpoints,
//! sending/receiving messages, and handling close frames.

use std::fmt;

use thiserror::Error;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite;

/// A WebSocket message (text or binary).
#[derive(Debug, Clone, PartialEq)]
pub enum WsMessage {
    Text(String),
    Binary(Vec<u8>),
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    Close(Option<CloseFrame>),
}

/// A WebSocket close frame with code and reason.
#[derive(Debug, Clone, PartialEq)]
pub struct CloseFrame {
    pub code: u16,
    pub reason: String,
}

impl fmt::Display for CloseFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CloseFrame({}, {:?})", self.code, self.reason)
    }
}

/// WebSocket ready state (mirrors the JS WebSocket.readyState).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ReadyState {
    Connecting = 0,
    Open = 1,
    Closing = 2,
    Closed = 3,
}

impl fmt::Display for ReadyState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connecting => write!(f, "CONNECTING"),
            Self::Open => write!(f, "OPEN"),
            Self::Closing => write!(f, "CLOSING"),
            Self::Closed => write!(f, "CLOSED"),
        }
    }
}

/// Errors from WebSocket operations.
#[derive(Debug, Error)]
pub enum WsError {
    #[error("connection failed: {0}")]
    ConnectionFailed(String),
    #[error("send failed: {0}")]
    SendFailed(String),
    #[error("receive failed: {0}")]
    ReceiveFailed(String),
    #[error("invalid URL: {0}")]
    InvalidUrl(String),
    #[error("not connected")]
    NotConnected,
    #[error("already connected")]
    AlreadyConnected,
    #[error("protocol error: {0}")]
    Protocol(String),
}

pub type WsResult<T> = Result<T, WsError>;

impl From<tungstenite::Error> for WsError {
    fn from(e: tungstenite::Error) -> Self {
        Self::ConnectionFailed(e.to_string())
    }
}

/// Configuration for a WebSocket connection.
#[derive(Debug, Clone)]
pub struct WsConfig {
    /// Subprotocols to request during handshake.
    pub protocols: Vec<String>,
    /// Maximum message size in bytes (0 = unlimited).
    pub max_message_size: usize,
    /// Origin header value.
    pub origin: Option<String>,
}

impl Default for WsConfig {
    fn default() -> Self {
        Self {
            protocols: Vec::new(),
            max_message_size: 64 * 1024 * 1024, // 64 MB
            origin: None,
        }
    }
}

/// A WebSocket connection handle.
///
/// Wraps a `tokio-tungstenite` connection with an async sender/receiver
/// model. The actual I/O runs in a spawned task; this handle provides
/// `send()` and `recv()` methods.
pub struct WebSocket {
    url: String,
    state: ReadyState,
    send_tx: Option<mpsc::Sender<WsMessage>>,
    recv_rx: Option<mpsc::Receiver<WsMessage>>,
    protocol: Option<String>,
    buffered_amount: usize,
}

impl WebSocket {
    /// Create a new WebSocket that is not yet connected.
    pub fn new(url: &str) -> WsResult<Self> {
        validate_ws_url(url)?;
        Ok(Self {
            url: url.to_owned(),
            state: ReadyState::Closed,
            send_tx: None,
            recv_rx: None,
            protocol: None,
            buffered_amount: 0,
        })
    }

    /// Connect to the WebSocket server.
    ///
    /// This spawns a background task that handles the I/O.
    pub async fn connect(&mut self, config: &WsConfig) -> WsResult<()> {
        if self.state == ReadyState::Open {
            return Err(WsError::AlreadyConnected);
        }

        self.state = ReadyState::Connecting;

        let mut request = tungstenite::http::Request::builder()
            .uri(&self.url)
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket");

        if let Some(origin) = &config.origin {
            request = request.header("Origin", origin.as_str());
        }

        if !config.protocols.is_empty() {
            let proto_str = config.protocols.join(", ");
            request = request.header("Sec-WebSocket-Protocol", proto_str.as_str());
        }

        let request = request
            .body(())
            .map_err(|e| WsError::ConnectionFailed(e.to_string()))?;

        let (ws_stream, response) =
            tokio_tungstenite::connect_async(request)
                .await
                .map_err(|e| WsError::ConnectionFailed(e.to_string()))?;

        // Extract negotiated protocol.
        self.protocol = response
            .headers()
            .get("Sec-WebSocket-Protocol")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        let (write, read) = futures_util_split(ws_stream);

        // Channel for outgoing messages.
        let (send_tx, mut send_rx) = mpsc::channel::<WsMessage>(64);
        // Channel for incoming messages.
        let (recv_tx, recv_rx) = mpsc::channel::<WsMessage>(64);

        // Spawn writer task.
        tokio::spawn(async move {
            use futures_util::SinkExt;
            let mut write = write;
            while let Some(msg) = send_rx.recv().await {
                let tung_msg = ws_to_tungstenite(msg);
                if write.send(tung_msg).await.is_err() {
                    break;
                }
            }
        });

        // Spawn reader task.
        let max_size = config.max_message_size;
        tokio::spawn(async move {
            use futures_util::StreamExt;
            let mut read = read;
            while let Some(result) = read.next().await {
                match result {
                    Ok(msg) => {
                        let ws_msg = tungstenite_to_ws(msg);
                        if max_size > 0 {
                            if let WsMessage::Binary(ref data) = ws_msg {
                                if data.len() > max_size {
                                    continue;
                                }
                            }
                            if let WsMessage::Text(ref text) = ws_msg {
                                if text.len() > max_size {
                                    continue;
                                }
                            }
                        }
                        if recv_tx.send(ws_msg).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        self.send_tx = Some(send_tx);
        self.recv_rx = Some(recv_rx);
        self.state = ReadyState::Open;

        Ok(())
    }

    /// Send a message.
    pub async fn send(&mut self, msg: WsMessage) -> WsResult<()> {
        if self.state != ReadyState::Open {
            return Err(WsError::NotConnected);
        }
        let size = match &msg {
            WsMessage::Text(t) => t.len(),
            WsMessage::Binary(b) => b.len(),
            _ => 0,
        };
        self.buffered_amount += size;
        if let Some(tx) = &self.send_tx {
            tx.send(msg)
                .await
                .map_err(|e| WsError::SendFailed(e.to_string()))?;
        }
        self.buffered_amount = self.buffered_amount.saturating_sub(size);
        Ok(())
    }

    /// Send a text message (convenience).
    pub async fn send_text(&mut self, text: &str) -> WsResult<()> {
        self.send(WsMessage::Text(text.to_owned())).await
    }

    /// Send a binary message (convenience).
    pub async fn send_binary(&mut self, data: Vec<u8>) -> WsResult<()> {
        self.send(WsMessage::Binary(data)).await
    }

    /// Receive the next message, or `None` if closed.
    pub async fn recv(&mut self) -> Option<WsMessage> {
        if let Some(rx) = &mut self.recv_rx {
            let msg = rx.recv().await;
            if msg.is_none() {
                self.state = ReadyState::Closed;
            }
            msg
        } else {
            None
        }
    }

    /// Close the connection with an optional close frame.
    pub async fn close(&mut self, code: u16, reason: &str) -> WsResult<()> {
        if self.state != ReadyState::Open {
            return Ok(());
        }
        self.state = ReadyState::Closing;
        self.send(WsMessage::Close(Some(CloseFrame {
            code,
            reason: reason.to_owned(),
        })))
        .await
        .ok();
        // Drop channels.
        self.send_tx = None;
        self.recv_rx = None;
        self.state = ReadyState::Closed;
        Ok(())
    }

    /// Current ready state.
    pub fn ready_state(&self) -> ReadyState {
        self.state
    }

    /// URL this WebSocket is connected to.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// The negotiated subprotocol (if any).
    pub fn protocol(&self) -> Option<&str> {
        self.protocol.as_deref()
    }

    /// Approximate buffered amount of outgoing data.
    pub fn buffered_amount(&self) -> usize {
        self.buffered_amount
    }
}

/// Validate that a URL uses ws:// or wss:// scheme.
fn validate_ws_url(url: &str) -> WsResult<()> {
    if url.starts_with("ws://") || url.starts_with("wss://") {
        Ok(())
    } else {
        Err(WsError::InvalidUrl(format!(
            "expected ws:// or wss:// URL, got: {url}"
        )))
    }
}

/// Convert our `WsMessage` to tungstenite's `Message`.
fn ws_to_tungstenite(msg: WsMessage) -> tungstenite::Message {
    match msg {
        WsMessage::Text(t) => tungstenite::Message::Text(t),
        WsMessage::Binary(b) => tungstenite::Message::Binary(b),
        WsMessage::Ping(p) => tungstenite::Message::Ping(p),
        WsMessage::Pong(p) => tungstenite::Message::Pong(p),
        WsMessage::Close(Some(f)) => tungstenite::Message::Close(Some(
            tungstenite::protocol::CloseFrame {
                code: tungstenite::protocol::frame::coding::CloseCode::from(f.code),
                reason: f.reason.into(),
            },
        )),
        WsMessage::Close(None) => tungstenite::Message::Close(None),
    }
}

/// Convert tungstenite's `Message` to our `WsMessage`.
fn tungstenite_to_ws(msg: tungstenite::Message) -> WsMessage {
    match msg {
        tungstenite::Message::Text(t) => WsMessage::Text(t),
        tungstenite::Message::Binary(b) => WsMessage::Binary(b),
        tungstenite::Message::Ping(p) => WsMessage::Ping(p),
        tungstenite::Message::Pong(p) => WsMessage::Pong(p),
        tungstenite::Message::Close(Some(f)) => WsMessage::Close(Some(CloseFrame {
            code: f.code.into(),
            reason: f.reason.to_string(),
        })),
        tungstenite::Message::Close(None) => WsMessage::Close(None),
        tungstenite::Message::Frame(_) => WsMessage::Binary(Vec::new()),
    }
}

/// Split a WebSocket stream into write/read halves.
///
/// This is a thin wrapper because we need the futures_util StreamExt/SinkExt
/// that come via tungstenite.
fn futures_util_split<S>(
    stream: S,
) -> (
    futures_util::stream::SplitSink<S, tungstenite::Message>,
    futures_util::stream::SplitStream<S>,
)
where
    S: futures_util::Sink<tungstenite::Message> + futures_util::Stream + Unpin,
{
    use futures_util::StreamExt;
    stream.split()
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_ws_urls() {
        assert!(validate_ws_url("ws://localhost:8080/chat").is_ok());
        assert!(validate_ws_url("wss://example.com/ws").is_ok());
        assert!(validate_ws_url("http://example.com").is_err());
        assert!(validate_ws_url("ftp://example.com").is_err());
        assert!(validate_ws_url("").is_err());
    }

    #[test]
    fn ws_new_valid_url() {
        let ws = WebSocket::new("ws://localhost:9090/echo").unwrap();
        assert_eq!(ws.ready_state(), ReadyState::Closed);
        assert_eq!(ws.url(), "ws://localhost:9090/echo");
        assert!(ws.protocol().is_none());
    }

    #[test]
    fn ws_new_invalid_url() {
        assert!(WebSocket::new("http://bad").is_err());
    }

    #[test]
    fn ready_state_display() {
        assert_eq!(ReadyState::Connecting.to_string(), "CONNECTING");
        assert_eq!(ReadyState::Open.to_string(), "OPEN");
        assert_eq!(ReadyState::Closing.to_string(), "CLOSING");
        assert_eq!(ReadyState::Closed.to_string(), "CLOSED");
    }

    #[test]
    fn close_frame_display() {
        let frame = CloseFrame {
            code: 1000,
            reason: "Normal".to_string(),
        };
        assert!(frame.to_string().contains("1000"));
        assert!(frame.to_string().contains("Normal"));
    }

    #[test]
    fn ws_config_defaults() {
        let config = WsConfig::default();
        assert!(config.protocols.is_empty());
        assert_eq!(config.max_message_size, 64 * 1024 * 1024);
        assert!(config.origin.is_none());
    }

    #[test]
    fn message_clone_eq() {
        let m1 = WsMessage::Text("hello".to_string());
        let m2 = m1.clone();
        assert_eq!(m1, m2);

        let m3 = WsMessage::Binary(vec![1, 2, 3]);
        assert_ne!(m1, m3);

        let m4 = WsMessage::Close(Some(CloseFrame {
            code: 1000,
            reason: "bye".to_string(),
        }));
        let m5 = m4.clone();
        assert_eq!(m4, m5);
    }

    #[test]
    fn message_conversion_roundtrip_text() {
        let msg = WsMessage::Text("hello world".to_string());
        let tung = ws_to_tungstenite(msg.clone());
        let back = tungstenite_to_ws(tung);
        assert_eq!(msg, back);
    }

    #[test]
    fn message_conversion_roundtrip_binary() {
        let msg = WsMessage::Binary(vec![0xDE, 0xAD, 0xBE, 0xEF]);
        let tung = ws_to_tungstenite(msg.clone());
        let back = tungstenite_to_ws(tung);
        assert_eq!(msg, back);
    }

    #[test]
    fn message_conversion_close() {
        let msg = WsMessage::Close(Some(CloseFrame {
            code: 1001,
            reason: "going away".to_string(),
        }));
        let tung = ws_to_tungstenite(msg.clone());
        let back = tungstenite_to_ws(tung);
        assert_eq!(msg, back);
    }

    #[test]
    fn message_conversion_close_none() {
        let msg = WsMessage::Close(None);
        let tung = ws_to_tungstenite(msg.clone());
        let back = tungstenite_to_ws(tung);
        assert_eq!(msg, back);
    }

    #[test]
    fn message_conversion_ping_pong() {
        let ping = WsMessage::Ping(vec![1, 2, 3]);
        let tung = ws_to_tungstenite(ping.clone());
        let back = tungstenite_to_ws(tung);
        assert_eq!(ping, back);

        let pong = WsMessage::Pong(vec![4, 5, 6]);
        let tung = ws_to_tungstenite(pong.clone());
        let back = tungstenite_to_ws(tung);
        assert_eq!(pong, back);
    }

    #[tokio::test]
    async fn send_not_connected() {
        let mut ws = WebSocket::new("ws://localhost:9999/test").unwrap();
        let result = ws.send(WsMessage::Text("hi".into())).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn recv_none_when_disconnected() {
        let mut ws = WebSocket::new("ws://localhost:9999/test").unwrap();
        assert!(ws.recv().await.is_none());
    }
}

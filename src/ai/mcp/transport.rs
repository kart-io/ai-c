//! MCP Transport Layer Implementation
//!
//! Provides multiple transport mechanisms for MCP communication:
//! - WebSocket transport for real-time bidirectional communication
//! - HTTP transport for request/response patterns
//! - Stdio transport for local process communication

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    process::{Child, Command, Stdio},
    sync::{mpsc, RwLock},
    time::timeout,
};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info, warn};
use url::Url;

use crate::error::{AppError, AppResult};
use super::{
    errors::{MCPError, MCPResult},
    protocol::{MCPMessage, MessageId},
};

/// Transport type enumeration
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransportType {
    /// WebSocket transport
    WebSocket,
    /// HTTP transport
    Http,
    /// Standard I/O transport
    Stdio,
}

/// Connection status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    /// Not connected
    Disconnected,
    /// Connecting
    Connecting,
    /// Connected and ready
    Connected,
    /// Connection failed
    Failed(String),
    /// Connection closed
    Closed,
}

/// Transport statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TransportStats {
    /// Total messages sent
    pub messages_sent: u64,
    /// Total messages received
    pub messages_received: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Connection attempts
    pub connection_attempts: u64,
    /// Successful connections
    pub successful_connections: u64,
    /// Failed connections
    pub failed_connections: u64,
    /// Last activity timestamp
    pub last_activity: Option<DateTime<Utc>>,
    /// Average message latency
    pub average_latency: Duration,
    /// Connection uptime
    pub uptime: Duration,
    /// Connection start time
    pub connection_start: Option<Instant>,
}

impl TransportStats {
    /// Update statistics for sent message
    pub fn update_sent(&mut self, message_size: usize) {
        self.messages_sent += 1;
        self.bytes_sent += message_size as u64;
        self.last_activity = Some(Utc::now());
    }

    /// Update statistics for received message
    pub fn update_received(&mut self, message_size: usize) {
        self.messages_received += 1;
        self.bytes_received += message_size as u64;
        self.last_activity = Some(Utc::now());
    }

    /// Update connection statistics
    pub fn update_connection(&mut self, success: bool) {
        self.connection_attempts += 1;
        if success {
            self.successful_connections += 1;
            self.connection_start = Some(Instant::now());
        } else {
            self.failed_connections += 1;
        }
    }

    /// Update uptime
    pub fn update_uptime(&mut self) {
        if let Some(start) = self.connection_start {
            self.uptime = start.elapsed();
        }
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        if self.connection_attempts == 0 {
            0.0
        } else {
            self.successful_connections as f64 / self.connection_attempts as f64
        }
    }

    /// Get throughput (messages per second)
    pub fn throughput(&self) -> f64 {
        let total_messages = self.messages_sent + self.messages_received;
        if self.uptime.as_secs() == 0 {
            0.0
        } else {
            total_messages as f64 / self.uptime.as_secs_f64()
        }
    }
}

/// Transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    /// Transport type
    pub transport_type: TransportType,
    /// Connection URL or endpoint
    pub endpoint: String,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Read timeout
    pub read_timeout: Duration,
    /// Write timeout
    pub write_timeout: Duration,
    /// Keep-alive interval
    pub keep_alive_interval: Option<Duration>,
    /// Maximum message size
    pub max_message_size: usize,
    /// Reconnection attempts
    pub max_reconnect_attempts: u32,
    /// Reconnection delay
    pub reconnect_delay: Duration,
    /// Enable compression
    pub enable_compression: bool,
    /// Additional headers (for HTTP/WebSocket)
    pub headers: HashMap<String, String>,
    /// TLS configuration
    pub tls_config: Option<TlsConfig>,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            transport_type: TransportType::WebSocket,
            endpoint: "ws://localhost:8080/mcp".to_string(),
            connect_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(10),
            keep_alive_interval: Some(Duration::from_secs(30)),
            max_message_size: 1024 * 1024, // 1MB
            max_reconnect_attempts: 3,
            reconnect_delay: Duration::from_secs(5),
            enable_compression: true,
            headers: HashMap::new(),
            tls_config: None,
        }
    }
}

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Enable TLS verification
    pub verify_certificates: bool,
    /// Certificate file path
    pub cert_file: Option<String>,
    /// Private key file path
    pub key_file: Option<String>,
    /// CA bundle file path
    pub ca_file: Option<String>,
}

/// MCP transport trait
#[async_trait]
pub trait MCPTransport: Send + Sync {
    /// Connect to the remote endpoint
    async fn connect(&mut self) -> MCPResult<()>;

    /// Disconnect from the remote endpoint
    async fn disconnect(&mut self) -> MCPResult<()>;

    /// Send a message
    async fn send_message(&mut self, message: MCPMessage) -> MCPResult<()>;

    /// Receive a message
    async fn receive_message(&mut self) -> MCPResult<MCPMessage>;

    /// Check if connected
    fn is_connected(&self) -> bool;

    /// Get connection status
    fn status(&self) -> ConnectionStatus;

    /// Get transport statistics
    fn stats(&self) -> &TransportStats;

    /// Get transport configuration
    fn config(&self) -> &TransportConfig;

    /// Health check
    async fn health_check(&self) -> MCPResult<bool>;
}

/// WebSocket transport implementation
pub struct WebSocketTransport {
    /// Transport configuration
    config: TransportConfig,
    /// WebSocket connection
    ws_stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    /// Connection status
    status: ConnectionStatus,
    /// Transport statistics
    stats: TransportStats,
    /// Message receiver channel
    receiver: Option<mpsc::UnboundedReceiver<MCPMessage>>,
    /// Message sender channel
    sender: Option<mpsc::UnboundedSender<MCPMessage>>,
    /// Background task handle
    _task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl WebSocketTransport {
    /// Create a new WebSocket transport
    pub fn new(config: TransportConfig) -> Self {
        Self {
            config,
            ws_stream: None,
            status: ConnectionStatus::Disconnected,
            stats: TransportStats::default(),
            receiver: None,
            sender: None,
            _task_handle: None,
        }
    }

    /// Start background message handling task
    async fn start_message_handler(&mut self) -> MCPResult<()> {
        if let Some(mut ws_stream) = self.ws_stream.take() {
            let (tx, rx) = mpsc::unbounded_channel();
            let (out_tx, mut out_rx) = mpsc::unbounded_channel();

            self.receiver = Some(rx);
            self.sender = Some(out_tx);

            // Spawn background task for message handling
            let stats = Arc::new(RwLock::new(self.stats.clone()));
            let max_message_size = self.config.max_message_size;

            let handle = tokio::spawn(async move {
                loop {
                    tokio::select! {
                        // Handle incoming messages
                        msg = ws_stream.next() => {
                            use futures_util::StreamExt;
                            match msg {
                                Some(Ok(Message::Text(text))) => {
                                    if text.len() <= max_message_size {
                                        match MCPMessage::from_json(&text) {
                                            Ok(mcp_msg) => {
                                                stats.write().await.update_received(text.len());
                                                if tx.send(mcp_msg).is_err() {
                                                    break;
                                                }
                                            }
                                            Err(e) => {
                                                warn!("Failed to parse MCP message: {}", e);
                                            }
                                        }
                                    } else {
                                        warn!("Message too large: {} bytes", text.len());
                                    }
                                }
                                Some(Ok(Message::Close(_))) => {
                                    info!("WebSocket connection closed by remote");
                                    break;
                                }
                                Some(Err(e)) => {
                                    error!("WebSocket error: {}", e);
                                    break;
                                }
                                None => {
                                    debug!("WebSocket stream ended");
                                    break;
                                }
                                _ => {} // Ignore other message types
                            }
                        }
                        // Handle outgoing messages
                        out_msg = out_rx.recv() => {
                            if let Some(message) = out_msg {
                                match message.to_json() {
                                    Ok(json) => {
                                        if json.len() <= max_message_size {
                                            if let Err(e) = ws_stream.send(Message::Text(json.clone())).await {
                                                error!("Failed to send WebSocket message: {}", e);
                                                break;
                                            } else {
                                                stats.write().await.update_sent(json.len());
                                            }
                                        } else {
                                            warn!("Outgoing message too large: {} bytes", json.len());
                                        }
                                    }
                                    Err(e) => {
                                        error!("Failed to serialize message: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            });

            self._task_handle = Some(handle);
        }

        Ok(())
    }
}

#[async_trait]
impl MCPTransport for WebSocketTransport {
    async fn connect(&mut self) -> MCPResult<()> {
        self.status = ConnectionStatus::Connecting;
        self.stats.update_connection(false); // Increment attempt

        let url = Url::parse(&self.config.endpoint)
            .map_err(|e| MCPError::configuration(format!("Invalid WebSocket URL: {}", e)))?;

        info!("Connecting to WebSocket: {}", url);

        let connect_future = connect_async(&url);
        let result = timeout(self.config.connect_timeout, connect_future).await;

        match result {
            Ok(Ok((ws_stream, response))) => {
                info!("WebSocket connected with response: {:?}", response.status());
                self.ws_stream = Some(ws_stream);
                self.status = ConnectionStatus::Connected;
                self.stats.update_connection(true);

                // Start message handler
                self.start_message_handler().await?;

                Ok(())
            }
            Ok(Err(e)) => {
                let error_msg = format!("WebSocket connection failed: {}", e);
                self.status = ConnectionStatus::Failed(error_msg.clone());
                Err(MCPError::connection(error_msg))
            }
            Err(_) => {
                let error_msg = "WebSocket connection timed out";
                self.status = ConnectionStatus::Failed(error_msg.to_string());
                Err(MCPError::timeout(self.config.connect_timeout.as_millis() as u64))
            }
        }
    }

    async fn disconnect(&mut self) -> MCPResult<()> {
        if let Some(mut ws_stream) = self.ws_stream.take() {
            use futures_util::SinkExt;
            let _ = ws_stream.close(None).await;
            info!("WebSocket disconnected");
        }

        self.status = ConnectionStatus::Disconnected;
        self.receiver = None;
        self.sender = None;

        if let Some(handle) = self._task_handle.take() {
            handle.abort();
        }

        Ok(())
    }

    async fn send_message(&mut self, message: MCPMessage) -> MCPResult<()> {
        if !self.is_connected() {
            return Err(MCPError::connection("Not connected"));
        }

        if let Some(sender) = &self.sender {
            sender
                .send(message)
                .map_err(|_| MCPError::transport("Failed to send message to handler"))?;
            Ok(())
        } else {
            Err(MCPError::transport("Message sender not available"))
        }
    }

    async fn receive_message(&mut self) -> MCPResult<MCPMessage> {
        if !self.is_connected() {
            return Err(MCPError::connection("Not connected"));
        }

        if let Some(receiver) = &mut self.receiver {
            let receive_future = receiver.recv();
            let result = timeout(self.config.read_timeout, receive_future).await;

            match result {
                Ok(Some(message)) => Ok(message),
                Ok(None) => Err(MCPError::connection("Connection closed")),
                Err(_) => Err(MCPError::timeout(self.config.read_timeout.as_millis() as u64)),
            }
        } else {
            Err(MCPError::transport("Message receiver not available"))
        }
    }

    fn is_connected(&self) -> bool {
        matches!(self.status, ConnectionStatus::Connected)
    }

    fn status(&self) -> ConnectionStatus {
        self.status.clone()
    }

    fn stats(&self) -> &TransportStats {
        &self.stats
    }

    fn config(&self) -> &TransportConfig {
        &self.config
    }

    async fn health_check(&self) -> MCPResult<bool> {
        // For WebSocket, we can check if the connection is still alive
        Ok(self.is_connected())
    }
}

/// HTTP transport implementation
pub struct HttpTransport {
    /// Transport configuration
    config: TransportConfig,
    /// HTTP client
    client: reqwest::Client,
    /// Connection status
    status: ConnectionStatus,
    /// Transport statistics
    stats: TransportStats,
}

impl HttpTransport {
    /// Create a new HTTP transport
    pub fn new(config: TransportConfig) -> MCPResult<Self> {
        let mut client_builder = reqwest::Client::builder()
            .timeout(config.read_timeout)
            .user_agent("ai-commit-mcp-client/1.0.0");

        // Add custom headers
        let mut headers = reqwest::header::HeaderMap::new();
        for (key, value) in &config.headers {
            headers.insert(
                key.parse().map_err(|e| MCPError::configuration(format!("Invalid header key '{}': {}", key, e)))?,
                value.parse().map_err(|e| MCPError::configuration(format!("Invalid header value '{}': {}", value, e)))?,
            );
        }
        client_builder = client_builder.default_headers(headers);

        let client = client_builder
            .build()
            .map_err(|e| MCPError::configuration(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            config,
            client,
            status: ConnectionStatus::Disconnected,
            stats: TransportStats::default(),
        })
    }
}

#[async_trait]
impl MCPTransport for HttpTransport {
    async fn connect(&mut self) -> MCPResult<()> {
        self.status = ConnectionStatus::Connecting;
        self.stats.update_connection(false);

        // For HTTP, "connecting" means doing a health check
        let health_check = self.health_check().await;
        match health_check {
            Ok(true) => {
                self.status = ConnectionStatus::Connected;
                self.stats.update_connection(true);
                info!("HTTP transport connected to {}", self.config.endpoint);
                Ok(())
            }
            Ok(false) => {
                let error_msg = "HTTP endpoint health check failed";
                self.status = ConnectionStatus::Failed(error_msg.to_string());
                Err(MCPError::connection(error_msg))
            }
            Err(e) => {
                let error_msg = format!("HTTP connection failed: {}", e);
                self.status = ConnectionStatus::Failed(error_msg.clone());
                Err(MCPError::connection(error_msg))
            }
        }
    }

    async fn disconnect(&mut self) -> MCPResult<()> {
        self.status = ConnectionStatus::Disconnected;
        info!("HTTP transport disconnected");
        Ok(())
    }

    async fn send_message(&mut self, message: MCPMessage) -> MCPResult<()> {
        if !self.is_connected() {
            return Err(MCPError::connection("Not connected"));
        }

        let json = message.to_json()
            .map_err(|e| MCPError::serialization(format!("Failed to serialize message: {}", e)))?;

        if json.len() > self.config.max_message_size {
            return Err(MCPError::validation(format!("Message too large: {} bytes", json.len())));
        }

        let response = self
            .client
            .post(&self.config.endpoint)
            .header("Content-Type", "application/json")
            .body(json.clone())
            .send()
            .await
            .map_err(|e| MCPError::transport(format!("HTTP request failed: {}", e)))?;

        if response.status().is_success() {
            self.stats.update_sent(json.len());
            Ok(())
        } else {
            Err(MCPError::transport(format!("HTTP request failed with status: {}", response.status())))
        }
    }

    async fn receive_message(&mut self) -> MCPResult<MCPMessage> {
        // HTTP is request/response, so receiving is not applicable in the same way
        // This would typically be handled differently in an HTTP context
        Err(MCPError::feature_not_supported("HTTP transport does not support receiving messages directly"))
    }

    fn is_connected(&self) -> bool {
        matches!(self.status, ConnectionStatus::Connected)
    }

    fn status(&self) -> ConnectionStatus {
        self.status.clone()
    }

    fn stats(&self) -> &TransportStats {
        &self.stats
    }

    fn config(&self) -> &TransportConfig {
        &self.config
    }

    async fn health_check(&self) -> MCPResult<bool> {
        let health_endpoint = format!("{}/health", self.config.endpoint.trim_end_matches('/'));

        let result = timeout(Duration::from_secs(5), async {
            self.client
                .get(&health_endpoint)
                .send()
                .await
        }).await;

        match result {
            Ok(Ok(response)) => Ok(response.status().is_success()),
            _ => Ok(false),
        }
    }
}

/// Stdio transport implementation for local process communication
pub struct StdioTransport {
    /// Transport configuration
    config: TransportConfig,
    /// Child process
    child_process: Option<Child>,
    /// Connection status
    status: ConnectionStatus,
    /// Transport statistics
    stats: TransportStats,
    /// Message receiver
    receiver: Option<mpsc::UnboundedReceiver<MCPMessage>>,
    /// Message sender
    sender: Option<mpsc::UnboundedSender<MCPMessage>>,
    /// Background task handle
    _task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl StdioTransport {
    /// Create a new Stdio transport
    pub fn new(config: TransportConfig) -> Self {
        Self {
            config,
            child_process: None,
            status: ConnectionStatus::Disconnected,
            stats: TransportStats::default(),
            receiver: None,
            sender: None,
            _task_handle: None,
        }
    }

    /// Parse command from endpoint
    fn parse_command(&self) -> MCPResult<(String, Vec<String>)> {
        let parts: Vec<&str> = self.config.endpoint.split_whitespace().collect();
        if parts.is_empty() {
            return Err(MCPError::configuration("Empty command"));
        }

        let program = parts[0].to_string();
        let args = parts[1..].iter().map(|s| s.to_string()).collect();

        Ok((program, args))
    }
}

#[async_trait]
impl MCPTransport for StdioTransport {
    async fn connect(&mut self) -> MCPResult<()> {
        self.status = ConnectionStatus::Connecting;
        self.stats.update_connection(false);

        let (program, args) = self.parse_command()?;

        info!("Starting process: {} {:?}", program, args);

        let mut child = Command::new(&program)
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| MCPError::connection(format!("Failed to start process: {}", e)))?;

        let stdin = child.stdin.take()
            .ok_or_else(|| MCPError::connection("Failed to get stdin"))?;
        let stdout = child.stdout.take()
            .ok_or_else(|| MCPError::connection("Failed to get stdout"))?;

        let (tx, rx) = mpsc::unbounded_channel();
        let (out_tx, mut out_rx) = mpsc::unbounded_channel();

        self.receiver = Some(rx);
        self.sender = Some(out_tx);

        // Spawn background task for message handling
        let stats = Arc::new(RwLock::new(self.stats.clone()));
        let max_message_size = self.config.max_message_size;

        let handle = tokio::spawn(async move {
            let mut stdout_reader = BufReader::new(stdout);
            let mut stdin_writer = stdin;
            let mut line = String::new();

            loop {
                tokio::select! {
                    // Handle incoming messages from stdout
                    result = stdout_reader.read_line(&mut line) => {
                        match result {
                            Ok(0) => {
                                debug!("Process stdout closed");
                                break;
                            }
                            Ok(_) => {
                                if line.trim().is_empty() {
                                    line.clear();
                                    continue;
                                }

                                if line.len() <= max_message_size {
                                    match MCPMessage::from_json(line.trim()) {
                                        Ok(mcp_msg) => {
                                            stats.write().await.update_received(line.len());
                                            if tx.send(mcp_msg).is_err() {
                                                break;
                                            }
                                        }
                                        Err(e) => {
                                            warn!("Failed to parse MCP message: {}", e);
                                        }
                                    }
                                } else {
                                    warn!("Message too large: {} bytes", line.len());
                                }
                                line.clear();
                            }
                            Err(e) => {
                                error!("Failed to read from stdout: {}", e);
                                break;
                            }
                        }
                    }
                    // Handle outgoing messages to stdin
                    out_msg = out_rx.recv() => {
                        if let Some(message) = out_msg {
                            match message.to_json() {
                                Ok(json) => {
                                    if json.len() <= max_message_size {
                                        let line = format!("{}\n", json);
                                        if let Err(e) = stdin_writer.write_all(line.as_bytes()).await {
                                            error!("Failed to write to stdin: {}", e);
                                            break;
                                        } else {
                                            if let Err(e) = stdin_writer.flush().await {
                                                error!("Failed to flush stdin: {}", e);
                                                break;
                                            }
                                            stats.write().await.update_sent(line.len());
                                        }
                                    } else {
                                        warn!("Outgoing message too large: {} bytes", json.len());
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to serialize message: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        });

        self._task_handle = Some(handle);
        self.child_process = Some(child);
        self.status = ConnectionStatus::Connected;
        self.stats.update_connection(true);

        info!("Stdio transport connected to process");
        Ok(())
    }

    async fn disconnect(&mut self) -> MCPResult<()> {
        if let Some(handle) = self._task_handle.take() {
            handle.abort();
        }

        if let Some(mut child) = self.child_process.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
            info!("Child process terminated");
        }

        self.status = ConnectionStatus::Disconnected;
        self.receiver = None;
        self.sender = None;

        Ok(())
    }

    async fn send_message(&mut self, message: MCPMessage) -> MCPResult<()> {
        if !self.is_connected() {
            return Err(MCPError::connection("Not connected"));
        }

        if let Some(sender) = &self.sender {
            sender
                .send(message)
                .map_err(|_| MCPError::transport("Failed to send message to handler"))?;
            Ok(())
        } else {
            Err(MCPError::transport("Message sender not available"))
        }
    }

    async fn receive_message(&mut self) -> MCPResult<MCPMessage> {
        if !self.is_connected() {
            return Err(MCPError::connection("Not connected"));
        }

        if let Some(receiver) = &mut self.receiver {
            let receive_future = receiver.recv();
            let result = timeout(self.config.read_timeout, receive_future).await;

            match result {
                Ok(Some(message)) => Ok(message),
                Ok(None) => Err(MCPError::connection("Connection closed")),
                Err(_) => Err(MCPError::timeout(self.config.read_timeout.as_millis() as u64)),
            }
        } else {
            Err(MCPError::transport("Message receiver not available"))
        }
    }

    fn is_connected(&self) -> bool {
        matches!(self.status, ConnectionStatus::Connected)
    }

    fn status(&self) -> ConnectionStatus {
        self.status.clone()
    }

    fn stats(&self) -> &TransportStats {
        &self.stats
    }

    fn config(&self) -> &TransportConfig {
        &self.config
    }

    async fn health_check(&self) -> MCPResult<bool> {
        if let Some(child) = &self.child_process {
            // Check if process is still running
            match child.id() {
                Some(_) => Ok(true),
                None => Ok(false),
            }
        } else {
            Ok(false)
        }
    }
}

/// Transport factory for creating different transport types
pub struct TransportFactory;

impl TransportFactory {
    /// Create a transport based on configuration
    pub fn create_transport(config: TransportConfig) -> MCPResult<Box<dyn MCPTransport>> {
        match config.transport_type {
            TransportType::WebSocket => {
                Ok(Box::new(WebSocketTransport::new(config)))
            }
            TransportType::Http => {
                let transport = HttpTransport::new(config)?;
                Ok(Box::new(transport))
            }
            TransportType::Stdio => {
                Ok(Box::new(StdioTransport::new(config)))
            }
        }
    }

    /// Create WebSocket transport with URL
    pub fn create_websocket(url: impl Into<String>) -> MCPResult<Box<dyn MCPTransport>> {
        let config = TransportConfig {
            transport_type: TransportType::WebSocket,
            endpoint: url.into(),
            ..Default::default()
        };
        Self::create_transport(config)
    }

    /// Create HTTP transport with URL
    pub fn create_http(url: impl Into<String>) -> MCPResult<Box<dyn MCPTransport>> {
        let config = TransportConfig {
            transport_type: TransportType::Http,
            endpoint: url.into(),
            ..Default::default()
        };
        Self::create_transport(config)
    }

    /// Create Stdio transport with command
    pub fn create_stdio(command: impl Into<String>) -> MCPResult<Box<dyn MCPTransport>> {
        let config = TransportConfig {
            transport_type: TransportType::Stdio,
            endpoint: command.into(),
            ..Default::default()
        };
        Self::create_transport(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_config_default() {
        let config = TransportConfig::default();
        assert_eq!(config.transport_type, TransportType::WebSocket);
        assert!(config.connect_timeout > Duration::ZERO);
        assert!(config.max_message_size > 0);
    }

    #[test]
    fn test_transport_stats() {
        let mut stats = TransportStats::default();

        stats.update_sent(100);
        stats.update_received(200);

        assert_eq!(stats.messages_sent, 1);
        assert_eq!(stats.messages_received, 1);
        assert_eq!(stats.bytes_sent, 100);
        assert_eq!(stats.bytes_received, 200);
    }

    #[test]
    fn test_connection_status() {
        let status = ConnectionStatus::Connected;
        assert_eq!(status, ConnectionStatus::Connected);

        let failed_status = ConnectionStatus::Failed("Test error".to_string());
        assert!(matches!(failed_status, ConnectionStatus::Failed(_)));
    }

    #[tokio::test]
    async fn test_websocket_transport_creation() {
        let config = TransportConfig {
            transport_type: TransportType::WebSocket,
            endpoint: "ws://localhost:8080/mcp".to_string(),
            ..Default::default()
        };

        let transport = WebSocketTransport::new(config);
        assert!(!transport.is_connected());
        assert_eq!(transport.status(), ConnectionStatus::Disconnected);
    }

    #[tokio::test]
    async fn test_transport_factory() {
        let ws_transport = TransportFactory::create_websocket("ws://localhost:8080/mcp");
        assert!(ws_transport.is_ok());

        let http_transport = TransportFactory::create_http("http://localhost:8080/mcp");
        assert!(http_transport.is_ok());

        let stdio_transport = TransportFactory::create_stdio("python mcp_server.py");
        assert!(stdio_transport.is_ok());
    }
}
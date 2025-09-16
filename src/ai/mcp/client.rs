//! MCP Client Implementation
//!
//! Provides a complete MCP client for connecting to MCP servers
//! and making requests for resources, tools, and other services.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    sync::{mpsc, RwLock, Mutex},
    time::timeout,
};
use tracing::{debug, error, info, warn};

use crate::error::{AppError, AppResult};
use super::{
    errors::{MCPError, MCPResult},
    protocol::{
        MCPMessage, MCPRequest, MCPResponse, MCPNotification, MethodName, MessageId,
        InitializeParams, InitializeResult, ClientInfo, ProtocolVersion,
    },
    transport::{MCPTransport, TransportConfig, TransportFactory, ConnectionStatus},
    resources::{ResourceUri, ResourceContent},
    tools::{ToolCall, ToolResult},
};

/// Client status enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientStatus {
    /// Client not connected
    Disconnected,
    /// Client connecting
    Connecting,
    /// Client initializing
    Initializing,
    /// Client ready for requests
    Ready,
    /// Client error state
    Error(String),
    /// Client shutting down
    ShuttingDown,
}

/// MCP client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPClientConfig {
    /// Transport configuration
    pub transport: TransportConfig,
    /// Client information
    pub client_info: ClientInfo,
    /// Protocol version to use
    pub protocol_version: ProtocolVersion,
    /// Request timeout
    pub request_timeout: Duration,
    /// Maximum concurrent requests
    pub max_concurrent_requests: usize,
    /// Auto-reconnect on connection loss
    pub auto_reconnect: bool,
    /// Reconnect attempts
    pub max_reconnect_attempts: u32,
    /// Reconnect delay
    pub reconnect_delay: Duration,
    /// Enable request/response logging
    pub enable_logging: bool,
}

impl Default for MCPClientConfig {
    fn default() -> Self {
        Self {
            transport: TransportConfig::default(),
            client_info: ClientInfo {
                name: "ai-commit-mcp-client".to_string(),
                version: "1.0.0".to_string(),
            },
            protocol_version: ProtocolVersion::CURRENT,
            request_timeout: Duration::from_secs(30),
            max_concurrent_requests: 10,
            auto_reconnect: true,
            max_reconnect_attempts: 3,
            reconnect_delay: Duration::from_secs(5),
            enable_logging: true,
        }
    }
}

/// Pending request information
#[derive(Debug)]
struct PendingRequest {
    /// Request ID
    id: MessageId,
    /// Response sender
    response_sender: tokio::sync::oneshot::Sender<MCPResult<MCPResponse>>,
    /// Request timestamp
    timestamp: Instant,
    /// Request timeout
    timeout: Duration,
}

/// Client statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ClientStats {
    /// Total requests sent
    pub requests_sent: u64,
    /// Total responses received
    pub responses_received: u64,
    /// Total notifications received
    pub notifications_received: u64,
    /// Failed requests
    pub failed_requests: u64,
    /// Timeout requests
    pub timeout_requests: u64,
    /// Average response time
    pub average_response_time: Duration,
    /// Connection uptime
    pub uptime: Duration,
    /// Last activity timestamp
    pub last_activity: Option<DateTime<Utc>>,
    /// Reconnection count
    pub reconnection_count: u32,
}

impl ClientStats {
    /// Update request statistics
    pub fn update_request(&mut self, success: bool, response_time: Duration) {
        self.requests_sent += 1;
        if success {
            self.responses_received += 1;
        } else {
            self.failed_requests += 1;
        }

        // Update average response time
        let total_time = self.average_response_time.as_nanos() as u64 * (self.responses_received - 1) + response_time.as_nanos() as u64;
        if self.responses_received > 0 {
            self.average_response_time = Duration::from_nanos(total_time / self.responses_received);
        }

        self.last_activity = Some(Utc::now());
    }

    /// Update timeout statistics
    pub fn update_timeout(&mut self) {
        self.timeout_requests += 1;
        self.failed_requests += 1;
        self.last_activity = Some(Utc::now());
    }

    /// Update notification statistics
    pub fn update_notification(&mut self) {
        self.notifications_received += 1;
        self.last_activity = Some(Utc::now());
    }

    /// Update reconnection statistics
    pub fn update_reconnection(&mut self) {
        self.reconnection_count += 1;
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        if self.requests_sent == 0 {
            0.0
        } else {
            self.responses_received as f64 / self.requests_sent as f64
        }
    }
}

/// MCP client implementation
pub struct MCPClient {
    /// Client configuration
    config: MCPClientConfig,
    /// Transport layer
    transport: Arc<Mutex<Box<dyn MCPTransport>>>,
    /// Client status
    status: Arc<RwLock<ClientStatus>>,
    /// Pending requests
    pending_requests: Arc<RwLock<HashMap<String, PendingRequest>>>,
    /// Client statistics
    stats: Arc<RwLock<ClientStats>>,
    /// Notification handlers
    notification_handlers: Arc<RwLock<HashMap<String, Box<dyn NotificationHandler>>>>,
    /// Server capabilities (received during initialization)
    server_capabilities: Arc<RwLock<Option<serde_json::Value>>>,
    /// Background task handles
    task_handles: Arc<RwLock<Vec<tokio::task::JoinHandle<()>>>>,
    /// Connection start time
    connection_start: Arc<RwLock<Option<Instant>>>,
}

/// Notification handler trait
#[async_trait]
pub trait NotificationHandler: Send + Sync {
    /// Handle notification
    async fn handle_notification(&self, notification: MCPNotification) -> MCPResult<()>;
}

impl MCPClient {
    /// Create a new MCP client
    pub async fn new(config: MCPClientConfig) -> MCPResult<Self> {
        let transport = TransportFactory::create_transport(config.transport.clone())?;

        Ok(Self {
            config,
            transport: Arc::new(Mutex::new(transport)),
            status: Arc::new(RwLock::new(ClientStatus::Disconnected)),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(ClientStats::default())),
            notification_handlers: Arc::new(RwLock::new(HashMap::new())),
            server_capabilities: Arc::new(RwLock::new(None)),
            task_handles: Arc::new(RwLock::new(Vec::new())),
            connection_start: Arc::new(RwLock::new(None)),
        })
    }

    /// Connect to the MCP server
    pub async fn connect(&self) -> MCPResult<()> {
        info!("Connecting to MCP server...");

        {
            let mut status = self.status.write().await;
            *status = ClientStatus::Connecting;
        }

        // Connect transport
        {
            let mut transport = self.transport.lock().await;
            transport.connect().await?;
        }

        // Start message handling
        self.start_message_handler().await?;

        // Initialize protocol
        self.initialize_protocol().await?;

        {
            let mut status = self.status.write().await;
            *status = ClientStatus::Ready;
            let mut connection_start = self.connection_start.write().await;
            *connection_start = Some(Instant::now());
        }

        info!("MCP client connected and ready");
        Ok(())
    }

    /// Disconnect from the MCP server
    pub async fn disconnect(&self) -> MCPResult<()> {
        info!("Disconnecting from MCP server...");

        {
            let mut status = self.status.write().await;
            *status = ClientStatus::ShuttingDown;
        }

        // Cancel pending requests
        {
            let mut pending = self.pending_requests.write().await;
            for (_, request) in pending.drain() {
                let _ = request.response_sender.send(Err(MCPError::connection("Client disconnecting")));
            }
        }

        // Stop background tasks
        {
            let mut handles = self.task_handles.write().await;
            for handle in handles.drain(..) {
                handle.abort();
            }
        }

        // Disconnect transport
        {
            let mut transport = self.transport.lock().await;
            transport.disconnect().await?;
        }

        {
            let mut status = self.status.write().await;
            *status = ClientStatus::Disconnected;
            let mut connection_start = self.connection_start.write().await;
            *connection_start = None;
        }

        info!("MCP client disconnected");
        Ok(())
    }

    /// Initialize MCP protocol
    async fn initialize_protocol(&self) -> MCPResult<()> {
        info!("Initializing MCP protocol...");

        {
            let mut status = self.status.write().await;
            *status = ClientStatus::Initializing;
        }

        let params = InitializeParams {
            protocol_version: self.config.protocol_version.clone(),
            client_info: self.config.client_info.clone(),
            capabilities: None, // Client capabilities can be specified here
        };

        let request = MCPRequest::with_params(
            MethodName::initialize(),
            serde_json::to_value(params)
                .map_err(|e| MCPError::serialization(format!("Failed to serialize initialize params: {}", e)))?,
        );

        let response = self.send_request_internal(request).await?;

        if let Some(result) = response.result {
            let init_result: InitializeResult = serde_json::from_value(result)
                .map_err(|e| MCPError::serialization(format!("Failed to parse initialize result: {}", e)))?;

            // Store server capabilities
            {
                let mut capabilities = self.server_capabilities.write().await;
                *capabilities = Some(serde_json::to_value(&init_result.capabilities)
                    .unwrap_or(serde_json::Value::Null));
            }

            info!(
                "MCP protocol initialized with server: {} {}",
                init_result.server_info.name, init_result.server_info.version
            );

            // Check protocol compatibility
            if !self.config.protocol_version.is_compatible(&init_result.protocol_version) {
                return Err(MCPError::version_mismatch(
                    self.config.protocol_version.to_string(),
                    init_result.protocol_version.to_string(),
                ));
            }

            Ok(())
        } else if let Some(error) = response.error {
            Err(MCPError::Protocol(error))
        } else {
            Err(MCPError::protocol("Invalid initialize response"))
        }
    }

    /// Start message handling background task
    async fn start_message_handler(&self) -> MCPResult<()> {
        let client = self.clone();
        let handle = tokio::spawn(async move {
            loop {
                let message = {
                    let mut transport = client.transport.lock().await;
                    match transport.receive_message().await {
                        Ok(msg) => msg,
                        Err(e) => {
                            error!("Failed to receive message: {}", e);
                            break;
                        }
                    }
                };

                if let Err(e) = client.handle_message(message).await {
                    error!("Failed to handle message: {}", e);
                }
            }
        });

        {
            let mut handles = self.task_handles.write().await;
            handles.push(handle);
        }

        // Start timeout checker
        let client = self.clone();
        let timeout_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                client.check_timeouts().await;
            }
        });

        {
            let mut handles = self.task_handles.write().await;
            handles.push(timeout_handle);
        }

        Ok(())
    }

    /// Handle incoming message
    async fn handle_message(&self, message: MCPMessage) -> MCPResult<()> {
        if self.config.enable_logging {
            debug!("Received message: {:?}", message);
        }

        match message {
            MCPMessage::Response(response) => {
                let request_id = response.id.to_string();
                let mut pending = self.pending_requests.write().await;

                if let Some(pending_request) = pending.remove(&request_id) {
                    let response_time = pending_request.timestamp.elapsed();
                    let success = response.is_success();

                    // Update statistics
                    {
                        let mut stats = self.stats.write().await;
                        stats.update_request(success, response_time);
                    }

                    // Send response to waiting request
                    let _ = pending_request.response_sender.send(Ok(response));
                } else {
                    warn!("Received response for unknown request ID: {}", request_id);
                }
            }
            MCPMessage::Notification(notification) => {
                // Update statistics
                {
                    let mut stats = self.stats.write().await;
                    stats.update_notification();
                }

                // Handle notification
                self.handle_notification(notification).await?;
            }
            MCPMessage::Request(_) => {
                // Client should not receive requests from server
                warn!("Received unexpected request from server");
            }
        }

        Ok(())
    }

    /// Handle notification
    async fn handle_notification(&self, notification: MCPNotification) -> MCPResult<()> {
        let method = notification.method.as_str();
        debug!("Handling notification: {}", method);

        let handlers = self.notification_handlers.read().await;
        if let Some(handler) = handlers.get(method) {
            handler.handle_notification(notification).await?;
        } else {
            debug!("No handler registered for notification: {}", method);
        }

        Ok(())
    }

    /// Check for timed out requests
    async fn check_timeouts(&self) {
        let mut pending = self.pending_requests.write().await;
        let mut timed_out = Vec::new();

        for (id, request) in pending.iter() {
            if request.timestamp.elapsed() > request.timeout {
                timed_out.push(id.clone());
            }
        }

        for id in timed_out {
            if let Some(request) = pending.remove(&id) {
                // Update statistics
                {
                    let mut stats = self.stats.write().await;
                    stats.update_timeout();
                }

                let _ = request.response_sender.send(Err(MCPError::timeout(request.timeout.as_millis() as u64)));
            }
        }
    }

    /// Send request and wait for response
    pub async fn send_request(&self, request: MCPRequest) -> MCPResult<MCPResponse> {
        self.send_request_with_timeout(request, self.config.request_timeout).await
    }

    /// Send request with custom timeout
    pub async fn send_request_with_timeout(
        &self,
        request: MCPRequest,
        timeout_duration: Duration,
    ) -> MCPResult<MCPResponse> {
        // Check if client is ready
        {
            let status = self.status.read().await;
            match *status {
                ClientStatus::Ready => {},
                ClientStatus::Disconnected => return Err(MCPError::connection("Client not connected")),
                ClientStatus::Connecting => return Err(MCPError::connection("Client still connecting")),
                ClientStatus::Initializing => return Err(MCPError::connection("Client still initializing")),
                ClientStatus::Error(ref msg) => return Err(MCPError::connection(format!("Client in error state: {}", msg))),
                ClientStatus::ShuttingDown => return Err(MCPError::connection("Client shutting down")),
            }
        }

        // Check concurrent request limit
        {
            let pending = self.pending_requests.read().await;
            if pending.len() >= self.config.max_concurrent_requests {
                return Err(MCPError::service_overloaded("Too many concurrent requests"));
            }
        }

        self.send_request_internal(request).await
    }

    /// Internal request sending implementation
    async fn send_request_internal(&self, request: MCPRequest) -> MCPResult<MCPResponse> {
        let (response_sender, response_receiver) = tokio::sync::oneshot::channel();

        let pending_request = PendingRequest {
            id: request.id.clone(),
            response_sender,
            timestamp: Instant::now(),
            timeout: self.config.request_timeout,
        };

        // Store pending request
        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(request.id.to_string(), pending_request);
        }

        // Send request
        {
            let mut transport = self.transport.lock().await;
            let message = MCPMessage::Request(request);
            transport.send_message(message).await?;
        }

        // Wait for response
        match response_receiver.await {
            Ok(result) => result,
            Err(_) => Err(MCPError::transport("Request cancelled")),
        }
    }

    /// List available resources
    pub async fn list_resources(&self) -> MCPResult<Vec<serde_json::Value>> {
        let request = MCPRequest::new(MethodName::list_resources());
        let response = self.send_request(request).await?;

        if let Some(result) = response.result {
            let resources = result.get("resources")
                .and_then(|r| r.as_array())
                .ok_or_else(|| MCPError::protocol("Invalid list_resources response"))?;

            Ok(resources.clone())
        } else {
            Err(MCPError::protocol("No result in list_resources response"))
        }
    }

    /// Read a resource
    pub async fn read_resource(&self, uri: &ResourceUri) -> MCPResult<ResourceContent> {
        let params = serde_json::json!({
            "uri": uri.as_str()
        });

        let request = MCPRequest::with_params(MethodName::read_resource(), params);
        let response = self.send_request(request).await?;

        if let Some(result) = response.result {
            let contents = result.get("contents")
                .and_then(|c| c.as_array())
                .ok_or_else(|| MCPError::protocol("Invalid read_resource response"))?;

            if let Some(content) = contents.first() {
                if let Some(text) = content.get("text").and_then(|t| t.as_str()) {
                    Ok(ResourceContent::Text(text.to_string()))
                } else if let Some(blob) = content.get("blob").and_then(|b| b.as_str()) {
                    let data = base64::decode(blob)
                        .map_err(|e| MCPError::serialization(format!("Invalid base64 data: {}", e)))?;
                    Ok(ResourceContent::Binary(data))
                } else {
                    Ok(ResourceContent::Text(String::new()))
                }
            } else {
                Err(MCPError::protocol("No content in read_resource response"))
            }
        } else {
            Err(MCPError::protocol("No result in read_resource response"))
        }
    }

    /// List available tools
    pub async fn list_tools(&self) -> MCPResult<Vec<serde_json::Value>> {
        let request = MCPRequest::new(MethodName::list_tools());
        let response = self.send_request(request).await?;

        if let Some(result) = response.result {
            let tools = result.get("tools")
                .and_then(|t| t.as_array())
                .ok_or_else(|| MCPError::protocol("Invalid list_tools response"))?;

            Ok(tools.clone())
        } else {
            Err(MCPError::protocol("No result in list_tools response"))
        }
    }

    /// Call a tool
    pub async fn call_tool(&self, call: ToolCall) -> MCPResult<ToolResult> {
        let params = serde_json::json!({
            "name": call.name,
            "arguments": call.arguments
        });

        let request = MCPRequest::with_params(MethodName::call_tool(), params);
        let response = self.send_request(request).await?;

        if let Some(result) = response.result {
            let content = result.get("content")
                .ok_or_else(|| MCPError::protocol("Invalid call_tool response"))?;

            let is_error = result.get("isError")
                .and_then(|e| e.as_bool())
                .unwrap_or(false);

            let tool_result = if is_error {
                ToolResult::error(call.call_id, "Tool execution failed".to_string())
            } else {
                // Parse content array
                let content_array = if let Some(arr) = content.as_array() {
                    arr.clone()
                } else {
                    vec![content.clone()]
                };

                let tool_content = content_array
                    .into_iter()
                    .filter_map(|c| serde_json::from_value(c).ok())
                    .collect();

                ToolResult::success(call.call_id, tool_content)
            };

            Ok(tool_result)
        } else {
            Err(MCPError::protocol("No result in call_tool response"))
        }
    }

    /// Register notification handler
    pub async fn register_notification_handler(
        &self,
        method: String,
        handler: Box<dyn NotificationHandler>,
    ) {
        let mut handlers = self.notification_handlers.write().await;
        handlers.insert(method, handler);
    }

    /// Get client status
    pub async fn status(&self) -> ClientStatus {
        self.status.read().await.clone()
    }

    /// Get client statistics
    pub async fn stats(&self) -> ClientStats {
        let mut stats = self.stats.read().await.clone();

        // Update uptime
        if let Some(start) = *self.connection_start.read().await {
            stats.uptime = start.elapsed();
        }

        stats
    }

    /// Get server capabilities
    pub async fn server_capabilities(&self) -> Option<serde_json::Value> {
        self.server_capabilities.read().await.clone()
    }

    /// Check if connected and ready
    pub async fn is_ready(&self) -> bool {
        matches!(*self.status.read().await, ClientStatus::Ready)
    }

    /// Ping the server
    pub async fn ping(&self) -> MCPResult<Duration> {
        let start = Instant::now();
        let request = MCPRequest::new(MethodName::ping());
        let _ = self.send_request(request).await?;
        Ok(start.elapsed())
    }
}

impl Clone for MCPClient {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            transport: Arc::clone(&self.transport),
            status: Arc::clone(&self.status),
            pending_requests: Arc::clone(&self.pending_requests),
            stats: Arc::clone(&self.stats),
            notification_handlers: Arc::clone(&self.notification_handlers),
            server_capabilities: Arc::clone(&self.server_capabilities),
            task_handles: Arc::clone(&self.task_handles),
            connection_start: Arc::clone(&self.connection_start),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_default() {
        let config = MCPClientConfig::default();
        assert_eq!(config.client_info.name, "ai-commit-mcp-client");
        assert_eq!(config.protocol_version, ProtocolVersion::CURRENT);
        assert!(config.auto_reconnect);
    }

    #[test]
    fn test_client_status() {
        let status = ClientStatus::Ready;
        assert_eq!(status, ClientStatus::Ready);

        let error_status = ClientStatus::Error("Test error".to_string());
        assert!(matches!(error_status, ClientStatus::Error(_)));
    }

    #[test]
    fn test_client_stats() {
        let mut stats = ClientStats::default();

        stats.update_request(true, Duration::from_millis(100));
        stats.update_request(false, Duration::from_millis(200));

        assert_eq!(stats.requests_sent, 2);
        assert_eq!(stats.responses_received, 1);
        assert_eq!(stats.failed_requests, 1);
        assert_eq!(stats.success_rate(), 0.5);

        stats.update_timeout();
        assert_eq!(stats.timeout_requests, 1);
        assert_eq!(stats.failed_requests, 2);

        stats.update_notification();
        assert_eq!(stats.notifications_received, 1);
    }

    #[tokio::test]
    async fn test_client_creation() {
        let config = MCPClientConfig::default();
        let result = MCPClient::new(config).await;
        assert!(result.is_ok());

        let client = result.unwrap();
        assert_eq!(client.status().await, ClientStatus::Disconnected);
        assert!(!client.is_ready().await);
    }
}
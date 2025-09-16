//! MCP Server Implementation
//!
//! Provides a complete MCP server for hosting resources, tools, and other services
//! that can be accessed by MCP clients.

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
        InitializeParams, InitializeResult, ServerInfo, ServerCapabilities, ProtocolVersion,
        ResourceCapabilities, ToolCapabilities,
    },
    transport::{MCPTransport, TransportConfig, TransportFactory, ConnectionStatus},
    resources::{ResourceManager, ResourceUri},
    tools::{ToolManager, ToolCall},
};

/// Server status enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerStatus {
    /// Server not started
    Stopped,
    /// Server starting
    Starting,
    /// Server running and accepting connections
    Running,
    /// Server error state
    Error(String),
    /// Server shutting down
    ShuttingDown,
}

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerConfig {
    /// Transport configuration
    pub transport: TransportConfig,
    /// Server information
    pub server_info: ServerInfo,
    /// Server capabilities
    pub capabilities: ServerCapabilities,
    /// Protocol version
    pub protocol_version: ProtocolVersion,
    /// Maximum concurrent connections
    pub max_connections: usize,
    /// Request timeout
    pub request_timeout: Duration,
    /// Enable request/response logging
    pub enable_logging: bool,
    /// Require client authentication
    pub require_auth: bool,
    /// Allowed client IDs (if authentication enabled)
    pub allowed_clients: Option<Vec<String>>,
}

impl Default for MCPServerConfig {
    fn default() -> Self {
        Self {
            transport: TransportConfig::default(),
            server_info: ServerInfo {
                name: "ai-commit-mcp-server".to_string(),
                version: "1.0.0".to_string(),
            },
            capabilities: ServerCapabilities {
                resources: Some(ResourceCapabilities {
                    subscribe: false,
                    list_changed: false,
                }),
                tools: Some(ToolCapabilities {
                    list_changed: false,
                }),
                sampling: None,
            },
            protocol_version: ProtocolVersion::CURRENT,
            max_connections: 10,
            request_timeout: Duration::from_secs(30),
            enable_logging: true,
            require_auth: false,
            allowed_clients: None,
        }
    }
}

/// Client connection information
#[derive(Debug, Clone)]
struct ClientConnection {
    /// Client ID
    id: String,
    /// Client transport
    transport: Arc<Mutex<Box<dyn MCPTransport>>>,
    /// Connection timestamp
    connected_at: DateTime<Utc>,
    /// Client info (from initialization)
    client_info: Option<super::protocol::ClientInfo>,
    /// Last activity
    last_activity: DateTime<Utc>,
    /// Request count
    request_count: u64,
}

/// Server statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ServerStats {
    /// Total connections
    pub total_connections: u64,
    /// Current active connections
    pub active_connections: u64,
    /// Total requests processed
    pub requests_processed: u64,
    /// Successful requests
    pub successful_requests: u64,
    /// Failed requests
    pub failed_requests: u64,
    /// Average request processing time
    pub average_request_time: Duration,
    /// Server uptime
    pub uptime: Duration,
    /// Last activity timestamp
    pub last_activity: Option<DateTime<Utc>>,
    /// Requests by method
    pub requests_by_method: HashMap<String, u64>,
}

impl ServerStats {
    /// Update request statistics
    pub fn update_request(&mut self, method: &str, success: bool, processing_time: Duration) {
        self.requests_processed += 1;

        if success {
            self.successful_requests += 1;
        } else {
            self.failed_requests += 1;
        }

        // Update average request time
        let total_time = self.average_request_time.as_nanos() as u64 * (self.requests_processed - 1) + processing_time.as_nanos() as u64;
        self.average_request_time = Duration::from_nanos(total_time / self.requests_processed);

        // Update method statistics
        *self.requests_by_method.entry(method.to_string()).or_insert(0) += 1;

        self.last_activity = Some(Utc::now());
    }

    /// Update connection statistics
    pub fn update_connection(&mut self, connected: bool) {
        if connected {
            self.total_connections += 1;
            self.active_connections += 1;
        } else {
            if self.active_connections > 0 {
                self.active_connections -= 1;
            }
        }
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        if self.requests_processed == 0 {
            0.0
        } else {
            self.successful_requests as f64 / self.requests_processed as f64
        }
    }
}

/// Request handler trait
#[async_trait]
pub trait RequestHandler: Send + Sync {
    /// Handle a request
    async fn handle_request(&self, request: MCPRequest, client_id: &str) -> MCPResult<MCPResponse>;

    /// Get supported methods
    fn supported_methods(&self) -> Vec<String>;
}

/// MCP server implementation
pub struct MCPServer {
    /// Server configuration
    config: MCPServerConfig,
    /// Server status
    status: Arc<RwLock<ServerStatus>>,
    /// Connected clients
    clients: Arc<RwLock<HashMap<String, ClientConnection>>>,
    /// Request handlers
    handlers: Arc<RwLock<HashMap<String, Arc<dyn RequestHandler>>>>,
    /// Resource manager
    resource_manager: Arc<ResourceManager>,
    /// Tool manager
    tool_manager: Arc<ToolManager>,
    /// Server statistics
    stats: Arc<RwLock<ServerStats>>,
    /// Background task handles
    task_handles: Arc<RwLock<Vec<tokio::task::JoinHandle<()>>>>,
    /// Server start time
    start_time: Arc<RwLock<Option<Instant>>>,
}

impl MCPServer {
    /// Create a new MCP server
    pub fn new(config: MCPServerConfig) -> Self {
        let resource_manager = Arc::new(ResourceManager::new());
        let tool_manager = Arc::new(ToolManager::new());

        Self {
            config,
            status: Arc::new(RwLock::new(ServerStatus::Stopped)),
            clients: Arc::new(RwLock::new(HashMap::new())),
            handlers: Arc::new(RwLock::new(HashMap::new())),
            resource_manager,
            tool_manager,
            stats: Arc::new(RwLock::new(ServerStats::default())),
            task_handles: Arc::new(RwLock::new(Vec::new())),
            start_time: Arc::new(RwLock::new(None)),
        }
    }

    /// Start the server
    pub async fn start(&self) -> MCPResult<()> {
        info!("Starting MCP server...");

        {
            let mut status = self.status.write().await;
            *status = ServerStatus::Starting;
        }

        // Register default handlers
        self.register_default_handlers().await?;

        // TODO: Start transport listeners based on transport type
        // For now, this is a placeholder for the actual transport server implementation

        {
            let mut status = self.status.write().await;
            *status = ServerStatus::Running;
            let mut start_time = self.start_time.write().await;
            *start_time = Some(Instant::now());
        }

        info!("MCP server started and listening");
        Ok(())
    }

    /// Stop the server
    pub async fn stop(&self) -> MCPResult<()> {
        info!("Stopping MCP server...");

        {
            let mut status = self.status.write().await;
            *status = ServerStatus::ShuttingDown;
        }

        // Disconnect all clients
        {
            let mut clients = self.clients.write().await;
            for (client_id, _) in clients.drain() {
                info!("Disconnecting client: {}", client_id);
                // TODO: Send disconnect notification to client
            }
        }

        // Stop background tasks
        {
            let mut handles = self.task_handles.write().await;
            for handle in handles.drain(..) {
                handle.abort();
            }
        }

        {
            let mut status = self.status.write().await;
            *status = ServerStatus::Stopped;
            let mut start_time = self.start_time.write().await;
            *start_time = None;
        }

        info!("MCP server stopped");
        Ok(())
    }

    /// Register default request handlers
    async fn register_default_handlers(&self) -> MCPResult<()> {
        // Register initialize handler
        let initialize_handler = Arc::new(InitializeHandler::new(self.config.clone()));
        self.register_handler("initialize".to_string(), initialize_handler).await?;

        // Register ping handler
        let ping_handler = Arc::new(PingHandler);
        self.register_handler("ping".to_string(), ping_handler).await?;

        // Register resource handlers
        let resource_list_handler = Arc::new(ResourceListHandler::new(Arc::clone(&self.resource_manager)));
        self.register_handler("resources/list".to_string(), resource_list_handler).await?;

        let resource_read_handler = Arc::new(ResourceReadHandler::new(Arc::clone(&self.resource_manager)));
        self.register_handler("resources/read".to_string(), resource_read_handler).await?;

        // Register tool handlers
        let tool_list_handler = Arc::new(ToolListHandler::new(Arc::clone(&self.tool_manager)));
        self.register_handler("tools/list".to_string(), tool_list_handler).await?;

        let tool_call_handler = Arc::new(ToolCallHandler::new(Arc::clone(&self.tool_manager)));
        self.register_handler("tools/call".to_string(), tool_call_handler).await?;

        info!("Default request handlers registered");
        Ok(())
    }

    /// Register a request handler
    pub async fn register_handler(
        &self,
        method: String,
        handler: Arc<dyn RequestHandler>,
    ) -> MCPResult<()> {
        let mut handlers = self.handlers.write().await;
        handlers.insert(method.clone(), handler);
        info!("Registered handler for method: {}", method);
        Ok(())
    }

    /// Handle incoming client connection
    pub async fn handle_client_connection(
        &self,
        client_id: String,
        transport: Box<dyn MCPTransport>,
    ) -> MCPResult<()> {
        info!("New client connection: {}", client_id);

        // Check connection limit
        {
            let clients = self.clients.read().await;
            if clients.len() >= self.config.max_connections {
                return Err(MCPError::service_overloaded("Maximum connections reached"));
            }
        }

        let client_connection = ClientConnection {
            id: client_id.clone(),
            transport: Arc::new(Mutex::new(transport)),
            connected_at: Utc::now(),
            client_info: None,
            last_activity: Utc::now(),
            request_count: 0,
        };

        // Add client to active connections
        {
            let mut clients = self.clients.write().await;
            clients.insert(client_id.clone(), client_connection);

            let mut stats = self.stats.write().await;
            stats.update_connection(true);
        }

        // Start client message handler
        let server = self.clone();
        let handle = tokio::spawn(async move {
            server.handle_client_messages(client_id).await;
        });

        {
            let mut handles = self.task_handles.write().await;
            handles.push(handle);
        }

        Ok(())
    }

    /// Handle messages from a specific client
    async fn handle_client_messages(&self, client_id: String) {
        loop {
            let message = {
                let clients = self.clients.read().await;
                if let Some(client) = clients.get(&client_id) {
                    let mut transport = client.transport.lock().await;
                    match transport.receive_message().await {
                        Ok(msg) => msg,
                        Err(e) => {
                            error!("Failed to receive message from client {}: {}", client_id, e);
                            break;
                        }
                    }
                } else {
                    error!("Client {} not found in connections", client_id);
                    break;
                }
            };

            if let Err(e) = self.handle_message(message, &client_id).await {
                error!("Failed to handle message from client {}: {}", client_id, e);
            }
        }

        // Remove client on disconnect
        self.disconnect_client(&client_id).await;
    }

    /// Handle incoming message from client
    async fn handle_message(&self, message: MCPMessage, client_id: &str) -> MCPResult<()> {
        if self.config.enable_logging {
            debug!("Received message from client {}: {:?}", client_id, message);
        }

        // Update client activity
        {
            let mut clients = self.clients.write().await;
            if let Some(client) = clients.get_mut(client_id) {
                client.last_activity = Utc::now();
                client.request_count += 1;
            }
        }

        match message {
            MCPMessage::Request(request) => {
                let response = self.handle_request(request, client_id).await;

                // Send response back to client
                if let Ok(resp) = response {
                    self.send_response_to_client(client_id, resp).await?;
                }
            }
            MCPMessage::Notification(notification) => {
                // Handle notification (server can receive notifications from clients)
                debug!("Received notification from client {}: {}", client_id, notification.method.as_str());
            }
            MCPMessage::Response(_) => {
                // Server should not normally receive responses
                warn!("Received unexpected response from client {}", client_id);
            }
        }

        Ok(())
    }

    /// Handle a request from client
    async fn handle_request(&self, request: MCPRequest, client_id: &str) -> MCPResult<MCPResponse> {
        let start_time = Instant::now();
        let method = request.method.as_str().to_string();

        debug!("Handling request from client {}: {}", client_id, method);

        let result = {
            let handlers = self.handlers.read().await;
            if let Some(handler) = handlers.get(&method) {
                handler.handle_request(request.clone(), client_id).await
            } else {
                Err(MCPError::method_not_found(&method))
            }
        };

        let processing_time = start_time.elapsed();
        let success = result.is_ok();

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.update_request(&method, success, processing_time);
        }

        match result {
            Ok(response) => {
                info!("Request {} from client {} processed successfully in {:?}", method, client_id, processing_time);
                Ok(response)
            }
            Err(e) => {
                error!("Request {} from client {} failed: {}", method, client_id, e);
                Ok(MCPResponse::error(request.id, e.to_protocol_error()))
            }
        }
    }

    /// Send response to client
    async fn send_response_to_client(&self, client_id: &str, response: MCPResponse) -> MCPResult<()> {
        let clients = self.clients.read().await;
        if let Some(client) = clients.get(client_id) {
            let mut transport = client.transport.lock().await;
            let message = MCPMessage::Response(response);
            transport.send_message(message).await?;
        }
        Ok(())
    }

    /// Disconnect a client
    async fn disconnect_client(&self, client_id: &str) {
        info!("Disconnecting client: {}", client_id);

        {
            let mut clients = self.clients.write().await;
            if clients.remove(client_id).is_some() {
                let mut stats = self.stats.write().await;
                stats.update_connection(false);
            }
        }
    }

    /// Get server status
    pub async fn status(&self) -> ServerStatus {
        self.status.read().await.clone()
    }

    /// Get server statistics
    pub async fn stats(&self) -> ServerStats {
        let mut stats = self.stats.read().await.clone();

        // Update uptime
        if let Some(start) = *self.start_time.read().await {
            stats.uptime = start.elapsed();
        }

        stats
    }

    /// Get connected clients
    pub async fn connected_clients(&self) -> Vec<String> {
        let clients = self.clients.read().await;
        clients.keys().cloned().collect()
    }

    /// Check if server is running
    pub async fn is_running(&self) -> bool {
        matches!(*self.status.read().await, ServerStatus::Running)
    }

    /// Get resource manager
    pub fn resource_manager(&self) -> &Arc<ResourceManager> {
        &self.resource_manager
    }

    /// Get tool manager
    pub fn tool_manager(&self) -> &Arc<ToolManager> {
        &self.tool_manager
    }
}

impl Clone for MCPServer {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            status: Arc::clone(&self.status),
            clients: Arc::clone(&self.clients),
            handlers: Arc::clone(&self.handlers),
            resource_manager: Arc::clone(&self.resource_manager),
            tool_manager: Arc::clone(&self.tool_manager),
            stats: Arc::clone(&self.stats),
            task_handles: Arc::clone(&self.task_handles),
            start_time: Arc::clone(&self.start_time),
        }
    }
}

/// Initialize request handler
struct InitializeHandler {
    config: MCPServerConfig,
}

impl InitializeHandler {
    fn new(config: MCPServerConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl RequestHandler for InitializeHandler {
    async fn handle_request(&self, request: MCPRequest, _client_id: &str) -> MCPResult<MCPResponse> {
        let params: InitializeParams = request.params
            .ok_or_else(|| MCPError::invalid_params("Missing initialization parameters"))
            .and_then(|p| serde_json::from_value(p)
                .map_err(|e| MCPError::invalid_params(format!("Invalid parameters: {}", e))))?;

        // Check protocol compatibility
        if !self.config.protocol_version.is_compatible(&params.protocol_version) {
            return Ok(MCPResponse::error(
                request.id,
                MCPError::version_mismatch(
                    params.protocol_version.to_string(),
                    self.config.protocol_version.to_string(),
                ).to_protocol_error(),
            ));
        }

        let result = InitializeResult {
            protocol_version: self.config.protocol_version.clone(),
            server_info: self.config.server_info.clone(),
            capabilities: self.config.capabilities.clone(),
        };

        Ok(MCPResponse::success(
            request.id,
            serde_json::to_value(result)
                .map_err(|e| MCPError::serialization(format!("Failed to serialize result: {}", e)))?,
        ))
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["initialize".to_string()]
    }
}

/// Ping request handler
struct PingHandler;

#[async_trait]
impl RequestHandler for PingHandler {
    async fn handle_request(&self, request: MCPRequest, _client_id: &str) -> MCPResult<MCPResponse> {
        Ok(MCPResponse::success(
            request.id,
            serde_json::json!({}),
        ))
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["ping".to_string()]
    }
}

/// Resource list handler
struct ResourceListHandler {
    resource_manager: Arc<ResourceManager>,
}

impl ResourceListHandler {
    fn new(resource_manager: Arc<ResourceManager>) -> Self {
        Self { resource_manager }
    }
}

#[async_trait]
impl RequestHandler for ResourceListHandler {
    async fn handle_request(&self, request: MCPRequest, _client_id: &str) -> MCPResult<MCPResponse> {
        self.resource_manager.handle_list_resources(request).await
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["resources/list".to_string()]
    }
}

/// Resource read handler
struct ResourceReadHandler {
    resource_manager: Arc<ResourceManager>,
}

impl ResourceReadHandler {
    fn new(resource_manager: Arc<ResourceManager>) -> Self {
        Self { resource_manager }
    }
}

#[async_trait]
impl RequestHandler for ResourceReadHandler {
    async fn handle_request(&self, request: MCPRequest, _client_id: &str) -> MCPResult<MCPResponse> {
        self.resource_manager.handle_read_resource(request).await
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["resources/read".to_string()]
    }
}

/// Tool list handler
struct ToolListHandler {
    tool_manager: Arc<ToolManager>,
}

impl ToolListHandler {
    fn new(tool_manager: Arc<ToolManager>) -> Self {
        Self { tool_manager }
    }
}

#[async_trait]
impl RequestHandler for ToolListHandler {
    async fn handle_request(&self, request: MCPRequest, _client_id: &str) -> MCPResult<MCPResponse> {
        self.tool_manager.handle_list_tools(request).await
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["tools/list".to_string()]
    }
}

/// Tool call handler
struct ToolCallHandler {
    tool_manager: Arc<ToolManager>,
}

impl ToolCallHandler {
    fn new(tool_manager: Arc<ToolManager>) -> Self {
        Self { tool_manager }
    }
}

#[async_trait]
impl RequestHandler for ToolCallHandler {
    async fn handle_request(&self, request: MCPRequest, _client_id: &str) -> MCPResult<MCPResponse> {
        self.tool_manager.handle_call_tool(request).await
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["tools/call".to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_default() {
        let config = MCPServerConfig::default();
        assert_eq!(config.server_info.name, "ai-commit-mcp-server");
        assert_eq!(config.protocol_version, ProtocolVersion::CURRENT);
        assert_eq!(config.max_connections, 10);
    }

    #[test]
    fn test_server_status() {
        let status = ServerStatus::Running;
        assert_eq!(status, ServerStatus::Running);

        let error_status = ServerStatus::Error("Test error".to_string());
        assert!(matches!(error_status, ServerStatus::Error(_)));
    }

    #[test]
    fn test_server_stats() {
        let mut stats = ServerStats::default();

        stats.update_request("test_method", true, Duration::from_millis(100));
        stats.update_request("test_method", false, Duration::from_millis(200));

        assert_eq!(stats.requests_processed, 2);
        assert_eq!(stats.successful_requests, 1);
        assert_eq!(stats.failed_requests, 1);
        assert_eq!(stats.success_rate(), 0.5);

        stats.update_connection(true);
        assert_eq!(stats.total_connections, 1);
        assert_eq!(stats.active_connections, 1);

        stats.update_connection(false);
        assert_eq!(stats.active_connections, 0);
    }

    #[tokio::test]
    async fn test_server_creation() {
        let config = MCPServerConfig::default();
        let server = MCPServer::new(config);

        assert_eq!(server.status().await, ServerStatus::Stopped);
        assert!(!server.is_running().await);

        let clients = server.connected_clients().await;
        assert!(clients.is_empty());
    }

    #[tokio::test]
    async fn test_initialize_handler() {
        let config = MCPServerConfig::default();
        let handler = InitializeHandler::new(config.clone());

        let params = InitializeParams {
            protocol_version: ProtocolVersion::CURRENT,
            client_info: super::super::protocol::ClientInfo {
                name: "test-client".to_string(),
                version: "1.0.0".to_string(),
            },
            capabilities: None,
        };

        let request = MCPRequest::with_params(
            MethodName::initialize(),
            serde_json::to_value(params).unwrap(),
        );

        let response = handler.handle_request(request, "test-client").await.unwrap();
        assert!(response.is_success());
        assert!(response.result.is_some());
    }

    #[tokio::test]
    async fn test_ping_handler() {
        let handler = PingHandler;
        let request = MCPRequest::new(MethodName::ping());

        let response = handler.handle_request(request, "test-client").await.unwrap();
        assert!(response.is_success());
    }
}
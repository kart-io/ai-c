//! MCP Protocol Core Implementation
//!
//! Implements the Model Context Protocol (MCP) core message structures
//! and JSON-RPC 2.0 protocol handling according to MCP specification.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::{self, Display},
    str::FromStr,
};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

/// MCP Protocol version identifier
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolVersion {
    /// Major version number
    pub major: u32,
    /// Minor version number
    pub minor: u32,
    /// Patch version number
    pub patch: u32,
}

impl ProtocolVersion {
    /// MCP Protocol version 2024-11-05
    pub const CURRENT: Self = Self {
        major: 2024,
        minor: 11,
        patch: 5,
    };

    /// Create a new protocol version
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Check if this version is compatible with another version
    pub fn is_compatible(&self, other: &Self) -> bool {
        // Same major version required for compatibility
        self.major == other.major
    }

    /// Check if this version supports a specific feature
    pub fn supports_feature(&self, feature: &str) -> bool {
        match feature {
            "tools" => self >= &Self::new(2024, 11, 1),
            "resources" => self >= &Self::new(2024, 11, 1),
            "sampling" => self >= &Self::new(2024, 11, 5),
            "notifications" => self >= &Self::new(2024, 11, 1),
            _ => false,
        }
    }
}

impl Display for ProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl FromStr for ProtocolVersion {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(AppError::mcp("Invalid version format"));
        }

        let major = parts[0].parse().map_err(|_| AppError::mcp("Invalid major version"))?;
        let minor = parts[1].parse().map_err(|_| AppError::mcp("Invalid minor version"))?;
        let patch = parts[2].parse().map_err(|_| AppError::mcp("Invalid patch version"))?;

        Ok(Self::new(major, minor, patch))
    }
}

impl PartialOrd for ProtocolVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ProtocolVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.major
            .cmp(&other.major)
            .then_with(|| self.minor.cmp(&other.minor))
            .then_with(|| self.patch.cmp(&other.patch))
    }
}

/// Message ID type for MCP messages
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageId {
    String(String),
    Number(u64),
}

impl MessageId {
    /// Generate a new random message ID
    pub fn generate() -> Self {
        Self::String(Uuid::new_v4().to_string())
    }

    /// Create from string
    pub fn from_string(s: String) -> Self {
        Self::String(s)
    }

    /// Create from number
    pub fn from_number(n: u64) -> Self {
        Self::Number(n)
    }
}

impl Display for MessageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageId::String(s) => write!(f, "{}", s),
            MessageId::Number(n) => write!(f, "{}", n),
        }
    }
}

/// Method name for MCP requests
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MethodName(String);

impl MethodName {
    /// Create a new method name
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Get the method name as string
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Standard MCP method names
    pub const INITIALIZE: MethodName = MethodName(String::new());
    pub const PING: MethodName = MethodName(String::new());
    pub const LIST_RESOURCES: MethodName = MethodName(String::new());
    pub const READ_RESOURCE: MethodName = MethodName(String::new());
    pub const LIST_TOOLS: MethodName = MethodName(String::new());
    pub const CALL_TOOL: MethodName = MethodName(String::new());
    pub const SAMPLING_CREATE_MESSAGE: MethodName = MethodName(String::new());
}

// Initialize standard method names
impl MethodName {
    pub fn initialize() -> Self {
        Self("initialize".to_string())
    }

    pub fn ping() -> Self {
        Self("ping".to_string())
    }

    pub fn list_resources() -> Self {
        Self("resources/list".to_string())
    }

    pub fn read_resource() -> Self {
        Self("resources/read".to_string())
    }

    pub fn list_tools() -> Self {
        Self("tools/list".to_string())
    }

    pub fn call_tool() -> Self {
        Self("tools/call".to_string())
    }

    pub fn sampling_create_message() -> Self {
        Self("sampling/createMessage".to_string())
    }
}

impl Display for MethodName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for MethodName {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for MethodName {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// MCP error code enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MCPErrorCode {
    /// Parse error
    ParseError = -32700,
    /// Invalid request
    InvalidRequest = -32600,
    /// Method not found
    MethodNotFound = -32601,
    /// Invalid parameters
    InvalidParams = -32602,
    /// Internal error
    InternalError = -32603,
    /// Server error
    ServerError = -32000,
}

impl MCPErrorCode {
    /// Get error message for the code
    pub fn message(&self) -> &'static str {
        match self {
            MCPErrorCode::ParseError => "Parse error",
            MCPErrorCode::InvalidRequest => "Invalid Request",
            MCPErrorCode::MethodNotFound => "Method not found",
            MCPErrorCode::InvalidParams => "Invalid params",
            MCPErrorCode::InternalError => "Internal error",
            MCPErrorCode::ServerError => "Server error",
        }
    }
}

/// MCP error structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPError {
    /// Error code
    pub code: i32,
    /// Error message
    pub message: String,
    /// Additional error data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl MCPError {
    /// Create a new MCP error
    pub fn new(code: MCPErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: code as i32,
            message: message.into(),
            data: None,
        }
    }

    /// Create error with additional data
    pub fn with_data(
        code: MCPErrorCode,
        message: impl Into<String>,
        data: serde_json::Value,
    ) -> Self {
        Self {
            code: code as i32,
            message: message.into(),
            data: Some(data),
        }
    }

    /// Create parse error
    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::new(MCPErrorCode::ParseError, message)
    }

    /// Create invalid request error
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(MCPErrorCode::InvalidRequest, message)
    }

    /// Create method not found error
    pub fn method_not_found(method: impl Into<String>) -> Self {
        Self::new(
            MCPErrorCode::MethodNotFound,
            format!("Method '{}' not found", method.into()),
        )
    }

    /// Create invalid parameters error
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(MCPErrorCode::InvalidParams, message)
    }

    /// Create internal error
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new(MCPErrorCode::InternalError, message)
    }

    /// Create server error
    pub fn server_error(message: impl Into<String>) -> Self {
        Self::new(MCPErrorCode::ServerError, message)
    }
}

/// MCP request message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPRequest {
    /// JSON-RPC version (must be "2.0")
    pub jsonrpc: String,
    /// Request ID
    pub id: MessageId,
    /// Method name
    pub method: MethodName,
    /// Request parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl MCPRequest {
    /// Create a new MCP request
    pub fn new(method: impl Into<MethodName>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: MessageId::generate(),
            method: method.into(),
            params: None,
        }
    }

    /// Create request with parameters
    pub fn with_params(
        method: impl Into<MethodName>,
        params: serde_json::Value,
    ) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: MessageId::generate(),
            method: method.into(),
            params: Some(params),
        }
    }

    /// Create request with specific ID
    pub fn with_id(
        id: MessageId,
        method: impl Into<MethodName>,
        params: Option<serde_json::Value>,
    ) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.into(),
            params,
        }
    }

    /// Validate the request structure
    pub fn validate(&self) -> AppResult<()> {
        if self.jsonrpc != "2.0" {
            return Err(AppError::mcp("Invalid JSON-RPC version"));
        }

        if self.method.as_str().is_empty() {
            return Err(AppError::mcp("Method name cannot be empty"));
        }

        Ok(())
    }
}

/// MCP response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPResponse {
    /// JSON-RPC version (must be "2.0")
    pub jsonrpc: String,
    /// Request ID
    pub id: MessageId,
    /// Response result (success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Response error (failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<MCPError>,
}

impl MCPResponse {
    /// Create a successful response
    pub fn success(id: MessageId, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response
    pub fn error(id: MessageId, error: MCPError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }

    /// Check if this is a successful response
    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }

    /// Check if this is an error response
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }

    /// Validate the response structure
    pub fn validate(&self) -> AppResult<()> {
        if self.jsonrpc != "2.0" {
            return Err(AppError::mcp("Invalid JSON-RPC version"));
        }

        if self.result.is_some() && self.error.is_some() {
            return Err(AppError::mcp("Response cannot have both result and error"));
        }

        if self.result.is_none() && self.error.is_none() {
            return Err(AppError::mcp("Response must have either result or error"));
        }

        Ok(())
    }
}

/// MCP notification message (no response expected)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPNotification {
    /// JSON-RPC version (must be "2.0")
    pub jsonrpc: String,
    /// Method name
    pub method: MethodName,
    /// Notification parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl MCPNotification {
    /// Create a new notification
    pub fn new(method: impl Into<MethodName>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params: None,
        }
    }

    /// Create notification with parameters
    pub fn with_params(
        method: impl Into<MethodName>,
        params: serde_json::Value,
    ) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params: Some(params),
        }
    }

    /// Validate the notification structure
    pub fn validate(&self) -> AppResult<()> {
        if self.jsonrpc != "2.0" {
            return Err(AppError::mcp("Invalid JSON-RPC version"));
        }

        if self.method.as_str().is_empty() {
            return Err(AppError::mcp("Method name cannot be empty"));
        }

        Ok(())
    }
}

/// MCP message enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MCPMessage {
    Request(MCPRequest),
    Response(MCPResponse),
    Notification(MCPNotification),
}

impl MCPMessage {
    /// Parse a JSON string into an MCP message
    pub fn from_json(json: &str) -> AppResult<Self> {
        serde_json::from_str(json).map_err(|e| AppError::mcp(format!("Failed to parse MCP message: {}", e)))
    }

    /// Serialize message to JSON string
    pub fn to_json(&self) -> AppResult<String> {
        serde_json::to_string(self).map_err(|e| AppError::mcp(format!("Failed to serialize MCP message: {}", e)))
    }

    /// Serialize message to pretty JSON string
    pub fn to_json_pretty(&self) -> AppResult<String> {
        serde_json::to_string_pretty(self).map_err(|e| AppError::mcp(format!("Failed to serialize MCP message: {}", e)))
    }

    /// Validate the message structure
    pub fn validate(&self) -> AppResult<()> {
        match self {
            MCPMessage::Request(req) => req.validate(),
            MCPMessage::Response(res) => res.validate(),
            MCPMessage::Notification(notif) => notif.validate(),
        }
    }

    /// Get the method name (for requests and notifications)
    pub fn method(&self) -> Option<&MethodName> {
        match self {
            MCPMessage::Request(req) => Some(&req.method),
            MCPMessage::Notification(notif) => Some(&notif.method),
            MCPMessage::Response(_) => None,
        }
    }

    /// Get the message ID (for requests and responses)
    pub fn id(&self) -> Option<&MessageId> {
        match self {
            MCPMessage::Request(req) => Some(&req.id),
            MCPMessage::Response(res) => Some(&res.id),
            MCPMessage::Notification(_) => None,
        }
    }

    /// Check if this is a request message
    pub fn is_request(&self) -> bool {
        matches!(self, MCPMessage::Request(_))
    }

    /// Check if this is a response message
    pub fn is_response(&self) -> bool {
        matches!(self, MCPMessage::Response(_))
    }

    /// Check if this is a notification message
    pub fn is_notification(&self) -> bool {
        matches!(self, MCPMessage::Notification(_))
    }
}

/// MCP initialization parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    /// Protocol version
    pub protocol_version: ProtocolVersion,
    /// Client information
    pub client_info: ClientInfo,
    /// Server capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<ServerCapabilities>,
}

/// Client information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    /// Client name
    pub name: String,
    /// Client version
    pub version: String,
}

/// Server capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    /// Supports listing resources
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourceCapabilities>,
    /// Supports tool execution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolCapabilities>,
    /// Supports message sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<SamplingCapabilities>,
}

/// Resource capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceCapabilities {
    /// Supports resource subscriptions
    #[serde(default)]
    pub subscribe: bool,
    /// Supports resource templates
    #[serde(default)]
    pub list_changed: bool,
}

/// Tool capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCapabilities {
    /// Supports listing tools
    #[serde(default)]
    pub list_changed: bool,
}

/// Sampling capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingCapabilities {
    /// Maximum sampling rate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_sampling_rate: Option<f64>,
}

/// MCP initialization result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    /// Protocol version
    pub protocol_version: ProtocolVersion,
    /// Server information
    pub server_info: ServerInfo,
    /// Server capabilities
    pub capabilities: ServerCapabilities,
}

/// Server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    /// Server name
    pub name: String,
    /// Server version
    pub version: String,
}

/// Protocol message statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ProtocolStats {
    /// Total messages processed
    pub total_messages: u64,
    /// Requests processed
    pub requests_processed: u64,
    /// Responses sent
    pub responses_sent: u64,
    /// Notifications sent
    pub notifications_sent: u64,
    /// Errors encountered
    pub errors_encountered: u64,
    /// Invalid messages received
    pub invalid_messages: u64,
    /// Last activity timestamp
    pub last_activity: Option<DateTime<Utc>>,
}

impl ProtocolStats {
    /// Update statistics for a processed message
    pub fn update_for_message(&mut self, message: &MCPMessage) {
        self.total_messages += 1;
        self.last_activity = Some(Utc::now());

        match message {
            MCPMessage::Request(_) => self.requests_processed += 1,
            MCPMessage::Response(_) => self.responses_sent += 1,
            MCPMessage::Notification(_) => self.notifications_sent += 1,
        }
    }

    /// Update statistics for an error
    pub fn update_for_error(&mut self) {
        self.errors_encountered += 1;
        self.last_activity = Some(Utc::now());
    }

    /// Update statistics for an invalid message
    pub fn update_for_invalid(&mut self) {
        self.invalid_messages += 1;
        self.last_activity = Some(Utc::now());
    }

    /// Get error rate
    pub fn error_rate(&self) -> f64 {
        if self.total_messages == 0 {
            0.0
        } else {
            self.errors_encountered as f64 / self.total_messages as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_version() {
        let v1 = ProtocolVersion::new(2024, 11, 5);
        let v2 = ProtocolVersion::new(2024, 11, 1);
        let v3 = ProtocolVersion::new(2023, 12, 1);

        assert!(v1 > v2);
        assert!(v1.is_compatible(&v2));
        assert!(!v1.is_compatible(&v3));

        assert!(v1.supports_feature("sampling"));
        assert!(!v2.supports_feature("sampling"));
        assert!(v2.supports_feature("tools"));
    }

    #[test]
    fn test_protocol_version_parsing() {
        let version_str = "2024.11.5";
        let version: ProtocolVersion = version_str.parse().unwrap();
        assert_eq!(version, ProtocolVersion::new(2024, 11, 5));
        assert_eq!(version.to_string(), version_str);
    }

    #[test]
    fn test_message_id() {
        let id1 = MessageId::from_string("test-id".to_string());
        let id2 = MessageId::from_number(123);

        assert_eq!(id1.to_string(), "test-id");
        assert_eq!(id2.to_string(), "123");
    }

    #[test]
    fn test_mcp_request() {
        let request = MCPRequest::new("test.method");
        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.method.as_str(), "test.method");
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_mcp_response() {
        let id = MessageId::from_string("test-id".to_string());
        let success_response = MCPResponse::success(id.clone(), serde_json::json!({"result": "ok"}));
        assert!(success_response.is_success());
        assert!(!success_response.is_error());
        assert!(success_response.validate().is_ok());

        let error_response = MCPResponse::error(id, MCPError::internal_error("test error"));
        assert!(!error_response.is_success());
        assert!(error_response.is_error());
        assert!(error_response.validate().is_ok());
    }

    #[test]
    fn test_mcp_notification() {
        let notification = MCPNotification::new("test.notification");
        assert_eq!(notification.jsonrpc, "2.0");
        assert_eq!(notification.method.as_str(), "test.notification");
        assert!(notification.validate().is_ok());
    }

    #[test]
    fn test_message_serialization() {
        let request = MCPRequest::new("test.method");
        let message = MCPMessage::Request(request);

        let json = message.to_json().unwrap();
        let parsed = MCPMessage::from_json(&json).unwrap();

        assert!(matches!(parsed, MCPMessage::Request(_)));
        assert!(parsed.validate().is_ok());
    }

    #[test]
    fn test_protocol_stats() {
        let mut stats = ProtocolStats::default();
        assert_eq!(stats.error_rate(), 0.0);

        let request = MCPRequest::new("test");
        let message = MCPMessage::Request(request);
        stats.update_for_message(&message);

        assert_eq!(stats.total_messages, 1);
        assert_eq!(stats.requests_processed, 1);

        stats.update_for_error();
        assert_eq!(stats.errors_encountered, 1);
        assert!(stats.error_rate() > 0.0);
    }
}
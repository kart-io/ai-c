//! MCP Protocol Error Handling
//!
//! Comprehensive error handling for the Model Context Protocol implementation
//! with proper error codes, messages, and recovery strategies.

use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use thiserror::Error;

use crate::error::AppError;
use super::protocol::{MCPError as ProtocolError, MCPErrorCode, MessageId};

/// Result type for MCP operations
pub type MCPResult<T> = Result<T, MCPError>;

/// Comprehensive MCP error enumeration
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum MCPError {
    /// Protocol-level errors
    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    /// Transport-level errors
    #[error("Transport error: {message}")]
    Transport { message: String },

    /// Connection errors
    #[error("Connection error: {message}")]
    Connection { message: String },

    /// Authentication errors
    #[error("Authentication error: {message}")]
    Authentication { message: String },

    /// Authorization errors
    #[error("Authorization error: {message}")]
    Authorization { message: String },

    /// Resource-related errors
    #[error("Resource error: {message}")]
    Resource { message: String },

    /// Tool-related errors
    #[error("Tool error: {message}")]
    Tool { message: String },

    /// Serialization/deserialization errors
    #[error("Serialization error: {message}")]
    Serialization { message: String },

    /// Timeout errors
    #[error("Timeout error: operation timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    /// Rate limiting errors
    #[error("Rate limit exceeded: {message}")]
    RateLimit { message: String },

    /// Configuration errors
    #[error("Configuration error: {message}")]
    Configuration { message: String },

    /// Server unavailable
    #[error("Server unavailable: {message}")]
    ServerUnavailable { message: String },

    /// Client errors
    #[error("Client error: {message}")]
    Client { message: String },

    /// Internal server errors
    #[error("Internal server error: {message}")]
    InternalServer { message: String },

    /// Validation errors
    #[error("Validation error: {message}")]
    Validation { message: String },

    /// Feature not supported
    #[error("Feature not supported: {feature}")]
    FeatureNotSupported { feature: String },

    /// Version mismatch
    #[error("Version mismatch: client {client_version}, server {server_version}")]
    VersionMismatch {
        client_version: String,
        server_version: String,
    },

    /// Resource not found
    #[error("Resource not found: {uri}")]
    ResourceNotFound { uri: String },

    /// Tool not found
    #[error("Tool not found: {name}")]
    ToolNotFound { name: String },

    /// Permission denied
    #[error("Permission denied: {action}")]
    PermissionDenied { action: String },

    /// Invalid request
    #[error("Invalid request: {message}")]
    InvalidRequest { message: String },

    /// Service overloaded
    #[error("Service overloaded: {message}")]
    ServiceOverloaded { message: String },

    /// Unknown error
    #[error("Unknown error: {message}")]
    Unknown { message: String },
}

impl MCPError {
    /// Create a transport error
    pub fn transport(message: impl Into<String>) -> Self {
        Self::Transport {
            message: message.into(),
        }
    }

    /// Create a connection error
    pub fn connection(message: impl Into<String>) -> Self {
        Self::Connection {
            message: message.into(),
        }
    }

    /// Create an authentication error
    pub fn authentication(message: impl Into<String>) -> Self {
        Self::Authentication {
            message: message.into(),
        }
    }

    /// Create an authorization error
    pub fn authorization(message: impl Into<String>) -> Self {
        Self::Authorization {
            message: message.into(),
        }
    }

    /// Create a resource error
    pub fn resource(message: impl Into<String>) -> Self {
        Self::Resource {
            message: message.into(),
        }
    }

    /// Create a tool error
    pub fn tool(message: impl Into<String>) -> Self {
        Self::Tool {
            message: message.into(),
        }
    }

    /// Create a serialization error
    pub fn serialization(message: impl Into<String>) -> Self {
        Self::Serialization {
            message: message.into(),
        }
    }

    /// Create a timeout error
    pub fn timeout(timeout_ms: u64) -> Self {
        Self::Timeout { timeout_ms }
    }

    /// Create a rate limit error
    pub fn rate_limit(message: impl Into<String>) -> Self {
        Self::RateLimit {
            message: message.into(),
        }
    }

    /// Create a configuration error
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration {
            message: message.into(),
        }
    }

    /// Create a server unavailable error
    pub fn server_unavailable(message: impl Into<String>) -> Self {
        Self::ServerUnavailable {
            message: message.into(),
        }
    }

    /// Create a client error
    pub fn client(message: impl Into<String>) -> Self {
        Self::Client {
            message: message.into(),
        }
    }

    /// Create an internal server error
    pub fn internal_server(message: impl Into<String>) -> Self {
        Self::InternalServer {
            message: message.into(),
        }
    }

    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    /// Create a feature not supported error
    pub fn feature_not_supported(feature: impl Into<String>) -> Self {
        Self::FeatureNotSupported {
            feature: feature.into(),
        }
    }

    /// Create a version mismatch error
    pub fn version_mismatch(
        client_version: impl Into<String>,
        server_version: impl Into<String>,
    ) -> Self {
        Self::VersionMismatch {
            client_version: client_version.into(),
            server_version: server_version.into(),
        }
    }

    /// Create a resource not found error
    pub fn resource_not_found(uri: impl Into<String>) -> Self {
        Self::ResourceNotFound { uri: uri.into() }
    }

    /// Create a tool not found error
    pub fn tool_not_found(name: impl Into<String>) -> Self {
        Self::ToolNotFound { name: name.into() }
    }

    /// Create a permission denied error
    pub fn permission_denied(action: impl Into<String>) -> Self {
        Self::PermissionDenied {
            action: action.into(),
        }
    }

    /// Create an invalid request error
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::InvalidRequest {
            message: message.into(),
        }
    }

    /// Create a service overloaded error
    pub fn service_overloaded(message: impl Into<String>) -> Self {
        Self::ServiceOverloaded {
            message: message.into(),
        }
    }

    /// Create an unknown error
    pub fn unknown(message: impl Into<String>) -> Self {
        Self::Unknown {
            message: message.into(),
        }
    }

    /// Convert to protocol error for JSON-RPC response
    pub fn to_protocol_error(&self) -> ProtocolError {
        match self {
            MCPError::Protocol(err) => err.clone(),
            MCPError::Transport { message } => ProtocolError::server_error(message),
            MCPError::Connection { message } => ProtocolError::server_error(message),
            MCPError::Authentication { message } => ProtocolError::server_error(message),
            MCPError::Authorization { message } => ProtocolError::server_error(message),
            MCPError::Resource { message } => ProtocolError::server_error(message),
            MCPError::Tool { message } => ProtocolError::server_error(message),
            MCPError::Serialization { message } => ProtocolError::parse_error(message),
            MCPError::Timeout { timeout_ms } => {
                ProtocolError::server_error(format!("Operation timed out after {}ms", timeout_ms))
            }
            MCPError::RateLimit { message } => ProtocolError::server_error(message),
            MCPError::Configuration { message } => ProtocolError::server_error(message),
            MCPError::ServerUnavailable { message } => ProtocolError::server_error(message),
            MCPError::Client { message } => ProtocolError::invalid_request(message),
            MCPError::InternalServer { message } => ProtocolError::internal_error(message),
            MCPError::Validation { message } => ProtocolError::invalid_params(message),
            MCPError::FeatureNotSupported { feature } => {
                ProtocolError::method_not_found(format!("Feature '{}' not supported", feature))
            }
            MCPError::VersionMismatch {
                client_version,
                server_version,
            } => ProtocolError::server_error(format!(
                "Version mismatch: client {}, server {}",
                client_version, server_version
            )),
            MCPError::ResourceNotFound { uri } => {
                ProtocolError::server_error(format!("Resource not found: {}", uri))
            }
            MCPError::ToolNotFound { name } => {
                ProtocolError::method_not_found(format!("Tool '{}' not found", name))
            }
            MCPError::PermissionDenied { action } => {
                ProtocolError::server_error(format!("Permission denied: {}", action))
            }
            MCPError::InvalidRequest { message } => ProtocolError::invalid_request(message),
            MCPError::ServiceOverloaded { message } => ProtocolError::server_error(message),
            MCPError::Unknown { message } => ProtocolError::internal_error(message),
        }
    }

    /// Get error category for metrics and logging
    pub fn category(&self) -> ErrorCategory {
        match self {
            MCPError::Protocol(_) => ErrorCategory::Protocol,
            MCPError::Transport { .. } => ErrorCategory::Transport,
            MCPError::Connection { .. } => ErrorCategory::Connection,
            MCPError::Authentication { .. } => ErrorCategory::Authentication,
            MCPError::Authorization { .. } => ErrorCategory::Authorization,
            MCPError::Resource { .. } => ErrorCategory::Resource,
            MCPError::Tool { .. } => ErrorCategory::Tool,
            MCPError::Serialization { .. } => ErrorCategory::Serialization,
            MCPError::Timeout { .. } => ErrorCategory::Timeout,
            MCPError::RateLimit { .. } => ErrorCategory::RateLimit,
            MCPError::Configuration { .. } => ErrorCategory::Configuration,
            MCPError::ServerUnavailable { .. } => ErrorCategory::ServerUnavailable,
            MCPError::Client { .. } => ErrorCategory::Client,
            MCPError::InternalServer { .. } => ErrorCategory::InternalServer,
            MCPError::Validation { .. } => ErrorCategory::Validation,
            MCPError::FeatureNotSupported { .. } => ErrorCategory::FeatureNotSupported,
            MCPError::VersionMismatch { .. } => ErrorCategory::VersionMismatch,
            MCPError::ResourceNotFound { .. } => ErrorCategory::ResourceNotFound,
            MCPError::ToolNotFound { .. } => ErrorCategory::ToolNotFound,
            MCPError::PermissionDenied { .. } => ErrorCategory::PermissionDenied,
            MCPError::InvalidRequest { .. } => ErrorCategory::InvalidRequest,
            MCPError::ServiceOverloaded { .. } => ErrorCategory::ServiceOverloaded,
            MCPError::Unknown { .. } => ErrorCategory::Unknown,
        }
    }

    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            MCPError::Transport { .. } => true,
            MCPError::Connection { .. } => true,
            MCPError::Timeout { .. } => true,
            MCPError::RateLimit { .. } => true,
            MCPError::ServerUnavailable { .. } => true,
            MCPError::ServiceOverloaded { .. } => true,
            MCPError::InternalServer { .. } => true,
            _ => false,
        }
    }

    /// Get suggested retry delay in milliseconds
    pub fn retry_delay_ms(&self) -> Option<u64> {
        match self {
            MCPError::RateLimit { .. } => Some(5000),      // 5 seconds
            MCPError::ServerUnavailable { .. } => Some(10000), // 10 seconds
            MCPError::ServiceOverloaded { .. } => Some(3000),  // 3 seconds
            MCPError::Connection { .. } => Some(1000),     // 1 second
            MCPError::Transport { .. } => Some(1000),      // 1 second
            MCPError::Timeout { .. } => Some(2000),        // 2 seconds
            MCPError::InternalServer { .. } => Some(5000), // 5 seconds
            _ => None,
        }
    }

    /// Get error severity level
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            MCPError::Authentication { .. } => ErrorSeverity::Critical,
            MCPError::Authorization { .. } => ErrorSeverity::Critical,
            MCPError::InternalServer { .. } => ErrorSeverity::High,
            MCPError::ServerUnavailable { .. } => ErrorSeverity::High,
            MCPError::Connection { .. } => ErrorSeverity::Medium,
            MCPError::Transport { .. } => ErrorSeverity::Medium,
            MCPError::Timeout { .. } => ErrorSeverity::Medium,
            MCPError::RateLimit { .. } => ErrorSeverity::Medium,
            MCPError::ServiceOverloaded { .. } => ErrorSeverity::Medium,
            MCPError::ResourceNotFound { .. } => ErrorSeverity::Low,
            MCPError::ToolNotFound { .. } => ErrorSeverity::Low,
            MCPError::Validation { .. } => ErrorSeverity::Low,
            MCPError::InvalidRequest { .. } => ErrorSeverity::Low,
            _ => ErrorSeverity::Medium,
        }
    }
}

/// Error category for classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorCategory {
    Protocol,
    Transport,
    Connection,
    Authentication,
    Authorization,
    Resource,
    Tool,
    Serialization,
    Timeout,
    RateLimit,
    Configuration,
    ServerUnavailable,
    Client,
    InternalServer,
    Validation,
    FeatureNotSupported,
    VersionMismatch,
    ResourceNotFound,
    ToolNotFound,
    PermissionDenied,
    InvalidRequest,
    ServiceOverloaded,
    Unknown,
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Error context for detailed error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    /// Request ID that caused the error
    pub request_id: Option<MessageId>,
    /// Method name that was being executed
    pub method: Option<String>,
    /// Timestamp when the error occurred
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Additional context data
    pub context: serde_json::Value,
    /// Stack trace or call chain
    pub stack_trace: Option<Vec<String>>,
}

impl ErrorContext {
    /// Create a new error context
    pub fn new() -> Self {
        Self {
            request_id: None,
            method: None,
            timestamp: chrono::Utc::now(),
            context: serde_json::Value::Null,
            stack_trace: None,
        }
    }

    /// Add request ID to context
    pub fn with_request_id(mut self, request_id: MessageId) -> Self {
        self.request_id = Some(request_id);
        self
    }

    /// Add method name to context
    pub fn with_method(mut self, method: impl Into<String>) -> Self {
        self.method = Some(method.into());
        self
    }

    /// Add context data
    pub fn with_context(mut self, context: serde_json::Value) -> Self {
        self.context = context;
        self
    }

    /// Add stack trace
    pub fn with_stack_trace(mut self, stack_trace: Vec<String>) -> Self {
        self.stack_trace = Some(stack_trace);
        self
    }
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Error statistics for monitoring
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ErrorStats {
    /// Total errors by category
    pub errors_by_category: std::collections::HashMap<ErrorCategory, u64>,
    /// Total errors by severity
    pub errors_by_severity: std::collections::HashMap<ErrorSeverity, u64>,
    /// Total retryable errors
    pub retryable_errors: u64,
    /// Total non-retryable errors
    pub non_retryable_errors: u64,
    /// Last error timestamp
    pub last_error: Option<chrono::DateTime<chrono::Utc>>,
}

impl ErrorStats {
    /// Update statistics for a new error
    pub fn update(&mut self, error: &MCPError) {
        let category = error.category();
        let severity = error.severity();

        *self.errors_by_category.entry(category).or_insert(0) += 1;
        *self.errors_by_severity.entry(severity).or_insert(0) += 1;

        if error.is_retryable() {
            self.retryable_errors += 1;
        } else {
            self.non_retryable_errors += 1;
        }

        self.last_error = Some(chrono::Utc::now());
    }

    /// Get total error count
    pub fn total_errors(&self) -> u64 {
        self.retryable_errors + self.non_retryable_errors
    }

    /// Get error rate by category
    pub fn error_rate_by_category(&self, category: ErrorCategory) -> f64 {
        let total = self.total_errors();
        if total == 0 {
            0.0
        } else {
            let category_errors = self.errors_by_category.get(&category).unwrap_or(&0);
            *category_errors as f64 / total as f64
        }
    }

    /// Get error rate by severity
    pub fn error_rate_by_severity(&self, severity: ErrorSeverity) -> f64 {
        let total = self.total_errors();
        if total == 0 {
            0.0
        } else {
            let severity_errors = self.errors_by_severity.get(&severity).unwrap_or(&0);
            *severity_errors as f64 / total as f64
        }
    }

    /// Get retryable error rate
    pub fn retryable_error_rate(&self) -> f64 {
        let total = self.total_errors();
        if total == 0 {
            0.0
        } else {
            self.retryable_errors as f64 / total as f64
        }
    }
}

/// Convert from AppError to MCPError
impl From<AppError> for MCPError {
    fn from(err: AppError) -> Self {
        match err {
            AppError::Git(_) => MCPError::resource(err.to_string()),
            AppError::Io(_) => MCPError::transport(err.to_string()),
            AppError::Json(_) => MCPError::serialization(err.to_string()),
            AppError::Agent(_) => MCPError::internal_server(err.to_string()),
            AppError::Config(_) => MCPError::configuration(err.to_string()),
            AppError::Mcp(_) => MCPError::unknown(err.to_string()),
            AppError::Validation(_) => MCPError::validation(err.to_string()),
            AppError::Network(_) => MCPError::transport(err.to_string()),
            AppError::Auth(_) => MCPError::authentication(err.to_string()),
            AppError::Permission(_) => MCPError::authorization(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = MCPError::transport("Connection failed");
        assert_eq!(error.category(), ErrorCategory::Transport);
        assert!(error.is_retryable());
        assert_eq!(error.retry_delay_ms(), Some(1000));
    }

    #[test]
    fn test_error_severity() {
        let auth_error = MCPError::authentication("Invalid token");
        assert_eq!(auth_error.severity(), ErrorSeverity::Critical);

        let not_found_error = MCPError::resource_not_found("file://test.txt");
        assert_eq!(not_found_error.severity(), ErrorSeverity::Low);
    }

    #[test]
    fn test_protocol_error_conversion() {
        let mcp_error = MCPError::timeout(5000);
        let protocol_error = mcp_error.to_protocol_error();

        assert_eq!(protocol_error.code, MCPErrorCode::ServerError as i32);
        assert!(protocol_error.message.contains("5000ms"));
    }

    #[test]
    fn test_error_stats() {
        let mut stats = ErrorStats::default();

        let error1 = MCPError::transport("Test error 1");
        let error2 = MCPError::authentication("Test error 2");

        stats.update(&error1);
        stats.update(&error2);

        assert_eq!(stats.total_errors(), 2);
        assert_eq!(stats.retryable_errors, 1);
        assert_eq!(stats.non_retryable_errors, 1);
        assert!(stats.error_rate_by_category(ErrorCategory::Transport) > 0.0);
    }

    #[test]
    fn test_error_context() {
        let context = ErrorContext::new()
            .with_method("test.method")
            .with_context(serde_json::json!({"key": "value"}));

        assert_eq!(context.method, Some("test.method".to_string()));
        assert!(context.timestamp <= chrono::Utc::now());
    }
}
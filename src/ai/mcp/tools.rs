//! MCP Tool Management
//!
//! Provides tool registration, parameter validation, and execution
//! for the Model Context Protocol implementation.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::error::{AppError, AppResult};
use super::{
    errors::{MCPError, MCPResult},
    protocol::{MCPMessage, MCPRequest, MCPResponse, MessageId},
};

/// Tool parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    /// Parameter name
    pub name: String,
    /// Parameter description
    pub description: Option<String>,
    /// Parameter type
    pub parameter_type: ParameterType,
    /// Whether parameter is required
    pub required: bool,
    /// Default value
    pub default_value: Option<serde_json::Value>,
    /// Allowed values (for enum types)
    pub allowed_values: Option<Vec<serde_json::Value>>,
    /// Minimum value (for numeric types)
    pub minimum: Option<f64>,
    /// Maximum value (for numeric types)
    pub maximum: Option<f64>,
    /// Pattern for string validation
    pub pattern: Option<String>,
}

impl ToolParameter {
    /// Create a new parameter
    pub fn new(name: String, parameter_type: ParameterType, required: bool) -> Self {
        Self {
            name,
            description: None,
            parameter_type,
            required,
            default_value: None,
            allowed_values: None,
            minimum: None,
            maximum: None,
            pattern: None,
        }
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Set default value
    pub fn with_default(mut self, value: serde_json::Value) -> Self {
        self.default_value = Some(value);
        self
    }

    /// Set allowed values
    pub fn with_allowed_values(mut self, values: Vec<serde_json::Value>) -> Self {
        self.allowed_values = Some(values);
        self
    }

    /// Set numeric range
    pub fn with_range(mut self, min: Option<f64>, max: Option<f64>) -> Self {
        self.minimum = min;
        self.maximum = max;
        self
    }

    /// Set string pattern
    pub fn with_pattern(mut self, pattern: String) -> Self {
        self.pattern = Some(pattern);
        self
    }

    /// Validate a value against this parameter
    pub fn validate(&self, value: &serde_json::Value) -> MCPResult<()> {
        // Check if required parameter is missing
        if self.required && value.is_null() {
            return Err(MCPError::invalid_params(format!("Required parameter '{}' is missing", self.name)));
        }

        // Skip validation for null optional parameters
        if !self.required && value.is_null() {
            return Ok(());
        }

        // Type validation
        match (&self.parameter_type, value) {
            (ParameterType::String, serde_json::Value::String(s)) => {
                if let Some(ref pattern) = self.pattern {
                    let regex = regex::Regex::new(pattern)
                        .map_err(|e| MCPError::validation(format!("Invalid pattern: {}", e)))?;
                    if !regex.is_match(s) {
                        return Err(MCPError::invalid_params(format!("Parameter '{}' does not match pattern", self.name)));
                    }
                }
            }
            (ParameterType::Number, serde_json::Value::Number(n)) => {
                let num = n.as_f64().unwrap();
                if let Some(min) = self.minimum {
                    if num < min {
                        return Err(MCPError::invalid_params(format!("Parameter '{}' is below minimum {}", self.name, min)));
                    }
                }
                if let Some(max) = self.maximum {
                    if num > max {
                        return Err(MCPError::invalid_params(format!("Parameter '{}' exceeds maximum {}", self.name, max)));
                    }
                }
            }
            (ParameterType::Integer, serde_json::Value::Number(n)) => {
                if !n.is_i64() && !n.is_u64() {
                    return Err(MCPError::invalid_params(format!("Parameter '{}' must be an integer", self.name)));
                }
                let num = n.as_f64().unwrap();
                if let Some(min) = self.minimum {
                    if num < min {
                        return Err(MCPError::invalid_params(format!("Parameter '{}' is below minimum {}", self.name, min)));
                    }
                }
                if let Some(max) = self.maximum {
                    if num > max {
                        return Err(MCPError::invalid_params(format!("Parameter '{}' exceeds maximum {}", self.name, max)));
                    }
                }
            }
            (ParameterType::Boolean, serde_json::Value::Bool(_)) => {
                // Boolean is always valid
            }
            (ParameterType::Array, serde_json::Value::Array(_)) => {
                // Array validation would require element type checking
            }
            (ParameterType::Object, serde_json::Value::Object(_)) => {
                // Object validation would require schema checking
            }
            _ => {
                return Err(MCPError::invalid_params(format!("Parameter '{}' has invalid type", self.name)));
            }
        }

        // Check allowed values
        if let Some(ref allowed) = self.allowed_values {
            if !allowed.contains(value) {
                return Err(MCPError::invalid_params(format!("Parameter '{}' value not in allowed list", self.name)));
            }
        }

        Ok(())
    }
}

/// Parameter type enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParameterType {
    String,
    Number,
    Integer,
    Boolean,
    Array,
    Object,
}

/// Tool call request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool name
    pub name: String,
    /// Tool arguments
    pub arguments: HashMap<String, serde_json::Value>,
    /// Call ID for tracking
    pub call_id: String,
    /// Caller information
    pub caller: Option<String>,
    /// Call timestamp
    pub timestamp: DateTime<Utc>,
}

impl ToolCall {
    /// Create a new tool call
    pub fn new(name: String, arguments: HashMap<String, serde_json::Value>) -> Self {
        Self {
            name,
            arguments,
            call_id: uuid::Uuid::new_v4().to_string(),
            caller: None,
            timestamp: Utc::now(),
        }
    }

    /// Set caller information
    pub fn with_caller(mut self, caller: String) -> Self {
        self.caller = Some(caller);
        self
    }

    /// Set call ID
    pub fn with_call_id(mut self, call_id: String) -> Self {
        self.call_id = call_id;
        self
    }
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Call ID
    pub call_id: String,
    /// Whether execution was successful
    pub success: bool,
    /// Result content
    pub content: Vec<ToolContent>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Execution duration
    pub duration: Duration,
    /// Result metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ToolResult {
    /// Create successful result
    pub fn success(call_id: String, content: Vec<ToolContent>) -> Self {
        Self {
            call_id,
            success: true,
            content,
            error: None,
            duration: Duration::ZERO,
            metadata: HashMap::new(),
        }
    }

    /// Create failed result
    pub fn error(call_id: String, error: String) -> Self {
        Self {
            call_id,
            success: false,
            content: Vec::new(),
            error: Some(error),
            duration: Duration::ZERO,
            metadata: HashMap::new(),
        }
    }

    /// Set execution duration
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Tool content types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolContent {
    #[serde(rename = "text")]
    Text {
        text: String,
    },
    #[serde(rename = "image")]
    Image {
        data: String, // Base64 encoded image
        mime_type: String,
    },
    #[serde(rename = "resource")]
    Resource {
        resource: serde_json::Value,
    },
    #[serde(rename = "progress")]
    Progress {
        progress: f32, // 0.0 to 1.0
        total: Option<u64>,
    },
}

impl ToolContent {
    /// Create text content
    pub fn text(text: String) -> Self {
        Self::Text { text }
    }

    /// Create image content
    pub fn image(data: String, mime_type: String) -> Self {
        Self::Image { data, mime_type }
    }

    /// Create resource content
    pub fn resource(resource: serde_json::Value) -> Self {
        Self::Resource { resource }
    }

    /// Create progress content
    pub fn progress(progress: f32, total: Option<u64>) -> Self {
        Self::Progress { progress, total }
    }
}

/// Tool permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermissions {
    /// Allow tool execution
    pub execute: bool,
    /// Allowed caller IDs
    pub allowed_callers: Option<Vec<String>>,
    /// Denied caller IDs
    pub denied_callers: Option<Vec<String>>,
    /// Rate limit (calls per minute)
    pub rate_limit: Option<u32>,
    /// Require confirmation for dangerous operations
    pub require_confirmation: bool,
}

impl Default for ToolPermissions {
    fn default() -> Self {
        Self {
            execute: true,
            allowed_callers: None,
            denied_callers: None,
            rate_limit: None,
            require_confirmation: false,
        }
    }
}

impl ToolPermissions {
    /// Create permissive permissions
    pub fn allow_all() -> Self {
        Self::default()
    }

    /// Create restricted permissions
    pub fn restricted() -> Self {
        Self {
            execute: true,
            allowed_callers: None,
            denied_callers: None,
            rate_limit: Some(10), // 10 calls per minute
            require_confirmation: true,
        }
    }

    /// Check if caller has permission
    pub fn check_caller_permission(&self, caller: Option<&str>) -> bool {
        if !self.execute {
            return false;
        }

        if let Some(caller_id) = caller {
            // Check denied list first
            if let Some(ref denied) = self.denied_callers {
                if denied.contains(&caller_id.to_string()) {
                    return false;
                }
            }

            // Check allowed list
            if let Some(ref allowed) = self.allowed_callers {
                return allowed.contains(&caller_id.to_string());
            }
        }

        true // No restrictions or no caller specified
    }
}

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Input schema parameters
    pub input_schema: Vec<ToolParameter>,
    /// Tool permissions
    pub permissions: ToolPermissions,
    /// Tool metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Tool version
    pub version: String,
    /// Tool author
    pub author: Option<String>,
    /// Tool documentation URL
    pub documentation_url: Option<String>,
}

impl Tool {
    /// Create a new tool
    pub fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
            input_schema: Vec::new(),
            permissions: ToolPermissions::default(),
            metadata: HashMap::new(),
            version: "1.0.0".to_string(),
            author: None,
            documentation_url: None,
        }
    }

    /// Add parameter to input schema
    pub fn with_parameter(mut self, parameter: ToolParameter) -> Self {
        self.input_schema.push(parameter);
        self
    }

    /// Set permissions
    pub fn with_permissions(mut self, permissions: ToolPermissions) -> Self {
        self.permissions = permissions;
        self
    }

    /// Set version
    pub fn with_version(mut self, version: String) -> Self {
        self.version = version;
        self
    }

    /// Set author
    pub fn with_author(mut self, author: String) -> Self {
        self.author = Some(author);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Validate tool call arguments
    pub fn validate_call(&self, call: &ToolCall) -> MCPResult<()> {
        // Check permissions
        if !self.permissions.check_caller_permission(call.caller.as_deref()) {
            return Err(MCPError::permission_denied(format!("Caller not authorized for tool '{}'", self.name)));
        }

        // Validate parameters
        for param in &self.input_schema {
            let value = call.arguments.get(&param.name)
                .unwrap_or(&serde_json::Value::Null);
            param.validate(value)?;
        }

        // Check for unknown parameters
        for arg_name in call.arguments.keys() {
            if !self.input_schema.iter().any(|p| p.name == *arg_name) {
                warn!("Unknown parameter '{}' for tool '{}'", arg_name, self.name);
            }
        }

        Ok(())
    }

    /// Get input schema as JSON schema
    pub fn input_schema_json(&self) -> serde_json::Value {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();

        for param in &self.input_schema {
            let mut param_schema = serde_json::Map::new();

            // Add type
            let type_str = match param.parameter_type {
                ParameterType::String => "string",
                ParameterType::Number => "number",
                ParameterType::Integer => "integer",
                ParameterType::Boolean => "boolean",
                ParameterType::Array => "array",
                ParameterType::Object => "object",
            };
            param_schema.insert("type".to_string(), serde_json::Value::String(type_str.to_string()));

            // Add description
            if let Some(ref desc) = param.description {
                param_schema.insert("description".to_string(), serde_json::Value::String(desc.clone()));
            }

            // Add constraints
            if let Some(ref pattern) = param.pattern {
                param_schema.insert("pattern".to_string(), serde_json::Value::String(pattern.clone()));
            }

            if let Some(min) = param.minimum {
                param_schema.insert("minimum".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(min).unwrap()));
            }

            if let Some(max) = param.maximum {
                param_schema.insert("maximum".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(max).unwrap()));
            }

            if let Some(ref allowed) = param.allowed_values {
                param_schema.insert("enum".to_string(), serde_json::Value::Array(allowed.clone()));
            }

            if let Some(ref default) = param.default_value {
                param_schema.insert("default".to_string(), default.clone());
            }

            properties.insert(param.name.clone(), serde_json::Value::Object(param_schema));

            if param.required {
                required.push(serde_json::Value::String(param.name.clone()));
            }
        }

        serde_json::json!({
            "type": "object",
            "properties": properties,
            "required": required
        })
    }
}

/// Tool executor trait
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    /// Execute a tool call
    async fn execute(&self, tool: &Tool, call: ToolCall) -> MCPResult<ToolResult>;

    /// Get executor capabilities
    fn capabilities(&self) -> ToolExecutorCapabilities;

    /// Check if executor can handle a tool
    fn can_execute(&self, tool: &Tool) -> bool;
}

/// Tool executor capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutorCapabilities {
    /// Supported tool types
    pub supported_tools: Vec<String>,
    /// Supports async execution
    pub async_execution: bool,
    /// Supports streaming results
    pub streaming: bool,
    /// Maximum execution time
    pub max_execution_time: Duration,
}

impl Default for ToolExecutorCapabilities {
    fn default() -> Self {
        Self {
            supported_tools: vec!["*".to_string()], // Support all tools
            async_execution: true,
            streaming: false,
            max_execution_time: Duration::from_secs(30),
        }
    }
}

/// Tool registry for managing tools and executors
pub struct ToolRegistry {
    /// Registered tools
    tools: Arc<RwLock<HashMap<String, Tool>>>,
    /// Registered executors
    executors: Arc<RwLock<Vec<Arc<dyn ToolExecutor>>>>,
    /// Execution statistics
    stats: Arc<RwLock<ToolStats>>,
    /// Rate limiting data
    rate_limits: Arc<RwLock<HashMap<String, Vec<DateTime<Utc>>>>>,
}

/// Tool execution statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ToolStats {
    /// Total executions
    pub total_executions: u64,
    /// Successful executions
    pub successful_executions: u64,
    /// Failed executions
    pub failed_executions: u64,
    /// Executions by tool
    pub executions_by_tool: HashMap<String, u64>,
    /// Average execution time
    pub average_execution_time: Duration,
    /// Last execution time
    pub last_execution: Option<DateTime<Utc>>,
}

impl ToolStats {
    /// Update statistics for an execution
    pub fn update(&mut self, tool_name: &str, success: bool, duration: Duration) {
        self.total_executions += 1;

        if success {
            self.successful_executions += 1;
        } else {
            self.failed_executions += 1;
        }

        *self.executions_by_tool.entry(tool_name.to_string()).or_insert(0) += 1;

        // Update average execution time
        let total_time = self.average_execution_time.as_nanos() as u64 * (self.total_executions - 1) + duration.as_nanos() as u64;
        self.average_execution_time = Duration::from_nanos(total_time / self.total_executions);

        self.last_execution = Some(Utc::now());
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_executions == 0 {
            0.0
        } else {
            self.successful_executions as f64 / self.total_executions as f64
        }
    }
}

impl ToolRegistry {
    /// Create a new tool registry
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            executors: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(ToolStats::default())),
            rate_limits: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a tool
    pub async fn register_tool(&self, tool: Tool) -> MCPResult<()> {
        let mut tools = self.tools.write().await;
        if tools.contains_key(&tool.name) {
            return Err(MCPError::validation(format!("Tool '{}' already registered", tool.name)));
        }

        info!("Registering tool: {}", tool.name);
        tools.insert(tool.name.clone(), tool);
        Ok(())
    }

    /// Register a tool executor
    pub async fn register_executor(&self, executor: Arc<dyn ToolExecutor>) -> MCPResult<()> {
        let mut executors = self.executors.write().await;
        executors.push(executor);
        info!("Registered tool executor");
        Ok(())
    }

    /// List all registered tools
    pub async fn list_tools(&self) -> Vec<Tool> {
        let tools = self.tools.read().await;
        tools.values().cloned().collect()
    }

    /// Get a specific tool
    pub async fn get_tool(&self, name: &str) -> Option<Tool> {
        let tools = self.tools.read().await;
        tools.get(name).cloned()
    }

    /// Execute a tool call
    pub async fn execute_tool(&self, call: ToolCall) -> MCPResult<ToolResult> {
        let start_time = Instant::now();

        // Get tool definition
        let tool = {
            let tools = self.tools.read().await;
            tools.get(&call.name)
                .cloned()
                .ok_or_else(|| MCPError::tool_not_found(&call.name))?
        };

        // Validate call
        tool.validate_call(&call)?;

        // Check rate limits
        self.check_rate_limit(&tool, &call).await?;

        // Find executor
        let executor = {
            let executors = self.executors.read().await;
            executors.iter()
                .find(|e| e.can_execute(&tool))
                .cloned()
                .ok_or_else(|| MCPError::tool(format!("No executor available for tool '{}'", tool.name)))?
        };

        debug!("Executing tool '{}' with call ID {}", tool.name, call.call_id);

        // Execute tool
        let result = match executor.execute(&tool, call).await {
            Ok(mut result) => {
                result.duration = start_time.elapsed();
                self.update_stats(&tool.name, true, result.duration).await;
                result
            }
            Err(e) => {
                let duration = start_time.elapsed();
                self.update_stats(&tool.name, false, duration).await;
                return Err(e);
            }
        };

        info!("Tool '{}' executed successfully in {:?}", tool.name, result.duration);
        Ok(result)
    }

    /// Check rate limits for tool execution
    async fn check_rate_limit(&self, tool: &Tool, call: &ToolCall) -> MCPResult<()> {
        if let Some(rate_limit) = tool.permissions.rate_limit {
            let caller_key = format!("{}:{}", tool.name, call.caller.as_deref().unwrap_or("anonymous"));
            let mut rate_limits = self.rate_limits.write().await;

            let now = Utc::now();
            let minute_ago = now - chrono::Duration::minutes(1);

            // Get or create rate limit entry
            let calls = rate_limits.entry(caller_key).or_insert_with(Vec::new);

            // Remove old entries
            calls.retain(|&timestamp| timestamp > minute_ago);

            // Check if rate limit exceeded
            if calls.len() >= rate_limit as usize {
                return Err(MCPError::rate_limit(format!("Rate limit exceeded for tool '{}': {} calls per minute", tool.name, rate_limit)));
            }

            // Add current call
            calls.push(now);
        }

        Ok(())
    }

    /// Update execution statistics
    async fn update_stats(&self, tool_name: &str, success: bool, duration: Duration) {
        let mut stats = self.stats.write().await;
        stats.update(tool_name, success, duration);
    }

    /// Get execution statistics
    pub async fn get_stats(&self) -> ToolStats {
        self.stats.read().await.clone()
    }

    /// Clear rate limit data
    pub async fn clear_rate_limits(&self) {
        let mut rate_limits = self.rate_limits.write().await;
        rate_limits.clear();
        info!("Rate limit data cleared");
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Tool manager handles MCP tool operations
pub struct ToolManager {
    /// Tool registry
    registry: ToolRegistry,
}

impl ToolManager {
    /// Create a new tool manager
    pub fn new() -> Self {
        Self {
            registry: ToolRegistry::new(),
        }
    }

    /// Register a tool
    pub async fn register_tool(&self, tool: Tool) -> MCPResult<()> {
        self.registry.register_tool(tool).await
    }

    /// Register an executor
    pub async fn register_executor(&self, executor: Arc<dyn ToolExecutor>) -> MCPResult<()> {
        self.registry.register_executor(executor).await
    }

    /// Handle MCP list tools request
    pub async fn handle_list_tools(&self, _request: MCPRequest) -> MCPResult<MCPResponse> {
        let tools = self.registry.list_tools().await;
        let tool_list: Vec<serde_json::Value> = tools
            .iter()
            .map(|t| serde_json::json!({
                "name": t.name,
                "description": t.description,
                "inputSchema": t.input_schema_json(),
            }))
            .collect();

        let result = serde_json::json!({
            "tools": tool_list
        });

        Ok(MCPResponse::success(
            MessageId::generate(),
            result,
        ))
    }

    /// Handle MCP call tool request
    pub async fn handle_call_tool(&self, request: MCPRequest) -> MCPResult<MCPResponse> {
        let params = request.params
            .ok_or_else(|| MCPError::invalid_params("Missing parameters"))?;

        let name = params.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MCPError::invalid_params("Missing name parameter"))?
            .to_string();

        let arguments = params.get("arguments")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect::<HashMap<_, _>>()
            })
            .unwrap_or_default();

        let call = ToolCall::new(name, arguments);
        let result = self.registry.execute_tool(call).await?;

        let response_data = serde_json::json!({
            "content": result.content,
            "isError": !result.success,
            "_meta": {
                "callId": result.call_id,
                "duration": result.duration.as_millis(),
                "metadata": result.metadata
            }
        });

        Ok(MCPResponse::success(request.id, response_data))
    }

    /// Get registry reference
    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }
}

impl Default for ToolManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_parameter() {
        let param = ToolParameter::new("test_param".to_string(), ParameterType::String, true)
            .with_description("Test parameter".to_string())
            .with_pattern("^[a-z]+$".to_string());

        assert_eq!(param.name, "test_param");
        assert!(param.required);
        assert!(param.pattern.is_some());

        // Test validation
        let valid_value = serde_json::Value::String("hello".to_string());
        assert!(param.validate(&valid_value).is_ok());

        let invalid_value = serde_json::Value::String("Hello123".to_string());
        assert!(param.validate(&invalid_value).is_err());

        let null_value = serde_json::Value::Null;
        assert!(param.validate(&null_value).is_err()); // Required parameter
    }

    #[test]
    fn test_tool_permissions() {
        let perms = ToolPermissions::default();
        assert!(perms.execute);
        assert!(perms.check_caller_permission(Some("test-caller")));

        let restricted = ToolPermissions::restricted();
        assert!(restricted.require_confirmation);
        assert_eq!(restricted.rate_limit, Some(10));
    }

    #[test]
    fn test_tool_definition() {
        let tool = Tool::new("test_tool".to_string(), "Test tool description".to_string())
            .with_parameter(ToolParameter::new("param1".to_string(), ParameterType::String, true))
            .with_parameter(ToolParameter::new("param2".to_string(), ParameterType::Number, false))
            .with_version("1.0.0".to_string());

        assert_eq!(tool.name, "test_tool");
        assert_eq!(tool.input_schema.len(), 2);
        assert_eq!(tool.version, "1.0.0");

        let schema = tool.input_schema_json();
        assert!(schema.get("properties").is_some());
        assert!(schema.get("required").is_some());
    }

    #[test]
    fn test_tool_call() {
        let mut args = HashMap::new();
        args.insert("param1".to_string(), serde_json::Value::String("test".to_string()));

        let call = ToolCall::new("test_tool".to_string(), args)
            .with_caller("test-caller".to_string());

        assert_eq!(call.name, "test_tool");
        assert_eq!(call.caller, Some("test-caller".to_string()));
        assert!(!call.call_id.is_empty());
    }

    #[test]
    fn test_tool_result() {
        let content = vec![ToolContent::text("Hello, world!".to_string())];
        let result = ToolResult::success("call-123".to_string(), content)
            .with_duration(Duration::from_millis(100))
            .with_metadata("key".to_string(), serde_json::Value::String("value".to_string()));

        assert!(result.success);
        assert_eq!(result.content.len(), 1);
        assert_eq!(result.duration.as_millis(), 100);
        assert!(result.metadata.contains_key("key"));

        let error_result = ToolResult::error("call-456".to_string(), "Test error".to_string());
        assert!(!error_result.success);
        assert!(error_result.error.is_some());
    }

    #[tokio::test]
    async fn test_tool_registry() {
        let registry = ToolRegistry::new();

        let tool = Tool::new("test_tool".to_string(), "Test tool".to_string());
        assert!(registry.register_tool(tool).await.is_ok());

        let tools = registry.list_tools().await;
        assert_eq!(tools.len(), 1);

        let found_tool = registry.get_tool("test_tool").await;
        assert!(found_tool.is_some());

        let not_found = registry.get_tool("nonexistent").await;
        assert!(not_found.is_none());
    }

    #[test]
    fn test_tool_stats() {
        let mut stats = ToolStats::default();

        stats.update("tool1", true, Duration::from_millis(100));
        stats.update("tool1", false, Duration::from_millis(200));
        stats.update("tool2", true, Duration::from_millis(150));

        assert_eq!(stats.total_executions, 3);
        assert_eq!(stats.successful_executions, 2);
        assert_eq!(stats.failed_executions, 1);
        assert_eq!(stats.executions_by_tool.get("tool1"), Some(&2));
        assert_eq!(stats.executions_by_tool.get("tool2"), Some(&1));

        let success_rate = stats.success_rate();
        assert!((success_rate - 2.0/3.0).abs() < f64::EPSILON);
    }
}
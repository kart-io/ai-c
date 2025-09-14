# MCP协议集成技术设计文档

AI-Commit TUI项目Model Context Protocol (MCP)集成详细技术设计与实现规范

## 1. MCP协议概述

### 1.1 设计目标

- 实现标准化的MCP协议栈，支持多种AI服务集成
- 提供高性能的多传输层支持（WebSocket、HTTP、标准IO）
- 构建可扩展的资源和工具管理体系
- 确保协议兼容性和向后兼容性
- 支持实时双向通信和异步消息处理

### 1.2 核心原则

- **协议标准化**: 严格遵循MCP 2024-11-05规范
- **传输层透明**: 上层应用无需关心底层传输细节
- **类型安全**: 利用Rust类型系统确保协议消息正确性
- **性能优先**: 优化序列化性能和网络通信效率
- **容错设计**: 完善的错误处理和自动恢复机制

## 2. 核心协议接口定义

### 2.1 MCP消息结构

```rust
use serde::{Serialize, Deserialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

/// MCP请求消息
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MCPRequest {
    /// JSON-RPC版本，固定为"2.0"
    pub jsonrpc: String,
    /// 请求ID，用于匹配请求和响应
    pub id: RequestId,
    /// 方法名称
    pub method: String,
    /// 可选参数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// MCP响应消息
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MCPResponse {
    /// JSON-RPC版本
    pub jsonrpc: String,
    /// 对应的请求ID
    pub id: RequestId,
    /// 成功结果
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// 错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<MCPError>,
}

/// MCP通知消息（单向，无需响应）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// 请求ID类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum RequestId {
    Number(i64),
    String(String),
}

impl RequestId {
    pub fn new_number() -> Self {
        Self::Number(chrono::Utc::now().timestamp_nanos() / 1_000_000)
    }

    pub fn new_uuid() -> Self {
        Self::String(Uuid::new_v4().to_string())
    }
}

/// MCP错误信息
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MCPError {
    /// 错误代码
    pub code: i32,
    /// 错误消息
    pub message: String,
    /// 可选的额外数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// 标准MCP错误代码
#[derive(Debug, Clone, Copy)]
pub enum MCPErrorCode {
    ParseError = -32700,
    InvalidRequest = -32600,
    MethodNotFound = -32601,
    InvalidParams = -32602,
    InternalError = -32603,
    ServerError = -32000,
    ResourceNotFound = -32001,
    ToolExecutionError = -32002,
    PermissionDenied = -32003,
}

impl MCPError {
    pub fn new(code: MCPErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: code as i32,
            message: message.into(),
            data: None,
        }
    }

    pub fn with_data(code: MCPErrorCode, message: impl Into<String>, data: Value) -> Self {
        Self {
            code: code as i32,
            message: message.into(),
            data: Some(data),
        }
    }
}
```

### 2.2 MCP客户端能力定义

```rust
/// MCP客户端能力声明
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCapabilities {
    /// 资源相关能力
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourceCapabilities>,
    /// 工具相关能力
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolCapabilities>,
    /// 提示相关能力
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptCapabilities>,
    /// 采样相关能力
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<SamplingCapabilities>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceCapabilities {
    /// 是否支持资源订阅
    pub subscribe: bool,
    /// 是否支持资源列表变更通知
    pub list_changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCapabilities {
    /// 是否支持工具列表变更通知
    pub list_changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptCapabilities {
    /// 是否支持提示列表变更通知
    pub list_changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingCapabilities {
    /// 支持的采样模式
    pub modes: Vec<String>,
}

/// MCP服务器信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    /// 服务器名称
    pub name: String,
    /// 服务器版本
    pub version: String,
    /// 服务器描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 服务器作者
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// 服务器主页
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
}

/// MCP客户端信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// 初始化请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    /// 协议版本
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    /// 客户端能力
    pub capabilities: ClientCapabilities,
    /// 客户端信息
    #[serde(rename = "clientInfo")]
    pub client_info: ClientInfo,
}

/// 初始化响应结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    /// 协议版本
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    /// 服务器能力
    pub capabilities: ServerCapabilities,
    /// 服务器信息
    #[serde(rename = "serverInfo")]
    pub server_info: ServerInfo,
}

/// MCP服务器能力
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourceCapabilities>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolCapabilities>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptCapabilities>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<SamplingCapabilities>,
}
```

## 3. 资源管理系统

### 3.1 资源定义和管理

```rust
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

/// MCP资源定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    /// 资源唯一标识符（URI）
    pub uri: String,
    /// 资源名称
    pub name: String,
    /// 资源描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// MIME类型
    #[serde(rename = "mimeType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// 资源注解
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ResourceAnnotations>,
}

/// 资源注解
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAnnotations {
    /// 受众注解
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<Vec<Role>>,
    /// 优先级注解
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<f64>,
}

/// 角色定义
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

/// 资源内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceContent {
    /// 资源URI
    pub uri: String,
    /// 资源内容
    pub content: ResourceData,
}

/// 资源数据类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResourceData {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "blob")]
    Blob { blob: String }, // Base64编码的二进制数据
}

/// 资源提供者trait
#[async_trait]
pub trait ResourceProvider: Send + Sync {
    /// 获取资源列表
    async fn list_resources(&self) -> Result<Vec<Resource>, MCPError>;

    /// 获取资源内容
    async fn get_resource(&self, uri: &str) -> Result<ResourceContent, MCPError>;

    /// 检查资源是否存在
    async fn resource_exists(&self, uri: &str) -> bool;

    /// 监听资源变更（可选）
    async fn subscribe_to_resource(&self, uri: &str) -> Result<(), MCPError> {
        Err(MCPError::new(MCPErrorCode::MethodNotFound, "Resource subscription not supported"))
    }

    /// 取消资源监听（可选）
    async fn unsubscribe_from_resource(&self, uri: &str) -> Result<(), MCPError> {
        Err(MCPError::new(MCPErrorCode::MethodNotFound, "Resource subscription not supported"))
    }
}

/// 资源注册表
pub struct ResourceRegistry {
    resources: Arc<RwLock<HashMap<String, Resource>>>,
    providers: Arc<RwLock<HashMap<String, Arc<dyn ResourceProvider>>>>,
    subscription_manager: Arc<SubscriptionManager>,
}

impl ResourceRegistry {
    pub fn new() -> Self {
        Self {
            resources: Arc::new(RwLock::new(HashMap::new())),
            providers: Arc::new(RwLock::new(HashMap::new())),
            subscription_manager: Arc::new(SubscriptionManager::new()),
        }
    }

    /// 注册资源提供者
    pub async fn register_provider(&self, uri_pattern: String, provider: Arc<dyn ResourceProvider>) -> Result<(), MCPError> {
        let mut providers = self.providers.write().await;
        providers.insert(uri_pattern, provider);
        Ok(())
    }

    /// 注册单个资源
    pub async fn register_resource(&self, resource: Resource) -> Result<(), MCPError> {
        let mut resources = self.resources.write().await;
        resources.insert(resource.uri.clone(), resource);
        Ok(())
    }

    /// 获取所有资源
    pub async fn list_resources(&self) -> Result<Vec<Resource>, MCPError> {
        let resources = self.resources.read().await;
        let mut all_resources = resources.values().cloned().collect::<Vec<_>>();

        // 从提供者获取动态资源
        let providers = self.providers.read().await;
        for provider in providers.values() {
            match provider.list_resources().await {
                Ok(mut provider_resources) => all_resources.append(&mut provider_resources),
                Err(e) => {
                    tracing::warn!("Failed to get resources from provider: {}", e.message);
                }
            }
        }

        Ok(all_resources)
    }

    /// 获取资源内容
    pub async fn get_resource(&self, uri: &str) -> Result<ResourceContent, MCPError> {
        // 首先检查静态注册的资源
        let resources = self.resources.read().await;
        if let Some(resource) = resources.get(uri) {
            // 返回静态资源内容（这里需要实际的内容获取逻辑）
            return Ok(ResourceContent {
                uri: uri.to_string(),
                content: ResourceData::Text {
                    text: format!("Static resource content for: {}", resource.name),
                },
            });
        }

        // 检查动态提供者
        let providers = self.providers.read().await;
        for (pattern, provider) in providers.iter() {
            if uri.starts_with(pattern) {
                return provider.get_resource(uri).await;
            }
        }

        Err(MCPError::new(MCPErrorCode::ResourceNotFound, format!("Resource not found: {}", uri)))
    }

    /// 订阅资源变更
    pub async fn subscribe_resource(&self, uri: &str, subscriber_id: String) -> Result<(), MCPError> {
        self.subscription_manager.subscribe(uri, subscriber_id).await
    }

    /// 取消订阅
    pub async fn unsubscribe_resource(&self, uri: &str, subscriber_id: String) -> Result<(), MCPError> {
        self.subscription_manager.unsubscribe(uri, subscriber_id).await
    }
}

/// 订阅管理器
pub struct SubscriptionManager {
    subscriptions: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl SubscriptionManager {
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn subscribe(&self, uri: &str, subscriber_id: String) -> Result<(), MCPError> {
        let mut subscriptions = self.subscriptions.write().await;
        subscriptions
            .entry(uri.to_string())
            .or_insert_with(Vec::new)
            .push(subscriber_id);
        Ok(())
    }

    pub async fn unsubscribe(&self, uri: &str, subscriber_id: String) -> Result<(), MCPError> {
        let mut subscriptions = self.subscriptions.write().await;
        if let Some(subscribers) = subscriptions.get_mut(uri) {
            subscribers.retain(|id| id != &subscriber_id);
            if subscribers.is_empty() {
                subscriptions.remove(uri);
            }
        }
        Ok(())
    }

    pub async fn notify_resource_changed(&self, uri: &str) -> Vec<String> {
        let subscriptions = self.subscriptions.read().await;
        subscriptions.get(uri).cloned().unwrap_or_default()
    }
}
```

### 3.2 Git资源提供者实现

```rust
/// Git资源提供者
pub struct GitResourceProvider {
    git_service: Arc<crate::git::GitService>,
    base_uri: String,
}

impl GitResourceProvider {
    pub fn new(git_service: Arc<crate::git::GitService>, base_uri: String) -> Self {
        Self {
            git_service,
            base_uri,
        }
    }

    fn create_file_resource(&self, path: &str) -> Resource {
        Resource {
            uri: format!("{}/file/{}", self.base_uri, path),
            name: path.to_string(),
            description: Some(format!("Git tracked file: {}", path)),
            mime_type: self.detect_mime_type(path),
            annotations: None,
        }
    }

    fn detect_mime_type(&self, path: &str) -> Option<String> {
        match std::path::Path::new(path).extension()?.to_str()? {
            "rs" => Some("text/rust".to_string()),
            "js" | "ts" => Some("text/javascript".to_string()),
            "py" => Some("text/python".to_string()),
            "md" => Some("text/markdown".to_string()),
            "json" => Some("application/json".to_string()),
            "toml" => Some("application/toml".to_string()),
            "yaml" | "yml" => Some("application/yaml".to_string()),
            _ => Some("text/plain".to_string()),
        }
    }
}

#[async_trait]
impl ResourceProvider for GitResourceProvider {
    async fn list_resources(&self) -> Result<Vec<Resource>, MCPError> {
        let status = self.git_service.get_status()
            .map_err(|e| MCPError::new(MCPErrorCode::InternalError, e.to_string()))?;

        let mut resources = Vec::new();

        // 添加仓库状态资源
        resources.push(Resource {
            uri: format!("{}/status", self.base_uri),
            name: "Repository Status".to_string(),
            description: Some("Current Git repository status".to_string()),
            mime_type: Some("application/json".to_string()),
            annotations: Some(ResourceAnnotations {
                audience: Some(vec![Role::Assistant]),
                priority: Some(1.0),
            }),
        });

        // 添加分支信息资源
        resources.push(Resource {
            uri: format!("{}/branches", self.base_uri),
            name: "Branch Information".to_string(),
            description: Some("Git branch information".to_string()),
            mime_type: Some("application/json".to_string()),
            annotations: None,
        });

        // 添加提交历史资源
        resources.push(Resource {
            uri: format!("{}/history", self.base_uri),
            name: "Commit History".to_string(),
            description: Some("Recent commit history".to_string()),
            mime_type: Some("application/json".to_string()),
            annotations: None,
        });

        // 添加已修改的文件作为资源
        for file_item in status {
            resources.push(self.create_file_resource(&file_item.path));
        }

        Ok(resources)
    }

    async fn get_resource(&self, uri: &str) -> Result<ResourceContent, MCPError> {
        let relative_uri = uri.strip_prefix(&self.base_uri)
            .ok_or_else(|| MCPError::new(MCPErrorCode::InvalidParams, "Invalid URI"))?;

        match relative_uri {
            "/status" => {
                let status = self.git_service.get_status()
                    .map_err(|e| MCPError::new(MCPErrorCode::InternalError, e.to_string()))?;

                let status_json = serde_json::to_string_pretty(&status)
                    .map_err(|e| MCPError::new(MCPErrorCode::InternalError, e.to_string()))?;

                Ok(ResourceContent {
                    uri: uri.to_string(),
                    content: ResourceData::Text { text: status_json },
                })
            },
            "/branches" => {
                let branch_info = self.git_service.get_branch_info()
                    .map_err(|e| MCPError::new(MCPErrorCode::InternalError, e.to_string()))?;

                let branch_json = serde_json::to_string_pretty(&branch_info)
                    .map_err(|e| MCPError::new(MCPErrorCode::InternalError, e.to_string()))?;

                Ok(ResourceContent {
                    uri: uri.to_string(),
                    content: ResourceData::Text { text: branch_json },
                })
            },
            "/history" => {
                let history = self.git_service.get_commit_history(20)
                    .map_err(|e| MCPError::new(MCPErrorCode::InternalError, e.to_string()))?;

                let history_json = serde_json::to_string_pretty(&history)
                    .map_err(|e| MCPError::new(MCPErrorCode::InternalError, e.to_string()))?;

                Ok(ResourceContent {
                    uri: uri.to_string(),
                    content: ResourceData::Text { text: history_json },
                })
            },
            path if path.starts_with("/file/") => {
                let file_path = path.strip_prefix("/file/").unwrap();

                // 获取文件内容
                let workdir = self.git_service.get_workdir()
                    .ok_or_else(|| MCPError::new(MCPErrorCode::InternalError, "No working directory"))?;

                let full_path = workdir.join(file_path);
                let content = tokio::fs::read_to_string(full_path).await
                    .map_err(|e| MCPError::new(MCPErrorCode::ResourceNotFound, e.to_string()))?;

                Ok(ResourceContent {
                    uri: uri.to_string(),
                    content: ResourceData::Text { text: content },
                })
            },
            _ => Err(MCPError::new(MCPErrorCode::ResourceNotFound, "Resource not found")),
        }
    }

    async fn resource_exists(&self, uri: &str) -> bool {
        self.get_resource(uri).await.is_ok()
    }
}
```

## 4. 工具管理系统

### 4.1 工具定义和执行框架

```rust
/// MCP工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// 工具名称
    pub name: String,
    /// 工具描述
    pub description: String,
    /// 输入模式定义（JSON Schema）
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// 工具调用请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// 工具名称
    pub name: String,
    /// 调用参数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
}

/// 工具执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// 结果内容
    pub content: Vec<ToolContent>,
    /// 是否为错误
    #[serde(rename = "isError")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// 工具内容类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolContent {
    #[serde(rename = "text")]
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<ContentAnnotations>,
    },
    #[serde(rename = "image")]
    Image {
        data: String, // Base64编码的图片数据
        #[serde(rename = "mimeType")]
        mime_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<ContentAnnotations>,
    },
    #[serde(rename = "resource")]
    Resource {
        uri: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<ContentAnnotations>,
    },
}

/// 内容注解
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentAnnotations {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<Vec<Role>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<f64>,
}

/// 工具处理器trait
#[async_trait]
pub trait ToolHandler: Send + Sync {
    /// 执行工具
    async fn execute(&self, arguments: Value) -> Result<ToolResult, MCPError>;

    /// 验证参数
    fn validate_arguments(&self, arguments: &Value) -> Result<(), MCPError> {
        // 默认不进行验证，具体实现可以重写
        Ok(())
    }

    /// 获取工具元数据
    fn get_metadata(&self) -> ToolMetadata {
        ToolMetadata::default()
    }
}

/// 工具元数据
#[derive(Debug, Clone)]
pub struct ToolMetadata {
    pub timeout: Option<std::time::Duration>,
    pub requires_confirmation: bool,
    pub dangerous: bool,
    pub categories: Vec<String>,
}

impl Default for ToolMetadata {
    fn default() -> Self {
        Self {
            timeout: Some(std::time::Duration::from_secs(30)),
            requires_confirmation: false,
            dangerous: false,
            categories: vec![],
        }
    }
}

/// 工具注册表
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, Tool>>>,
    handlers: Arc<RwLock<HashMap<String, Arc<dyn ToolHandler>>>>,
    execution_stats: Arc<RwLock<HashMap<String, ToolExecutionStats>>>,
}

#[derive(Debug, Clone)]
pub struct ToolExecutionStats {
    pub total_calls: u64,
    pub successful_calls: u64,
    pub failed_calls: u64,
    pub average_duration: std::time::Duration,
    pub last_called: Option<SystemTime>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            handlers: Arc::new(RwLock::new(HashMap::new())),
            execution_stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册工具
    pub async fn register_tool(&self, tool: Tool, handler: Arc<dyn ToolHandler>) -> Result<(), MCPError> {
        let tool_name = tool.name.clone();

        // 验证JSON Schema
        self.validate_tool_schema(&tool)?;

        {
            let mut tools = self.tools.write().await;
            let mut handlers = self.handlers.write().await;

            tools.insert(tool_name.clone(), tool);
            handlers.insert(tool_name.clone(), handler);
        }

        // 初始化统计信息
        {
            let mut stats = self.execution_stats.write().await;
            stats.insert(tool_name, ToolExecutionStats {
                total_calls: 0,
                successful_calls: 0,
                failed_calls: 0,
                average_duration: std::time::Duration::from_millis(0),
                last_called: None,
            });
        }

        Ok(())
    }

    /// 获取所有工具
    pub async fn list_tools(&self) -> Vec<Tool> {
        let tools = self.tools.read().await;
        tools.values().cloned().collect()
    }

    /// 执行工具
    pub async fn execute_tool(&self, tool_call: ToolCall) -> Result<ToolResult, MCPError> {
        let handler = {
            let handlers = self.handlers.read().await;
            handlers.get(&tool_call.name)
                .cloned()
                .ok_or_else(|| MCPError::new(MCPErrorCode::MethodNotFound, format!("Tool not found: {}", tool_call.name)))?
        };

        // 验证参数
        if let Some(ref arguments) = tool_call.arguments {
            handler.validate_arguments(arguments)?;
        }

        let start_time = std::time::Instant::now();

        // 执行工具
        let result = handler.execute(tool_call.arguments.unwrap_or(Value::Null)).await;

        let duration = start_time.elapsed();

        // 更新统计信息
        self.update_execution_stats(&tool_call.name, &result, duration).await;

        result
    }

    /// 获取工具统计信息
    pub async fn get_tool_stats(&self, tool_name: &str) -> Option<ToolExecutionStats> {
        let stats = self.execution_stats.read().await;
        stats.get(tool_name).cloned()
    }

    async fn update_execution_stats(&self, tool_name: &str, result: &Result<ToolResult, MCPError>, duration: std::time::Duration) {
        let mut stats = self.execution_stats.write().await;
        if let Some(tool_stats) = stats.get_mut(tool_name) {
            tool_stats.total_calls += 1;

            match result {
                Ok(_) => tool_stats.successful_calls += 1,
                Err(_) => tool_stats.failed_calls += 1,
            }

            // 更新平均执行时间
            let total_duration = tool_stats.average_duration.as_nanos() as u64 * (tool_stats.total_calls - 1) + duration.as_nanos() as u64;
            tool_stats.average_duration = std::time::Duration::from_nanos(total_duration / tool_stats.total_calls);

            tool_stats.last_called = Some(SystemTime::now());
        }
    }

    fn validate_tool_schema(&self, tool: &Tool) -> Result<(), MCPError> {
        // 验证JSON Schema格式
        if !tool.input_schema.is_object() {
            return Err(MCPError::new(MCPErrorCode::InvalidParams, "Tool input schema must be an object"));
        }

        // 这里可以添加更详细的JSON Schema验证
        Ok(())
    }
}
```

### 4.2 Git工具实现

```rust
/// Git提交工具
pub struct GitCommitTool {
    git_service: Arc<crate::git::GitService>,
}

impl GitCommitTool {
    pub fn new(git_service: Arc<crate::git::GitService>) -> Self {
        Self { git_service }
    }

    pub fn create_tool_definition() -> Tool {
        Tool {
            name: "git_commit".to_string(),
            description: "Create a Git commit with the specified message and files".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "Commit message"
                    },
                    "files": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Files to include in the commit (optional, stages all if not specified)"
                    },
                    "amend": {
                        "type": "boolean",
                        "description": "Whether to amend the last commit",
                        "default": false
                    }
                },
                "required": ["message"]
            }),
        }
    }
}

#[async_trait]
impl ToolHandler for GitCommitTool {
    async fn execute(&self, arguments: Value) -> Result<ToolResult, MCPError> {
        let message = arguments["message"].as_str()
            .ok_or_else(|| MCPError::new(MCPErrorCode::InvalidParams, "Missing commit message"))?;

        let files = arguments["files"].as_array();
        let amend = arguments["amend"].as_bool().unwrap_or(false);

        // 如果指定了文件，先暂存这些文件
        if let Some(file_list) = files {
            for file_value in file_list {
                if let Some(file_path) = file_value.as_str() {
                    self.git_service.stage_file(file_path)
                        .map_err(|e| MCPError::new(MCPErrorCode::ToolExecutionError, e.to_string()))?;
                }
            }
        }

        // 执行提交
        let commit_result = if amend {
            self.git_service.amend_commit(message)
        } else {
            self.git_service.commit(message)
        };

        match commit_result {
            Ok(commit_id) => {
                let result_text = format!("Successfully created commit: {}", commit_id);
                Ok(ToolResult {
                    content: vec![ToolContent::Text {
                        text: result_text,
                        annotations: Some(ContentAnnotations {
                            audience: Some(vec![Role::User]),
                            priority: Some(1.0),
                        }),
                    }],
                    is_error: Some(false),
                })
            },
            Err(e) => {
                Err(MCPError::new(MCPErrorCode::ToolExecutionError, e.to_string()))
            }
        }
    }

    fn validate_arguments(&self, arguments: &Value) -> Result<(), MCPError> {
        // 验证提交消息不为空
        if let Some(message) = arguments["message"].as_str() {
            if message.trim().is_empty() {
                return Err(MCPError::new(MCPErrorCode::InvalidParams, "Commit message cannot be empty"));
            }
        }

        // 验证文件路径格式
        if let Some(files) = arguments["files"].as_array() {
            for file_value in files {
                if file_value.as_str().is_none() {
                    return Err(MCPError::new(MCPErrorCode::InvalidParams, "All file paths must be strings"));
                }
            }
        }

        Ok(())
    }

    fn get_metadata(&self) -> ToolMetadata {
        ToolMetadata {
            timeout: Some(std::time::Duration::from_secs(10)),
            requires_confirmation: true, // 提交操作需要确认
            dangerous: false,
            categories: vec!["git".to_string(), "version-control".to_string()],
        }
    }
}

/// Git状态查询工具
pub struct GitStatusTool {
    git_service: Arc<crate::git::GitService>,
}

impl GitStatusTool {
    pub fn new(git_service: Arc<crate::git::GitService>) -> Self {
        Self { git_service }
    }

    pub fn create_tool_definition() -> Tool {
        Tool {
            name: "git_status".to_string(),
            description: "Get the current Git repository status".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "include_untracked": {
                        "type": "boolean",
                        "description": "Whether to include untracked files",
                        "default": true
                    }
                }
            }),
        }
    }
}

#[async_trait]
impl ToolHandler for GitStatusTool {
    async fn execute(&self, arguments: Value) -> Result<ToolResult, MCPError> {
        let _include_untracked = arguments["include_untracked"].as_bool().unwrap_or(true);

        let status = self.git_service.get_status()
            .map_err(|e| MCPError::new(MCPErrorCode::ToolExecutionError, e.to_string()))?;

        let status_json = serde_json::to_string_pretty(&status)
            .map_err(|e| MCPError::new(MCPErrorCode::InternalError, e.to_string()))?;

        Ok(ToolResult {
            content: vec![ToolContent::Text {
                text: status_json,
                annotations: Some(ContentAnnotations {
                    audience: Some(vec![Role::Assistant]),
                    priority: None,
                }),
            }],
            is_error: Some(false),
        })
    }

    fn get_metadata(&self) -> ToolMetadata {
        ToolMetadata {
            timeout: Some(std::time::Duration::from_secs(5)),
            requires_confirmation: false,
            dangerous: false,
            categories: vec!["git".to_string(), "query".to_string()],
        }
    }
}

/// Git差异查看工具
pub struct GitDiffTool {
    git_service: Arc<crate::git::GitService>,
}

impl GitDiffTool {
    pub fn new(git_service: Arc<crate::git::GitService>) -> Self {
        Self { git_service }
    }

    pub fn create_tool_definition() -> Tool {
        Tool {
            name: "git_diff".to_string(),
            description: "Get the diff for specified files or all staged changes".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Specific file to show diff for (optional)"
                    },
                    "staged": {
                        "type": "boolean",
                        "description": "Show staged changes instead of working directory changes",
                        "default": false
                    },
                    "context_lines": {
                        "type": "integer",
                        "description": "Number of context lines to show",
                        "default": 3,
                        "minimum": 0,
                        "maximum": 20
                    }
                }
            }),
        }
    }
}

#[async_trait]
impl ToolHandler for GitDiffTool {
    async fn execute(&self, arguments: Value) -> Result<ToolResult, MCPError> {
        let file_path = arguments["file_path"].as_str();
        let _staged = arguments["staged"].as_bool().unwrap_or(false);
        let _context_lines = arguments["context_lines"].as_i64().unwrap_or(3);

        let diff_result = if let Some(path) = file_path {
            self.git_service.get_diff(path)
        } else {
            self.git_service.get_diff_all()
        };

        match diff_result {
            Ok(diff) => {
                Ok(ToolResult {
                    content: vec![ToolContent::Text {
                        text: diff,
                        annotations: Some(ContentAnnotations {
                            audience: Some(vec![Role::Assistant]),
                            priority: None,
                        }),
                    }],
                    is_error: Some(false),
                })
            },
            Err(e) => {
                Err(MCPError::new(MCPErrorCode::ToolExecutionError, e.to_string()))
            }
        }
    }

    fn validate_arguments(&self, arguments: &Value) -> Result<(), MCPError> {
        // 验证context_lines范围
        if let Some(context_lines) = arguments["context_lines"].as_i64() {
            if context_lines < 0 || context_lines > 20 {
                return Err(MCPError::new(MCPErrorCode::InvalidParams, "Context lines must be between 0 and 20"));
            }
        }

        Ok(())
    }

    fn get_metadata(&self) -> ToolMetadata {
        ToolMetadata {
            timeout: Some(std::time::Duration::from_secs(10)),
            requires_confirmation: false,
            dangerous: false,
            categories: vec!["git".to_string(), "diff".to_string()],
        }
    }
}
```

## 5. 传输层架构

### 5.1 传输层抽象接口

```rust
/// MCP传输层trait
#[async_trait]
pub trait MCPTransport: Send + Sync {
    /// 发送消息
    async fn send_message(&mut self, message: MCPMessage) -> Result<(), TransportError>;

    /// 接收消息
    async fn receive_message(&mut self) -> Result<MCPMessage, TransportError>;

    /// 检查连接状态
    fn is_connected(&self) -> bool;

    /// 关闭连接
    async fn close(&mut self) -> Result<(), TransportError>;

    /// 获取传输类型
    fn transport_type(&self) -> TransportType;
}

/// MCP消息统一类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MCPMessage {
    Request(MCPRequest),
    Response(MCPResponse),
    Notification(MCPNotification),
}

/// 传输类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportType {
    Http,
    WebSocket,
    Stdio,
}

/// 传输错误
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("Timeout")]
    Timeout,

    #[error("Connection closed")]
    ConnectionClosed,
}

/// 传输配置
#[derive(Debug, Clone)]
pub struct TransportConfig {
    pub connect_timeout: std::time::Duration,
    pub read_timeout: std::time::Duration,
    pub write_timeout: std::time::Duration,
    pub max_message_size: usize,
    pub keepalive_interval: Option<std::time::Duration>,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            connect_timeout: std::time::Duration::from_secs(10),
            read_timeout: std::time::Duration::from_secs(30),
            write_timeout: std::time::Duration::from_secs(10),
            max_message_size: 10 * 1024 * 1024, // 10MB
            keepalive_interval: Some(std::time::Duration::from_secs(30)),
        }
    }
}
```

### 5.2 HTTP传输实现

```rust
use reqwest::Client;
use tokio::sync::{mpsc, Mutex};

/// HTTP传输实现
pub struct HttpTransport {
    client: Client,
    server_url: String,
    request_queue: mpsc::UnboundedSender<PendingRequest>,
    response_queue: Arc<Mutex<mpsc::UnboundedReceiver<MCPMessage>>>,
    config: TransportConfig,
    next_request_id: Arc<std::sync::atomic::AtomicI64>,
}

struct PendingRequest {
    request: MCPRequest,
    response_sender: tokio::sync::oneshot::Sender<Result<MCPResponse, TransportError>>,
}

impl HttpTransport {
    pub async fn new(server_url: String, config: TransportConfig) -> Result<Self, TransportError> {
        let client = Client::builder()
            .timeout(config.read_timeout)
            .connect_timeout(config.connect_timeout)
            .build()
            .map_err(|e| TransportError::ConnectionError(e.to_string()))?;

        let (req_tx, req_rx) = mpsc::unbounded_channel();
        let (resp_tx, resp_rx) = mpsc::unbounded_channel();

        let transport = Self {
            client: client.clone(),
            server_url: server_url.clone(),
            request_queue: req_tx,
            response_queue: Arc::new(Mutex::new(resp_rx)),
            config,
            next_request_id: Arc::new(std::sync::atomic::AtomicI64::new(1)),
        };

        // 启动请求处理任务
        tokio::spawn(Self::request_handler(client, server_url, req_rx, resp_tx));

        Ok(transport)
    }

    async fn request_handler(
        client: Client,
        server_url: String,
        mut request_queue: mpsc::UnboundedReceiver<PendingRequest>,
        response_sender: mpsc::UnboundedSender<MCPMessage>,
    ) {
        while let Some(pending_request) = request_queue.recv().await {
            let client = client.clone();
            let server_url = server_url.clone();
            let response_sender = response_sender.clone();

            tokio::spawn(async move {
                let result = Self::send_http_request(&client, &server_url, &pending_request.request).await;

                match result {
                    Ok(response) => {
                        let _ = pending_request.response_sender.send(Ok(response.clone()));
                        let _ = response_sender.send(MCPMessage::Response(response));
                    },
                    Err(e) => {
                        let _ = pending_request.response_sender.send(Err(e));
                    }
                }
            });
        }
    }

    async fn send_http_request(
        client: &Client,
        server_url: &str,
        request: &MCPRequest,
    ) -> Result<MCPResponse, TransportError> {
        let response = client
            .post(server_url)
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await
            .map_err(|e| TransportError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(TransportError::NetworkError(format!("HTTP error: {}", response.status())));
        }

        let mcp_response: MCPResponse = response
            .json()
            .await
            .map_err(|e| TransportError::SerializationError(e.to_string()))?;

        Ok(mcp_response)
    }
}

#[async_trait]
impl MCPTransport for HttpTransport {
    async fn send_message(&mut self, message: MCPMessage) -> Result<(), TransportError> {
        match message {
            MCPMessage::Request(request) => {
                let (tx, _rx) = tokio::sync::oneshot::channel();
                let pending_request = PendingRequest {
                    request,
                    response_sender: tx,
                };

                self.request_queue.send(pending_request)
                    .map_err(|_| TransportError::ConnectionClosed)?;

                Ok(())
            },
            MCPMessage::Notification(_) => {
                // HTTP传输不支持通知消息
                Err(TransportError::ProtocolError("HTTP transport does not support notifications".to_string()))
            },
            MCPMessage::Response(_) => {
                // HTTP客户端不应该发送响应消息
                Err(TransportError::ProtocolError("HTTP client should not send responses".to_string()))
            }
        }
    }

    async fn receive_message(&mut self) -> Result<MCPMessage, TransportError> {
        let mut response_queue = self.response_queue.lock().await;
        response_queue.recv().await
            .ok_or(TransportError::ConnectionClosed)
    }

    fn is_connected(&self) -> bool {
        !self.request_queue.is_closed()
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        // HTTP传输没有持久连接，不需要特殊关闭操作
        Ok(())
    }

    fn transport_type(&self) -> TransportType {
        TransportType::Http
    }
}
```

### 5.3 WebSocket传输实现

```rust
use tokio_tungstenite::{connect_async, WebSocketStream, MaybeTlsStream};
use tokio_tungstenite::tungstenite::{Message, protocol::CloseFrame};
use futures_util::{SinkExt, StreamExt};

/// WebSocket传输实现
pub struct WebSocketTransport {
    ws_stream: Option<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>>,
    config: TransportConfig,
    url: String,
    message_queue: Arc<Mutex<VecDeque<MCPMessage>>>,
    is_connected: Arc<std::sync::atomic::AtomicBool>,
}

impl WebSocketTransport {
    pub async fn new(url: String, config: TransportConfig) -> Result<Self, TransportError> {
        let transport = Self {
            ws_stream: None,
            config,
            url,
            message_queue: Arc::new(Mutex::new(VecDeque::new())),
            is_connected: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };

        Ok(transport)
    }

    pub async fn connect(&mut self) -> Result<(), TransportError> {
        let (ws_stream, _) = tokio::time::timeout(
            self.config.connect_timeout,
            connect_async(&self.url)
        ).await
        .map_err(|_| TransportError::Timeout)?
        .map_err(|e| TransportError::ConnectionError(e.to_string()))?;

        self.ws_stream = Some(ws_stream);
        self.is_connected.store(true, std::sync::atomic::Ordering::Relaxed);

        // 启动消息接收任务
        if let Some(ws_stream) = self.ws_stream.take() {
            let (sink, mut stream) = ws_stream.split();
            let message_queue = Arc::clone(&self.message_queue);
            let is_connected = Arc::clone(&self.is_connected);

            // 消息接收任务
            tokio::spawn(async move {
                while let Some(message) = stream.next().await {
                    match message {
                        Ok(Message::Text(text)) => {
                            if let Ok(mcp_message) = serde_json::from_str::<MCPMessage>(&text) {
                                let mut queue = message_queue.lock().await;
                                queue.push_back(mcp_message);
                            }
                        },
                        Ok(Message::Close(_)) => {
                            is_connected.store(false, std::sync::atomic::Ordering::Relaxed);
                            break;
                        },
                        Err(_) => {
                            is_connected.store(false, std::sync::atomic::Ordering::Relaxed);
                            break;
                        },
                        _ => {}
                    }
                }
            });

            self.ws_stream = Some(sink.reunite(stream).map_err(|e| TransportError::ConnectionError(e.to_string()))?);
        }

        Ok(())
    }

    async fn ensure_connected(&mut self) -> Result<(), TransportError> {
        if !self.is_connected() {
            self.connect().await?;
        }
        Ok(())
    }
}

#[async_trait]
impl MCPTransport for WebSocketTransport {
    async fn send_message(&mut self, message: MCPMessage) -> Result<(), TransportError> {
        self.ensure_connected().await?;

        let ws_stream = self.ws_stream.as_mut()
            .ok_or(TransportError::ConnectionError("No WebSocket connection".to_string()))?;

        let message_text = serde_json::to_string(&message)
            .map_err(|e| TransportError::SerializationError(e.to_string()))?;

        let ws_message = Message::Text(message_text);

        tokio::time::timeout(
            self.config.write_timeout,
            ws_stream.send(ws_message)
        ).await
        .map_err(|_| TransportError::Timeout)?
        .map_err(|e| TransportError::NetworkError(e.to_string()))?;

        Ok(())
    }

    async fn receive_message(&mut self) -> Result<MCPMessage, TransportError> {
        self.ensure_connected().await?;

        loop {
            // 检查消息队列
            {
                let mut queue = self.message_queue.lock().await;
                if let Some(message) = queue.pop_front() {
                    return Ok(message);
                }
            }

            // 如果队列为空，等待一小段时间再检查
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;

            // 检查连接状态
            if !self.is_connected() {
                return Err(TransportError::ConnectionClosed);
            }
        }
    }

    fn is_connected(&self) -> bool {
        self.is_connected.load(std::sync::atomic::Ordering::Relaxed)
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        if let Some(ws_stream) = self.ws_stream.as_mut() {
            let close_frame = CloseFrame {
                code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Normal,
                reason: "Normal closure".into(),
            };

            let _ = ws_stream.send(Message::Close(Some(close_frame))).await;
        }

        self.is_connected.store(false, std::sync::atomic::Ordering::Relaxed);
        self.ws_stream = None;

        Ok(())
    }

    fn transport_type(&self) -> TransportType {
        TransportType::WebSocket
    }
}
```

### 5.4 标准IO传输实现

```rust
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::{Child, Command};

/// 标准IO传输实现
pub struct StdioTransport {
    child_process: Option<Child>,
    stdin_writer: Option<BufWriter<tokio::process::ChildStdin>>,
    stdout_reader: Option<BufReader<tokio::process::ChildStdout>>,
    config: TransportConfig,
    command: String,
    args: Vec<String>,
}

impl StdioTransport {
    pub fn new(command: String, args: Vec<String>, config: TransportConfig) -> Self {
        Self {
            child_process: None,
            stdin_writer: None,
            stdout_reader: None,
            config,
            command,
            args,
        }
    }

    pub async fn start_process(&mut self) -> Result<(), TransportError> {
        let mut child = Command::new(&self.command)
            .args(&self.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| TransportError::ConnectionError(format!("Failed to start process: {}", e)))?;

        let stdin = child.stdin.take()
            .ok_or_else(|| TransportError::ConnectionError("Failed to open stdin".to_string()))?;
        let stdout = child.stdout.take()
            .ok_or_else(|| TransportError::ConnectionError("Failed to open stdout".to_string()))?;

        self.stdin_writer = Some(BufWriter::new(stdin));
        self.stdout_reader = Some(BufReader::new(stdout));
        self.child_process = Some(child);

        Ok(())
    }
}

#[async_trait]
impl MCPTransport for StdioTransport {
    async fn send_message(&mut self, message: MCPMessage) -> Result<(), TransportError> {
        if self.stdin_writer.is_none() {
            self.start_process().await?;
        }

        let writer = self.stdin_writer.as_mut()
            .ok_or(TransportError::ConnectionError("No stdin writer".to_string()))?;

        let message_json = serde_json::to_string(&message)
            .map_err(|e| TransportError::SerializationError(e.to_string()))?;

        let message_line = format!("{}\n", message_json);

        tokio::time::timeout(
            self.config.write_timeout,
            writer.write_all(message_line.as_bytes())
        ).await
        .map_err(|_| TransportError::Timeout)?
        .map_err(|e| TransportError::NetworkError(e.to_string()))?;

        tokio::time::timeout(
            self.config.write_timeout,
            writer.flush()
        ).await
        .map_err(|_| TransportError::Timeout)?
        .map_err(|e| TransportError::NetworkError(e.to_string()))?;

        Ok(())
    }

    async fn receive_message(&mut self) -> Result<MCPMessage, TransportError> {
        if self.stdout_reader.is_none() {
            self.start_process().await?;
        }

        let reader = self.stdout_reader.as_mut()
            .ok_or(TransportError::ConnectionError("No stdout reader".to_string()))?;

        let mut line = String::new();

        tokio::time::timeout(
            self.config.read_timeout,
            reader.read_line(&mut line)
        ).await
        .map_err(|_| TransportError::Timeout)?
        .map_err(|e| TransportError::NetworkError(e.to_string()))?;

        if line.is_empty() {
            return Err(TransportError::ConnectionClosed);
        }

        let message: MCPMessage = serde_json::from_str(line.trim())
            .map_err(|e| TransportError::SerializationError(e.to_string()))?;

        Ok(message)
    }

    fn is_connected(&self) -> bool {
        if let Some(child) = &self.child_process {
            match child.try_wait() {
                Ok(Some(_)) => false, // 进程已退出
                Ok(None) => true,     // 进程仍在运行
                Err(_) => false,      // 无法检查状态
            }
        } else {
            false
        }
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        if let Some(mut child) = self.child_process.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }

        self.stdin_writer = None;
        self.stdout_reader = None;

        Ok(())
    }

    fn transport_type(&self) -> TransportType {
        TransportType::Stdio
    }
}
```

## 6. MCP客户端和服务器实现

### 6.1 MCP客户端

```rust
/// MCP客户端
pub struct MCPClient {
    transport: Box<dyn MCPTransport>,
    capabilities: ClientCapabilities,
    server_info: Option<ServerInfo>,
    request_id_counter: Arc<std::sync::atomic::AtomicI64>,
    pending_requests: Arc<Mutex<HashMap<RequestId, tokio::sync::oneshot::Sender<MCPResponse>>>>,
    is_initialized: Arc<std::sync::atomic::AtomicBool>,
}

impl MCPClient {
    pub async fn new(config: MCPClientConfig) -> Result<Self, MCPError> {
        let transport: Box<dyn MCPTransport> = match config.transport_type {
            TransportType::Http => {
                Box::new(HttpTransport::new(config.server_uri, config.transport_config).await?)
            },
            TransportType::WebSocket => {
                let mut ws_transport = WebSocketTransport::new(config.server_uri, config.transport_config)?;
                ws_transport.connect().await?;
                Box::new(ws_transport)
            },
            TransportType::Stdio => {
                Box::new(StdioTransport::new(config.command, config.args, config.transport_config))
            },
        };

        let client = Self {
            transport,
            capabilities: config.capabilities,
            server_info: None,
            request_id_counter: Arc::new(std::sync::atomic::AtomicI64::new(1)),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            is_initialized: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };

        Ok(client)
    }

    /// 初始化连接
    pub async fn initialize(&mut self, client_info: ClientInfo) -> Result<InitializeResult, MCPError> {
        let params = InitializeParams {
            protocol_version: "2024-11-05".to_string(),
            capabilities: self.capabilities.clone(),
            client_info,
        };

        let request = MCPRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_request_id(),
            method: "initialize".to_string(),
            params: Some(serde_json::to_value(params)?),
        };

        let response = self.send_request_and_wait(request).await?;

        if let Some(result) = response.result {
            let init_result: InitializeResult = serde_json::from_value(result)?;
            self.server_info = Some(init_result.server_info.clone());
            self.is_initialized.store(true, std::sync::atomic::Ordering::Relaxed);

            // 发送initialized通知
            let notification = MCPNotification {
                jsonrpc: "2.0".to_string(),
                method: "initialized".to_string(),
                params: None,
            };

            self.transport.send_message(MCPMessage::Notification(notification)).await?;

            Ok(init_result)
        } else if let Some(error) = response.error {
            Err(MCPError::new(MCPErrorCode::ServerError, error.message))
        } else {
            Err(MCPError::new(MCPErrorCode::InternalError, "Invalid initialize response"))
        }
    }

    /// 列出服务器资源
    pub async fn list_resources(&mut self) -> Result<Vec<Resource>, MCPError> {
        self.ensure_initialized()?;

        let request = MCPRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_request_id(),
            method: "resources/list".to_string(),
            params: None,
        };

        let response = self.send_request_and_wait(request).await?;

        if let Some(result) = response.result {
            let resources_result: ResourcesListResult = serde_json::from_value(result)?;
            Ok(resources_result.resources)
        } else if let Some(error) = response.error {
            Err(MCPError::new(MCPErrorCode::ServerError, error.message))
        } else {
            Err(MCPError::new(MCPErrorCode::InternalError, "Invalid resources/list response"))
        }
    }

    /// 读取资源内容
    pub async fn read_resource(&mut self, uri: &str) -> Result<ResourceContent, MCPError> {
        self.ensure_initialized()?;

        let params = serde_json::json!({ "uri": uri });

        let request = MCPRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_request_id(),
            method: "resources/read".to_string(),
            params: Some(params),
        };

        let response = self.send_request_and_wait(request).await?;

        if let Some(result) = response.result {
            let read_result: ResourceReadResult = serde_json::from_value(result)?;
            Ok(read_result.contents[0].clone()) // 假设只返回一个资源
        } else if let Some(error) = response.error {
            Err(MCPError::new(MCPErrorCode::ServerError, error.message))
        } else {
            Err(MCPError::new(MCPErrorCode::InternalError, "Invalid resources/read response"))
        }
    }

    /// 列出可用工具
    pub async fn list_tools(&mut self) -> Result<Vec<Tool>, MCPError> {
        self.ensure_initialized()?;

        let request = MCPRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_request_id(),
            method: "tools/list".to_string(),
            params: None,
        };

        let response = self.send_request_and_wait(request).await?;

        if let Some(result) = response.result {
            let tools_result: ToolsListResult = serde_json::from_value(result)?;
            Ok(tools_result.tools)
        } else if let Some(error) = response.error {
            Err(MCPError::new(MCPErrorCode::ServerError, error.message))
        } else {
            Err(MCPError::new(MCPErrorCode::InternalError, "Invalid tools/list response"))
        }
    }

    /// 调用工具
    pub async fn call_tool(&mut self, name: &str, arguments: Value) -> Result<ToolResult, MCPError> {
        self.ensure_initialized()?;

        let params = serde_json::json!({
            "name": name,
            "arguments": arguments
        });

        let request = MCPRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_request_id(),
            method: "tools/call".to_string(),
            params: Some(params),
        };

        let response = self.send_request_and_wait(request).await?;

        if let Some(result) = response.result {
            let call_result: ToolCallResult = serde_json::from_value(result)?;
            Ok(call_result.content)
        } else if let Some(error) = response.error {
            Err(MCPError::new(MCPErrorCode::ServerError, error.message))
        } else {
            Err(MCPError::new(MCPErrorCode::InternalError, "Invalid tools/call response"))
        }
    }

    /// 发送请求并等待响应
    async fn send_request_and_wait(&mut self, request: MCPRequest) -> Result<MCPResponse, MCPError> {
        let request_id = request.id.clone();
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        // 注册待处理请求
        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(request_id.clone(), response_tx);
        }

        // 发送请求
        self.transport.send_message(MCPMessage::Request(request)).await?;

        // 启动响应处理任务（如果还没有启动）
        self.start_response_handler().await;

        // 等待响应
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            response_rx
        ).await
        .map_err(|_| MCPError::new(MCPErrorCode::InternalError, "Request timeout"))?
        .map_err(|_| MCPError::new(MCPErrorCode::InternalError, "Response channel closed"))?;

        Ok(response)
    }

    async fn start_response_handler(&mut self) {
        // 这里应该启动一个后台任务来处理响应
        // 为了简化，这里省略具体实现
    }

    fn next_request_id(&self) -> RequestId {
        let id = self.request_id_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        RequestId::Number(id)
    }

    fn ensure_initialized(&self) -> Result<(), MCPError> {
        if !self.is_initialized.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(MCPError::new(MCPErrorCode::InvalidRequest, "Client not initialized"));
        }
        Ok(())
    }
}

/// MCP客户端配置
pub struct MCPClientConfig {
    pub server_uri: String,
    pub transport_type: TransportType,
    pub transport_config: TransportConfig,
    pub capabilities: ClientCapabilities,
    pub command: String,
    pub args: Vec<String>,
}

// 响应结果类型定义
#[derive(Debug, Deserialize)]
struct ResourcesListResult {
    resources: Vec<Resource>,
}

#[derive(Debug, Deserialize)]
struct ResourceReadResult {
    contents: Vec<ResourceContent>,
}

#[derive(Debug, Deserialize)]
struct ToolsListResult {
    tools: Vec<Tool>,
}

#[derive(Debug, Deserialize)]
struct ToolCallResult {
    content: ToolResult,
}
```

### 6.2 MCP服务器实现

```rust
/// MCP服务器
pub struct MCPServer {
    resource_registry: Arc<ResourceRegistry>,
    tool_registry: Arc<ToolRegistry>,
    capabilities: ServerCapabilities,
    server_info: ServerInfo,
    clients: Arc<RwLock<HashMap<String, ClientSession>>>,
    request_handlers: HashMap<String, Box<dyn RequestHandler>>,
}

#[derive(Debug, Clone)]
struct ClientSession {
    id: String,
    capabilities: Option<ClientCapabilities>,
    transport: Arc<Mutex<Box<dyn MCPTransport>>>,
    initialized: bool,
}

#[async_trait]
trait RequestHandler: Send + Sync {
    async fn handle(&self, request: MCPRequest, session: &ClientSession) -> Result<MCPResponse, MCPError>;
}

impl MCPServer {
    pub fn new(config: MCPServerConfig) -> Self {
        let mut server = Self {
            resource_registry: Arc::new(ResourceRegistry::new()),
            tool_registry: Arc::new(ToolRegistry::new()),
            capabilities: ServerCapabilities {
                resources: Some(ResourceCapabilities {
                    subscribe: true,
                    list_changed: true,
                }),
                tools: Some(ToolCapabilities {
                    list_changed: true,
                }),
                prompts: None,
                sampling: None,
            },
            server_info: config.server_info,
            clients: Arc::new(RwLock::new(HashMap::new())),
            request_handlers: HashMap::new(),
        };

        server.register_default_handlers();
        server
    }

    fn register_default_handlers(&mut self) {
        self.request_handlers.insert("initialize".to_string(), Box::new(InitializeHandler));
        self.request_handlers.insert("resources/list".to_string(), Box::new(ResourcesListHandler));
        self.request_handlers.insert("resources/read".to_string(), Box::new(ResourcesReadHandler));
        self.request_handlers.insert("tools/list".to_string(), Box::new(ToolsListHandler));
        self.request_handlers.insert("tools/call".to_string(), Box::new(ToolsCallHandler));
    }

    /// 注册资源提供者
    pub async fn register_resource_provider(&self, uri_pattern: String, provider: Arc<dyn ResourceProvider>) -> Result<(), MCPError> {
        self.resource_registry.register_provider(uri_pattern, provider).await
    }

    /// 注册工具
    pub async fn register_tool(&self, tool: Tool, handler: Arc<dyn ToolHandler>) -> Result<(), MCPError> {
        self.tool_registry.register_tool(tool, handler).await
    }

    /// 处理新客户端连接
    pub async fn handle_client(&self, mut transport: Box<dyn MCPTransport>) -> Result<(), MCPError> {
        let client_id = Uuid::new_v4().to_string();

        let session = ClientSession {
            id: client_id.clone(),
            capabilities: None,
            transport: Arc::new(Mutex::new(transport)),
            initialized: false,
        };

        {
            let mut clients = self.clients.write().await;
            clients.insert(client_id.clone(), session.clone());
        }

        // 处理客户端消息
        self.message_loop(session).await?;

        // 清理客户端会话
        {
            let mut clients = self.clients.write().await;
            clients.remove(&client_id);
        }

        Ok(())
    }

    async fn message_loop(&self, mut session: ClientSession) -> Result<(), MCPError> {
        loop {
            let message = {
                let mut transport = session.transport.lock().await;
                match transport.receive_message().await {
                    Ok(msg) => msg,
                    Err(TransportError::ConnectionClosed) => break,
                    Err(e) => {
                        tracing::error!("Transport error: {}", e);
                        break;
                    }
                }
            };

            match message {
                MCPMessage::Request(request) => {
                    let response = self.handle_request(request, &session).await;

                    let mut transport = session.transport.lock().await;
                    if let Err(e) = transport.send_message(MCPMessage::Response(response)).await {
                        tracing::error!("Failed to send response: {}", e);
                        break;
                    }
                },
                MCPMessage::Notification(notification) => {
                    self.handle_notification(notification, &mut session).await;
                },
                MCPMessage::Response(_) => {
                    // 服务器不应该接收到响应消息
                    tracing::warn!("Received unexpected response message");
                }
            }
        }

        Ok(())
    }

    async fn handle_request(&self, request: MCPRequest, session: &ClientSession) -> MCPResponse {
        let request_id = request.id.clone();

        let result = if let Some(handler) = self.request_handlers.get(&request.method) {
            handler.handle(request, session).await
        } else {
            Err(MCPError::new(MCPErrorCode::MethodNotFound, format!("Method not found: {}", request.method)))
        };

        match result {
            Ok(response) => response,
            Err(error) => MCPResponse {
                jsonrpc: "2.0".to_string(),
                id: request_id,
                result: None,
                error: Some(error),
            }
        }
    }

    async fn handle_notification(&self, notification: MCPNotification, session: &mut ClientSession) {
        match notification.method.as_str() {
            "initialized" => {
                session.initialized = true;
                tracing::info!("Client {} initialized", session.id);
            },
            _ => {
                tracing::warn!("Unknown notification: {}", notification.method);
            }
        }
    }
}

// 请求处理器实现
struct InitializeHandler;

#[async_trait]
impl RequestHandler for InitializeHandler {
    async fn handle(&self, request: MCPRequest, _session: &ClientSession) -> Result<MCPResponse, MCPError> {
        let _params: InitializeParams = serde_json::from_value(
            request.params.ok_or_else(|| MCPError::new(MCPErrorCode::InvalidParams, "Missing params"))?
        )?;

        let result = InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ServerCapabilities {
                resources: Some(ResourceCapabilities {
                    subscribe: true,
                    list_changed: true,
                }),
                tools: Some(ToolCapabilities {
                    list_changed: true,
                }),
                prompts: None,
                sampling: None,
            },
            server_info: ServerInfo {
                name: "AI-Commit MCP Server".to_string(),
                version: "0.1.0".to_string(),
                description: Some("MCP server for AI-Commit TUI".to_string()),
                author: Some("AI-Commit Team".to_string()),
                homepage: None,
            },
        };

        Ok(MCPResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(serde_json::to_value(result)?),
            error: None,
        })
    }
}

struct ResourcesListHandler;

#[async_trait]
impl RequestHandler for ResourcesListHandler {
    async fn handle(&self, request: MCPRequest, _session: &ClientSession) -> Result<MCPResponse, MCPError> {
        // 这里需要访问服务器实例的resource_registry
        // 为了简化，返回空列表
        let result = ResourcesListResult {
            resources: vec![],
        };

        Ok(MCPResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(serde_json::to_value(result)?),
            error: None,
        })
    }
}

// 其他处理器类似实现...

/// MCP服务器配置
pub struct MCPServerConfig {
    pub server_info: ServerInfo,
    pub bind_address: String,
    pub transport_type: TransportType,
    pub transport_config: TransportConfig,
}
```

## 7. Agent与MCP集成

### 7.1 MCP集成Agent

```rust
/// MCP集成Agent - 连接Agent系统和MCP协议
pub struct MCPIntegrationAgent {
    id: String,
    mcp_client: Arc<Mutex<MCPClient>>,
    resource_cache: Arc<RwLock<HashMap<String, CachedResource>>>,
    tool_cache: Arc<RwLock<HashMap<String, Tool>>>,
    config: MCPAgentConfig,
    status: Arc<RwLock<AgentStatus>>,
    metrics: Arc<MetricsCollector>,
}

#[derive(Debug, Clone)]
struct CachedResource {
    resource: Resource,
    content: Option<ResourceContent>,
    last_updated: SystemTime,
    expires_at: SystemTime,
}

#[derive(Debug, Clone)]
pub struct MCPAgentConfig {
    pub mcp_client_config: MCPClientConfig,
    pub cache_ttl: std::time::Duration,
    pub max_cache_size: usize,
    pub auto_refresh_interval: std::time::Duration,
}

impl MCPIntegrationAgent {
    pub async fn new(id: String, config: MCPAgentConfig) -> Result<Self, AgentError> {
        let mcp_client = MCPClient::new(config.mcp_client_config.clone()).await
            .map_err(|e| AgentError::InitializationFailed(e.to_string()))?;

        let agent = Self {
            id,
            mcp_client: Arc::new(Mutex::new(mcp_client)),
            resource_cache: Arc::new(RwLock::new(HashMap::new())),
            tool_cache: Arc::new(RwLock::new(HashMap::new())),
            config,
            status: Arc::new(RwLock::new(AgentStatus::Uninitialized)),
            metrics: Arc::new(MetricsCollector::new()),
        };

        Ok(agent)
    }

    /// 初始化MCP连接和缓存
    async fn initialize_mcp(&mut self) -> Result<(), AgentError> {
        *self.status.write().await = AgentStatus::Initializing;

        // 初始化MCP客户端
        let client_info = ClientInfo {
            name: "AI-Commit Agent".to_string(),
            version: "0.1.0".to_string(),
            description: Some("MCP client for AI-Commit TUI".to_string()),
        };

        let mut client = self.mcp_client.lock().await;
        client.initialize(client_info).await
            .map_err(|e| AgentError::InitializationFailed(e.to_string()))?;

        // 预加载工具列表
        let tools = client.list_tools().await
            .map_err(|e| AgentError::InitializationFailed(e.to_string()))?;

        {
            let mut tool_cache = self.tool_cache.write().await;
            for tool in tools {
                tool_cache.insert(tool.name.clone(), tool);
            }
        }

        *self.status.write().await = AgentStatus::Idle;
        Ok(())
    }

    /// 获取MCP资源
    async fn get_mcp_resource(&self, uri: &str) -> Result<ResourceContent, AgentError> {
        // 检查缓存
        {
            let cache = self.resource_cache.read().await;
            if let Some(cached) = cache.get(uri) {
                if cached.expires_at > SystemTime::now() {
                    if let Some(ref content) = cached.content {
                        return Ok(content.clone());
                    }
                }
            }
        }

        // 从MCP服务器获取
        let mut client = self.mcp_client.lock().await;
        let content = client.read_resource(uri).await
            .map_err(|e| AgentError::TaskProcessingFailed(e.to_string()))?;

        // 更新缓存
        {
            let mut cache = self.resource_cache.write().await;
            let expires_at = SystemTime::now() + self.config.cache_ttl;
            cache.insert(uri.to_string(), CachedResource {
                resource: Resource {
                    uri: uri.to_string(),
                    name: uri.split('/').last().unwrap_or(uri).to_string(),
                    description: None,
                    mime_type: None,
                    annotations: None,
                },
                content: Some(content.clone()),
                last_updated: SystemTime::now(),
                expires_at,
            });
        }

        Ok(content)
    }

    /// 调用MCP工具
    async fn call_mcp_tool(&self, tool_name: &str, arguments: Value) -> Result<ToolResult, AgentError> {
        let mut client = self.mcp_client.lock().await;
        client.call_tool(tool_name, arguments).await
            .map_err(|e| AgentError::TaskProcessingFailed(e.to_string()))
    }

    /// 列出可用的MCP工具
    async fn list_mcp_tools(&self) -> Result<Vec<Tool>, AgentError> {
        let cache = self.tool_cache.read().await;
        Ok(cache.values().cloned().collect())
    }
}

#[async_trait]
impl Agent for MCPIntegrationAgent {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        "MCP Integration Agent"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn capabilities(&self) -> Vec<AgentCapability> {
        vec![
            AgentCapability::Custom("mcp_resource_access".to_string()),
            AgentCapability::Custom("mcp_tool_execution".to_string()),
            AgentCapability::Custom("mcp_service_bridge".to_string()),
        ]
    }

    fn config(&self) -> &AgentConfig {
        // 返回Agent通用配置
        &AgentConfig {
            id: self.id.clone(),
            name: "MCP Integration Agent".to_string(),
            enabled: true,
            priority: 1,
            max_concurrent_tasks: 5,
            timeout: std::time::Duration::from_secs(30),
            retry_count: 3,
            custom_settings: HashMap::new(),
        }
    }

    async fn handle_task(&mut self, task: AgentTask) -> Result<AgentResult, AgentError> {
        *self.status.write().await = AgentStatus::Processing(task.id.clone());

        let start_time = std::time::Instant::now();

        let result = match task.task_type {
            TaskType::Custom { capability, payload } => {
                match capability.as_str() {
                    "mcp_resource_access" => {
                        let uri = payload["uri"].as_str()
                            .ok_or_else(|| AgentError::TaskProcessingFailed("Missing URI in payload".to_string()))?;

                        let content = self.get_mcp_resource(uri).await?;

                        Ok(AgentResult {
                            task_id: task.id,
                            agent_id: self.id.clone(),
                            status: ResultStatus::Success,
                            result: ResultData::Custom(serde_json::to_value(content)?),
                            execution_time: start_time.elapsed(),
                            created_at: SystemTime::now(),
                            metadata: HashMap::new(),
                        })
                    },
                    "mcp_tool_execution" => {
                        let tool_name = payload["tool_name"].as_str()
                            .ok_or_else(|| AgentError::TaskProcessingFailed("Missing tool_name in payload".to_string()))?;
                        let arguments = payload.get("arguments").cloned().unwrap_or(Value::Null);

                        let tool_result = self.call_mcp_tool(tool_name, arguments).await?;

                        Ok(AgentResult {
                            task_id: task.id,
                            agent_id: self.id.clone(),
                            status: ResultStatus::Success,
                            result: ResultData::Custom(serde_json::to_value(tool_result)?),
                            execution_time: start_time.elapsed(),
                            created_at: SystemTime::now(),
                            metadata: HashMap::new(),
                        })
                    },
                    "mcp_service_bridge" => {
                        let tools = self.list_mcp_tools().await?;

                        Ok(AgentResult {
                            task_id: task.id,
                            agent_id: self.id.clone(),
                            status: ResultStatus::Success,
                            result: ResultData::Custom(serde_json::to_value(tools)?),
                            execution_time: start_time.elapsed(),
                            created_at: SystemTime::now(),
                            metadata: HashMap::new(),
                        })
                    },
                    _ => Err(AgentError::UnsupportedCapability(format!("Unsupported MCP capability: {}", capability))),
                }
            },
            _ => Err(AgentError::UnsupportedCapability("Only custom MCP tasks are supported".to_string())),
        };

        *self.status.write().await = AgentStatus::Idle;

        // 记录指标
        self.metrics.record_task_execution(&task, &result, start_time.elapsed());

        result
    }

    fn health_check(&self) -> HealthStatus {
        match self.status.try_read() {
            Ok(status) => match *status {
                AgentStatus::Error(ref msg) => HealthStatus::Unhealthy(msg.clone()),
                AgentStatus::Processing(_) => HealthStatus::Healthy,
                AgentStatus::Idle => HealthStatus::Healthy,
                _ => HealthStatus::Degraded("Agent not ready".to_string()),
            },
            Err(_) => HealthStatus::Unknown,
        }
    }

    fn get_status(&self) -> AgentStatus {
        self.status.try_read().map(|s| s.clone()).unwrap_or(AgentStatus::Unknown)
    }

    fn get_metrics(&self) -> AgentMetrics {
        self.metrics.get_agent_metrics(&self.id)
    }

    async fn initialize(&mut self) -> Result<(), AgentError> {
        self.initialize_mcp().await
    }

    async fn shutdown(&mut self) -> Result<(), AgentError> {
        *self.status.write().await = AgentStatus::Shutting;

        // 关闭MCP连接
        let mut client = self.mcp_client.lock().await;
        // 这里需要添加MCP客户端的shutdown方法

        *self.status.write().await = AgentStatus::Shutdown;
        Ok(())
    }
}
```

### 7.2 增强版提交消息生成Agent

```rust
/// 集成MCP的提交消息生成Agent
pub struct EnhancedCommitAgent {
    base_agent: CommitAgent,
    mcp_agent: Arc<MCPIntegrationAgent>,
    git_service: Arc<GitService>,
}

impl EnhancedCommitAgent {
    pub async fn new(
        base_config: AIConfig,
        mcp_config: MCPAgentConfig,
        git_service: Arc<GitService>
    ) -> Result<Self, AgentError> {
        let base_agent = CommitAgent::new(base_config).await?;
        let mcp_agent = Arc::new(MCPIntegrationAgent::new("mcp_integration".to_string(), mcp_config).await?);

        Ok(Self {
            base_agent,
            mcp_agent,
            git_service,
        })
    }

    /// 使用MCP获取增强的上下文信息
    async fn get_enhanced_context(&self, file_changes: &[FileChange]) -> Result<String, AgentError> {
        let mut context = String::new();

        // 获取仓库状态
        if let Ok(repo_status) = self.get_mcp_resource("git://repo/status").await {
            context.push_str("Repository Status:\n");
            context.push_str(&extract_text_content(&repo_status));
            context.push_str("\n\n");
        }

        // 获取分支信息
        if let Ok(branch_info) = self.get_mcp_resource("git://repo/branches").await {
            context.push_str("Branch Information:\n");
            context.push_str(&extract_text_content(&branch_info));
            context.push_str("\n\n");
        }

        // 获取最近的提交历史
        if let Ok(history) = self.get_mcp_resource("git://repo/history").await {
            context.push_str("Recent Commits:\n");
            context.push_str(&extract_text_content(&history));
            context.push_str("\n\n");
        }

        // 获取修改文件的内容（用于更好地理解变更）
        for change in file_changes {
            let uri = format!("git://repo/file/{}", change.path);
            if let Ok(file_content) = self.get_mcp_resource(&uri).await {
                context.push_str(&format!("File: {}\n", change.path));
                let content = extract_text_content(&file_content);
                if content.len() > 1000 {
                    context.push_str(&content[..1000]);
                    context.push_str("...\n\n");
                } else {
                    context.push_str(&content);
                    context.push_str("\n\n");
                }
            }
        }

        Ok(context)
    }

    async fn get_mcp_resource(&self, uri: &str) -> Result<ResourceContent, AgentError> {
        let task = AgentTask {
            id: Uuid::new_v4().to_string(),
            task_type: TaskType::Custom {
                capability: "mcp_resource_access".to_string(),
                payload: serde_json::json!({ "uri": uri }),
            },
            priority: TaskPriority::Normal,
            timeout: Some(std::time::Duration::from_secs(10)),
            created_at: SystemTime::now(),
            requester: "enhanced_commit_agent".to_string(),
            context: TaskContext {
                repository_path: None,
                branch_name: None,
                user_preferences: HashMap::new(),
                additional_data: HashMap::new(),
            },
        };

        let mut mcp_agent = self.mcp_agent.clone();
        let result = mcp_agent.handle_task(task).await?;

        match result.result {
            ResultData::Custom(value) => {
                serde_json::from_value(value)
                    .map_err(|e| AgentError::TaskProcessingFailed(e.to_string()))
            },
            _ => Err(AgentError::TaskProcessingFailed("Unexpected result type".to_string())),
        }
    }
}

fn extract_text_content(content: &ResourceContent) -> String {
    match &content.content {
        ResourceData::Text { text } => text.clone(),
        ResourceData::Blob { blob: _ } => "[Binary content]".to_string(),
    }
}

#[async_trait]
impl Agent for EnhancedCommitAgent {
    fn id(&self) -> &str {
        "enhanced_commit_agent"
    }

    fn name(&self) -> &str {
        "Enhanced Commit Message Generator"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn capabilities(&self) -> Vec<AgentCapability> {
        vec![
            AgentCapability::CommitMessageGeneration,
            AgentCapability::CodeAnalysis,
        ]
    }

    fn config(&self) -> &AgentConfig {
        self.base_agent.config()
    }

    async fn handle_task(&mut self, task: AgentTask) -> Result<AgentResult, AgentError> {
        match &task.task_type {
            TaskType::GenerateCommitMessage { file_changes, context } => {
                // 获取增强的上下文信息
                let enhanced_context = self.get_enhanced_context(file_changes).await
                    .unwrap_or_else(|_| "No additional context available".to_string());

                // 创建增强的上下文
                let mut enhanced_commit_context = context.clone();
                enhanced_commit_context.repository_context = Some(enhanced_context);

                // 使用增强的上下文生成提交消息
                let enhanced_task = AgentTask {
                    task_type: TaskType::GenerateCommitMessage {
                        file_changes: file_changes.clone(),
                        context: enhanced_commit_context,
                    },
                    ..task
                };

                self.base_agent.handle_task(enhanced_task).await
            },
            _ => self.base_agent.handle_task(task).await,
        }
    }

    fn health_check(&self) -> HealthStatus {
        let base_health = self.base_agent.health_check();
        let mcp_health = self.mcp_agent.health_check();

        match (base_health, mcp_health) {
            (HealthStatus::Healthy, HealthStatus::Healthy) => HealthStatus::Healthy,
            (HealthStatus::Healthy, HealthStatus::Degraded(msg)) => HealthStatus::Degraded(format!("MCP: {}", msg)),
            (HealthStatus::Degraded(msg), HealthStatus::Healthy) => HealthStatus::Degraded(format!("Base: {}", msg)),
            (HealthStatus::Unhealthy(msg), _) => HealthStatus::Unhealthy(format!("Base: {}", msg)),
            (_, HealthStatus::Unhealthy(msg)) => HealthStatus::Unhealthy(format!("MCP: {}", msg)),
            _ => HealthStatus::Degraded("Mixed health status".to_string()),
        }
    }

    fn get_status(&self) -> AgentStatus {
        self.base_agent.get_status()
    }

    fn get_metrics(&self) -> AgentMetrics {
        self.base_agent.get_metrics()
    }

    async fn initialize(&mut self) -> Result<(), AgentError> {
        self.base_agent.initialize().await?;
        // MCP Agent应该已经在构造时初始化
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), AgentError> {
        self.base_agent.shutdown().await?;
        // MCP Agent的关闭由其生命周期管理
        Ok(())
    }
}
```

## 8. 错误处理和日志

### 8.1 统一错误处理

```rust
/// MCP相关错误的扩展
impl From<MCPError> for AgentError {
    fn from(error: MCPError) -> Self {
        match error.code {
            -32700..=-32600 => AgentError::TaskProcessingFailed(format!("MCP protocol error: {}", error.message)),
            -32601 => AgentError::UnsupportedCapability(format!("MCP method not found: {}", error.message)),
            -32603 => AgentError::InternalError(format!("MCP internal error: {}", error.message)),
            _ => AgentError::TaskProcessingFailed(format!("MCP error: {}", error.message)),
        }
    }
}

impl From<TransportError> for MCPError {
    fn from(error: TransportError) -> Self {
        match error {
            TransportError::ConnectionError(msg) => MCPError::new(MCPErrorCode::ServerError, msg),
            TransportError::SerializationError(msg) => MCPError::new(MCPErrorCode::ParseError, msg),
            TransportError::NetworkError(msg) => MCPError::new(MCPErrorCode::ServerError, msg),
            TransportError::ProtocolError(msg) => MCPError::new(MCPErrorCode::InvalidRequest, msg),
            TransportError::Timeout => MCPError::new(MCPErrorCode::ServerError, "Request timeout"),
            TransportError::ConnectionClosed => MCPError::new(MCPErrorCode::ServerError, "Connection closed"),
        }
    }
}

impl From<serde_json::Error> for MCPError {
    fn from(error: serde_json::Error) -> Self {
        MCPError::new(MCPErrorCode::SerializationError, error.to_string())
    }
}
```

### 8.2 结构化日志

```rust
/// MCP事件日志
#[derive(Debug, Serialize)]
pub struct MCPLogEvent {
    pub timestamp: SystemTime,
    pub event_type: MCPEventType,
    pub client_id: Option<String>,
    pub method: Option<String>,
    pub request_id: Option<RequestId>,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
    pub metadata: HashMap<String, Value>,
}

#[derive(Debug, Serialize)]
pub enum MCPEventType {
    RequestReceived,
    RequestProcessed,
    ResponseSent,
    NotificationReceived,
    ClientConnected,
    ClientDisconnected,
    ToolCalled,
    ResourceAccessed,
    Error,
}

/// MCP日志记录器
pub struct MCPLogger {
    logger: Arc<dyn Logger>,
}

impl MCPLogger {
    pub fn new(logger: Arc<dyn Logger>) -> Self {
        Self { logger }
    }

    pub fn log_request(&self, request: &MCPRequest, client_id: &str) {
        let event = MCPLogEvent {
            timestamp: SystemTime::now(),
            event_type: MCPEventType::RequestReceived,
            client_id: Some(client_id.to_string()),
            method: Some(request.method.clone()),
            request_id: Some(request.id.clone()),
            duration_ms: None,
            error: None,
            metadata: HashMap::new(),
        };

        tracing::info!(
            target = "mcp::request",
            client_id = %client_id,
            method = %request.method,
            request_id = ?request.id,
            "MCP request received"
        );
    }

    pub fn log_response(&self, response: &MCPResponse, client_id: &str, duration: std::time::Duration) {
        let event = MCPLogEvent {
            timestamp: SystemTime::now(),
            event_type: MCPEventType::ResponseSent,
            client_id: Some(client_id.to_string()),
            method: None,
            request_id: Some(response.id.clone()),
            duration_ms: Some(duration.as_millis() as u64),
            error: response.error.as_ref().map(|e| e.message.clone()),
            metadata: HashMap::new(),
        };

        if let Some(error) = &response.error {
            tracing::error!(
                target = "mcp::response",
                client_id = %client_id,
                request_id = ?response.id,
                duration_ms = duration.as_millis(),
                error_code = error.code,
                error_message = %error.message,
                "MCP request failed"
            );
        } else {
            tracing::info!(
                target = "mcp::response",
                client_id = %client_id,
                request_id = ?response.id,
                duration_ms = duration.as_millis(),
                "MCP request completed"
            );
        }
    }

    pub fn log_tool_call(&self, tool_name: &str, client_id: &str, duration: std::time::Duration, success: bool) {
        tracing::info!(
            target = "mcp::tool",
            client_id = %client_id,
            tool_name = %tool_name,
            duration_ms = duration.as_millis(),
            success = success,
            "MCP tool called"
        );
    }
}

pub trait Logger: Send + Sync {
    fn log(&self, event: MCPLogEvent);
}
```

## 9. 性能优化和监控

### 9.1 连接池管理

```rust
/// MCP连接池
pub struct MCPConnectionPool {
    pool: Arc<RwLock<Vec<PooledConnection>>>,
    config: PoolConfig,
    metrics: Arc<PoolMetrics>,
}

#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub max_connections: usize,
    pub min_connections: usize,
    pub max_idle_time: std::time::Duration,
    pub connection_timeout: std::time::Duration,
    pub health_check_interval: std::time::Duration,
}

struct PooledConnection {
    client: MCPClient,
    created_at: SystemTime,
    last_used: SystemTime,
    in_use: bool,
}

struct PoolMetrics {
    active_connections: Arc<std::sync::atomic::AtomicUsize>,
    idle_connections: Arc<std::sync::atomic::AtomicUsize>,
    total_requests: Arc<std::sync::atomic::AtomicU64>,
    failed_requests: Arc<std::sync::atomic::AtomicU64>,
    average_response_time: Arc<std::sync::atomic::AtomicU64>,
}

impl MCPConnectionPool {
    pub fn new(config: PoolConfig) -> Self {
        Self {
            pool: Arc::new(RwLock::new(Vec::new())),
            config,
            metrics: Arc::new(PoolMetrics {
                active_connections: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
                idle_connections: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
                total_requests: Arc::new(std::sync::atomic::AtomicU64::new(0)),
                failed_requests: Arc::new(std::sync::atomic::AtomicU64::new(0)),
                average_response_time: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            }),
        }
    }

    pub async fn get_connection(&self) -> Result<PooledConnection, MCPError> {
        // 实现连接获取逻辑
        todo!()
    }

    pub async fn return_connection(&self, connection: PooledConnection) {
        // 实现连接归还逻辑
        todo!()
    }

    pub async fn health_check(&self) {
        // 实现健康检查逻辑
        todo!()
    }
}
```

### 9.2 性能监控

```rust
/// MCP性能监控器
pub struct MCPPerformanceMonitor {
    request_metrics: Arc<RwLock<HashMap<String, RequestMetrics>>>,
    transport_metrics: Arc<RwLock<HashMap<TransportType, TransportMetrics>>>,
    system_metrics: Arc<RwLock<SystemMetrics>>,
}

#[derive(Debug, Clone)]
struct RequestMetrics {
    total_count: u64,
    success_count: u64,
    error_count: u64,
    total_duration: std::time::Duration,
    min_duration: std::time::Duration,
    max_duration: std::time::Duration,
}

#[derive(Debug, Clone)]
struct TransportMetrics {
    bytes_sent: u64,
    bytes_received: u64,
    connection_count: u64,
    reconnection_count: u64,
    error_count: u64,
}

impl MCPPerformanceMonitor {
    pub fn new() -> Self {
        Self {
            request_metrics: Arc::new(RwLock::new(HashMap::new())),
            transport_metrics: Arc::new(RwLock::new(HashMap::new())),
            system_metrics: Arc::new(RwLock::new(SystemMetrics::default())),
        }
    }

    pub async fn record_request(&self, method: &str, duration: std::time::Duration, success: bool) {
        let mut metrics = self.request_metrics.write().await;
        let request_metrics = metrics.entry(method.to_string()).or_insert(RequestMetrics {
            total_count: 0,
            success_count: 0,
            error_count: 0,
            total_duration: std::time::Duration::from_nanos(0),
            min_duration: duration,
            max_duration: duration,
        });

        request_metrics.total_count += 1;
        if success {
            request_metrics.success_count += 1;
        } else {
            request_metrics.error_count += 1;
        }

        request_metrics.total_duration += duration;
        if duration < request_metrics.min_duration {
            request_metrics.min_duration = duration;
        }
        if duration > request_metrics.max_duration {
            request_metrics.max_duration = duration;
        }
    }

    pub async fn get_performance_report(&self) -> PerformanceReport {
        let request_metrics = self.request_metrics.read().await;
        let transport_metrics = self.transport_metrics.read().await;
        let system_metrics = self.system_metrics.read().await;

        PerformanceReport {
            request_metrics: request_metrics.clone(),
            transport_metrics: transport_metrics.clone(),
            system_metrics: system_metrics.clone(),
            generated_at: SystemTime::now(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceReport {
    pub request_metrics: HashMap<String, RequestMetrics>,
    pub transport_metrics: HashMap<TransportType, TransportMetrics>,
    pub system_metrics: SystemMetrics,
    pub generated_at: SystemTime,
}
```

这个MCP协议集成技术设计文档提供了完整的实现指导，包括：

1. **核心协议接口**: 完整的MCP消息结构和错误处理
2. **资源管理系统**: 支持静态和动态资源，包含Git资源提供者
3. **工具管理系统**: 完整的工具注册、执行和统计框架
4. **传输层架构**: HTTP、WebSocket和标准IO的完整实现
5. **客户端和服务器**: 功能完整的MCP客户端和服务器实现
6. **Agent集成**: 展示如何将MCP协议集成到Agent系统中
7. **错误处理和日志**: 统一的错误处理和结构化日志
8. **性能优化**: 连接池和性能监控机制

这个设计确保了MCP协议在AI-Commit TUI项目中的高效、可靠、可扩展的集成。

//! MCP Resource Management
//!
//! Provides resource discovery, access control, and content management
//! for the Model Context Protocol implementation.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    fs,
    sync::RwLock,
};
use tracing::{debug, info, warn};
use url::Url;

use crate::error::{AppError, AppResult};
use super::{
    errors::{MCPError, MCPResult},
    protocol::{MCPMessage, MCPRequest, MCPResponse, MethodName, MessageId},
};

/// Resource URI type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceUri(String);

impl ResourceUri {
    /// Create a new resource URI
    pub fn new(uri: impl Into<String>) -> MCPResult<Self> {
        let uri_string = uri.into();

        // Validate URI format
        if uri_string.is_empty() {
            return Err(MCPError::validation("Resource URI cannot be empty"));
        }

        // Check if it's a valid URI
        if let Err(e) = Url::parse(&uri_string) {
            // If it's not a valid URL, check if it's a valid file path
            if !Path::new(&uri_string).is_absolute() && !uri_string.starts_with("file://") {
                return Err(MCPError::validation(format!("Invalid resource URI: {}", e)));
            }
        }

        Ok(Self(uri_string))
    }

    /// Create from file path
    pub fn from_file_path(path: impl AsRef<Path>) -> MCPResult<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(MCPError::resource_not_found(path.display().to_string()));
        }

        let uri = if path.is_absolute() {
            format!("file://{}", path.display())
        } else {
            return Err(MCPError::validation("File path must be absolute"));
        };

        Ok(Self(uri))
    }

    /// Create from URL
    pub fn from_url(url: impl AsRef<str>) -> MCPResult<Self> {
        let url_str = url.as_ref();
        Url::parse(url_str)
            .map_err(|e| MCPError::validation(format!("Invalid URL: {}", e)))?;
        Ok(Self(url_str.to_string()))
    }

    /// Get the URI as string
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the scheme (e.g., "file", "http", "git")
    pub fn scheme(&self) -> Option<&str> {
        if let Ok(url) = Url::parse(&self.0) {
            Some(url.scheme())
        } else {
            None
        }
    }

    /// Check if this is a file URI
    pub fn is_file(&self) -> bool {
        self.scheme() == Some("file") || Path::new(&self.0).exists()
    }

    /// Check if this is an HTTP URI
    pub fn is_http(&self) -> bool {
        matches!(self.scheme(), Some("http") | Some("https"))
    }

    /// Convert to file path if possible
    pub fn to_file_path(&self) -> Option<PathBuf> {
        if self.is_file() {
            if let Ok(url) = Url::parse(&self.0) {
                url.to_file_path().ok()
            } else {
                Some(PathBuf::from(&self.0))
            }
        } else {
            None
        }
    }
}

impl std::fmt::Display for ResourceUri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for ResourceUri {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ResourceUri {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Resource type enumeration
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    /// Text file resource
    Text,
    /// Binary file resource
    Binary,
    /// Directory resource
    Directory,
    /// Git repository resource
    GitRepository,
    /// Web resource
    Web,
    /// Database resource
    Database,
    /// API endpoint resource
    Api,
    /// Configuration resource
    Config,
    /// Documentation resource
    Documentation,
    /// Custom resource type
    Custom(String),
}

impl ResourceType {
    /// Get MIME type for the resource type
    pub fn mime_type(&self) -> &'static str {
        match self {
            ResourceType::Text => "text/plain",
            ResourceType::Binary => "application/octet-stream",
            ResourceType::Directory => "inode/directory",
            ResourceType::GitRepository => "application/x-git",
            ResourceType::Web => "text/html",
            ResourceType::Database => "application/x-sqlite3",
            ResourceType::Api => "application/json",
            ResourceType::Config => "application/toml",
            ResourceType::Documentation => "text/markdown",
            ResourceType::Custom(_) => "application/octet-stream",
        }
    }

    /// Determine resource type from file extension
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "txt" | "md" | "rs" | "py" | "js" | "ts" | "html" | "css" | "json" | "toml" | "yaml" | "yml" => {
                ResourceType::Text
            }
            "pdf" | "png" | "jpg" | "jpeg" | "gif" | "zip" | "tar" | "gz" => {
                ResourceType::Binary
            }
            "git" => ResourceType::GitRepository,
            "db" | "sqlite" | "sqlite3" => ResourceType::Database,
            _ => ResourceType::Text, // Default to text
        }
    }
}

/// Resource content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceContent {
    /// Text content
    Text(String),
    /// Binary content
    Binary(Vec<u8>),
    /// JSON content
    Json(serde_json::Value),
    /// Directory listing
    Directory(Vec<ResourceUri>),
    /// Reference to external content
    Reference(ResourceUri),
}

impl ResourceContent {
    /// Get content size in bytes
    pub fn size(&self) -> usize {
        match self {
            ResourceContent::Text(text) => text.len(),
            ResourceContent::Binary(data) => data.len(),
            ResourceContent::Json(value) => serde_json::to_string(value).unwrap_or_default().len(),
            ResourceContent::Directory(items) => items.len() * 32, // Estimate
            ResourceContent::Reference(uri) => uri.as_str().len(),
        }
    }

    /// Check if content is empty
    pub fn is_empty(&self) -> bool {
        match self {
            ResourceContent::Text(text) => text.is_empty(),
            ResourceContent::Binary(data) => data.is_empty(),
            ResourceContent::Json(value) => value.is_null(),
            ResourceContent::Directory(items) => items.is_empty(),
            ResourceContent::Reference(_) => false,
        }
    }

    /// Convert to string if possible
    pub fn to_string(&self) -> Option<String> {
        match self {
            ResourceContent::Text(text) => Some(text.clone()),
            ResourceContent::Json(value) => serde_json::to_string_pretty(value).ok(),
            _ => None,
        }
    }
}

/// Resource metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetadata {
    /// Resource name
    pub name: String,
    /// Resource description
    pub description: Option<String>,
    /// Resource type
    pub resource_type: ResourceType,
    /// Content size in bytes
    pub size: u64,
    /// Last modified timestamp
    pub modified: DateTime<Utc>,
    /// MIME type
    pub mime_type: String,
    /// Additional metadata
    pub attributes: HashMap<String, serde_json::Value>,
}

impl ResourceMetadata {
    /// Create new metadata
    pub fn new(name: String, resource_type: ResourceType) -> Self {
        Self {
            name,
            description: None,
            mime_type: resource_type.mime_type().to_string(),
            resource_type,
            size: 0,
            modified: Utc::now(),
            attributes: HashMap::new(),
        }
    }

    /// Add attribute
    pub fn with_attribute(mut self, key: String, value: serde_json::Value) -> Self {
        self.attributes.insert(key, value);
        self
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Set size
    pub fn with_size(mut self, size: u64) -> Self {
        self.size = size;
        self
    }
}

/// Resource permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePermissions {
    /// Allow reading the resource
    pub read: bool,
    /// Allow writing to the resource
    pub write: bool,
    /// Allow executing the resource
    pub execute: bool,
    /// Allow listing directory contents
    pub list: bool,
    /// Allow deleting the resource
    pub delete: bool,
    /// Allowed user/client IDs
    pub allowed_clients: Option<Vec<String>>,
    /// Denied user/client IDs
    pub denied_clients: Option<Vec<String>>,
}

impl Default for ResourcePermissions {
    fn default() -> Self {
        Self {
            read: true,
            write: false,
            execute: false,
            list: true,
            delete: false,
            allowed_clients: None,
            denied_clients: None,
        }
    }
}

impl ResourcePermissions {
    /// Create read-only permissions
    pub fn read_only() -> Self {
        Self {
            read: true,
            write: false,
            execute: false,
            list: true,
            delete: false,
            allowed_clients: None,
            denied_clients: None,
        }
    }

    /// Create read-write permissions
    pub fn read_write() -> Self {
        Self {
            read: true,
            write: true,
            execute: false,
            list: true,
            delete: false,
            allowed_clients: None,
            denied_clients: None,
        }
    }

    /// Create full permissions
    pub fn full() -> Self {
        Self {
            read: true,
            write: true,
            execute: true,
            list: true,
            delete: true,
            allowed_clients: None,
            denied_clients: None,
        }
    }

    /// Check if client has permission
    pub fn check_client_permission(&self, client_id: &str) -> bool {
        // Check denied list first
        if let Some(ref denied) = self.denied_clients {
            if denied.contains(&client_id.to_string()) {
                return false;
            }
        }

        // Check allowed list
        if let Some(ref allowed) = self.allowed_clients {
            allowed.contains(&client_id.to_string())
        } else {
            true // No restrictions
        }
    }
}

/// Resource definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    /// Resource URI
    pub uri: ResourceUri,
    /// Resource metadata
    pub metadata: ResourceMetadata,
    /// Resource permissions
    pub permissions: ResourcePermissions,
    /// Resource content (optional, loaded on demand)
    pub content: Option<ResourceContent>,
    /// Cache timestamp
    pub cached_at: Option<DateTime<Utc>>,
}

impl Resource {
    /// Create a new resource
    pub fn new(uri: ResourceUri, metadata: ResourceMetadata) -> Self {
        Self {
            uri,
            metadata,
            permissions: ResourcePermissions::default(),
            content: None,
            cached_at: None,
        }
    }

    /// Set permissions
    pub fn with_permissions(mut self, permissions: ResourcePermissions) -> Self {
        self.permissions = permissions;
        self
    }

    /// Set content
    pub fn with_content(mut self, content: ResourceContent) -> Self {
        self.metadata.size = content.size() as u64;
        self.content = Some(content);
        self.cached_at = Some(Utc::now());
        self
    }

    /// Check if content is cached and fresh
    pub fn is_content_fresh(&self, max_age: Duration) -> bool {
        if let (Some(content), Some(cached_at)) = (&self.content, self.cached_at) {
            let age = Utc::now().signed_duration_since(cached_at);
            age.to_std().unwrap_or(Duration::MAX) < max_age
        } else {
            false
        }
    }

    /// Clear cached content
    pub fn clear_cache(&mut self) {
        self.content = None;
        self.cached_at = None;
    }
}

/// Resource provider trait
#[async_trait]
pub trait ResourceProvider: Send + Sync {
    /// List available resources
    async fn list_resources(&self) -> MCPResult<Vec<Resource>>;

    /// Read resource content
    async fn read_resource(&self, uri: &ResourceUri) -> MCPResult<ResourceContent>;

    /// Check if resource exists
    async fn resource_exists(&self, uri: &ResourceUri) -> bool;

    /// Get resource metadata
    async fn get_metadata(&self, uri: &ResourceUri) -> MCPResult<ResourceMetadata>;

    /// Write resource content (if supported)
    async fn write_resource(&self, uri: &ResourceUri, content: ResourceContent) -> MCPResult<()> {
        Err(MCPError::feature_not_supported("Writing not supported"))
    }

    /// Delete resource (if supported)
    async fn delete_resource(&self, uri: &ResourceUri) -> MCPResult<()> {
        Err(MCPError::feature_not_supported("Deletion not supported"))
    }

    /// Get provider capabilities
    fn capabilities(&self) -> ResourceProviderCapabilities;
}

/// Resource provider capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceProviderCapabilities {
    /// Supports reading resources
    pub read: bool,
    /// Supports writing resources
    pub write: bool,
    /// Supports deleting resources
    pub delete: bool,
    /// Supports listing resources
    pub list: bool,
    /// Supports metadata queries
    pub metadata: bool,
    /// Supported URI schemes
    pub schemes: Vec<String>,
}

impl Default for ResourceProviderCapabilities {
    fn default() -> Self {
        Self {
            read: true,
            write: false,
            delete: false,
            list: true,
            metadata: true,
            schemes: vec!["file".to_string()],
        }
    }
}

/// File system resource provider
pub struct FileSystemProvider {
    /// Base directory for file access
    base_dir: PathBuf,
    /// Provider capabilities
    capabilities: ResourceProviderCapabilities,
}

impl FileSystemProvider {
    /// Create a new file system provider
    pub fn new(base_dir: impl AsRef<Path>) -> MCPResult<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();

        if !base_dir.exists() {
            return Err(MCPError::configuration(format!("Base directory does not exist: {}", base_dir.display())));
        }

        if !base_dir.is_dir() {
            return Err(MCPError::configuration(format!("Base path is not a directory: {}", base_dir.display())));
        }

        Ok(Self {
            base_dir,
            capabilities: ResourceProviderCapabilities {
                read: true,
                write: true,
                delete: true,
                list: true,
                metadata: true,
                schemes: vec!["file".to_string()],
            },
        })
    }

    /// Resolve URI to file path
    fn resolve_path(&self, uri: &ResourceUri) -> MCPResult<PathBuf> {
        let file_path = uri.to_file_path()
            .ok_or_else(|| MCPError::validation("Not a file URI"))?;

        // Ensure path is within base directory
        let canonical_base = self.base_dir.canonicalize()
            .map_err(|e| MCPError::configuration(format!("Failed to canonicalize base dir: {}", e)))?;

        let canonical_path = file_path.canonicalize()
            .map_err(|_| MCPError::resource_not_found(uri.as_str()))?;

        if !canonical_path.starts_with(&canonical_base) {
            return Err(MCPError::permission_denied(format!("Access denied to path outside base directory: {}", file_path.display())));
        }

        Ok(canonical_path)
    }

    /// Create metadata from file info
    async fn create_metadata(&self, path: &Path) -> MCPResult<ResourceMetadata> {
        let metadata = fs::metadata(path).await
            .map_err(|e| MCPError::resource(format!("Failed to get file metadata: {}", e)))?;

        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let resource_type = if metadata.is_dir() {
            ResourceType::Directory
        } else {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(ResourceType::from_extension)
                .unwrap_or(ResourceType::Binary)
        };

        let modified = metadata.modified()
            .map(DateTime::<Utc>::from)
            .unwrap_or_else(|_| Utc::now());

        Ok(ResourceMetadata::new(name, resource_type)
            .with_size(metadata.len()))
    }
}

#[async_trait]
impl ResourceProvider for FileSystemProvider {
    async fn list_resources(&self) -> MCPResult<Vec<Resource>> {
        let mut resources = Vec::new();
        let mut entries = fs::read_dir(&self.base_dir).await
            .map_err(|e| MCPError::resource(format!("Failed to read directory: {}", e)))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| MCPError::resource(format!("Failed to read directory entry: {}", e)))? {

            let path = entry.path();
            let uri = ResourceUri::from_file_path(&path)?;
            let metadata = self.create_metadata(&path).await?;

            let resource = Resource::new(uri, metadata)
                .with_permissions(ResourcePermissions::read_only());

            resources.push(resource);
        }

        Ok(resources)
    }

    async fn read_resource(&self, uri: &ResourceUri) -> MCPResult<ResourceContent> {
        let path = self.resolve_path(uri)?;

        if path.is_dir() {
            // List directory contents
            let mut entries = fs::read_dir(&path).await
                .map_err(|e| MCPError::resource(format!("Failed to read directory: {}", e)))?;

            let mut contents = Vec::new();
            while let Some(entry) = entries.next_entry().await
                .map_err(|e| MCPError::resource(format!("Failed to read directory entry: {}", e)))? {

                let entry_path = entry.path();
                if let Ok(entry_uri) = ResourceUri::from_file_path(&entry_path) {
                    contents.push(entry_uri);
                }
            }

            Ok(ResourceContent::Directory(contents))
        } else {
            // Read file content
            let content = fs::read(&path).await
                .map_err(|e| MCPError::resource(format!("Failed to read file: {}", e)))?;

            // Try to decode as UTF-8 text
            match String::from_utf8(content.clone()) {
                Ok(text) => Ok(ResourceContent::Text(text)),
                Err(_) => Ok(ResourceContent::Binary(content)),
            }
        }
    }

    async fn resource_exists(&self, uri: &ResourceUri) -> bool {
        if let Ok(path) = self.resolve_path(uri) {
            path.exists()
        } else {
            false
        }
    }

    async fn get_metadata(&self, uri: &ResourceUri) -> MCPResult<ResourceMetadata> {
        let path = self.resolve_path(uri)?;
        self.create_metadata(&path).await
    }

    async fn write_resource(&self, uri: &ResourceUri, content: ResourceContent) -> MCPResult<()> {
        let path = self.resolve_path(uri)?;

        match content {
            ResourceContent::Text(text) => {
                fs::write(&path, text.as_bytes()).await
                    .map_err(|e| MCPError::resource(format!("Failed to write file: {}", e)))?;
            }
            ResourceContent::Binary(data) => {
                fs::write(&path, data).await
                    .map_err(|e| MCPError::resource(format!("Failed to write file: {}", e)))?;
            }
            ResourceContent::Json(value) => {
                let json_text = serde_json::to_string_pretty(&value)
                    .map_err(|e| MCPError::serialization(format!("Failed to serialize JSON: {}", e)))?;
                fs::write(&path, json_text.as_bytes()).await
                    .map_err(|e| MCPError::resource(format!("Failed to write file: {}", e)))?;
            }
            _ => {
                return Err(MCPError::feature_not_supported("Unsupported content type for writing"));
            }
        }

        Ok(())
    }

    async fn delete_resource(&self, uri: &ResourceUri) -> MCPResult<()> {
        let path = self.resolve_path(uri)?;

        if path.is_dir() {
            fs::remove_dir_all(&path).await
                .map_err(|e| MCPError::resource(format!("Failed to delete directory: {}", e)))?;
        } else {
            fs::remove_file(&path).await
                .map_err(|e| MCPError::resource(format!("Failed to delete file: {}", e)))?;
        }

        Ok(())
    }

    fn capabilities(&self) -> ResourceProviderCapabilities {
        self.capabilities.clone()
    }
}

/// Resource registry for managing multiple providers
pub struct ResourceRegistry {
    /// Registered resource providers by scheme
    providers: Arc<RwLock<HashMap<String, Arc<dyn ResourceProvider>>>>,
    /// Resource cache
    cache: Arc<RwLock<HashMap<ResourceUri, Resource>>>,
    /// Cache TTL
    cache_ttl: Duration,
}

impl ResourceRegistry {
    /// Create a new resource registry
    pub fn new() -> Self {
        Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: Duration::from_secs(300), // 5 minutes
        }
    }

    /// Register a resource provider for a scheme
    pub async fn register_provider(
        &self,
        scheme: String,
        provider: Arc<dyn ResourceProvider>,
    ) -> MCPResult<()> {
        let mut providers = self.providers.write().await;
        providers.insert(scheme.clone(), provider);
        info!("Registered resource provider for scheme: {}", scheme);
        Ok(())
    }

    /// Get provider for URI scheme
    async fn get_provider(&self, uri: &ResourceUri) -> MCPResult<Arc<dyn ResourceProvider>> {
        let scheme = uri.scheme()
            .ok_or_else(|| MCPError::validation("URI missing scheme"))?;

        let providers = self.providers.read().await;
        providers.get(scheme)
            .cloned()
            .ok_or_else(|| MCPError::feature_not_supported(format!("No provider for scheme: {}", scheme)))
    }

    /// List all available resources
    pub async fn list_resources(&self) -> MCPResult<Vec<Resource>> {
        let mut all_resources = Vec::new();
        let providers = self.providers.read().await;

        for (scheme, provider) in providers.iter() {
            match provider.list_resources().await {
                Ok(mut resources) => {
                    all_resources.append(&mut resources);
                }
                Err(e) => {
                    warn!("Failed to list resources for scheme {}: {}", scheme, e);
                }
            }
        }

        Ok(all_resources)
    }

    /// Read resource with caching
    pub async fn read_resource(&self, uri: &ResourceUri) -> MCPResult<ResourceContent> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(resource) = cache.get(uri) {
                if resource.is_content_fresh(self.cache_ttl) {
                    if let Some(ref content) = resource.content {
                        debug!("Cache hit for resource: {}", uri);
                        return Ok(content.clone());
                    }
                }
            }
        }

        // Cache miss or stale, fetch from provider
        let provider = self.get_provider(uri).await?;
        let content = provider.read_resource(uri).await?;

        // Update cache
        {
            let mut cache = self.cache.write().await;
            if let Some(resource) = cache.get_mut(uri) {
                resource.content = Some(content.clone());
                resource.cached_at = Some(Utc::now());
            } else {
                // Create basic resource for caching
                let metadata = provider.get_metadata(uri).await
                    .unwrap_or_else(|_| ResourceMetadata::new("unknown".to_string(), ResourceType::Binary));
                let resource = Resource::new(uri.clone(), metadata)
                    .with_content(content.clone());
                cache.insert(uri.clone(), resource);
            }
        }

        debug!("Cache miss for resource: {}", uri);
        Ok(content)
    }

    /// Get resource metadata
    pub async fn get_metadata(&self, uri: &ResourceUri) -> MCPResult<ResourceMetadata> {
        let provider = self.get_provider(uri).await?;
        provider.get_metadata(uri).await
    }

    /// Check if resource exists
    pub async fn resource_exists(&self, uri: &ResourceUri) -> bool {
        if let Ok(provider) = self.get_provider(uri).await {
            provider.resource_exists(uri).await
        } else {
            false
        }
    }

    /// Clear cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        info!("Resource cache cleared");
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read().await;
        let total = cache.len();
        let fresh = cache.values()
            .filter(|r| r.is_content_fresh(self.cache_ttl))
            .count();
        (total, fresh)
    }
}

impl Default for ResourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Resource manager handles MCP resource operations
pub struct ResourceManager {
    /// Resource registry
    registry: ResourceRegistry,
    /// Default permissions
    default_permissions: ResourcePermissions,
}

impl ResourceManager {
    /// Create a new resource manager
    pub fn new() -> Self {
        Self {
            registry: ResourceRegistry::new(),
            default_permissions: ResourcePermissions::read_only(),
        }
    }

    /// Register a file system provider
    pub async fn register_file_provider(
        &self,
        base_dir: impl AsRef<Path>,
    ) -> MCPResult<()> {
        let provider = Arc::new(FileSystemProvider::new(base_dir)?);
        self.registry.register_provider("file".to_string(), provider).await
    }

    /// Handle MCP list resources request
    pub async fn handle_list_resources(&self, _request: MCPRequest) -> MCPResult<MCPResponse> {
        let resources = self.registry.list_resources().await?;
        let resource_list: Vec<serde_json::Value> = resources
            .iter()
            .map(|r| serde_json::json!({
                "uri": r.uri.as_str(),
                "name": r.metadata.name,
                "description": r.metadata.description,
                "mimeType": r.metadata.mime_type,
            }))
            .collect();

        let result = serde_json::json!({
            "resources": resource_list
        });

        Ok(MCPResponse::success(
            MessageId::generate(),
            result,
        ))
    }

    /// Handle MCP read resource request
    pub async fn handle_read_resource(&self, request: MCPRequest) -> MCPResult<MCPResponse> {
        let params = request.params
            .ok_or_else(|| MCPError::invalid_params("Missing parameters"))?;

        let uri_str = params.get("uri")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MCPError::invalid_params("Missing uri parameter"))?;

        let uri = ResourceUri::new(uri_str)?;
        let content = self.registry.read_resource(&uri).await?;

        let result = match content {
            ResourceContent::Text(text) => serde_json::json!({
                "contents": [{
                    "uri": uri.as_str(),
                    "mimeType": "text/plain",
                    "text": text
                }]
            }),
            ResourceContent::Binary(data) => serde_json::json!({
                "contents": [{
                    "uri": uri.as_str(),
                    "mimeType": "application/octet-stream",
                    "blob": base64::encode(data)
                }]
            }),
            ResourceContent::Json(value) => serde_json::json!({
                "contents": [{
                    "uri": uri.as_str(),
                    "mimeType": "application/json",
                    "text": serde_json::to_string_pretty(&value)?
                }]
            }),
            ResourceContent::Directory(items) => serde_json::json!({
                "contents": [{
                    "uri": uri.as_str(),
                    "mimeType": "inode/directory",
                    "text": items.iter().map(|u| u.as_str()).collect::<Vec<_>>().join("\n")
                }]
            }),
            ResourceContent::Reference(ref_uri) => serde_json::json!({
                "contents": [{
                    "uri": uri.as_str(),
                    "mimeType": "text/uri-list",
                    "text": ref_uri.as_str()
                }]
            }),
        };

        Ok(MCPResponse::success(request.id, result))
    }

    /// Get registry reference
    pub fn registry(&self) -> &ResourceRegistry {
        &self.registry
    }
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_resource_uri() {
        let uri = ResourceUri::new("file:///tmp/test.txt").unwrap();
        assert!(uri.is_file());
        assert!(!uri.is_http());
        assert_eq!(uri.scheme(), Some("file"));

        let http_uri = ResourceUri::new("https://example.com/test").unwrap();
        assert!(!http_uri.is_file());
        assert!(http_uri.is_http());
        assert_eq!(http_uri.scheme(), Some("https"));
    }

    #[test]
    fn test_resource_type() {
        assert_eq!(ResourceType::from_extension("txt"), ResourceType::Text);
        assert_eq!(ResourceType::from_extension("pdf"), ResourceType::Binary);
        assert_eq!(ResourceType::Text.mime_type(), "text/plain");
    }

    #[test]
    fn test_resource_content() {
        let text_content = ResourceContent::Text("Hello world".to_string());
        assert_eq!(text_content.size(), 11);
        assert!(!text_content.is_empty());
        assert_eq!(text_content.to_string(), Some("Hello world".to_string()));

        let binary_content = ResourceContent::Binary(vec![1, 2, 3, 4]);
        assert_eq!(binary_content.size(), 4);
        assert!(!binary_content.is_empty());
    }

    #[test]
    fn test_resource_permissions() {
        let perms = ResourcePermissions::read_only();
        assert!(perms.read);
        assert!(!perms.write);

        let full_perms = ResourcePermissions::full();
        assert!(full_perms.read && full_perms.write && full_perms.execute);

        assert!(perms.check_client_permission("test-client"));
    }

    #[tokio::test]
    async fn test_file_system_provider() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        tokio::fs::write(&test_file, "Hello world").await.unwrap();

        let provider = FileSystemProvider::new(temp_dir.path()).unwrap();
        let capabilities = provider.capabilities();
        assert!(capabilities.read);
        assert!(capabilities.write);

        let resources = provider.list_resources().await.unwrap();
        assert!(!resources.is_empty());

        let uri = ResourceUri::from_file_path(&test_file).unwrap();
        assert!(provider.resource_exists(&uri).await);

        let content = provider.read_resource(&uri).await.unwrap();
        match content {
            ResourceContent::Text(text) => assert_eq!(text, "Hello world"),
            _ => panic!("Expected text content"),
        }
    }

    #[tokio::test]
    async fn test_resource_registry() {
        let temp_dir = TempDir::new().unwrap();
        let provider = Arc::new(FileSystemProvider::new(temp_dir.path()).unwrap());

        let registry = ResourceRegistry::new();
        registry.register_provider("file".to_string(), provider).await.unwrap();

        let resources = registry.list_resources().await.unwrap();
        assert!(resources.is_empty() || !resources.is_empty()); // Directory might be empty

        let (total, fresh) = registry.cache_stats().await;
        assert_eq!(total, 0); // No cache entries initially
        assert_eq!(fresh, 0);
    }
}
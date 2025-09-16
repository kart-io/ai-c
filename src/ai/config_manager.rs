//! Agent configuration management system
//!
//! Provides centralized configuration management for agents with
//! hot-reloading capabilities and configuration validation.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, error, info, warn};

use super::AgentConfig;

/// Configuration error types
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
pub enum ConfigError {
    #[error("Configuration not found: {0}")]
    NotFound(String),

    #[error("Configuration validation failed: {0}")]
    ValidationFailed(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Configuration store error: {0}")]
    StoreError(String),

    #[error("Configuration watcher error: {0}")]
    WatcherError(String),
}

impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> Self {
        ConfigError::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(err: serde_json::Error) -> Self {
        ConfigError::SerializationError(err.to_string())
    }
}

/// Configuration change watcher callback
pub type ConfigWatcher = Arc<dyn Fn(&str, &AgentConfig) -> Result<(), ConfigError> + Send + Sync>;

/// Configuration store trait for different backends
#[async_trait]
pub trait ConfigStore: Send + Sync {
    /// Load configuration by agent ID
    async fn load_config(&self, agent_id: &str) -> Result<AgentConfig, ConfigError>;

    /// Save configuration for agent
    async fn save_config(&self, agent_id: &str, config: &AgentConfig) -> Result<(), ConfigError>;

    /// List all available configurations
    async fn list_configs(&self) -> Result<Vec<String>, ConfigError>;

    /// Delete configuration for agent
    async fn delete_config(&self, agent_id: &str) -> Result<(), ConfigError>;

    /// Check if configuration exists
    async fn exists(&self, agent_id: &str) -> Result<bool, ConfigError>;
}

/// File-based configuration store
#[derive(Debug)]
pub struct FileConfigStore {
    config_dir: PathBuf,
}

impl FileConfigStore {
    pub fn new(config_dir: PathBuf) -> Self {
        Self { config_dir }
    }

    fn get_config_path(&self, agent_id: &str) -> PathBuf {
        self.config_dir.join(format!("{}.json", agent_id))
    }
}

#[async_trait]
impl ConfigStore for FileConfigStore {
    async fn load_config(&self, agent_id: &str) -> Result<AgentConfig, ConfigError> {
        let path = self.get_config_path(agent_id);

        if !path.exists() {
            return Err(ConfigError::NotFound(format!("Configuration file not found for agent: {}", agent_id)));
        }

        let content = tokio::fs::read_to_string(&path).await?;
        let config: AgentConfig = serde_json::from_str(&content)?;

        debug!("Loaded configuration for agent: {}", agent_id);
        Ok(config)
    }

    async fn save_config(&self, agent_id: &str, config: &AgentConfig) -> Result<(), ConfigError> {
        // Ensure config directory exists
        tokio::fs::create_dir_all(&self.config_dir).await?;

        let path = self.get_config_path(agent_id);
        let content = serde_json::to_string_pretty(config)?;

        tokio::fs::write(&path, content).await?;
        debug!("Saved configuration for agent: {}", agent_id);
        Ok(())
    }

    async fn list_configs(&self) -> Result<Vec<String>, ConfigError> {
        if !self.config_dir.exists() {
            return Ok(Vec::new());
        }

        let mut entries = tokio::fs::read_dir(&self.config_dir).await?;
        let mut configs = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if path.extension().map_or(false, |ext| ext == "json") {
                    configs.push(stem.to_string());
                }
            }
        }

        Ok(configs)
    }

    async fn delete_config(&self, agent_id: &str) -> Result<(), ConfigError> {
        let path = self.get_config_path(agent_id);

        if path.exists() {
            tokio::fs::remove_file(&path).await?;
            debug!("Deleted configuration for agent: {}", agent_id);
        }

        Ok(())
    }

    async fn exists(&self, agent_id: &str) -> Result<bool, ConfigError> {
        let path = self.get_config_path(agent_id);
        Ok(path.exists())
    }
}

/// Agent configuration manager with hot-reloading support
pub struct ConfigManager {
    /// Configuration store backend
    config_store: Arc<dyn ConfigStore>,
    /// In-memory configuration cache
    config_cache: Arc<RwLock<HashMap<String, AgentConfig>>>,
    /// Configuration watchers
    config_watchers: Arc<RwLock<HashMap<String, Vec<ConfigWatcher>>>>,
    /// Default configurations for agent types
    default_configs: Arc<RwLock<HashMap<String, AgentConfig>>>,
    /// Configuration validation rules
    validators: Arc<RwLock<Vec<Arc<dyn ConfigValidator>>>>,
    /// Hot-reload configuration
    hot_reload_enabled: bool,
    /// Reload check interval
    reload_interval: Duration,
}

impl ConfigManager {
    /// Create new configuration manager
    pub fn new(store: Arc<dyn ConfigStore>) -> Self {
        Self {
            config_store: store,
            config_cache: Arc::new(RwLock::new(HashMap::new())),
            config_watchers: Arc::new(RwLock::new(HashMap::new())),
            default_configs: Arc::new(RwLock::new(HashMap::new())),
            validators: Arc::new(RwLock::new(Vec::new())),
            hot_reload_enabled: false,
            reload_interval: Duration::from_secs(30),
        }
    }

    /// Enable hot-reloading with specified interval
    pub fn with_hot_reload(mut self, interval: Duration) -> Self {
        self.hot_reload_enabled = true;
        self.reload_interval = interval;
        self
    }

    /// Add configuration validator
    pub async fn add_validator(&self, validator: Arc<dyn ConfigValidator>) {
        self.validators.write().await.push(validator);
    }

    /// Get agent configuration with caching
    pub async fn get_agent_config(&self, agent_id: &str) -> Result<AgentConfig, ConfigError> {
        // Try cache first
        {
            let cache = self.config_cache.read().await;
            if let Some(config) = cache.get(agent_id) {
                return Ok(config.clone());
            }
        }

        // Load from store
        let config = match self.config_store.load_config(agent_id).await {
            Ok(config) => config,
            Err(ConfigError::NotFound(_)) => {
                // Use default configuration if available
                let defaults = self.default_configs.read().await;
                if let Some(default_config) = defaults.get(agent_id) {
                    default_config.clone()
                } else {
                    return Err(ConfigError::NotFound(format!("No configuration found for agent: {}", agent_id)));
                }
            }
            Err(e) => return Err(e),
        };

        // Validate configuration
        self.validate_config(&config).await?;

        // Cache configuration
        {
            let mut cache = self.config_cache.write().await;
            cache.insert(agent_id.to_string(), config.clone());
        }

        Ok(config)
    }

    /// Update agent configuration
    pub async fn update_agent_config(&self, agent_id: &str, config: AgentConfig) -> Result<(), ConfigError> {
        // Validate new configuration
        self.validate_config(&config).await?;

        // Save to store
        self.config_store.save_config(agent_id, &config).await?;

        // Update cache
        {
            let mut cache = self.config_cache.write().await;
            cache.insert(agent_id.to_string(), config.clone());
        }

        // Notify watchers
        self.notify_config_watchers(agent_id, &config).await;

        info!("Updated configuration for agent: {}", agent_id);
        Ok(())
    }

    /// Set default configuration for agent type
    pub async fn set_default_config(&self, agent_type: &str, config: AgentConfig) {
        let mut defaults = self.default_configs.write().await;
        defaults.insert(agent_type.to_string(), config);
    }

    /// Watch configuration changes for an agent
    pub async fn watch_config(&self, agent_id: &str, callback: ConfigWatcher) -> Result<(), ConfigError> {
        let mut watchers = self.config_watchers.write().await;
        watchers.entry(agent_id.to_string()).or_insert_with(Vec::new).push(callback);

        debug!("Added configuration watcher for agent: {}", agent_id);
        Ok(())
    }

    /// Reload all configurations from store
    pub async fn reload_all_configs(&self) -> Result<(), ConfigError> {
        let agent_ids = self.config_store.list_configs().await?;

        for agent_id in &agent_ids {
            if let Err(e) = self.reload_agent_config(agent_id).await {
                warn!("Failed to reload config for agent {}: {}", agent_id, e);
            }
        }

        info!("Reloaded {} configurations", agent_ids.len());
        Ok(())
    }

    /// Reload specific agent configuration
    pub async fn reload_agent_config(&self, agent_id: &str) -> Result<(), ConfigError> {
        let new_config = self.config_store.load_config(agent_id).await?;

        // Validate new configuration
        self.validate_config(&new_config).await?;

        // Check if configuration actually changed
        let changed = {
            let cache = self.config_cache.read().await;
            match cache.get(agent_id) {
                Some(old_config) => {
                    // Simple comparison - in production, you might want a more sophisticated diff
                    serde_json::to_string(old_config)? != serde_json::to_string(&new_config)?
                }
                None => true,
            }
        };

        if changed {
            // Update cache
            {
                let mut cache = self.config_cache.write().await;
                cache.insert(agent_id.to_string(), new_config.clone());
            }

            // Notify watchers
            self.notify_config_watchers(agent_id, &new_config).await;

            debug!("Reloaded configuration for agent: {}", agent_id);
        }

        Ok(())
    }

    /// Start hot-reload background task
    pub fn start_hot_reload(&self) -> mpsc::UnboundedReceiver<String> {
        let (tx, rx) = mpsc::unbounded_channel();

        if self.hot_reload_enabled {
            let config_manager = Arc::new(self.clone());
            let reload_interval = self.reload_interval;

            tokio::spawn(async move {
                let mut interval = tokio::time::interval(reload_interval);

                loop {
                    interval.tick().await;

                    if let Err(e) = config_manager.reload_all_configs().await {
                        error!("Hot-reload failed: {}", e);
                        let _ = tx.send(format!("Hot-reload error: {}", e));
                    }
                }
            });
        }

        rx
    }

    /// Validate configuration using registered validators
    async fn validate_config(&self, config: &AgentConfig) -> Result<(), ConfigError> {
        let validators = self.validators.read().await;

        for validator in validators.iter() {
            validator.validate(config).await?;
        }

        Ok(())
    }

    /// Notify all watchers of configuration changes
    async fn notify_config_watchers(&self, agent_id: &str, config: &AgentConfig) {
        let watchers = self.config_watchers.read().await;

        if let Some(agent_watchers) = watchers.get(agent_id) {
            for watcher in agent_watchers {
                if let Err(e) = watcher(agent_id, config) {
                    warn!("Configuration watcher failed for agent {}: {}", agent_id, e);
                }
            }
        }
    }
}

// Implement Clone manually to work around Arc limitations
impl Clone for ConfigManager {
    fn clone(&self) -> Self {
        Self {
            config_store: Arc::clone(&self.config_store),
            config_cache: Arc::clone(&self.config_cache),
            config_watchers: Arc::clone(&self.config_watchers),
            default_configs: Arc::clone(&self.default_configs),
            validators: Arc::clone(&self.validators),
            hot_reload_enabled: self.hot_reload_enabled,
            reload_interval: self.reload_interval,
        }
    }
}

/// Configuration validator trait
#[async_trait]
pub trait ConfigValidator: Send + Sync {
    async fn validate(&self, config: &AgentConfig) -> Result<(), ConfigError>;
}

/// Basic configuration validator
#[derive(Debug)]
pub struct BasicConfigValidator;

#[async_trait]
impl ConfigValidator for BasicConfigValidator {
    async fn validate(&self, config: &AgentConfig) -> Result<(), ConfigError> {
        if config.id.is_empty() {
            return Err(ConfigError::ValidationFailed("Agent ID cannot be empty".to_string()));
        }

        if config.name.is_empty() {
            return Err(ConfigError::ValidationFailed("Agent name cannot be empty".to_string()));
        }

        if config.max_concurrent_tasks == 0 {
            return Err(ConfigError::ValidationFailed("Max concurrent tasks must be greater than 0".to_string()));
        }

        if config.timeout.as_secs() == 0 {
            return Err(ConfigError::ValidationFailed("Timeout must be greater than 0".to_string()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use uuid::Uuid;

    async fn create_test_config_manager() -> (ConfigManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let store = Arc::new(FileConfigStore::new(temp_dir.path().to_path_buf()));
        let manager = ConfigManager::new(store);
        (manager, temp_dir)
    }

    fn create_test_config() -> AgentConfig {
        AgentConfig {
            id: Uuid::new_v4().to_string(),
            name: "test-agent".to_string(),
            enabled: true,
            priority: 5,
            max_concurrent_tasks: 10,
            timeout: Duration::from_secs(30),
            retry_count: 3,
            custom_settings: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_config_store_operations() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileConfigStore::new(temp_dir.path().to_path_buf());
        let config = create_test_config();
        let agent_id = &config.id;

        // Test save and load
        store.save_config(agent_id, &config).await.unwrap();
        let loaded_config = store.load_config(agent_id).await.unwrap();
        assert_eq!(config.id, loaded_config.id);
        assert_eq!(config.name, loaded_config.name);

        // Test exists
        assert!(store.exists(agent_id).await.unwrap());
        assert!(!store.exists("nonexistent").await.unwrap());

        // Test list configs
        let configs = store.list_configs().await.unwrap();
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0], config.id);

        // Test delete
        store.delete_config(agent_id).await.unwrap();
        assert!(!store.exists(agent_id).await.unwrap());
    }

    #[tokio::test]
    async fn test_config_manager_caching() {
        let (manager, _temp_dir) = create_test_config_manager().await;
        let config = create_test_config();
        let agent_id = &config.id;

        // Update config (should be cached)
        manager.update_agent_config(agent_id, config.clone()).await.unwrap();

        // Get config (should come from cache)
        let loaded_config = manager.get_agent_config(agent_id).await.unwrap();
        assert_eq!(config.id, loaded_config.id);
    }

    #[tokio::test]
    async fn test_config_validation() {
        let (manager, _temp_dir) = create_test_config_manager().await;
        manager.add_validator(Arc::new(BasicConfigValidator)).await;

        let mut invalid_config = create_test_config();
        invalid_config.id = String::new(); // Invalid: empty ID

        let result = manager.update_agent_config("test", invalid_config).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigError::ValidationFailed(_)));
    }

    #[tokio::test]
    async fn test_default_config() {
        let (manager, _temp_dir) = create_test_config_manager().await;
        let default_config = create_test_config();

        manager.set_default_config("test-agent", default_config.clone()).await;

        // Should get default config for non-existent agent of this type
        let config = manager.get_agent_config("test-agent").await.unwrap();
        assert_eq!(config.name, default_config.name);
    }

    #[tokio::test]
    async fn test_config_watchers() {
        let (manager, _temp_dir) = create_test_config_manager().await;
        let config = create_test_config();
        let agent_id = &config.id;

        let (tx, mut rx) = mpsc::unbounded_channel();
        let watcher = Arc::new(move |agent_id: &str, _config: &AgentConfig| {
            tx.send(agent_id.to_string()).unwrap();
            Ok(())
        });

        manager.watch_config(agent_id, watcher).await.unwrap();
        manager.update_agent_config(agent_id, config).await.unwrap();

        // Should receive notification
        let notified_agent_id = rx.recv().await.unwrap();
        assert_eq!(notified_agent_id, *agent_id);
    }
}
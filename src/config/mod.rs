//! Configuration management module
//!
//! Provides comprehensive configuration management with:
//! - TOML-based configuration files
//! - Environment variable overrides
//! - Hot reloading support
//! - Validation and migration

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info};

use crate::error::{AppError, AppResult};

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Application settings
    pub app: AppConfig,
    /// Git-related settings
    pub git: GitConfig,
    /// UI configuration
    pub ui: UIConfig,
    /// Agent system configuration
    pub agents: AgentConfig,
    /// MCP protocol configuration
    pub mcp: McpConfig,
    /// Performance monitoring configuration
    pub performance: PerformanceConfig,
}

impl Config {
    /// Load configuration from default locations
    ///
    /// Search order:
    /// 1. ./ai-commit.toml
    /// 2. ~/.config/ai-commit/config.toml
    /// 3. Default configuration
    pub async fn load() -> AppResult<Self> {
        info!("Loading application configuration");

        // Try current directory first
        if let Ok(config) = Self::load_from_file("./ai-commit.toml").await {
            info!("Loaded configuration from ./ai-commit.toml");
            return Ok(config);
        }

        // Try user config directory
        if let Some(config_path) = Self::get_user_config_path() {
            if let Ok(config) = Self::load_from_file(&config_path).await {
                info!("Loaded configuration from {}", config_path.display());
                return Ok(config);
            }
        }

        // Use default configuration
        info!("Using default configuration");
        Ok(Self::default())
    }

    /// Load configuration from a specific file
    pub async fn load_from_file<P: AsRef<Path>>(path: P) -> AppResult<Self> {
        let path = path.as_ref();
        debug!("Loading configuration from: {}", path.display());

        let content = fs::read_to_string(path)
            .await
            .map_err(|e| AppError::Io(e))?;

        let config: Config = toml::from_str(&content)
            .map_err(|e| AppError::application(&format!("Failed to parse config file: {}", e)))?;

        config.validate()?;

        Ok(config)
    }

    /// Save configuration to a file
    pub async fn save_to_file<P: AsRef<Path>>(&self, path: P) -> AppResult<()> {
        let path = path.as_ref();
        debug!("Saving configuration to: {}", path.display());

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| AppError::Io(e))?;
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| AppError::application(&format!("Failed to serialize config: {}", e)))?;

        fs::write(path, content)
            .await
            .map_err(|e| AppError::Io(e))?;

        info!("Configuration saved to: {}", path.display());
        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> AppResult<()> {
        debug!("Validating configuration");

        // Validate performance thresholds
        if self.performance.startup_timeout_ms == 0 {
            return Err(AppError::application(
                "startup_timeout_ms must be greater than 0"
            ));
        }

        if self.performance.git_status_timeout_ms == 0 {
            return Err(AppError::application(
                "git_status_timeout_ms must be greater than 0"
            ));
        }

        // Validate UI settings
        if self.ui.sidebar_width < 10 || self.ui.sidebar_width > 100 {
            return Err(AppError::application(
                "sidebar_width must be between 10 and 100"
            ));
        }

        debug!("Configuration validation passed");
        Ok(())
    }

    /// Get user configuration directory path
    fn get_user_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|mut path| {
            path.push("ai-commit");
            path.push("config.toml");
            path
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            app: AppConfig::default(),
            git: GitConfig::default(),
            ui: UIConfig::default(),
            agents: AgentConfig::default(),
            mcp: McpConfig::default(),
            performance: PerformanceConfig::default(),
        }
    }
}

/// Application-level configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Application name
    pub name: String,
    /// Application version
    pub version: String,
    /// Debug mode
    pub debug: bool,
    /// Log level
    pub log_level: String,
    /// Auto-save interval in seconds
    pub auto_save_interval: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            name: "AI-Commit TUI".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            debug: cfg!(debug_assertions),
            log_level: if cfg!(debug_assertions) {
                "debug"
            } else {
                "info"
            }
            .to_string(),
            auto_save_interval: 300, // 5 minutes
        }
    }
}

/// Git-related configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitConfig {
    /// Default branch name for new repositories
    pub default_branch: String,
    /// Enable status caching
    pub enable_status_cache: bool,
    /// Status cache TTL in seconds
    pub status_cache_ttl: u64,
    /// Maximum files to process in status operations
    pub max_files: usize,
    /// Enable submodule support
    pub enable_submodules: bool,
    /// Enable LFS support
    pub enable_lfs: bool,
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            default_branch: "main".to_string(),
            enable_status_cache: true,
            status_cache_ttl: 30,
            max_files: 10000,
            enable_submodules: true,
            enable_lfs: true,
        }
    }
}

/// UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIConfig {
    /// Theme name
    pub theme: String,
    /// Sidebar width in characters
    pub sidebar_width: u16,
    /// Show sidebar by default
    pub show_sidebar: bool,
    /// Default tab to show on startup
    pub default_tab: String,
    /// Enable mouse support
    pub enable_mouse: bool,
    /// Refresh rate in milliseconds (for animations)
    pub refresh_rate_ms: u64,
    /// Enable syntax highlighting
    pub enable_syntax_highlighting: bool,
}

impl Default for UIConfig {
    fn default() -> Self {
        Self {
            theme: "default".to_string(),
            sidebar_width: 25,
            show_sidebar: true,
            default_tab: "Status".to_string(),
            enable_mouse: true,
            refresh_rate_ms: 100,
            enable_syntax_highlighting: true,
        }
    }
}

/// Agent system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Enable agent system
    pub enabled: bool,
    /// Maximum concurrent agents
    pub max_concurrent_agents: usize,
    /// Agent task timeout in seconds
    pub task_timeout_seconds: u64,
    /// Health check interval in seconds
    pub health_check_interval: u64,
    /// Enable agent metrics collection
    pub enable_metrics: bool,
    /// Agent communication timeout in milliseconds
    pub communication_timeout_ms: u64,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_concurrent_agents: 10,
            task_timeout_seconds: 30,
            health_check_interval: 30,
            enable_metrics: true,
            communication_timeout_ms: 5000,
        }
    }
}

/// MCP protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    /// Enable MCP protocol support
    pub enabled: bool,
    /// Maximum concurrent MCP connections
    pub max_connections: usize,
    /// Connection timeout in seconds
    pub connection_timeout_seconds: u64,
    /// Message timeout in milliseconds
    pub message_timeout_ms: u64,
    /// Enable protocol compression
    pub enable_compression: bool,
    /// MCP server configurations
    pub servers: Vec<McpServerConfig>,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_connections: 5,
            connection_timeout_seconds: 10,
            message_timeout_ms: 5000,
            enable_compression: true,
            servers: Vec::new(),
        }
    }
}

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Server name/identifier
    pub name: String,
    /// Server URL or command
    pub url: String,
    /// Transport type (websocket, http, stdio)
    pub transport: String,
    /// Enable this server
    pub enabled: bool,
    /// Authentication token (optional)
    pub auth_token: Option<String>,
    /// Server-specific settings
    pub settings: toml::Value,
}

/// Performance monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Enable performance monitoring
    pub enabled: bool,
    /// Application startup timeout in milliseconds
    pub startup_timeout_ms: u64,
    /// Git status operation timeout in milliseconds
    pub git_status_timeout_ms: u64,
    /// Agent initialization timeout in milliseconds
    pub agent_init_timeout_ms: u64,
    /// MCP communication timeout in milliseconds
    pub mcp_timeout_ms: u64,
    /// Memory usage warning threshold in MB
    pub memory_warning_mb: u64,
    /// Memory usage critical threshold in MB
    pub memory_critical_mb: u64,
    /// Enable performance warnings
    pub enable_warnings: bool,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            startup_timeout_ms: 1000,
            git_status_timeout_ms: 200,
            agent_init_timeout_ms: 500,
            mcp_timeout_ms: 100,
            memory_warning_mb: 150,
            memory_critical_mb: 600,
            enable_warnings: true,
        }
    }
}

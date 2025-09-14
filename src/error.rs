//! Error handling for AI-Commit TUI
//!
//! Provides a comprehensive error handling system following Rust best practices
//! with thiserror for error definitions and anyhow for error propagation.

use thiserror::Error;

/// Application result type alias
pub type AppResult<T> = std::result::Result<T, AppError>;

/// Main application error enum
///
/// Covers all major error categories in the application with structured
/// error information for debugging and user feedback.
#[derive(Error, Debug)]
pub enum AppError {
    /// Git repository operation errors
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    /// I/O operation errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration and serialization errors
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    /// Serialization/deserialization errors
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// HTTP client errors for AI services and MCP
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Terminal/UI operation errors
    #[error("Terminal error: {0}")]
    Terminal(String),

    /// Agent system errors
    #[error("Agent error: {message}")]
    Agent { message: String },

    /// MCP protocol errors
    #[error("MCP protocol error: {message}")]
    Mcp { message: String },

    /// Performance monitoring errors
    #[error("Performance error: {message}")]
    Performance { message: String },

    /// Application state errors
    #[error("State error: {message}")]
    State { message: String },

    /// Generic application errors
    #[error("Application error: {message}")]
    Application { message: String },
}

impl AppError {
    /// Create a new Agent error
    pub fn agent<S: Into<String>>(message: S) -> Self {
        Self::Agent {
            message: message.into(),
        }
    }

    /// Create a new MCP error
    pub fn mcp<S: Into<String>>(message: S) -> Self {
        Self::Mcp {
            message: message.into(),
        }
    }

    /// Create a new Performance error
    pub fn performance<S: Into<String>>(message: S) -> Self {
        Self::Performance {
            message: message.into(),
        }
    }

    /// Create a new State error
    pub fn state<S: Into<String>>(message: S) -> Self {
        Self::State {
            message: message.into(),
        }
    }

    /// Create a new Application error
    pub fn application<S: Into<String>>(message: S) -> Self {
        Self::Application {
            message: message.into(),
        }
    }

    /// Check if the error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            AppError::Git(_) => true,
            AppError::Http(_) => true,
            AppError::Agent { .. } => true,
            AppError::Mcp { .. } => true,
            AppError::Performance { .. } => true,
            AppError::Io(_) => false,
            AppError::Terminal(_) => false,
            AppError::Config(_) => false,
            AppError::Serde(_) => false,
            AppError::State { .. } => true,
            AppError::Application { .. } => true,
        }
    }

    /// Get error severity level
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            AppError::Git(_) => ErrorSeverity::Medium,
            AppError::Io(_) => ErrorSeverity::High,
            AppError::Config(_) => ErrorSeverity::High,
            AppError::Serde(_) => ErrorSeverity::Medium,
            AppError::Http(_) => ErrorSeverity::Medium,
            AppError::Terminal(_) => ErrorSeverity::High,
            AppError::Agent { .. } => ErrorSeverity::Medium,
            AppError::Mcp { .. } => ErrorSeverity::Medium,
            AppError::Performance { .. } => ErrorSeverity::Low,
            AppError::State { .. } => ErrorSeverity::Medium,
            AppError::Application { .. } => ErrorSeverity::Medium,
        }
    }
}

/// Error severity levels for monitoring and alerting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl ErrorSeverity {
    /// Convert severity to string for logging
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorSeverity::Low => "LOW",
            ErrorSeverity::Medium => "MEDIUM",
            ErrorSeverity::High => "HIGH",
            ErrorSeverity::Critical => "CRITICAL",
        }
    }
}

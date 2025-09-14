//! AI-Commit TUI - Intelligent Git commit tool with Agent architecture
//!
//! This library provides a Terminal User Interface for Git operations with
//! integrated AI agents and Model Context Protocol (MCP) support.
//!
//! # Architecture
//!
//! The application follows a layered architecture:
//! - **Presentation Layer**: TUI components built with ratatui
//! - **Application Layer**: State management and event handling
//! - **Domain Layer**: Git operations, Agent system, MCP protocol
//! - **Infrastructure Layer**: External services and system interfaces

pub mod ai;
pub mod app;
pub mod config;
pub mod error;
pub mod git;
pub mod ui;

pub use app::App;
pub use error::{AppError, AppResult};

use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize the logging system with structured logging
///
/// Performance requirement: Initialization < 50ms
///
/// # Features
/// - Structured JSON logging in production
/// - Human-readable logs in development
/// - Performance tracing support
/// - Configurable log levels via RUST_LOG environment variable
pub fn initialize_logging() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ai_commit=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    Ok(())
}

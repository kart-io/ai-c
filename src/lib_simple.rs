//! Simple library interface for basic testing

pub mod error;
pub mod config;

pub use error::{AppError, AppResult};
pub use config::Config;

use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize the logging system with structured logging
pub fn initialize_logging() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ai_commit=info,tower_http=debug".into())
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    Ok(())
}
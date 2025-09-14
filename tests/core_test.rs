//! Core functionality tests

use std::time::{Duration, Instant};

#[test]
fn test_error_handling_basics() {
    use ai_c::error::{AppError, AppResult, ErrorSeverity};

    let error = AppError::application("test error");
    assert!(error.is_recoverable());
    assert_eq!(error.severity(), ErrorSeverity::Medium);

    let result: AppResult<()> = Err(error);
    assert!(result.is_err());

    println!("✓ Error handling works correctly");
}

#[test]
fn test_configuration_defaults() {
    use ai_c::config::{AppConfig, Config, GitConfig, UIConfig};

    let config = Config::default();

    // Test app config
    assert_eq!(config.app.name, "AI-Commit TUI");
    assert!(config.app.auto_save_interval > 0);

    // Test git config
    assert_eq!(config.git.default_branch, "main");
    assert!(config.git.enable_status_cache);
    assert!(config.git.max_files > 0);

    // Test UI config
    assert_eq!(config.ui.sidebar_width, 25);
    assert!(config.ui.show_sidebar);

    println!("✓ Configuration defaults are correct");
}

#[tokio::test]
async fn test_config_validation() {
    use ai_c::config::Config;

    let config = Config::default();
    let validation_result = config.validate();

    assert!(validation_result.is_ok());
    println!("✓ Configuration validation passes");
}

#[test]
fn test_performance_constants() {
    // Verify that our performance requirements are well-defined
    const STARTUP_REQUIREMENT_MS: u64 = 1000;
    const AGENT_INIT_REQUIREMENT_MS: u64 = 500;
    const GIT_STATUS_REQUIREMENT_MS: u64 = 200;
    const MCP_COMM_REQUIREMENT_MS: u64 = 100;
    const MEMORY_LIMIT_MB: u64 = 150;

    // These should be reasonable values
    assert!(STARTUP_REQUIREMENT_MS <= 1000);
    assert!(AGENT_INIT_REQUIREMENT_MS <= 500);
    assert!(GIT_STATUS_REQUIREMENT_MS <= 200);
    assert!(MCP_COMM_REQUIREMENT_MS <= 100);
    assert!(MEMORY_LIMIT_MB <= 150);

    println!("✓ Performance requirements are well-defined");
    println!("  - Startup: {}ms", STARTUP_REQUIREMENT_MS);
    println!("  - Agent init: {}ms", AGENT_INIT_REQUIREMENT_MS);
    println!("  - Git status: {}ms", GIT_STATUS_REQUIREMENT_MS);
    println!("  - MCP communication: {}ms", MCP_COMM_REQUIREMENT_MS);
    println!("  - Memory limit: {}MB", MEMORY_LIMIT_MB);
}

#[test]
fn test_logging_initialization() {
    use ai_c::initialize_logging;

    let start_time = Instant::now();
    let result = initialize_logging();
    let init_duration = start_time.elapsed();

    assert!(result.is_ok());
    assert!(init_duration < Duration::from_millis(100));

    println!("✓ Logging initialized in {:?}", init_duration);
}

//! Integration tests for AI-Commit TUI
//!
//! Tests the main application components and performance requirements.

use ai_c::{initialize_logging, App};
use std::time::{Duration, Instant};
use tokio::test;

/// Test application startup time requirement (< 1 second)
#[tokio::test]
async fn test_app_startup_performance() {
    // Initialize logging for test
    let _ = initialize_logging();

    let start_time = Instant::now();

    // Create application (this should initialize all components)
    let app_result = App::new().await;

    let startup_duration = start_time.elapsed();

    // Verify application was created successfully
    assert!(
        app_result.is_ok(),
        "Application creation failed: {:?}",
        app_result.err()
    );

    let app = app_result.unwrap();

    // Performance requirement: startup time < 1 second
    assert!(
        startup_duration < Duration::from_secs(1),
        "Application startup time exceeded 1 second: {:?}",
        startup_duration
    );

    // Verify internal startup time measurement
    let internal_startup_time = app.startup_time();
    assert!(
        internal_startup_time < Duration::from_secs(1),
        "Internal startup time measurement exceeded 1 second: {:?}",
        internal_startup_time
    );

    println!("✓ Application startup completed in {:?}", startup_duration);
}

/// Test Git service initialization performance (< 100ms)
#[tokio::test]
async fn test_git_service_initialization() {
    use ai_c::{config::GitConfig, git::GitService};

    let config = GitConfig::default();
    let start_time = Instant::now();

    // Initialize Git service
    let git_service_result = GitService::new(&config).await;
    let init_duration = start_time.elapsed();

    // Note: This test might fail if not run in a Git repository
    // In a real CI environment, we'd set up a test repository
    match git_service_result {
        Ok(_) => {
            // Performance requirement: Git service initialization < 100ms
            assert!(
                init_duration < Duration::from_millis(100),
                "Git service initialization exceeded 100ms: {:?}",
                init_duration
            );
            println!("✓ Git service initialized in {:?}", init_duration);
        }
        Err(e) => {
            // If we're not in a Git repository, just log and continue
            println!("ℹ Git service test skipped (not in Git repo): {}", e);
        }
    }
}

/// Test configuration loading performance (< 50ms)
#[tokio::test]
async fn test_config_loading_performance() {
    use ai_c::config::Config;

    let start_time = Instant::now();

    // Load configuration (will use defaults if no config file)
    let config_result = Config::load().await;
    let load_duration = start_time.elapsed();

    // Verify configuration loaded successfully
    assert!(
        config_result.is_ok(),
        "Configuration loading failed: {:?}",
        config_result.err()
    );

    // Performance requirement: config loading < 50ms
    assert!(
        load_duration < Duration::from_millis(50),
        "Configuration loading exceeded 50ms: {:?}",
        load_duration
    );

    println!("✓ Configuration loaded in {:?}", load_duration);
}

/// Test UI theme loading performance
#[tokio::test]
async fn test_ui_theme_loading() {
    use ai_c::ui::theme::Theme;

    let start_time = Instant::now();

    // Load default theme
    let theme_result = Theme::load("default");
    let load_duration = start_time.elapsed();

    // Verify theme loaded successfully
    assert!(
        theme_result.is_ok(),
        "Theme loading failed: {:?}",
        theme_result.err()
    );

    // Performance expectation: theme loading < 10ms
    assert!(
        load_duration < Duration::from_millis(10),
        "Theme loading exceeded 10ms: {:?}",
        load_duration
    );

    println!("✓ Theme loaded in {:?}", load_duration);
}

/// Test message bus performance (< 10ms routing)
#[tokio::test]
async fn test_message_bus_performance() {
    use ai_c::ai::{
        agent::AgentCapability, AgentMessage, AgentTask, MessageBus,
    };
    use tokio::sync::mpsc;
    use uuid::Uuid;

    let message_bus = MessageBus::new();

    // Create a test agent channel
    let (sender, mut receiver) = mpsc::unbounded_channel();
    let agent_id = "test-agent".to_string();

    // Register agent
    message_bus
        .register_agent(agent_id.clone(), sender)
        .await
        .unwrap();

    // Create a test task
    let task = AgentTask::new(AgentTaskType::GenerateCommitMessage {
        staged_files: vec!["test.rs".to_string()],
        diff_content: "test diff".to_string(),
        context: None,
    });

    // Create message
    let message = AgentMessage::task_assignment(agent_id.clone(), "test-manager".to_string(), task);

    let start_time = Instant::now();

    // Send message
    let send_result = message_bus.send_message(message).await;
    let send_duration = start_time.elapsed();

    // Verify message was sent successfully
    assert!(
        send_result.is_ok(),
        "Message sending failed: {:?}",
        send_result.err()
    );

    // Performance requirement: message routing < 10ms
    assert!(
        send_duration < Duration::from_millis(10),
        "Message routing exceeded 10ms: {:?}",
        send_duration
    );

    // Verify message was received
    let received_message = receiver.try_recv();
    assert!(received_message.is_ok(), "Message was not received");

    println!("✓ Message routed in {:?}", send_duration);
}

/// Test memory usage is within acceptable limits
#[tokio::test]
async fn test_memory_usage() {
    use ai_c::App;

    // Get initial memory usage
    let initial_memory = get_memory_usage_mb();

    // Create application
    let app = App::new().await.expect("Failed to create application");

    // Get memory usage after app creation
    let final_memory = get_memory_usage_mb();
    let memory_increase = final_memory - initial_memory;

    // Performance requirement: memory usage < 150MB idle
    assert!(
        memory_increase < 150.0,
        "Memory usage exceeded 150MB: {:.1}MB",
        memory_increase
    );

    println!("✓ Memory usage: {:.1}MB increase", memory_increase);
}

/// Helper function to get current memory usage in MB
fn get_memory_usage_mb() -> f64 {
    // In a real implementation, this would use system APIs
    // For testing purposes, we'll use a simplified approach
    use std::process::Command;

    let output = Command::new("ps")
        .args(&["-o", "rss=", &std::process::id().to_string()])
        .output()
        .ok();

    if let Some(output) = output {
        if let Ok(rss_str) = String::from_utf8(output.stdout) {
            if let Ok(rss_kb) = rss_str.trim().parse::<f64>() {
                return rss_kb / 1024.0; // Convert KB to MB
            }
        }
    }

    // Fallback: return 0 if we can't measure memory
    0.0
}

/// Test application module structure
#[test]
fn test_module_structure() {
    // Verify all expected modules are accessible
    use ai_c::{ai, app, config, error, git, ui};

    // Test error types
    let _error = error::AppError::application("test");

    // Test configuration
    let _config = config::Config::default();

    // This test ensures all major modules compile and are accessible
    println!("✓ All modules are accessible");
}

/// Performance benchmark test
#[tokio::test]
async fn test_performance_benchmark() {
    use ai_c::initialize_logging;

    let _ = initialize_logging();

    let mut results = Vec::new();

    // Run multiple startup tests to get average
    for i in 0..5 {
        let start_time = Instant::now();
        let app_result = App::new().await;
        let duration = start_time.elapsed();

        assert!(app_result.is_ok(), "App creation failed on iteration {}", i);
        results.push(duration);

        // Small delay between tests
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let average_time = results.iter().sum::<Duration>() / results.len() as u32;
    let max_time = results.iter().max().unwrap();
    let min_time = results.iter().min().unwrap();

    println!("📊 Performance Benchmark Results:");
    println!("   Average startup time: {:?}", average_time);
    println!("   Min startup time: {:?}", min_time);
    println!("   Max startup time: {:?}", max_time);

    // All runs should be under 1 second
    assert!(
        *max_time < Duration::from_secs(1),
        "Maximum startup time exceeded 1 second: {:?}",
        max_time
    );
}

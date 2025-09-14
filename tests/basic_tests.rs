//! Basic functionality tests for core components

use std::time::{Duration, Instant};

/// Test basic error handling
#[test]
fn test_error_handling() {
    use ai_c::error::{AppError, AppResult};

    let error = AppError::application("test error");
    assert!(error.is_recoverable());

    let result: AppResult<()> = Err(error);
    assert!(result.is_err());

    println!("✓ Error handling works correctly");
}

/// Test configuration defaults
#[test]
fn test_config_defaults() {
    use ai_c::config::Config;

    let config = Config::default();
    assert_eq!(config.app.name, "AI-Commit TUI");
    assert!(config.git.enable_status_cache);
    assert_eq!(config.ui.sidebar_width, 25);

    println!("✓ Configuration defaults are correct");
}

/// Test theme loading
#[test]
fn test_theme_loading() {
    use ai_c::ui::theme::Theme;

    let start_time = Instant::now();
    let theme = Theme::load("default").expect("Failed to load default theme");
    let load_duration = start_time.elapsed();

    assert_eq!(theme.name, "default");
    assert!(load_duration < Duration::from_millis(10));

    println!("✓ Theme loaded in {:?}", load_duration);
}

/// Test agent capability matching
#[test]
fn test_agent_capabilities() {
    use ai_c::ai::{AgentCapability, AgentTask};
    use ai_c::ai::agent::AgentTaskType;

    let capability = AgentCapability::CommitMessageGeneration;
    let task = AgentTask::new(AgentTaskType::GenerateCommitMessage {
        staged_files: vec!["test.rs".to_string()],
        diff_content: "test diff".to_string(),
        context: None,
    });

    assert!(capability.matches_task(&task));
    println!("✓ Agent capability matching works");
}

/// Test message bus creation
#[tokio::test]
async fn test_message_bus_creation() {
    use ai_c::ai::MessageBus;

    let start_time = Instant::now();
    let message_bus = MessageBus::new();
    let creation_duration = start_time.elapsed();

    assert!(creation_duration < Duration::from_millis(1));
    println!("✓ Message bus created in {:?}", creation_duration);
}

/// Test file status parsing
#[test]
fn test_git_status_flags() {
    use ai_c::git::GitStatusFlags;

    let flags = GitStatusFlags {
        index_new: true,
        wt_modified: true,
        ..Default::default()
    };

    assert!(flags.is_staged());
    assert!(flags.is_modified());
    assert_eq!(flags.status_char(), 'M');

    println!("✓ Git status flags work correctly");
}

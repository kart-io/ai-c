//! Unit tests for CommitAgent
//!
//! Tests the AI commit message generation functionality

use ai_c::{
    ai::{
        agents::{
            commit_agent::{
                CommitAgent, CommitAgentConfig, CommitGenerationRequest, CommitMessageStyle,
            },
        },
        Agent, // Import the Agent trait
    },
    git::{FileStatus, GitStatusFlags},
};
use chrono::Utc;

/// Create a test CommitAgent
fn create_test_agent() -> CommitAgent {
    let config = CommitAgentConfig::default();
    CommitAgent::new(config, None)
}

/// Create test file status data
fn create_test_files() -> Vec<FileStatus> {
    vec![
        FileStatus {
            path: "src/main.rs".to_string(),
            status: GitStatusFlags {
                index_new: false,
                index_modified: true,
                index_deleted: false,
                index_renamed: false,
                index_typechange: false,
                wt_new: false,
                wt_modified: true,
                wt_deleted: false,
                wt_renamed: false,
                wt_typechange: false,
                ignored: false,
                conflicted: false,
            },
            size: 1024,
            modified: Utc::now(),
            is_binary: false,
        },
        FileStatus {
            path: "src/ui/mod.rs".to_string(),
            status: GitStatusFlags {
                index_new: true,
                index_modified: false,
                index_deleted: false,
                index_renamed: false,
                index_typechange: false,
                wt_new: true,
                wt_modified: false,
                wt_deleted: false,
                wt_renamed: false,
                wt_typechange: false,
                ignored: false,
                conflicted: false,
            },
            size: 2048,
            modified: Utc::now(),
            is_binary: false,
        },
        FileStatus {
            path: "README.md".to_string(),
            status: GitStatusFlags {
                index_new: false,
                index_modified: true,
                index_deleted: false,
                index_renamed: false,
                index_typechange: false,
                wt_new: false,
                wt_modified: true,
                wt_deleted: false,
                wt_renamed: false,
                wt_typechange: false,
                ignored: false,
                conflicted: false,
            },
            size: 512,
            modified: Utc::now(),
            is_binary: false,
        },
    ]
}

#[tokio::test]
async fn test_commit_agent_creation() {
    let agent = create_test_agent();
    assert_eq!(agent.agent_type(), "commit-agent");
    assert!(!agent.capabilities().is_empty());
}

#[tokio::test]
async fn test_conventional_commit_generation() {
    let mut agent = create_test_agent();
    let files = create_test_files();

    let request = CommitGenerationRequest {
        staged_files: files,
        context: Some("Add new UI functionality".to_string()),
        style: CommitMessageStyle::Conventional,
        max_length: Some(72),
        include_description: true,
    };

    let result = agent.generate_commit_message(request).await;
    assert!(result.is_ok(), "Commit generation should succeed");

    let commit_message = result.unwrap();

    // Verify conventional commit format
    assert!(
        commit_message.subject.contains(":"),
        "Conventional commit should have type: format"
    );

    // Verify reasonable commit types
    let valid_types = ["feat", "fix", "docs", "chore", "refactor", "test"];
    let has_valid_type = valid_types.iter().any(|&t| commit_message.subject.starts_with(t));
    assert!(has_valid_type, "Should use valid commit type: {}", commit_message.subject);

    // Verify confidence is reasonable
    assert!(
        commit_message.confidence >= 0.0 && commit_message.confidence <= 1.0,
        "Confidence should be between 0 and 1"
    );

    // Verify alternatives provided
    assert!(!commit_message.alternatives.is_empty(), "Should provide alternatives");

    // Verify metadata
    assert_eq!(commit_message.metadata.files_count, 3, "Should count files correctly");
}

#[tokio::test]
async fn test_simple_commit_generation() {
    let mut agent = create_test_agent();
    let files = create_test_files();

    let request = CommitGenerationRequest {
        staged_files: files,
        context: None,
        style: CommitMessageStyle::Simple,
        max_length: Some(50),
        include_description: false,
    };

    let result = agent.generate_commit_message(request).await;
    assert!(result.is_ok(), "Simple commit generation should succeed");

    let commit_message = result.unwrap();

    // Verify simple format (no colons for type)
    assert!(
        commit_message.subject.len() <= 50,
        "Subject should respect max length"
    );

    // Should not have description for simple style when not requested
    assert!(commit_message.description.is_none(), "Simple style should not have description when not requested");
}

#[tokio::test]
async fn test_detailed_commit_generation() {
    let mut agent = create_test_agent();
    let files = create_test_files();

    let request = CommitGenerationRequest {
        staged_files: files,
        context: Some("Comprehensive UI overhaul".to_string()),
        style: CommitMessageStyle::Detailed,
        max_length: None,
        include_description: true,
    };

    let result = agent.generate_commit_message(request).await;
    assert!(result.is_ok(), "Detailed commit generation should succeed");

    let commit_message = result.unwrap();

    // Detailed should have description
    assert!(
        commit_message.description.is_some(),
        "Detailed style should include description"
    );

    let description = commit_message.description.unwrap();
    assert!(
        description.contains("•") || description.contains("-"),
        "Detailed description should have bullet points"
    );
}

#[tokio::test]
async fn test_custom_commit_generation() {
    let mut agent = create_test_agent();
    let files = create_test_files();

    let request = CommitGenerationRequest {
        staged_files: files,
        context: None,
        style: CommitMessageStyle::Custom("[{type}] {description}".to_string()),
        max_length: None,
        include_description: false,
    };

    let result = agent.generate_commit_message(request).await;
    assert!(result.is_ok(), "Custom commit generation should succeed");

    let commit_message = result.unwrap();

    // Custom format should be applied
    assert!(
        commit_message.subject.starts_with('[') && commit_message.subject.contains(']'),
        "Custom template should be applied: {}",
        commit_message.subject
    );
}

#[tokio::test]
async fn test_file_categorization() {
    let mut agent = create_test_agent();

    let files = vec![
        FileStatus {
            path: "src/main.rs".to_string(),
            status: GitStatusFlags::default(),
            size: 1024,
            modified: Utc::now(),
            is_binary: false,
        },
        FileStatus {
            path: "tests/unit_tests.rs".to_string(),
            status: GitStatusFlags::default(),
            size: 512,
            modified: Utc::now(),
            is_binary: false,
        },
        FileStatus {
            path: "README.md".to_string(),
            status: GitStatusFlags::default(),
            size: 256,
            modified: Utc::now(),
            is_binary: false,
        },
        FileStatus {
            path: "Cargo.toml".to_string(),
            status: GitStatusFlags::default(),
            size: 128,
            modified: Utc::now(),
            is_binary: false,
        },
    ];

    let request = CommitGenerationRequest {
        staged_files: files,
        context: None,
        style: CommitMessageStyle::Conventional,
        max_length: None,
        include_description: true,
    };

    let result = agent.generate_commit_message(request).await;
    assert!(result.is_ok(), "Should handle diverse file types");

    let commit_message = result.unwrap();

    // Should detect different file types and generate appropriate message
    assert!(!commit_message.subject.is_empty(), "Should generate subject");
    assert_eq!(commit_message.metadata.files_count, 4, "Should count all files");
}

#[tokio::test]
async fn test_empty_files_handling() {
    let mut agent = create_test_agent();

    let request = CommitGenerationRequest {
        staged_files: Vec::new(),
        context: None,
        style: CommitMessageStyle::Simple,
        max_length: None,
        include_description: false,
    };

    let result = agent.generate_commit_message(request).await;
    assert!(result.is_ok(), "Should handle empty files list");

    let commit_message = result.unwrap();
    assert_eq!(commit_message.metadata.files_count, 0, "Should show zero files");
}

#[tokio::test]
async fn test_subject_truncation() {
    let mut agent = create_test_agent();
    let files = create_test_files();

    let request = CommitGenerationRequest {
        staged_files: files,
        context: Some("This is a very long context that should cause the commit message to be quite lengthy and potentially exceed the maximum length limit".to_string()),
        style: CommitMessageStyle::Simple,
        max_length: Some(20), // Very short limit
        include_description: false,
    };

    let result = agent.generate_commit_message(request).await;
    assert!(result.is_ok(), "Should handle length limits");

    let commit_message = result.unwrap();
    assert!(
        commit_message.subject.len() <= 23, // 20 + "..." = 23
        "Subject should be truncated: '{}' (length: {})",
        commit_message.subject,
        commit_message.subject.len()
    );
}

#[tokio::test]
async fn test_confidence_calculation() {
    let mut agent = create_test_agent();

    // Test with many files (should increase confidence)
    let many_files: Vec<FileStatus> = (0..10)
        .map(|i| FileStatus {
            path: format!("src/file_{}.rs", i),
            status: GitStatusFlags::default(),
            size: 1024,
            modified: Utc::now(),
            is_binary: false,
        })
        .collect();

    let request = CommitGenerationRequest {
        staged_files: many_files,
        context: Some("Clear context provided".to_string()),
        style: CommitMessageStyle::Conventional,
        max_length: None,
        include_description: true,
    };

    let result = agent.generate_commit_message(request).await;
    assert!(result.is_ok(), "Should handle many files");

    let commit_message = result.unwrap();

    // More files should lead to higher confidence
    assert!(
        commit_message.confidence > 0.7,
        "Many files should increase confidence: {}",
        commit_message.confidence
    );
}

#[tokio::test]
async fn test_scope_detection() {
    let mut agent = create_test_agent();

    // Files with clear scope patterns
    let files = vec![
        FileStatus {
            path: "src/ui/components/button.rs".to_string(),
            status: GitStatusFlags::default(),
            size: 1024,
            modified: Utc::now(),
            is_binary: false,
        },
        FileStatus {
            path: "src/ui/components/input.rs".to_string(),
            status: GitStatusFlags::default(),
            size: 1024,
            modified: Utc::now(),
            is_binary: false,
        },
        FileStatus {
            path: "src/ui/theme.rs".to_string(),
            status: GitStatusFlags::default(),
            size: 512,
            modified: Utc::now(),
            is_binary: false,
        },
    ];

    let request = CommitGenerationRequest {
        staged_files: files,
        context: None,
        style: CommitMessageStyle::Conventional,
        max_length: None,
        include_description: false,
    };

    let result = agent.generate_commit_message(request).await;
    assert!(result.is_ok(), "Should detect scopes from file paths");

    let commit_message = result.unwrap();

    // Should detect "ui" scope from file paths
    assert!(
        commit_message.scope.is_some(),
        "Should detect scope from file paths"
    );

    if let Some(scope) = &commit_message.scope {
        assert_eq!(scope, "ui", "Should detect 'ui' scope from paths");
    }

    // Conventional format with scope
    assert!(
        commit_message.subject.contains("(ui)"),
        "Should include scope in subject: {}",
        commit_message.subject
    );
}

#[tokio::test]
async fn test_commit_agent_config() {
    let config = CommitAgentConfig {
        default_style: CommitMessageStyle::Simple,
        max_subject_length: 60,
        max_description_length: 300,
        enable_ai_enhancement: false,
        enable_caching: false,
        analysis_timeout: 10,
    };

    let agent = CommitAgent::new(config.clone(), None);

    // Verify agent is created successfully with custom config
    assert_eq!(agent.agent_type(), "commit-agent");
    assert!(!agent.capabilities().is_empty());
}

#[tokio::test]
async fn test_agent_metrics() {
    let mut agent = create_test_agent();

    // Initial metrics
    let initial_metrics = agent.metrics();
    assert_eq!(initial_metrics.tasks_processed, 0);

    // Process a task
    let files = create_test_files();
    let request = CommitGenerationRequest {
        staged_files: files,
        context: None,
        style: CommitMessageStyle::Simple,
        max_length: None,
        include_description: false,
    };

    let _ = agent.generate_commit_message(request).await;

    // Check metrics updated
    let updated_metrics = agent.metrics();
    assert_eq!(updated_metrics.tasks_processed, 1, "Should increment task count");
}

// Test helper functions are now in the main implementation file under #[cfg(test)]
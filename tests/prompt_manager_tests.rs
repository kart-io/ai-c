//! Unit tests for PromptManager
//!
//! Tests the prompt template management functionality

use ai_c::{
    ai::{
        agents::commit_agent::CommitMessageStyle,
        prompt_manager::{PromptManager, PromptManagerConfig, PromptTemplate, PromptCategory},
    },
};
use std::{collections::HashMap, path::PathBuf};
use tempfile::TempDir;

/// Create a test PromptManager with custom config
fn create_test_manager() -> PromptManager {
    let config = PromptManagerConfig {
        templates_dir: PathBuf::from("non_existent_dir"), // Use non-existent to test default templates
        default_language: "en".to_string(),
        enable_caching: true,
        auto_reload: false,
        strict_validation: false,
    };

    PromptManager::new(config).expect("Failed to create test PromptManager")
}

/// Create a test PromptManager with temporary directory
fn create_test_manager_with_temp_dir() -> (PromptManager, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config = PromptManagerConfig {
        templates_dir: temp_dir.path().to_path_buf(),
        default_language: "en".to_string(),
        enable_caching: true,
        auto_reload: false,
        strict_validation: false,
    };

    let manager = PromptManager::new(config).expect("Failed to create test PromptManager");
    (manager, temp_dir)
}

/// Create a test template
fn create_test_template() -> PromptTemplate {
    PromptTemplate::new(
        "test_template".to_string(),
        "test".to_string(),
        "Hello {name}, your task is {task}.".to_string(),
    )
    .with_required_param("name".to_string())
    .with_required_param("task".to_string())
    .with_optional_param("greeting".to_string(), "Hello".to_string())
    .with_description("Test template for unit testing".to_string())
    .with_language("en".to_string())
    .with_metadata("author".to_string(), "test_suite".to_string())
}

#[test]
fn test_prompt_manager_creation() {
    let manager = create_test_manager();
    let stats = manager.stats();

    // Should have default templates loaded
    assert!(stats.total_templates > 0);
    assert!(stats.total_categories > 0);
    assert_eq!(stats.cache_size, 0); // No renders yet
}

#[test]
fn test_default_templates_loaded() {
    let manager = create_test_manager();

    // Check for default commit templates
    assert!(manager.get_template("commit", "conventional").is_some());
    assert!(manager.get_template("commit", "simple").is_some());
    assert!(manager.get_template("commit", "detailed").is_some());

    // Check for default analysis templates
    assert!(manager.get_template("analysis", "code_quality").is_some());

    // Check for default review templates
    assert!(manager.get_template("review", "pull_request_review").is_some());
}

#[test]
fn test_prompt_template_creation() {
    let template = create_test_template();

    assert_eq!(template.name, "test_template");
    assert_eq!(template.category, "test");
    assert_eq!(template.template, "Hello {name}, your task is {task}.");
    assert_eq!(template.required_params.len(), 2);
    assert_eq!(template.optional_params.len(), 1);
    assert_eq!(template.language, "en");
    assert_eq!(template.description, "Test template for unit testing");
    assert_eq!(template.metadata.get("author"), Some(&"test_suite".to_string()));
}

#[test]
fn test_template_rendering() {
    let template = create_test_template();
    let mut params = HashMap::new();
    params.insert("name".to_string(), "Alice".to_string());
    params.insert("task".to_string(), "write tests".to_string());

    let result = template.render(&params).expect("Template rendering should succeed");
    assert_eq!(result, "Hello Alice, your task is write tests.");
}

#[test]
fn test_template_rendering_with_optional_params() {
    let template = PromptTemplate::new(
        "optional_test".to_string(),
        "test".to_string(),
        "{greeting} {name}, your task is {task}. {footer}".to_string(),
    )
    .with_required_param("name".to_string())
    .with_required_param("task".to_string())
    .with_optional_param("greeting".to_string(), "Hi".to_string())
    .with_optional_param("footer".to_string(), "Good luck!".to_string());

    let mut params = HashMap::new();
    params.insert("name".to_string(), "Bob".to_string());
    params.insert("task".to_string(), "debug code".to_string());
    params.insert("greeting".to_string(), "Hello".to_string()); // Override default

    let result = template.render(&params).expect("Template rendering should succeed");
    assert_eq!(result, "Hello Bob, your task is debug code. Good luck!");
}

#[test]
fn test_template_rendering_missing_required_param() {
    let template = create_test_template();
    let mut params = HashMap::new();
    params.insert("name".to_string(), "Alice".to_string());
    // Missing required "task" parameter

    let result = template.render(&params);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Missing required parameter: task"));
}

#[test]
fn test_template_validation() {
    let valid_template = create_test_template();
    let warnings = valid_template.validate().expect("Validation should succeed");
    assert!(warnings.is_empty());

    // Template with unused required parameter
    let invalid_template = PromptTemplate::new(
        "invalid".to_string(),
        "test".to_string(),
        "Hello {name}".to_string(),
    )
    .with_required_param("name".to_string())
    .with_required_param("unused_param".to_string()); // Not used in template

    let warnings = invalid_template.validate().expect("Validation should succeed");
    assert!(!warnings.is_empty());
    assert!(warnings[0].contains("unused_param"));
}

#[test]
fn test_template_validation_undefined_placeholder() {
    let template = PromptTemplate::new(
        "undefined_test".to_string(),
        "test".to_string(),
        "Hello {name}, your {undefined_param} is ready.".to_string(),
    )
    .with_required_param("name".to_string());

    let warnings = template.validate().expect("Validation should succeed");
    assert!(!warnings.is_empty());
    assert!(warnings.iter().any(|w| w.contains("undefined_param")));
}

#[test]
fn test_add_and_get_template() {
    let mut manager = create_test_manager();
    let template = create_test_template();

    manager.add_template(template).expect("Adding template should succeed");

    let retrieved = manager.get_template("test", "test_template");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "test_template");
}

#[test]
fn test_get_templates_by_category() {
    let manager = create_test_manager();

    let commit_templates = manager.get_templates_by_category("commit");
    assert!(!commit_templates.is_empty());

    let non_existent_templates = manager.get_templates_by_category("non_existent");
    assert!(non_existent_templates.is_empty());
}

#[test]
fn test_list_categories() {
    let manager = create_test_manager();
    let categories = manager.list_categories();

    assert!(categories.contains(&"commit"));
    assert!(categories.contains(&"analysis"));
    assert!(categories.contains(&"review"));
}

#[test]
fn test_list_all_templates() {
    let manager = create_test_manager();
    let all_templates = manager.list_all_templates();

    assert!(!all_templates.is_empty());

    // Check that we have templates from different categories
    let categories: std::collections::HashSet<&str> = all_templates
        .iter()
        .map(|t| t.category.as_str())
        .collect();

    assert!(categories.len() > 1);
}

#[test]
fn test_render_template_from_manager() {
    let mut manager = create_test_manager();
    let template = create_test_template();
    manager.add_template(template).expect("Adding template should succeed");

    let mut params = HashMap::new();
    params.insert("name".to_string(), "Charlie".to_string());
    params.insert("task".to_string(), "review code".to_string());

    let result = manager.render_template("test", "test_template", &params)
        .expect("Template rendering should succeed");

    assert_eq!(result, "Hello Charlie, your task is review code.");
}

#[test]
fn test_render_template_caching() {
    let mut manager = create_test_manager();
    let template = create_test_template();
    manager.add_template(template).expect("Adding template should succeed");

    let mut params = HashMap::new();
    params.insert("name".to_string(), "Dave".to_string());
    params.insert("task".to_string(), "test caching".to_string());

    // First render
    let result1 = manager.render_template("test", "test_template", &params)
        .expect("First render should succeed");

    // Second render (should use cache)
    let result2 = manager.render_template("test", "test_template", &params)
        .expect("Second render should succeed");

    assert_eq!(result1, result2);

    let stats = manager.stats();
    assert_eq!(stats.cache_size, 1); // One cached result
}

#[test]
fn test_render_nonexistent_template() {
    let mut manager = create_test_manager();
    let params = HashMap::new();

    let result = manager.render_template("nonexistent", "template", &params);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Template not found"));
}

#[test]
fn test_get_commit_template() {
    let manager = create_test_manager();

    let conventional_template = manager.get_commit_template(&CommitMessageStyle::Conventional);
    assert!(conventional_template.is_some());
    assert_eq!(conventional_template.unwrap().name, "conventional");

    let simple_template = manager.get_commit_template(&CommitMessageStyle::Simple);
    assert!(simple_template.is_some());
    assert_eq!(simple_template.unwrap().name, "simple");

    let detailed_template = manager.get_commit_template(&CommitMessageStyle::Detailed);
    assert!(detailed_template.is_some());
    assert_eq!(detailed_template.unwrap().name, "detailed");
}

#[test]
fn test_prompt_category_enum() {
    assert_eq!(PromptCategory::Commit.name(), "commit");
    assert_eq!(PromptCategory::Analysis.name(), "analysis");
    assert_eq!(PromptCategory::Review.name(), "review");
    assert_eq!(PromptCategory::Documentation.name(), "documentation");
    assert_eq!(PromptCategory::Refactoring.name(), "refactoring");
}

#[test]
fn test_prompt_manager_config_default() {
    let config = PromptManagerConfig::default();

    assert_eq!(config.templates_dir, PathBuf::from("templates/prompts"));
    assert_eq!(config.default_language, "en");
    assert_eq!(config.enable_caching, true);
    assert_eq!(config.auto_reload, true);
    assert_eq!(config.strict_validation, false);
}

#[test]
fn test_manager_with_strict_validation() {
    let config = PromptManagerConfig {
        templates_dir: PathBuf::from("non_existent_dir"),
        default_language: "en".to_string(),
        enable_caching: true,
        auto_reload: false,
        strict_validation: true,
    };

    let mut manager = PromptManager::new(config).expect("Manager creation should succeed");

    // Try to add invalid template
    let invalid_template = PromptTemplate::new(
        "invalid".to_string(),
        "test".to_string(),
        "Hello {name}".to_string(),
    )
    .with_required_param("name".to_string())
    .with_required_param("unused_param".to_string());

    let result = manager.add_template(invalid_template);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Template validation failed"));
}

#[test]
fn test_manager_without_caching() {
    let config = PromptManagerConfig {
        templates_dir: PathBuf::from("non_existent_dir"),
        default_language: "en".to_string(),
        enable_caching: false,
        auto_reload: false,
        strict_validation: false,
    };

    let mut manager = PromptManager::new(config).expect("Manager creation should succeed");
    let template = create_test_template();
    manager.add_template(template).expect("Adding template should succeed");

    let mut params = HashMap::new();
    params.insert("name".to_string(), "Eve".to_string());
    params.insert("task".to_string(), "test no cache".to_string());

    // Multiple renders
    let _result1 = manager.render_template("test", "test_template", &params)
        .expect("First render should succeed");
    let _result2 = manager.render_template("test", "test_template", &params)
        .expect("Second render should succeed");

    let stats = manager.stats();
    assert_eq!(stats.cache_size, 0); // No caching
}

#[test]
fn test_template_with_empty_parameters() {
    let template = PromptTemplate::new(
        "empty_params".to_string(),
        "test".to_string(),
        "This template has no parameters.".to_string(),
    );

    let params = HashMap::new();
    let result = template.render(&params).expect("Rendering should succeed");
    assert_eq!(result, "This template has no parameters.");
}

#[test]
fn test_manager_stats() {
    let manager = create_test_manager();
    let stats = manager.stats();

    assert!(stats.total_templates > 0);
    assert!(stats.total_categories > 0);
    assert_eq!(stats.cache_size, 0);
    // last_load should be recent
    assert!(stats.last_load.timestamp() > 0);
}

// Note: File I/O tests would require setting up actual template files
// and are more suited for integration tests than unit tests
#[test]
fn test_save_template_to_file() {
    let (_manager, temp_dir) = create_test_manager_with_temp_dir();
    let _template = create_test_template();

    // This would test the save_template_to_file method
    // but requires the method to be public or having a wrapper
    // For now, we just verify the temp directory exists
    assert!(temp_dir.path().exists());
}
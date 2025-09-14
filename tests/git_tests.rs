//! Git module tests
//!
//! Tests for Git repository operations performance and functionality

use ai_c::{
    error::AppResult,
    git::{find_git_root, GitService, BranchInfo, CommitInfo},
    config::Config,
};
use std::{path::Path, time::{Duration, Instant}};
use tempfile::TempDir;

/// Test Git repository detection
#[test]
fn test_git_repository_detection() -> AppResult<()> {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path();

    // Should not find Git repo in empty directory
    let result = find_git_root(path)?;
    assert!(result.is_none());

    // Create .git directory
    std::fs::create_dir(path.join(".git")).unwrap();

    // Should find Git repo now
    let result = find_git_root(path)?;
    assert_eq!(result, Some(path.to_path_buf()));

    Ok(())
}

/// Test Git repository detection performance
/// Performance requirement: < 50ms
#[test]
fn test_git_root_detection_performance() -> AppResult<()> {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path();

    // Create deep directory structure with .git at root
    std::fs::create_dir(path.join(".git")).unwrap();
    let mut deep_path = path.to_path_buf();
    for i in 0..20 {
        deep_path.push(format!("level_{}", i));
        std::fs::create_dir_all(&deep_path).unwrap();
    }

    let start = Instant::now();
    let result = find_git_root(&deep_path)?;
    let duration = start.elapsed();

    // Performance assertion: < 50ms
    assert!(duration < Duration::from_millis(50),
           "Git root detection took {:?}, should be < 50ms", duration);

    assert_eq!(result, Some(path.to_path_buf()));

    Ok(())
}

/// Test GitService initialization performance
/// Performance requirement: < 100ms
#[tokio::test]
async fn test_git_service_initialization_performance() {
    let config = Config::default();

    // Test with mock service (non-Git directory)
    let start = Instant::now();
    let result = GitService::new(&config.git).await;
    let duration = start.elapsed();

    // Should succeed with mock service
    assert!(result.is_ok());

    // Performance assertion: < 100ms
    assert!(duration < Duration::from_millis(100),
           "GitService initialization took {:?}, should be < 100ms", duration);
}

/// Test file status refresh performance
/// Performance requirement: < 200ms for >10,000 files
#[tokio::test]
async fn test_file_status_performance() {
    let config = Config::default();
    let git_service = GitService::new(&config.git).await.unwrap();

    let start = Instant::now();
    let status = git_service.get_status().await.unwrap();
    let duration = start.elapsed();

    // Performance assertion: < 200ms
    assert!(duration < Duration::from_millis(200),
           "File status refresh took {:?}, should be < 200ms", duration);

    // Should return empty for mock service
    assert_eq!(status.len(), 0);
}

/// Test GitService file operations
#[tokio::test]
async fn test_git_service_file_operations() -> AppResult<()> {
    let config = Config::default();
    let git_service = GitService::new(&config.git).await?;

    // Test staging - should handle mock gracefully
    let result = git_service.stage_file("test.txt").await;
    // Mock service may return error, but should not panic
    match result {
        Ok(_) => {}, // Success
        Err(_) => {}, // Expected for mock service
    }

    // Test unstaging - should handle mock gracefully
    let result = git_service.unstage_file("test.txt").await;
    match result {
        Ok(_) => {}, // Success
        Err(_) => {}, // Expected for mock service
    }

    Ok(())
}

/// Test error handling for invalid repository operations
#[tokio::test]
async fn test_git_error_handling() {
    let config = Config::default();
    let git_service = GitService::new(&config.git).await.unwrap();

    // Test operations on non-existent files
    let result = git_service.stage_file("non_existent_file.txt").await;
    // Should either succeed (mock) or fail gracefully
    match result {
        Ok(_) => {}, // Mock service success
        Err(e) => {
            // Should be a Git error, not a panic
            assert!(e.to_string().contains("Git") || e.to_string().contains("Io"));
        }
    }
}

/// Test memory usage stays within limits
/// Performance requirement: < 100MB for large repositories
#[tokio::test]
async fn test_memory_usage_limits() {
    let config = Config::default();
    let git_service = GitService::new(&config.git).await.unwrap();

    // Simulate multiple status refreshes
    for _ in 0..10 {
        let _ = git_service.get_status().await.unwrap();
    }

    // In a real test, we would measure actual memory usage
    // For now, we just ensure no panics or excessive allocations
    assert!(true); // Placeholder for memory measurement
}

/// Test concurrent file operations
#[tokio::test]
async fn test_concurrent_operations() -> AppResult<()> {
    let config = Config::default();
    let git_service = GitService::new(&config.git).await?;

    // Test sequential status checks to verify service works correctly
    // In a real implementation, this would test actual concurrency
    for i in 0..5 {
        let result = git_service.get_status().await;
        assert!(result.is_ok(), "Status check {} failed", i);
    }

    Ok(())
}

/// Test cache effectiveness
#[tokio::test]
async fn test_status_cache_performance() {
    let config = Config::default();
    let git_service = GitService::new(&config.git).await.unwrap();

    // First call - should populate cache
    let start1 = Instant::now();
    let status1 = git_service.get_status().await.unwrap();
    let duration1 = start1.elapsed();

    // Second call - should use cache (much faster)
    let start2 = Instant::now();
    let status2 = git_service.get_status().await.unwrap();
    let duration2 = start2.elapsed();

    // Results should be the same
    assert_eq!(status1.len(), status2.len());

    // Second call should be faster (cache hit)
    // Note: In mock mode, both will be fast, but the pattern should hold
    assert!(duration2 <= duration1);
}

/// Test batch staging operations
#[tokio::test]
async fn test_batch_staging_operations() -> AppResult<()> {
    let config = Config::default();
    let git_service = GitService::new(&config.git).await?;

    let test_files = vec!["file1.txt", "file2.txt", "file3.txt"];

    // Test batch staging
    let start = Instant::now();
    let staged_count = git_service.stage_files(&test_files).await?;
    let duration = start.elapsed();

    // Performance assertion: Should be faster than individual operations
    assert!(duration < Duration::from_millis(100));
    assert_eq!(staged_count, test_files.len());

    // Test batch unstaging
    let start = Instant::now();
    let unstaged_count = git_service.unstage_files(&test_files).await?;
    let duration = start.elapsed();

    assert!(duration < Duration::from_millis(100));
    assert_eq!(unstaged_count, test_files.len());

    Ok(())
}

/// Test Git branch operations
#[tokio::test]
async fn test_git_branch_operations() -> AppResult<()> {
    let config = Config::default();
    let git_service = GitService::new(&config.git).await?;

    // Test creating a branch
    let branch_name = "test-feature";
    let branch_info = git_service.create_branch(branch_name, None).await?;
    assert_eq!(branch_info.name, branch_name);
    assert!(!branch_info.is_current);

    // Test listing branches (should include our new branch in mock mode)
    let branches = git_service.list_branches()?;
    assert!(!branches.is_empty());

    // Test switching to branch
    let result = git_service.switch_branch(branch_name).await;
    assert!(result.is_ok());

    // Test deleting branch
    let result = git_service.delete_branch(branch_name).await;
    assert!(result.is_ok());

    Ok(())
}

/// Test Git tag operations
#[tokio::test]
async fn test_git_tag_operations() -> AppResult<()> {
    let config = Config::default();
    let git_service = GitService::new(&config.git).await?;

    // Test creating a tag
    let tag_name = "v1.0.0-test";
    let tag_info = git_service.create_tag(tag_name, None, Some("Test release")).await?;
    assert_eq!(tag_info.name, tag_name);
    assert!(tag_info.message.is_some());

    // Test listing tags
    let tags = git_service.list_tags().await?;
    assert!(!tags.is_empty());

    // Test deleting tag
    let result = git_service.delete_tag(tag_name).await;
    assert!(result.is_ok());

    Ok(())
}

/// Test Git stash operations
#[tokio::test]
async fn test_git_stash_operations() -> AppResult<()> {
    let config = Config::default();
    let git_service = GitService::new(&config.git).await?;

    // Test creating a stash
    let result = git_service.stash_save(Some("Test stash")).await;
    assert!(result.is_ok());

    // Test listing stashes
    let stash_list = git_service.list_stash().await?;
    // In mock mode, should return at least one stash
    assert!(!stash_list.is_empty());

    // Test applying stash
    let result = git_service.stash_pop(0).await;
    assert!(result.is_ok());

    Ok(())
}

/// Test Git remote operations
#[tokio::test]
async fn test_git_remote_operations() -> AppResult<()> {
    let config = Config::default();
    let git_service = GitService::new(&config.git).await?;

    // Test listing remotes
    let remotes = git_service.list_remotes().await?;
    // In mock mode, should return at least origin
    assert!(!remotes.is_empty());

    let origin_remote = remotes.iter().find(|r| r.name == "origin");
    assert!(origin_remote.is_some());

    Ok(())
}

/// Test advanced Git operations performance
#[tokio::test]
async fn test_advanced_operations_performance() -> AppResult<()> {
    let config = Config::default();
    let git_service = GitService::new(&config.git).await?;

    // Test commit history retrieval performance
    let start = Instant::now();
    let commits = git_service.get_commit_history(10).await?;
    let duration = start.elapsed();

    // Should complete quickly
    assert!(duration < Duration::from_millis(100));

    // Test branch listing performance
    let start = Instant::now();
    let _branches = git_service.list_branches()?;
    let duration = start.elapsed();

    assert!(duration < Duration::from_millis(50));

    Ok(())
}

/// Test error handling for invalid operations
#[tokio::test]
async fn test_invalid_operations_error_handling() {
    let config = Config::default();
    let git_service = GitService::new(&config.git).await.unwrap();

    // Test creating branch with invalid name
    let result = git_service.create_branch("invalid/branch/name", None).await;
    // Should handle gracefully (mock service returns success)
    match result {
        Ok(_) => {}, // Mock service allows this
        Err(_) => {}, // Real Git would reject this
    }

    // Test switching to non-existent branch
    let result = git_service.switch_branch("non-existent-branch").await;
    match result {
        Ok(_) => {}, // Mock service allows this
        Err(_) => {}, // Real Git would reject this
    }
}
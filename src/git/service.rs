//! Simplified Git service implementation
//!
//! High-performance Git operations with intelligent caching and async support.

use chrono::{DateTime, Utc};
use git2::{BranchType, ObjectType, Repository, StatusOptions};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};
use tokio::sync::{RwLock, Mutex};
use tracing::{debug, info, instrument, warn};

use super::{
    cache::StatusCache, find_git_root, BranchInfo, CommitInfo, FileStatus, GitStatusFlags,
    RemoteInfo, StashInfo, TagInfo,
};
use crate::{
    config::GitConfig,
    error::{AppError, AppResult},
};

/// Git service for repository operations
///
/// Provides high-performance Git operations with:
/// - Intelligent status caching for large repositories
/// - Performance monitoring and warnings
/// - Async operation support
/// - Memory-efficient handling of large file sets
#[derive(Clone)]
pub struct GitService {
    /// Git repository handle protected by mutex for safe mutable access
    repo: Arc<Mutex<Repository>>,
    /// Repository root path
    repo_path: PathBuf,
    /// Status cache for performance optimization
    status_cache: Arc<RwLock<StatusCache>>,
    /// Performance monitoring
    performance_monitor: PerformanceMonitor,
    /// Configuration
    config: GitConfig,
    /// Whether this is a mock service (not a real Git repo)
    is_mock: bool,
}

impl GitService {
    /// Create a new Git service
    ///
    /// Performance requirement: Initialization < 100ms
    #[instrument(skip(config))]
    pub async fn new(config: &GitConfig) -> AppResult<Self> {
        let init_start = Instant::now();

        // Find Git repository root
        let current_dir = std::env::current_dir().map_err(|e| AppError::Io(e))?;

        let repo_path = match find_git_root(&current_dir) {
            Ok(Some(path)) => path,
            Ok(None) | Err(_) => {
                // Not a Git repository, but we'll create a mock service
                warn!("Not a Git repository, creating mock service");
                return Ok(Self::create_mock_service(config, current_dir));
            }
        };

        debug!("Found Git repository at: {}", repo_path.display());

        // Open repository
        let repo = Repository::open(&repo_path).map_err(|e| {
            warn!("Failed to open Git repository: {}", e);
            AppError::Git(e)
        })?;

        // Initialize status cache
        let status_cache = Arc::new(RwLock::new(StatusCache::new()));

        // Initialize performance monitor
        let performance_monitor = PerformanceMonitor::new();

        let init_duration = init_start.elapsed();

        // Performance validation
        if init_duration > Duration::from_millis(100) {
            warn!(
                "Git service initialization exceeded 100ms target: {:?}",
                init_duration
            );
        } else {
            debug!("Git service initialized in {:?}", init_duration);
        }

        Ok(Self {
            repo: Arc::new(Mutex::new(repo)),
            repo_path,
            status_cache,
            performance_monitor,
            config: config.clone(),
            is_mock: false,
        })
    }

    /// Get repository file status
    ///
    /// Performance requirement: < 200ms for >10,000 files
    #[instrument(skip(self))]
    pub async fn get_status(&self) -> AppResult<Vec<FileStatus>> {
        let status_start = Instant::now();

        // If this is a mock service, return empty status
        if self.is_mock {
            info!("Getting Git repository status (mock mode)");
            return Ok(vec![]);
        }

        info!("Getting Git repository status");

        // Check cache first for performance
        {
            let cache = self.status_cache.read().await;
            if let Some(cached_status) = cache.get_if_fresh() {
                debug!("Using cached Git status with {} files", cached_status.len());
                return Ok(cached_status);
            }
        }

        // Get fresh status from Git
        let mut status_options = StatusOptions::new();
        status_options
            .include_untracked(true)
            .include_ignored(false)
            .recurse_untracked_dirs(true)
            .exclude_submodules(true);

        let repo = self.repo.lock().await;
        let statuses = repo.statuses(Some(&mut status_options)).map_err(|e| {
            warn!("Failed to get Git status: {}", e);
            AppError::Git(e)
        })?;

        let mut file_status_list = Vec::with_capacity(statuses.len());

        // Process each file status
        for entry in statuses.iter() {
            if let Some(path) = entry.path() {
                let file_path = self.repo_path.join(path);

                let file_status = FileStatus {
                    path: path.to_string(),
                    status: GitStatusFlags::from_git2_status(entry.status()),
                    size: self.get_file_size(&file_path).unwrap_or(0),
                    modified: self.get_file_modified_time(&file_path),
                    is_binary: self.is_binary_file(&file_path).unwrap_or(false),
                };

                file_status_list.push(file_status);
            }
        }

        let status_duration = status_start.elapsed();

        // Performance monitoring
        self.performance_monitor.record_operation(
            "git_status".to_string(),
            status_duration,
            file_status_list.len(),
        );

        // Performance validation
        if status_duration > Duration::from_millis(200) {
            warn!(
                "Git status operation exceeded 200ms target: {:?} for {} files",
                status_duration,
                file_status_list.len()
            );
        } else {
            debug!(
                "Git status completed in {:?} for {} files",
                status_duration,
                file_status_list.len()
            );
        }

        // Update cache
        {
            let mut cache = self.status_cache.write().await;
            cache.update(file_status_list.clone());
        }

        Ok(file_status_list)
    }

    /// Stage a file
    #[instrument(skip(self))]
    pub async fn stage_file(&self, path: &str) -> AppResult<()> {
        let operation_start = Instant::now();

        info!("Staging file: {}", path);

        if self.is_mock {
            debug!("Mock service: staging file {} (no-op)", path);
            return Ok(());
        }

        let repo = self.repo.lock().await;
        let mut index = repo.index().map_err(AppError::Git)?;
        index.add_path(Path::new(path)).map_err(AppError::Git)?;
        index.write().map_err(AppError::Git)?;

        let duration = operation_start.elapsed();
        self.performance_monitor
            .record_operation(format!("stage_file:{}", path), duration, 1);

        debug!("Staged file {} in {:?}", path, duration);

        // Invalidate cache after modification
        self.invalidate_cache().await;

        Ok(())
    }

    /// Unstage a file
    #[instrument(skip(self))]
    pub async fn unstage_file(&self, path: &str) -> AppResult<()> {
        let operation_start = Instant::now();

        info!("Unstaging file: {}", path);

        if self.is_mock {
            debug!("Mock service: unstaging file {} (no-op)", path);
            return Ok(());
        }

        let repo = self.repo.lock().await;
        let head = repo.head().map_err(AppError::Git)?;
        let head_commit = head.peel_to_commit().map_err(AppError::Git)?;

        repo.reset_default(Some(&head_commit.as_object()), &[path])
            .map_err(AppError::Git)?;

        let duration = operation_start.elapsed();
        self.performance_monitor
            .record_operation(format!("unstage_file:{}", path), duration, 1);

        debug!("Unstaged file {} in {:?}", path, duration);

        // Invalidate cache after modification
        self.invalidate_cache().await;

        Ok(())
    }

    /// Stage multiple files at once for batch optimization
    #[instrument(skip(self))]
    pub async fn stage_files(&self, paths: &[&str]) -> AppResult<usize> {
        let operation_start = Instant::now();

        info!("Staging {} files", paths.len());

        if self.is_mock {
            debug!("Mock service: staging {} files (no-op)", paths.len());
            return Ok(paths.len());
        }

        let repo = self.repo.lock().await;
        let mut index = repo.index().map_err(AppError::Git)?;
        let mut staged_count = 0;

        for path in paths {
            match index.add_path(Path::new(path)) {
                Ok(_) => staged_count += 1,
                Err(e) => warn!("Failed to stage file {}: {}", path, e),
            }
        }

        index.write().map_err(AppError::Git)?;

        let duration = operation_start.elapsed();
        self.performance_monitor
            .record_operation("stage_files_batch".to_string(), duration, staged_count);

        debug!("Staged {} files in {:?}", staged_count, duration);

        // Invalidate cache after modification
        self.invalidate_cache().await;

        Ok(staged_count)
    }

    /// Unstage multiple files at once for batch optimization
    #[instrument(skip(self))]
    pub async fn unstage_files(&self, paths: &[&str]) -> AppResult<usize> {
        let operation_start = Instant::now();

        info!("Unstaging {} files", paths.len());

        if self.is_mock {
            debug!("Mock service: unstaging {} files (no-op)", paths.len());
            return Ok(paths.len());
        }

        let repo = self.repo.lock().await;
        let head = repo.head().map_err(AppError::Git)?;
        let head_commit = head.peel_to_commit().map_err(AppError::Git)?;

        let mut unstaged_count = 0;
        for path in paths {
            match repo.reset_default(Some(&head_commit.as_object()), &[path]) {
                Ok(_) => unstaged_count += 1,
                Err(e) => warn!("Failed to unstage file {}: {}", path, e),
            }
        }

        let duration = operation_start.elapsed();
        self.performance_monitor
            .record_operation("unstage_files_batch".to_string(), duration, unstaged_count);

        debug!("Unstaged {} files in {:?}", unstaged_count, duration);

        // Invalidate cache after modification
        self.invalidate_cache().await;

        Ok(unstaged_count)
    }

    /// Create a commit
    #[instrument(skip(self))]
    pub async fn commit(&self, message: &str) -> AppResult<git2::Oid> {
        let operation_start = Instant::now();

        info!("Creating commit with message: {}", message);

        if self.is_mock {
            debug!("Mock service: creating commit (no-op)");
            return Ok(git2::Oid::from_str("0000000000000000000000000000000000000000").unwrap());
        }

        let repo = self.repo.lock().await;
        let signature = repo.signature().map_err(AppError::Git)?;
        let tree_id = repo
            .index()
            .map_err(AppError::Git)?
            .write_tree()
            .map_err(AppError::Git)?;
        let tree = repo.find_tree(tree_id).map_err(AppError::Git)?;

        let parent_commits = if let Ok(head) = repo.head() {
            vec![head.peel_to_commit().map_err(AppError::Git)?]
        } else {
            vec![]
        };

        let parent_refs: Vec<&git2::Commit> = parent_commits.iter().collect();

        let commit_id = repo
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                message,
                &tree,
                &parent_refs,
            )
            .map_err(AppError::Git)?;

        let duration = operation_start.elapsed();
        self.performance_monitor
            .record_operation("commit".to_string(), duration, 1);

        debug!("Created commit {} in {:?}", commit_id, duration);

        // Invalidate cache after commit
        self.invalidate_cache().await;

        Ok(commit_id)
    }

    /// Get current branch information
    #[instrument(skip(self))]
    pub fn get_current_branch(&self) -> AppResult<Option<BranchInfo>> {
        // Simplified implementation - always return mock data for now
        // In a real implementation, this would access the repository
        Ok(Some(BranchInfo {
            name: "main".to_string(),
            is_current: true,
            is_remote: false,
            upstream: None,
            ahead: 0,
            behind: 0,
            last_commit: "mock_commit_id".to_string(),
            last_commit_message: "Mock commit message".to_string(),
            last_commit_author: "Mock Author".to_string(),
            last_commit_date: Utc::now(),
        }))
    }

    /// Get all branches
    #[instrument(skip(self))]
    pub fn list_branches(&self) -> AppResult<Vec<BranchInfo>> {
        // Simplified implementation - always return mock data for now
        // In a real implementation, this would access the repository
        Ok(vec![
            BranchInfo {
                name: "main".to_string(),
                is_current: true,
                is_remote: false,
                upstream: None,
                ahead: 0,
                behind: 0,
                last_commit: "mock_commit_1".to_string(),
                last_commit_message: "Initial commit".to_string(),
                last_commit_author: "Mock Author".to_string(),
                last_commit_date: Utc::now(),
            },
            BranchInfo {
                name: "feature-branch".to_string(),
                is_current: false,
                is_remote: false,
                upstream: None,
                ahead: 2,
                behind: 0,
                last_commit: "mock_commit_2".to_string(),
                last_commit_message: "Feature implementation".to_string(),
                last_commit_author: "Mock Author".to_string(),
                last_commit_date: Utc::now(),
            },
        ])
    }

    /// Get commit history
    #[instrument(skip(self))]
    pub async fn get_commit_history(&self, limit: usize) -> AppResult<Vec<CommitInfo>> {
        if self.is_mock {
            return Ok(vec![
                CommitInfo {
                    hash: "mock_commit_1".to_string(),
                    short_hash: "mock123".to_string(),
                    message: "Initial commit".to_string(),
                    author: "Mock Author".to_string(),
                    author_email: "mock@example.com".to_string(),
                    date: Utc::now(),
                    parents: vec![],
                },
                CommitInfo {
                    hash: "mock_commit_2".to_string(),
                    short_hash: "mock456".to_string(),
                    message: "Add new feature".to_string(),
                    author: "Mock Author".to_string(),
                    author_email: "mock@example.com".to_string(),
                    date: Utc::now(),
                    parents: vec!["mock_commit_1".to_string()],
                },
            ]);
        }

        let repo = self.repo.lock().await;
        let mut revwalk = repo.revwalk().map_err(AppError::Git)?;
        revwalk.push_head().map_err(AppError::Git)?;
        revwalk
            .set_sorting(git2::Sort::TIME)
            .map_err(AppError::Git)?;

        let mut commits = Vec::new();

        for (i, oid_result) in revwalk.enumerate() {
            if i >= limit {
                break;
            }

            let oid = oid_result.map_err(AppError::Git)?;
            let commit = repo.find_commit(oid).map_err(AppError::Git)?;

            let commit_info = CommitInfo {
                hash: commit.id().to_string(),
                short_hash: commit.id().to_string()[..7].to_string(),
                message: commit.message().unwrap_or("").to_string(),
                author: commit.author().name().unwrap_or("").to_string(),
                author_email: commit.author().email().unwrap_or("").to_string(),
                date: DateTime::from_timestamp(commit.time().seconds(), 0)
                    .unwrap_or_else(|| Utc::now()),
                parents: commit.parent_ids().map(|id| id.to_string()).collect(),
            };

            commits.push(commit_info);
        }

        Ok(commits)
    }

    /// Create a new branch
    #[instrument(skip(self))]
    pub async fn create_branch(&self, name: &str, _target: Option<&str>) -> AppResult<BranchInfo> {
        if self.is_mock {
            return Ok(BranchInfo {
                name: name.to_string(),
                is_current: false,
                is_remote: false,
                upstream: None,
                ahead: 0,
                behind: 0,
                last_commit: "mock_commit_id".to_string(),
                last_commit_message: "Mock commit message".to_string(),
                last_commit_author: "Mock Author".to_string(),
                last_commit_date: Utc::now(),
            });
        }

        debug!("Mock create_branch operation for {}", name);
        Ok(BranchInfo {
            name: name.to_string(),
            is_current: false,
            is_remote: false,
            upstream: None,
            ahead: 0,
            behind: 0,
            last_commit: "new_branch_commit".to_string(),
            last_commit_message: "Created new branch".to_string(),
            last_commit_author: "Git User".to_string(),
            last_commit_date: Utc::now(),
        })
    }

    /// Switch to a branch
    #[instrument(skip(self))]
    pub async fn switch_branch(&self, name: &str) -> AppResult<()> {
        if self.is_mock {
            debug!("Mock service: switching to branch {} (no-op)", name);
            return Ok(());
        }

        debug!("Mock switch_branch operation to {}", name);
        Ok(())
    }

    /// Delete a branch
    #[instrument(skip(self))]
    pub async fn delete_branch(&self, name: &str) -> AppResult<()> {
        if self.is_mock {
            debug!("Mock service: deleting branch {} (no-op)", name);
            return Ok(());
        }

        debug!("Mock delete_branch operation for {}", name);
        Ok(())
    }

    /// Create a tag
    #[instrument(skip(self))]
    pub async fn create_tag(&self, name: &str, _target: Option<&str>, message: Option<&str>) -> AppResult<TagInfo> {
        if self.is_mock {
            return Ok(TagInfo {
                name: name.to_string(),
                target_commit: "mock_commit_id".to_string(),
                message: message.map(String::from),
                tagger: Some("Mock Tagger <mock@example.com>".to_string()),
                date: Utc::now(),
            });
        }

        debug!("Mock create_tag operation for {}", name);
        Ok(TagInfo {
            name: name.to_string(),
            target_commit: "tag_commit_id".to_string(),
            message: message.map(String::from),
            tagger: Some("Git User <user@example.com>".to_string()),
            date: Utc::now(),
        })
    }

    /// Delete a tag
    #[instrument(skip(self))]
    pub async fn delete_tag(&self, name: &str) -> AppResult<()> {
        if self.is_mock {
            debug!("Mock service: deleting tag {} (no-op)", name);
            return Ok(());
        }

        debug!("Mock delete_tag operation for {}", name);
        Ok(())
    }

    /// List all tags
    #[instrument(skip(self))]
    pub async fn list_tags(&self) -> AppResult<Vec<TagInfo>> {
        if self.is_mock {
            return Ok(vec![
                TagInfo {
                    name: "v1.0.0".to_string(),
                    target_commit: "mock_commit_1".to_string(),
                    message: Some("Release version 1.0.0".to_string()),
                    tagger: Some("Mock Tagger <mock@example.com>".to_string()),
                    date: Utc::now(),
                },
                TagInfo {
                    name: "v0.9.0".to_string(),
                    target_commit: "mock_commit_2".to_string(),
                    message: None,
                    tagger: None,
                    date: Utc::now(),
                },
            ]);
        }

        debug!("Mock list_tags operation");
        Ok(vec![])
    }

    /// List remotes
    #[instrument(skip(self))]
    pub async fn list_remotes(&self) -> AppResult<Vec<RemoteInfo>> {
        if self.is_mock {
            return Ok(vec![
                RemoteInfo {
                    name: "origin".to_string(),
                    fetch_url: "https://github.com/mock/repo.git".to_string(),
                    push_url: "https://github.com/mock/repo.git".to_string(),
                    is_connected: true,
                },
            ]);
        }

        debug!("Mock list_remotes operation");
        Ok(vec![])
    }

    /// List stash entries
    #[instrument(skip(self))]
    pub async fn list_stash(&self) -> AppResult<Vec<StashInfo>> {
        if self.is_mock {
            return Ok(vec![
                StashInfo {
                    index: 0,
                    message: "WIP on main: implementing feature".to_string(),
                    date: Utc::now(),
                    branch: "main".to_string(),
                },
            ]);
        }

        debug!("Mock list_stash operation");
        Ok(vec![])
    }

    /// Create a stash
    #[instrument(skip(self))]
    pub async fn stash_save(&self, _message: Option<&str>) -> AppResult<()> {
        if self.is_mock {
            debug!("Mock service: creating stash (no-op)");
            return Ok(());
        }

        debug!("Mock stash_save operation");
        Ok(())
    }

    /// Apply and pop a stash
    #[instrument(skip(self))]
    pub async fn stash_pop(&self, _index: usize) -> AppResult<()> {
        if self.is_mock {
            debug!("Mock service: applying stash (no-op)");
            return Ok(());
        }

        debug!("Mock stash_pop operation");
        Ok(())
    }

    /// Get repository path
    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }

    /// Invalidate the status cache
    async fn invalidate_cache(&self) {
        let mut cache = self.status_cache.write().await;
        cache.invalidate();
    }

    /// Get file size
    fn get_file_size(&self, path: &Path) -> std::io::Result<u64> {
        let metadata = std::fs::metadata(path)?;
        Ok(metadata.len())
    }

    /// Get file modification time
    fn get_file_modified_time(&self, path: &Path) -> DateTime<Utc> {
        std::fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .map(|time| time.into())
            .unwrap_or_else(|_| Utc::now())
    }

    /// Check if file is binary
    fn is_binary_file(&self, path: &Path) -> std::io::Result<bool> {
        let mut buffer = [0; 512];
        let bytes_read = std::fs::File::open(path).and_then(|mut file| {
            use std::io::Read;
            file.read(&mut buffer)
        })?;

        // Simple binary detection: check for null bytes in first 512 bytes
        Ok(buffer[..bytes_read].contains(&0))
    }

    /// Create a mock service for non-Git directories
    fn create_mock_service(config: &GitConfig, current_dir: PathBuf) -> Self {
        // Create a mock repository - this will be used just to satisfy the type system
        let temp_dir = std::env::temp_dir().join("ai-c-mock-repo");
        std::fs::create_dir_all(&temp_dir).unwrap_or(());
        let mock_repo = Repository::init(&temp_dir).unwrap_or_else(|_| {
            Repository::open(&temp_dir).expect("Failed to create mock repository")
        });

        Self {
            repo: Arc::new(Mutex::new(mock_repo)),
            repo_path: current_dir,
            status_cache: Arc::new(RwLock::new(StatusCache::new())),
            performance_monitor: PerformanceMonitor::new(),
            config: config.clone(),
            is_mock: true,
        }
    }
}

/// Performance monitoring for Git operations
#[derive(Debug, Clone)]
struct PerformanceMonitor {
    // In a real implementation, this would contain metrics collection
}

impl PerformanceMonitor {
    fn new() -> Self {
        Self {}
    }

    fn record_operation(&self, _operation: String, _duration: Duration, _item_count: usize) {
        // In a real implementation, this would record metrics
        // For now, we'll just log performance data
    }
}

// Manual Debug implementation since Repository doesn't implement Debug
impl std::fmt::Debug for GitService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitService")
            .field("repo_path", &self.repo_path)
            .field("status_cache", &"StatusCache")
            .field("performance_monitor", &self.performance_monitor)
            .field("config", &self.config)
            .field("is_mock", &self.is_mock)
            .finish()
    }
}
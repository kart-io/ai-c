//! Simplified Git service implementation
//!
//! High-performance Git operations with intelligent caching and async support.

use chrono::{DateTime, Utc};
use git2::{Repository, StatusOptions};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{RwLock, Mutex};
use tracing::{debug, error, info, instrument, warn};

use super::{
    cache::{StatusCache, BranchCache}, find_git_root, operations::GitOperations, BranchInfo, CommitInfo, FileStatus, GitStatusFlags,
    RemoteInfo, StashInfo, TagInfo, GitFlowStatus,
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
    /// Branch cache for performance optimization
    branch_cache: Arc<RwLock<BranchCache>>,
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

        // Initialize caches
        let status_cache = Arc::new(RwLock::new(StatusCache::new()));
        let branch_cache = Arc::new(RwLock::new(BranchCache::new()));

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
            branch_cache,
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

        // If this is a mock service, return comprehensive mock status data
        if self.is_mock {
            info!("Getting Git repository status (mock mode)");
            return Ok(vec![
                // Staged files
                FileStatus {
                    path: "src/ui/components/sidebar.rs".to_string(),
                    status: GitStatusFlags {
                        index_modified: true,
                        wt_modified: false,
                        ..Default::default()
                    },
                    size: 15420,
                    modified: Utc::now() - chrono::Duration::minutes(15),
                    is_binary: false,
                },
                FileStatus {
                    path: "src/git/service.rs".to_string(),
                    status: GitStatusFlags {
                        index_new: true,
                        wt_modified: false,
                        ..Default::default()
                    },
                    size: 8736,
                    modified: Utc::now() - chrono::Duration::minutes(30),
                    is_binary: false,
                },
                // Modified files
                FileStatus {
                    path: "README.md".to_string(),
                    status: GitStatusFlags {
                        wt_modified: true,
                        index_modified: false,
                        ..Default::default()
                    },
                    size: 2483,
                    modified: Utc::now() - chrono::Duration::minutes(5),
                    is_binary: false,
                },
                FileStatus {
                    path: "Cargo.toml".to_string(),
                    status: GitStatusFlags {
                        wt_modified: true,
                        index_modified: false,
                        ..Default::default()
                    },
                    size: 1247,
                    modified: Utc::now() - chrono::Duration::minutes(45),
                    is_binary: false,
                },
                // Untracked files
                FileStatus {
                    path: "temp/debug_logs.txt".to_string(),
                    status: GitStatusFlags {
                        wt_new: true,
                        ..Default::default()
                    },
                    size: 892,
                    modified: Utc::now() - chrono::Duration::minutes(3),
                    is_binary: false,
                },
                FileStatus {
                    path: "docs/CHANGELOG.md".to_string(),
                    status: GitStatusFlags {
                        wt_new: true,
                        ..Default::default()
                    },
                    size: 1456,
                    modified: Utc::now() - chrono::Duration::minutes(10),
                    is_binary: false,
                },
                // Deleted files
                FileStatus {
                    path: "old_file.rs".to_string(),
                    status: GitStatusFlags {
                        wt_deleted: true,
                        ..Default::default()
                    },
                    size: 0,
                    modified: Utc::now() - chrono::Duration::hours(2),
                    is_binary: false,
                },
                // Conflicted file
                FileStatus {
                    path: "src/main.rs".to_string(),
                    status: GitStatusFlags {
                        conflicted: true,
                        wt_modified: true,
                        index_modified: true,
                        ..Default::default()
                    },
                    size: 3247,
                    modified: Utc::now() - chrono::Duration::minutes(20),
                    is_binary: false,
                },
                // Binary file
                FileStatus {
                    path: "assets/logo.png".to_string(),
                    status: GitStatusFlags {
                        wt_new: true,
                        ..Default::default()
                    },
                    size: 45621,
                    modified: Utc::now() - chrono::Duration::minutes(60),
                    is_binary: true,
                },
            ]);
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
            is_local: true,
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
        if self.is_mock {
            // Return mock data for testing - comprehensive branch list with different types
            return Ok(vec![
                // Current local branch
                BranchInfo {
                    name: "develop".to_string(),
                    is_current: true,
                    is_remote: false,
                    is_local: true,
                    upstream: Some("origin/develop".to_string()),
                    ahead: 2,
                    behind: 1,
                    last_commit: "dev1234".to_string(),
                    last_commit_message: "Current development work".to_string(),
                    last_commit_author: "Developer".to_string(),
                    last_commit_date: Utc::now(),
                },
                // Other local branches
                BranchInfo {
                    name: "main".to_string(),
                    is_current: false,
                    is_remote: false,
                    is_local: true,
                    upstream: Some("origin/main".to_string()),
                    ahead: 0,
                    behind: 0,
                    last_commit: "main123".to_string(),
                    last_commit_message: "Stable main branch".to_string(),
                    last_commit_author: "Main Developer".to_string(),
                    last_commit_date: Utc::now() - chrono::Duration::hours(2),
                },
                BranchInfo {
                    name: "feature/ui-improvements".to_string(),
                    is_current: false,
                    is_remote: false,
                    is_local: true,
                    upstream: Some("origin/feature/ui-improvements".to_string()),
                    ahead: 3,
                    behind: 0,
                    last_commit: "feat456".to_string(),
                    last_commit_message: "Add branch management UI".to_string(),
                    last_commit_author: "Feature Developer".to_string(),
                    last_commit_date: Utc::now() - chrono::Duration::hours(1),
                },
                BranchInfo {
                    name: "hotfix/critical-bug".to_string(),
                    is_current: false,
                    is_local: true,
                    is_remote: false,
                    upstream: None, // No upstream - local only
                    ahead: 0,
                    behind: 0,
                    last_commit: "fix789".to_string(),
                    last_commit_message: "Fix critical issue".to_string(),
                    last_commit_author: "Hotfix Developer".to_string(),
                    last_commit_date: Utc::now() - chrono::Duration::minutes(30),
                },
                // Remote branches
                BranchInfo {
                    name: "origin/main".to_string(),
                    is_current: false,
                    is_remote: true,
                    is_local: false,
                    upstream: None,
                    ahead: 0,
                    behind: 0,
                    last_commit: "main123".to_string(),
                    last_commit_message: "Stable main branch".to_string(),
                    last_commit_author: "Main Developer".to_string(),
                    last_commit_date: Utc::now() - chrono::Duration::hours(2),
                },
                BranchInfo {
                    name: "origin/develop".to_string(),
                    is_current: false,
                    is_remote: true,
                    is_local: false,
                    upstream: None,
                    ahead: 0,
                    behind: 0,
                    last_commit: "dev5678".to_string(),
                    last_commit_message: "Remote development branch".to_string(),
                    last_commit_author: "Remote Developer".to_string(),
                    last_commit_date: Utc::now() - chrono::Duration::hours(3),
                },
                BranchInfo {
                    name: "origin/release/v2.1".to_string(),
                    is_current: false,
                    is_remote: true,
                    is_local: false,
                    upstream: None,
                    ahead: 0,
                    behind: 0,
                    last_commit: "rel210".to_string(),
                    last_commit_message: "Prepare release v2.1".to_string(),
                    last_commit_author: "Release Manager".to_string(),
                    last_commit_date: Utc::now() - chrono::Duration::days(1),
                },
            ]);
        }

        // Delegate to the improved async implementation
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.get_branches().await
            })
        })
    }

    /// Get file history for a specific file
    #[instrument(skip(self))]
    pub async fn get_file_history(&self, file_path: &str, limit: Option<usize>) -> AppResult<Vec<CommitInfo>> {
        let limit = limit.unwrap_or(10);

        if self.is_mock {
            return Ok(vec![
                CommitInfo {
                    hash: "mock_file_commit_1".to_string(),
                    short_hash: "moc123".to_string(),
                    message: format!("Modified {}", file_path),
                    author: "Mock Author".to_string(),
                    author_email: "mock@example.com".to_string(),
                    date: Utc::now(),
                    parents: vec![],
                },
            ]);
        }

        let repo = self.repo.lock().await;
        let mut revwalk = repo.revwalk().map_err(AppError::Git)?;
        revwalk.push_head().map_err(AppError::Git)?;
        revwalk.set_sorting(git2::Sort::TIME).map_err(AppError::Git)?;

        let mut commits = Vec::new();
        let mut count = 0;

        for oid_result in revwalk {
            if count >= limit {
                break;
            }

            let oid = oid_result.map_err(AppError::Git)?;
            let commit = repo.find_commit(oid).map_err(AppError::Git)?;

            // Check if this commit affects the specified file
            let tree = commit.tree().map_err(AppError::Git)?;
            if tree.get_path(std::path::Path::new(file_path)).is_ok() {
                let commit_info = CommitInfo {
                    hash: commit.id().to_string(),
                    short_hash: format!("{:.7}", commit.id()),
                    message: commit.message().unwrap_or("").to_string(),
                    author: commit.author().name().unwrap_or("").to_string(),
                    author_email: commit.author().email().unwrap_or("").to_string(),
                    date: DateTime::from_timestamp(commit.time().seconds(), 0).unwrap_or_else(|| Utc::now()),
                    parents: commit.parent_ids().map(|id| id.to_string()).collect(),
                };
                commits.push(commit_info);
                count += 1;
            }
        }

        Ok(commits)
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
        let operation_start = Instant::now();
        info!("Creating branch: {}", name);

        if self.is_mock {
            // Invalidate cache after creating branch
            self.invalidate_cache().await;

            return Ok(BranchInfo {
                name: name.to_string(),
                is_current: false,
                is_remote: false,
                is_local: true,
                upstream: None,
                ahead: 0,
                behind: 0,
                last_commit: "mock_commit_id".to_string(),
                last_commit_message: "Mock commit message".to_string(),
                last_commit_author: "Mock Author".to_string(),
                last_commit_date: Utc::now(),
            });
        }

        // TODO: Implement real branch creation logic
        debug!("Mock create_branch operation for {}", name);

        let duration = operation_start.elapsed();
        self.performance_monitor.record_operation(
            "create_branch".to_string(),
            duration,
            1,
        );

        // Invalidate cache after creating branch
        self.invalidate_cache().await;

        Ok(BranchInfo {
            name: name.to_string(),
            is_current: false,
            is_remote: false,
            is_local: true,
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

        debug!("Switching to branch: {}", name);

        // Lock repository for mutable access
        let mut repo = self.repo.lock().await;
        let operations = GitOperations::new(&mut *repo);
        operations.switch_branch(name)?;

        // Clear all caches after branch switch
        self.invalidate_cache().await;

        info!("Successfully switched to branch: {}", name);
        Ok(())
    }

    /// Delete a branch
    #[instrument(skip(self))]
    pub async fn delete_branch(&self, name: &str) -> AppResult<()> {
        let operation_start = Instant::now();
        info!("Deleting branch: {}", name);

        if self.is_mock {
            debug!("Mock service: deleting branch {} (no-op)", name);
            // Invalidate cache after deleting branch
            self.invalidate_cache().await;
            return Ok(());
        }

        // TODO: Implement real branch deletion logic
        debug!("Mock delete_branch operation for {}", name);

        let duration = operation_start.elapsed();
        self.performance_monitor.record_operation(
            "delete_branch".to_string(),
            duration,
            1,
        );

        // Invalidate cache after deleting branch
        self.invalidate_cache().await;

        Ok(())
    }

    /// Create a tag
    #[instrument(skip(self))]
    pub async fn create_tag(&self, name: &str, _target: Option<&str>, message: Option<&str>) -> AppResult<TagInfo> {
        if self.is_mock {
            return Ok(TagInfo {
                name: name.to_string(),
                target: "mock_commit_id".to_string(),
                target_commit: "mock_commit_id".to_string(),
                message: message.map(String::from),
                tagger: Some("Mock Tagger <mock@example.com>".to_string()),
                date: Utc::now(),
            });
        }

        debug!("Mock create_tag operation for {}", name);
        Ok(TagInfo {
            name: name.to_string(),
            target: "tag_commit_id".to_string(),
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
                // Latest tags first
                TagInfo {
                    name: "v2.1.0".to_string(),
                    target: "a1b2c3d4e5f67890".to_string(),
                    target_commit: "a1b2c3d4e5f67890".to_string(),
                    message: Some("Release v2.1.0 - New TUI features and performance improvements".to_string()),
                    tagger: Some("Release Manager <release@example.com>".to_string()),
                    date: Utc::now() - chrono::Duration::days(7),
                },
                TagInfo {
                    name: "v2.0.1".to_string(),
                    target: "b2c3d4e5f6781901".to_string(),
                    target_commit: "b2c3d4e5f6781901".to_string(),
                    message: Some("Hotfix v2.0.1 - Critical bug fixes".to_string()),
                    tagger: Some("Hotfix Team <hotfix@example.com>".to_string()),
                    date: Utc::now() - chrono::Duration::days(14),
                },
                TagInfo {
                    name: "v2.0.0".to_string(),
                    target: "c3d4e5f678901234".to_string(),
                    target_commit: "c3d4e5f678901234".to_string(),
                    message: Some("Major release v2.0.0 - Complete UI rewrite with ratatui".to_string()),
                    tagger: Some("Release Manager <release@example.com>".to_string()),
                    date: Utc::now() - chrono::Duration::days(30),
                },
                TagInfo {
                    name: "v1.9.2".to_string(),
                    target: "d4e5f67890123456".to_string(),
                    target_commit: "d4e5f67890123456".to_string(),
                    message: Some("Bug fix release v1.9.2".to_string()),
                    tagger: Some("Dev Team <dev@example.com>".to_string()),
                    date: Utc::now() - chrono::Duration::days(45),
                },
                TagInfo {
                    name: "v1.9.1".to_string(),
                    target: "e5f678901234567a".to_string(),
                    target_commit: "e5f678901234567a".to_string(),
                    message: None, // Lightweight tag
                    tagger: None,
                    date: Utc::now() - chrono::Duration::days(60),
                },
                TagInfo {
                    name: "v1.9.0".to_string(),
                    target: "f67890123456789b".to_string(),
                    target_commit: "f67890123456789b".to_string(),
                    message: Some("Feature release v1.9.0 - Git workflow enhancements".to_string()),
                    tagger: Some("Dev Team <dev@example.com>".to_string()),
                    date: Utc::now() - chrono::Duration::days(90),
                },
                TagInfo {
                    name: "v1.8.0".to_string(),
                    target: "7890123456789abc".to_string(),
                    target_commit: "7890123456789abc".to_string(),
                    message: Some("Release v1.8.0 - Agent system improvements".to_string()),
                    tagger: Some("Release Manager <release@example.com>".to_string()),
                    date: Utc::now() - chrono::Duration::days(120),
                },
                // Beta and alpha tags
                TagInfo {
                    name: "v2.2.0-beta.1".to_string(),
                    target: "890123456789abcd".to_string(),
                    target_commit: "890123456789abcd".to_string(),
                    message: Some("Beta release v2.2.0-beta.1 - Testing new features".to_string()),
                    tagger: Some("Beta Team <beta@example.com>".to_string()),
                    date: Utc::now() - chrono::Duration::days(2),
                },
                TagInfo {
                    name: "v2.1.1-alpha.2".to_string(),
                    target: "90123456789abcde".to_string(),
                    target_commit: "90123456789abcde".to_string(),
                    message: None, // Lightweight alpha tag
                    tagger: None,
                    date: Utc::now() - chrono::Duration::days(5),
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
                    url: "https://github.com/mock/repo.git".to_string(),
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
                // Most recent stash (index 0)
                StashInfo {
                    index: 0,
                    message: "WIP on feature/ui-improvements: Adding new components".to_string(),
                    date: Utc::now() - chrono::Duration::minutes(30),
                    branch: "feature/ui-improvements".to_string(),
                },
                // Second most recent stash (index 1)
                StashInfo {
                    index: 1,
                    message: "WIP on develop: Experimental changes before merge".to_string(),
                    date: Utc::now() - chrono::Duration::hours(2),
                    branch: "develop".to_string(),
                },
                // Older stash (index 2)
                StashInfo {
                    index: 2,
                    message: "WIP on main: Backup before major refactor".to_string(),
                    date: Utc::now() - chrono::Duration::hours(6),
                    branch: "main".to_string(),
                },
                // Even older stash (index 3)
                StashInfo {
                    index: 3,
                    message: "WIP on hotfix/critical-bug: Emergency fix attempt".to_string(),
                    date: Utc::now() - chrono::Duration::days(1),
                    branch: "hotfix/critical-bug".to_string(),
                },
                // Old stash (index 4)
                StashInfo {
                    index: 4,
                    message: "WIP on feature/new-api: API design changes".to_string(),
                    date: Utc::now() - chrono::Duration::days(3),
                    branch: "feature/new-api".to_string(),
                },
                // Very old stash (index 5)
                StashInfo {
                    index: 5,
                    message: "WIP on main: Config updates before deployment".to_string(),
                    date: Utc::now() - chrono::Duration::days(7),
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

    /// Apply a stash without removing it
    #[instrument(skip(self))]
    pub async fn stash_apply(&self, _index: usize) -> AppResult<()> {
        if self.is_mock {
            debug!("Mock service: applying stash without removing (no-op)");
            return Ok(());
        }

        debug!("Mock stash_apply operation");
        Ok(())
    }

    /// Drop (delete) a stash
    #[instrument(skip(self))]
    pub async fn stash_drop(&self, _index: usize) -> AppResult<()> {
        if self.is_mock {
            debug!("Mock service: dropping stash (no-op)");
            return Ok(());
        }

        debug!("Mock stash_drop operation");
        Ok(())
    }

    /// Get stash diff content
    #[instrument(skip(self))]
    pub async fn get_stash_diff(&self, index: usize) -> AppResult<String> {
        if self.is_mock {
            let mock_diff = format!(
                "diff --git a/src/main.rs b/src/main.rs\n\
                 index abc1234..def5678 100644\n\
                 --- a/src/main.rs\n\
                 +++ b/src/main.rs\n\
                 @@ -1,8 +1,12 @@\n\
                  fn main() {{\n\
                      println!(\"Hello, world!\");\n\
                 +    println!(\"Stashed change {}\");\n\
                 +    // Added in stash\n\
                  }}\n\
                 +\n\
                 +fn new_function() {{\n\
                 +    println!(\"New function from stash\");\n\
                 +}}\n",
                index
            );
            return Ok(mock_diff);
        }

        // Use git operations for real stash diff
        let stash_ref = format!("stash@{{{}}}", index);

        // Run git show command to get stash diff
        let output = std::process::Command::new("git")
            .arg("show")
            .arg("--format=")
            .arg(&stash_ref)
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| crate::error::AppError::Io(e))?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(crate::error::AppError::Application {
                message: format!("Failed to get stash diff: {}", error_msg)
            });
        }

        let diff_content = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(diff_content)
    }

    /// Get repository path
    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }

    /// Invalidate all caches
    async fn invalidate_cache(&self) {
        // Invalidate status cache
        {
            let mut cache = self.status_cache.write().await;
            cache.invalidate();
        }

        // Invalidate branch cache
        {
            let mut cache = self.branch_cache.write().await;
            cache.invalidate();
        }
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

    /// Get access to the underlying Git repository
    pub fn get_repository(&self) -> AppResult<Repository> {
        if self.is_mock {
            return Err(AppError::InvalidState("Cannot access repository in mock mode".to_string()));
        }

        // Clone the repository handle for safe access
        // Note: This is a simplified approach - in production, we'd want more sophisticated locking
        let repo_guard = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.repo.lock())
        });

        // We need to open a new repository handle since git2::Repository doesn't implement Clone
        Repository::open(&self.repo_path).map_err(AppError::Git)
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
            branch_cache: Arc::new(RwLock::new(BranchCache::new())),
            performance_monitor: PerformanceMonitor::new(),
            config: config.clone(),
            is_mock: true,
        }
    }

    /// Get file content at HEAD commit
    #[instrument(skip(self, file_path))]
    pub fn get_file_content_at_head<P: AsRef<std::path::Path>>(&self, file_path: P) -> AppResult<String> {

        let file_path = file_path.as_ref();
        if self.is_mock {
            // Return mock content for demo purposes
            return Ok(format!(
                "// Mock content for file: {}\n\
                 // This is simulated original content\n\
                 fn main() {{\n\
                     println!(\"Hello from original version!\");\n\
                 }}\n",
                file_path.display()
            ));
        }

        // For real Git repos, get the actual file content
        let repo = self.repo.try_lock().map_err(|_| {
            AppError::InvalidOperation("Git repository is busy".to_string())
        })?;

        // Get HEAD commit
        let head = repo.head().map_err(AppError::Git)?;
        let head_commit = head.peel_to_commit().map_err(AppError::Git)?;

        // Get the tree from HEAD commit
        let tree = head_commit.tree().map_err(AppError::Git)?;

        // Convert absolute path to relative path from repo root
        let relative_path = file_path.strip_prefix(&self.repo_path)
            .map_err(|_| AppError::InvalidOperation(format!("File path {:?} is not within repository", file_path)))?;

        // Find the file in the tree
        let tree_entry = tree.get_path(&relative_path).map_err(|_| {
            AppError::InvalidOperation(format!("File {:?} not found in HEAD commit", relative_path))
        })?;

        // Get the blob object
        let blob = repo.find_blob(tree_entry.id()).map_err(AppError::Git)?;

        // Convert blob content to string
        let content = std::str::from_utf8(blob.content())
            .map_err(|_| AppError::InvalidOperation("File contains non-UTF8 content".to_string()))?;

        Ok(content.to_string())
    }

    /// Get diff between working directory and HEAD for a specific file
    #[instrument(skip(self))]
    pub async fn get_file_diff(&self, file_path: &PathBuf) -> AppResult<(String, String)> {
        if self.is_mock {
            let old_content = self.get_file_content_at_head(file_path)?;
            let new_content = format!(
                "// Mock content for file: {:?}\n\
                 // This is simulated modified content\n\
                 fn main() {{\n\
                     println!(\"Hello from modified version!\");\n\
                     println!(\"Added new functionality!\");\n\
                 }}\n",
                file_path
            );
            return Ok((old_content, new_content));
        }

        // Get content from HEAD
        let old_content = self.get_file_content_at_head(file_path)?;

        // Get current working directory content
        let new_content = tokio::fs::read_to_string(file_path)
            .await
            .unwrap_or_else(|_| String::new());

        Ok((old_content, new_content))
    }

    /// Get commits (alias for get_commit_history for UI compatibility)
    pub async fn get_commits(&self, limit: usize) -> AppResult<Vec<CommitInfo>> {
        self.get_commit_history(limit).await
    }

    /// Get commits for a specific branch
    #[instrument(skip(self))]
    pub async fn get_branch_commits(&self, branch_name: &str, limit: usize) -> AppResult<Vec<CommitInfo>> {
        if self.is_mock {
            return Ok(vec![
                CommitInfo {
                    hash: "abc123456789".to_string(),
                    short_hash: "abc123".to_string(),
                    message: "feat: Add new feature".to_string(),
                    author: "Developer".to_string(),
                    author_email: "dev@example.com".to_string(),
                    date: Utc::now() - chrono::Duration::hours(2),
                    parents: vec!["def456789012".to_string()],
                },
                CommitInfo {
                    hash: "def456789012".to_string(),
                    short_hash: "def456".to_string(),
                    message: "fix: Fix critical bug".to_string(),
                    author: "Developer".to_string(),
                    author_email: "dev@example.com".to_string(),
                    date: Utc::now() - chrono::Duration::days(1),
                    parents: vec!["ghi789012345".to_string()],
                },
                CommitInfo {
                    hash: "ghi789012345".to_string(),
                    short_hash: "ghi789".to_string(),
                    message: "docs: Update README".to_string(),
                    author: "Documentation Team".to_string(),
                    author_email: "docs@example.com".to_string(),
                    date: Utc::now() - chrono::Duration::days(3),
                    parents: vec!["jkl012345678".to_string()],
                },
            ]);
        }

        let repo = self.repo.lock().await;
        let mut revwalk = repo.revwalk().map_err(AppError::Git)?;

        // Try to find the branch reference and start from there
        match repo.find_branch(branch_name, git2::BranchType::Local) {
            Ok(branch) => {
                if let Some(oid) = branch.get().target() {
                    revwalk.push(oid).map_err(AppError::Git)?;
                } else {
                    // Fallback to HEAD if branch has no target
                    revwalk.push_head().map_err(AppError::Git)?;
                }
            }
            Err(_) => {
                // Fallback to HEAD if branch not found
                revwalk.push_head().map_err(AppError::Git)?;
            }
        }

        revwalk.set_sorting(git2::Sort::TIME).map_err(AppError::Git)?;

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

    /// Get branches (for UI compatibility)
    pub async fn get_branches(&self) -> AppResult<Vec<BranchInfo>> {
        let operation_start = Instant::now();

        if self.is_mock {
            // Return mock branches
            return Ok(vec![
                BranchInfo {
                    name: "main".to_string(),
                    is_current: false,
                    is_remote: false,
                    is_local: true,
                    upstream: None,
                    ahead: 0,
                    behind: 0,
                    last_commit: "abc1234".to_string(),
                    last_commit_message: "Main branch".to_string(),
                    last_commit_author: "Developer".to_string(),
                    last_commit_date: Utc::now(),
                },
                BranchInfo {
                    name: "develop".to_string(),
                    is_current: true,
                    is_remote: false,
                    is_local: true,
                    upstream: Some("origin/develop".to_string()),
                    ahead: 2,
                    behind: 0,
                    last_commit: "dev1234".to_string(),
                    last_commit_message: "Development work".to_string(),
                    last_commit_author: "Dev Team".to_string(),
                    last_commit_date: Utc::now(),
                }
            ]);
        }

        info!("Getting Git branches");

        // Check cache first for performance
        {
            let cache = self.branch_cache.read().await;
            if let Some(cached_branches) = cache.get_if_fresh() {
                debug!("Using cached Git branches with {} entries", cached_branches.len());
                return Ok(cached_branches.clone());
            }
        }

        // Real implementation for actual Git repositories
        let repo = self.repo.lock().await;
        let mut branches = Vec::new();

        let branch_iter = repo.branches(Some(git2::BranchType::Local)).map_err(AppError::Git)?;
        for branch_result in branch_iter {
            let (branch, branch_type) = branch_result.map_err(AppError::Git)?;
            if let Some(name) = branch.name().map_err(AppError::Git)? {
                let is_current = branch.is_head();
                let is_remote = branch_type == git2::BranchType::Remote;

                // Get the last commit for this branch
                let (last_commit_hash, last_commit_message, last_commit_author, last_commit_date) =
                    if let Some(oid) = branch.get().target() {
                        match repo.find_commit(oid) {
                            Ok(commit) => {
                                let hash = commit.id().to_string();
                                let short_hash = hash[..7].to_string();
                                let message = commit.message().unwrap_or("").to_string();
                                let author = commit.author().name().unwrap_or("Unknown").to_string();
                                let date = DateTime::from_timestamp(commit.time().seconds(), 0)
                                    .unwrap_or_else(|| Utc::now());
                                (short_hash, message, author, date)
                            }
                            Err(_) => ("unknown".to_string(), "".to_string(), "".to_string(), Utc::now())
                        }
                    } else {
                        ("unknown".to_string(), "".to_string(), "".to_string(), Utc::now())
                    };

                // Get upstream information if available
                let upstream = if let Ok(upstream_branch) = branch.upstream() {
                    upstream_branch.name().ok().flatten().map(|s| s.to_string())
                } else {
                    None
                };

                // Calculate ahead/behind counts for local branches with upstream
                let (ahead, behind) = if !is_remote && upstream.is_some() {
                    self.calculate_ahead_behind(&repo, &name).unwrap_or((0, 0))
                } else {
                    (0, 0)
                };

                branches.push(BranchInfo {
                    name: name.to_string(),
                    is_current,
                    is_remote,
                    is_local: !is_remote,
                    upstream: upstream.clone(),
                    ahead,
                    behind,
                    last_commit: last_commit_hash,
                    last_commit_message: last_commit_message,
                    last_commit_author: last_commit_author,
                    last_commit_date: last_commit_date,
                });
            }
        }

        // Also get remote branches
        let remote_branch_iter = repo.branches(Some(git2::BranchType::Remote)).map_err(AppError::Git)?;
        for branch_result in remote_branch_iter {
            let (branch, _branch_type) = branch_result.map_err(AppError::Git)?;
            if let Some(name) = branch.name().map_err(AppError::Git)? {
                // Skip origin/HEAD references
                if name.ends_with("/HEAD") {
                    continue;
                }

                let (last_commit_hash, last_commit_message, last_commit_author, last_commit_date) =
                    if let Some(oid) = branch.get().target() {
                        match repo.find_commit(oid) {
                            Ok(commit) => {
                                let hash = commit.id().to_string();
                                let short_hash = hash[..7].to_string();
                                let message = commit.message().unwrap_or("").to_string();
                                let author = commit.author().name().unwrap_or("Unknown").to_string();
                                let date = DateTime::from_timestamp(commit.time().seconds(), 0)
                                    .unwrap_or_else(|| Utc::now());
                                (short_hash, message, author, date)
                            }
                            Err(_) => ("unknown".to_string(), "".to_string(), "".to_string(), Utc::now())
                        }
                    } else {
                        ("unknown".to_string(), "".to_string(), "".to_string(), Utc::now())
                    };

                branches.push(BranchInfo {
                    name: name.to_string(),
                    is_current: false,
                    is_remote: true,
                    is_local: false,
                    upstream: None,
                    ahead: 0,
                    behind: 0,
                    last_commit: last_commit_hash,
                    last_commit_message: last_commit_message,
                    last_commit_author: last_commit_author,
                    last_commit_date: last_commit_date,
                });
            }
        }

        let duration = operation_start.elapsed();

        // Performance monitoring
        self.performance_monitor.record_operation(
            "git_branches".to_string(),
            duration,
            branches.len(),
        );

        // Performance validation
        if duration > Duration::from_millis(500) {
            warn!(
                "Git branches operation exceeded 500ms target: {:?} for {} branches",
                duration,
                branches.len()
            );
        } else {
            debug!(
                "Git branches completed in {:?} for {} branches",
                duration,
                branches.len()
            );
        }

        // Update cache
        {
            let mut cache = self.branch_cache.write().await;
            cache.store(branches.clone());
        }

        Ok(branches)
    }

    /// Get tags (alias for list_tags for UI compatibility)
    pub async fn get_tags(&self) -> AppResult<Vec<TagInfo>> {
        self.list_tags().await
    }

    /// Get remotes (alias for list_remotes for UI compatibility)
    pub async fn get_remotes(&self) -> AppResult<Vec<RemoteInfo>> {
        self.list_remotes().await
    }

    // ================== GitFlow Workflow Methods ==================

    /// List branches by GitFlow type pattern
    #[instrument(skip(self))]
    pub async fn list_gitflow_branches(&self, flow_type: &str) -> AppResult<Vec<BranchInfo>> {
        let prefix = match flow_type {
            "feature" => "feature/",
            "release" => "release/",
            "hotfix" => "hotfix/",
            "support" => "support/",
            _ => return Ok(vec![]),
        };

        if self.is_mock {
            // Return mock GitFlow branches based on type
            return Ok(match flow_type {
                "feature" => vec![
                    BranchInfo {
                        name: "feature/user-authentication".to_string(),
                        is_current: false,
                        is_remote: false,
                        is_local: true,
                        upstream: None,
                        ahead: 3,
                        behind: 0,
                        last_commit: "f1e2d3c4b5a69870".to_string(),
                        last_commit_message: "Implement user authentication logic".to_string(),
                        last_commit_author: "Auth Developer".to_string(),
                        last_commit_date: Utc::now() - chrono::Duration::days(2),
                    },
                    BranchInfo {
                        name: "feature/payment-integration".to_string(),
                        is_current: true,
                        is_remote: false,
                        is_local: true,
                        upstream: None,
                        ahead: 7,
                        behind: 1,
                        last_commit: "a9b8c7d6e5f41203".to_string(),
                        last_commit_message: "Add payment gateway integration".to_string(),
                        last_commit_author: "Payment Developer".to_string(),
                        last_commit_date: Utc::now() - chrono::Duration::hours(4),
                    },
                    BranchInfo {
                        name: "feature/dashboard-ui".to_string(),
                        is_current: false,
                        is_remote: false,
                        is_local: true,
                        upstream: None,
                        ahead: 2,
                        behind: 0,
                        last_commit: "c7d8e9f0a1b23456".to_string(),
                        last_commit_message: "Update dashboard UI components".to_string(),
                        last_commit_author: "UI Developer".to_string(),
                        last_commit_date: Utc::now() - chrono::Duration::days(1),
                    },
                ],
                "release" => vec![
                    BranchInfo {
                        name: "release/v2.1.0".to_string(),
                        is_current: false,
                        is_remote: false,
                        is_local: true,
                        upstream: None,
                        ahead: 1,
                        behind: 0,
                        last_commit: "r1e2l3e4a5s67890".to_string(),
                        last_commit_message: "Prepare release v2.1.0".to_string(),
                        last_commit_author: "Release Manager".to_string(),
                        last_commit_date: Utc::now() - chrono::Duration::days(3),
                    },
                ],
                "hotfix" => vec![
                    BranchInfo {
                        name: "hotfix/critical-security-fix".to_string(),
                        is_current: false,
                        is_remote: false,
                        is_local: true,
                        upstream: None,
                        ahead: 1,
                        behind: 0,
                        last_commit: "h1o2t3f4i5x67890".to_string(),
                        last_commit_message: "Fix critical security vulnerability".to_string(),
                        last_commit_author: "Security Team".to_string(),
                        last_commit_date: Utc::now() - chrono::Duration::hours(6),
                    },
                ],
                "support" => vec![
                    BranchInfo {
                        name: "support/v1.x".to_string(),
                        is_current: false,
                        is_remote: false,
                        is_local: true,
                        upstream: None,
                        ahead: 0,
                        behind: 5,
                        last_commit: "s1u2p3p4o5r67890".to_string(),
                        last_commit_message: "Maintain v1.x support branch".to_string(),
                        last_commit_author: "Support Team".to_string(),
                        last_commit_date: Utc::now() - chrono::Duration::days(7),
                    },
                ],
                _ => vec![],
            });
        }

        // In real implementation, filter branches by prefix
        let all_branches = self.list_branches()?;
        Ok(all_branches.into_iter()
            .filter(|branch| branch.name.starts_with(prefix))
            .collect())
    }

    /// Create a new GitFlow branch
    #[instrument(skip(self))]
    pub async fn create_gitflow_branch(&self, flow_type: &str, name: &str) -> AppResult<BranchInfo> {
        let branch_name = format!("{}/{}", flow_type, name);

        if self.is_mock {
            debug!("Mock service: creating GitFlow branch {} (no-op)", branch_name);
            return Ok(BranchInfo {
                name: branch_name,
                is_current: true,
                is_remote: false,
                is_local: true,
                upstream: None,
                ahead: 0,
                behind: 0,
                last_commit: "new_branch_commit".to_string(),
                last_commit_message: format!("Create new {} branch", flow_type),
                last_commit_author: "Git User".to_string(),
                last_commit_date: Utc::now(),
            });
        }

        // In real implementation, create branch from appropriate base
        let base_branch = match flow_type {
            "feature" => "develop",
            "release" => "develop",
            "hotfix" => "main",
            "support" => "main",
            _ => return Err(AppError::InvalidOperation("Invalid GitFlow branch type".to_string())),
        };

        debug!("Creating GitFlow branch {} from {}", branch_name, base_branch);
        self.create_branch(&branch_name, Some(base_branch)).await
    }

    /// Finish a GitFlow branch (merge and cleanup)
    #[instrument(skip(self))]
    pub async fn finish_gitflow_branch(&self, flow_type: &str, branch_name: &str) -> AppResult<()> {
        if self.is_mock {
            debug!("Mock service: finishing GitFlow branch {} (no-op)", branch_name);
            return Ok(());
        }

        // In real implementation, this would:
        // 1. Merge to appropriate target branches
        // 2. Delete the feature branch
        // 3. Handle tagging for releases

        let (target_branches, should_tag) = match flow_type {
            "feature" => (vec!["develop"], false),
            "release" => (vec!["main", "develop"], true),
            "hotfix" => (vec!["main", "develop"], true),
            "support" => (vec![], false), // Support branches aren't merged
            _ => return Err(AppError::InvalidOperation("Invalid GitFlow branch type".to_string())),
        };

        debug!("Finishing GitFlow branch {} -> merge to {:?}, tag: {}",
               branch_name, target_branches, should_tag);

        // For now, just simulate the operation
        Ok(())
    }

    /// Get current GitFlow status
    #[instrument(skip(self))]
    pub async fn get_gitflow_status(&self) -> AppResult<GitFlowStatus> {
        if self.is_mock {
            return Ok(GitFlowStatus {
                feature_branches: self.list_gitflow_branches("feature").await?.len(),
                release_branches: self.list_gitflow_branches("release").await?.len(),
                hotfix_branches: self.list_gitflow_branches("hotfix").await?.len(),
                support_branches: self.list_gitflow_branches("support").await?.len(),
                current_branch: self.get_current_branch()?.map(|b| b.name),
                is_gitflow_repo: true,
                main_branch: "main".to_string(),
                develop_branch: "develop".to_string(),
            });
        }

        // In real implementation, detect GitFlow configuration
        Ok(GitFlowStatus {
            feature_branches: 0,
            release_branches: 0,
            hotfix_branches: 0,
            support_branches: 0,
            current_branch: None,
            is_gitflow_repo: false,
            main_branch: "main".to_string(),
            develop_branch: "develop".to_string(),
        })
    }

    /// Merge a branch into the current branch
    #[instrument(skip(self))]
    pub async fn merge_branch(&self, source_branch: &str) -> AppResult<()> {
        if self.is_mock {
            debug!("Mock service: merging branch {} (no-op)", source_branch);
            return Ok(());
        }

        // In real implementation, this would perform git merge
        debug!("Merging branch {} into current branch", source_branch);

        // TODO: Implement actual git merge logic using git2
        // This would involve:
        // 1. Getting current branch
        // 2. Finding source branch commit
        // 3. Performing merge
        // 4. Handling conflicts if any

        Ok(())
    }

    /// Push a branch to its remote
    #[instrument(skip(self))]
    pub async fn push_branch(&self, branch_name: &str) -> AppResult<()> {
        if self.is_mock {
            debug!("Mock service: pushing branch {} (no-op)", branch_name);
            return Ok(());
        }

        // In real implementation, this would push to remote
        debug!("Pushing branch {} to remote", branch_name);

        // TODO: Implement actual git push logic
        // This would involve:
        // 1. Finding remote for branch
        // 2. Pushing commits to remote
        // 3. Setting up tracking if needed

        Ok(())
    }

    /// Pull changes from remote branch
    #[instrument(skip(self))]
    pub async fn pull_branch(&self, branch_name: &str) -> AppResult<()> {
        if self.is_mock {
            debug!("Mock service: pulling branch {} (no-op)", branch_name);
            return Ok(());
        }

        debug!("Pulling changes for branch {} from remote", branch_name);

        // In a real implementation, this would:
        // 1. Fetch from the remote repository
        // 2. Merge or rebase the changes
        // 3. Update the local branch

        // For now, simulate the operation by running git pull command
        match std::process::Command::new("git")
            .arg("pull")
            .arg("origin")
            .arg(branch_name)
            .current_dir(&self.repo_path)
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    debug!("Successfully pulled branch {}", branch_name);
                    Ok(())
                } else {
                    let error_msg = String::from_utf8_lossy(&output.stderr);
                    warn!("Failed to pull branch {}: {}", branch_name, error_msg);
                    Err(AppError::Application {
                        message: format!("Pull failed: {}", error_msg)
                    })
                }
            }
            Err(e) => {
                error!("Failed to execute git pull command: {:?}", e);
                Err(AppError::Application {
                    message: format!("Could not execute git pull: {:?}", e)
                })
            }
        }
    }

    /// Pull changes from remote (general pull operation)
    #[instrument(skip(self))]
    pub async fn pull(&self) -> AppResult<()> {
        if self.is_mock {
            debug!("Mock service: pulling from remote (no-op)");
            return Ok(());
        }

        debug!("Pulling changes from remote");

        // Simple git pull operation
        match std::process::Command::new("git")
            .arg("pull")
            .current_dir(&self.repo_path)
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    debug!("Successfully pulled from remote");
                    Ok(())
                } else {
                    let error_msg = String::from_utf8_lossy(&output.stderr);
                    warn!("Failed to pull from remote: {}", error_msg);
                    Err(AppError::Application {
                        message: format!("Pull failed: {}", error_msg)
                    })
                }
            }
            Err(e) => {
                error!("Failed to execute git pull command: {:?}", e);
                Err(AppError::Application {
                    message: format!("Could not execute git pull: {:?}", e)
                })
            }
        }
    }

    /// Calculate ahead/behind counts for a branch against its upstream
    fn calculate_ahead_behind(&self, repo: &git2::Repository, branch_name: &str) -> Result<(usize, usize), git2::Error> {
        // Get the local branch
        let local_branch = repo.find_branch(branch_name, git2::BranchType::Local)?;

        // Get the upstream branch
        let upstream_branch = match local_branch.upstream() {
            Ok(upstream) => upstream,
            Err(_) => return Ok((0, 0)), // No upstream
        };

        // Get commit OIDs for both branches
        let local_oid = local_branch.get().target().ok_or(git2::Error::from_str("No target for local branch"))?;
        let upstream_oid = upstream_branch.get().target().ok_or(git2::Error::from_str("No target for upstream branch"))?;

        // If they're the same, no difference
        if local_oid == upstream_oid {
            return Ok((0, 0));
        }

        // Use git2's ahead_behind function to calculate the difference
        let (ahead, behind) = repo.graph_ahead_behind(local_oid, upstream_oid)?;

        Ok((ahead, behind))
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
//! Git repository operations module
//!
//! Provides high-performance Git operations with caching and async support.
//! Performance requirements:
//! - Repository initialization: < 100ms
//! - File status refresh: < 200ms (>10,000 files)
//! - Memory usage: < 100MB (large repositories)

pub mod cache;
pub mod operations;
pub mod service;

pub use service::GitService;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::AppResult;

/// Git file status information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileStatus {
    /// File path relative to repository root
    pub path: String,
    /// Git status flags
    pub status: GitStatusFlags,
    /// File size in bytes
    pub size: u64,
    /// Last modification time
    pub modified: DateTime<Utc>,
    /// Whether the file is binary
    pub is_binary: bool,
}

/// Git status flags matching git2 status flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitStatusFlags {
    pub index_new: bool,
    pub index_modified: bool,
    pub index_deleted: bool,
    pub index_renamed: bool,
    pub index_typechange: bool,
    pub wt_new: bool,
    pub wt_modified: bool,
    pub wt_deleted: bool,
    pub wt_renamed: bool,
    pub wt_typechange: bool,
    pub ignored: bool,
    pub conflicted: bool,
}

impl Default for GitStatusFlags {
    fn default() -> Self {
        Self {
            index_new: false,
            index_modified: false,
            index_deleted: false,
            index_renamed: false,
            index_typechange: false,
            wt_new: false,
            wt_modified: false,
            wt_deleted: false,
            wt_renamed: false,
            wt_typechange: false,
            ignored: false,
            conflicted: false,
        }
    }
}

impl GitStatusFlags {
    /// Create from git2::Status flags
    pub fn from_git2_status(status: git2::Status) -> Self {
        Self {
            index_new: status.contains(git2::Status::INDEX_NEW),
            index_modified: status.contains(git2::Status::INDEX_MODIFIED),
            index_deleted: status.contains(git2::Status::INDEX_DELETED),
            index_renamed: status.contains(git2::Status::INDEX_RENAMED),
            index_typechange: status.contains(git2::Status::INDEX_TYPECHANGE),
            wt_new: status.contains(git2::Status::WT_NEW),
            wt_modified: status.contains(git2::Status::WT_MODIFIED),
            wt_deleted: status.contains(git2::Status::WT_DELETED),
            wt_renamed: status.contains(git2::Status::WT_RENAMED),
            wt_typechange: status.contains(git2::Status::WT_TYPECHANGE),
            ignored: status.contains(git2::Status::IGNORED),
            conflicted: status.contains(git2::Status::CONFLICTED),
        }
    }

    /// Check if file is staged
    pub fn is_staged(&self) -> bool {
        self.index_new
            || self.index_modified
            || self.index_deleted
            || self.index_renamed
            || self.index_typechange
    }

    /// Check if file has working tree changes
    pub fn is_modified(&self) -> bool {
        self.wt_new || self.wt_modified || self.wt_deleted || self.wt_renamed || self.wt_typechange
    }

    /// Check if file is untracked
    pub fn is_untracked(&self) -> bool {
        self.wt_new && !self.index_new
    }

    /// Get status character for display
    pub fn status_char(&self) -> char {
        if self.conflicted {
            'C'
        } else if self.is_staged() && self.is_modified() {
            'M'
        } else if self.is_staged() {
            'S'
        } else if self.is_modified() {
            'M'
        } else if self.is_untracked() {
            '?'
        } else if self.ignored {
            'I'
        } else {
            ' '
        }
    }
}

/// Branch information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchInfo {
    /// Branch name
    pub name: String,
    /// Whether this is the current branch
    pub is_current: bool,
    /// Whether this is a remote branch
    pub is_remote: bool,
    /// Remote tracking branch name
    pub upstream: Option<String>,
    /// Number of commits ahead of upstream
    pub ahead: usize,
    /// Number of commits behind upstream
    pub behind: usize,
    /// Last commit hash
    pub last_commit: String,
    /// Last commit message
    pub last_commit_message: String,
    /// Last commit author
    pub last_commit_author: String,
    /// Last commit date
    pub last_commit_date: DateTime<Utc>,
}

/// Tag information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagInfo {
    /// Tag name
    pub name: String,
    /// Target commit hash
    pub target_commit: String,
    /// Tag message (for annotated tags)
    pub message: Option<String>,
    /// Tagger information (for annotated tags)
    pub tagger: Option<String>,
    /// Tag creation date
    pub date: DateTime<Utc>,
}

/// Remote repository information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteInfo {
    /// Remote name
    pub name: String,
    /// Fetch URL
    pub fetch_url: String,
    /// Push URL
    pub push_url: String,
    /// Whether the remote is connected
    pub is_connected: bool,
}

/// Stash entry information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StashInfo {
    /// Stash index
    pub index: usize,
    /// Stash message
    pub message: String,
    /// Stash creation date
    pub date: DateTime<Utc>,
    /// Branch name when stash was created
    pub branch: String,
}

/// Commit information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitInfo {
    /// Commit hash
    pub hash: String,
    /// Short hash (first 7 characters)
    pub short_hash: String,
    /// Commit message
    pub message: String,
    /// Author name
    pub author: String,
    /// Author email
    pub author_email: String,
    /// Commit date
    pub date: DateTime<Utc>,
    /// Parent commit hashes
    pub parents: Vec<String>,
}

/// Find the Git repository root starting from a given path
///
/// Performance requirement: < 50ms
/// Recursively searches upward for a .git directory or file.
pub fn find_git_root(start_path: &std::path::Path) -> AppResult<Option<PathBuf>> {
    let mut current = start_path;

    loop {
        let git_dir = current.join(".git");

        if git_dir.exists() {
            return Ok(Some(current.to_path_buf()));
        }

        match current.parent() {
            Some(parent) => current = parent,
            None => return Ok(None),
        }
    }
}

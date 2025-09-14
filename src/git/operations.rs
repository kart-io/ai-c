//! Advanced Git operations
//!
//! Extended Git operations for workflow management, including
//! branch operations, tagging, and Git Flow support.

use chrono::{DateTime, Utc};
use git2::{BranchType, ObjectType, Repository};
use std::time::{Duration, Instant};
use tracing::{debug, info, instrument, warn};

use crate::{
    error::{AppError, AppResult},
    git::{BranchInfo, RemoteInfo, StashInfo, TagInfo},
};

/// Extended Git operations for advanced workflow management
pub struct GitOperations<'repo> {
    repo: &'repo mut Repository,
}

impl<'repo> GitOperations<'repo> {
    /// Create a new GitOperations instance
    pub fn new(repo: &'repo mut Repository) -> Self {
        Self { repo }
    }

    /// Create a new branch
    #[instrument(skip(self))]
    pub fn create_branch(&self, name: &str, target: Option<&str>) -> AppResult<BranchInfo> {
        let operation_start = Instant::now();

        info!("Creating branch: {}", name);

        // Get target commit (default to HEAD)
        let target_commit = if let Some(target_ref) = target {
            self.repo
                .revparse_single(target_ref)
                .map_err(AppError::Git)?
                .peel_to_commit()
                .map_err(AppError::Git)?
        } else {
            self.repo
                .head()
                .map_err(AppError::Git)?
                .peel_to_commit()
                .map_err(AppError::Git)?
        };

        // Create the branch
        let branch = self
            .repo
            .branch(name, &target_commit, false)
            .map_err(AppError::Git)?;

        let branch_info = BranchInfo {
            name: name.to_string(),
            is_current: false,
            is_remote: false,
            upstream: None,
            ahead: 0,
            behind: 0,
            last_commit: target_commit.id().to_string(),
            last_commit_message: target_commit.message().unwrap_or("").to_string(),
            last_commit_author: target_commit.author().name().unwrap_or("").to_string(),
            last_commit_date: DateTime::from_timestamp(target_commit.time().seconds(), 0)
                .unwrap_or_else(|| Utc::now()),
        };

        let duration = operation_start.elapsed();
        debug!("Created branch {} in {:?}", name, duration);

        Ok(branch_info)
    }

    /// Switch to a branch
    #[instrument(skip(self))]
    pub fn switch_branch(&self, name: &str) -> AppResult<()> {
        let operation_start = Instant::now();

        info!("Switching to branch: {}", name);

        // Find the branch
        let branch = self
            .repo
            .find_branch(name, BranchType::Local)
            .map_err(AppError::Git)?;

        // Get the branch reference
        let branch_ref = branch.get();
        let branch_name = format!("refs/heads/{}", name);

        // Set HEAD to point to the branch
        self.repo.set_head(&branch_name).map_err(AppError::Git)?;

        // Checkout the branch
        let commit = branch_ref.peel_to_commit().map_err(AppError::Git)?;
        let tree = commit.tree().map_err(AppError::Git)?;

        let mut checkout_options = git2::build::CheckoutBuilder::new();
        checkout_options.safe();

        self.repo
            .checkout_tree(tree.as_object(), Some(&mut checkout_options))
            .map_err(AppError::Git)?;

        let duration = operation_start.elapsed();
        debug!("Switched to branch {} in {:?}", name, duration);

        Ok(())
    }

    /// Delete a branch
    #[instrument(skip(self))]
    pub fn delete_branch(&self, name: &str) -> AppResult<()> {
        let operation_start = Instant::now();

        info!("Deleting branch: {}", name);

        // Find and delete the branch
        let mut branch = self
            .repo
            .find_branch(name, BranchType::Local)
            .map_err(AppError::Git)?;

        branch.delete().map_err(AppError::Git)?;

        let duration = operation_start.elapsed();
        debug!("Deleted branch {} in {:?}", name, duration);

        Ok(())
    }

    /// Create a tag
    #[instrument(skip(self))]
    pub fn create_tag(
        &self,
        name: &str,
        target: Option<&str>,
        message: Option<&str>,
    ) -> AppResult<TagInfo> {
        let operation_start = Instant::now();

        info!("Creating tag: {}", name);

        // Get target object (default to HEAD)
        let target_obj = if let Some(target_ref) = target {
            self.repo
                .revparse_single(target_ref)
                .map_err(AppError::Git)?
        } else {
            self.repo
                .head()
                .map_err(AppError::Git)?
                .peel(ObjectType::Commit)
                .map_err(AppError::Git)?
        };

        let target_commit = target_obj.peel_to_commit().map_err(AppError::Git)?;

        let tag_id = if let Some(msg) = message {
            // Create annotated tag
            let signature = self.repo.signature().map_err(AppError::Git)?;
            self.repo
                .tag(name, &target_obj, &signature, msg, false)
                .map_err(AppError::Git)?
        } else {
            // Create lightweight tag
            self.repo
                .tag_lightweight(name, &target_obj, false)
                .map_err(AppError::Git)?;
            target_obj.id()
        };

        let tag_info = TagInfo {
            name: name.to_string(),
            target_commit: target_commit.id().to_string(),
            message: message.map(String::from),
            tagger: Some(
                self.repo
                    .signature()
                    .map(|sig| {
                        format!(
                            "{} <{}>",
                            sig.name().unwrap_or(""),
                            sig.email().unwrap_or("")
                        )
                    })
                    .unwrap_or_else(|_| "Unknown".to_string()),
            ),
            date: Utc::now(),
        };

        let duration = operation_start.elapsed();
        debug!("Created tag {} in {:?}", name, duration);

        Ok(tag_info)
    }

    /// Delete a tag
    #[instrument(skip(self))]
    pub fn delete_tag(&self, name: &str) -> AppResult<()> {
        let operation_start = Instant::now();

        info!("Deleting tag: {}", name);

        self.repo.tag_delete(name).map_err(AppError::Git)?;

        let duration = operation_start.elapsed();
        debug!("Deleted tag {} in {:?}", name, duration);

        Ok(())
    }

    /// List all tags
    #[instrument(skip(self))]
    pub fn list_tags(&self) -> AppResult<Vec<TagInfo>> {
        let mut tags = Vec::new();

        self.repo
            .tag_foreach(|oid, name_bytes| -> bool {
                if let Ok(name) = std::str::from_utf8(name_bytes) {
                    // Remove "refs/tags/" prefix
                    let tag_name = name.strip_prefix("refs/tags/").unwrap_or(name);

                    if let Ok(obj) = self.repo.find_object(oid, None) {
                        let (target_commit, message, tagger_info) =
                            if obj.kind() == Some(ObjectType::Tag) {
                                // Annotated tag
                                if let Some(tag) = obj.as_tag() {
                                    let target = tag.target().unwrap();
                                    let commit = target.peel_to_commit().unwrap();
                                    (
                                        commit.id().to_string(),
                                        tag.message().map(String::from),
                                        tag.tagger().map(|sig| {
                                            format!(
                                                "{} <{}>",
                                                sig.name().unwrap_or(""),
                                                sig.email().unwrap_or("")
                                            )
                                        }),
                                    )
                                } else {
                                    (oid.to_string(), None, None)
                                }
                            } else {
                                // Lightweight tag - points directly to commit
                                if let Ok(commit) = obj.peel_to_commit() {
                                    (commit.id().to_string(), None, None)
                                } else {
                                    (oid.to_string(), None, None)
                                }
                            };

                        let tag_info = TagInfo {
                            name: tag_name.to_string(),
                            target_commit,
                            message,
                            tagger: tagger_info,
                            date: Utc::now(), // TODO: Get actual tag creation date
                        };

                        tags.push(tag_info);
                    }
                }
                true // Continue iteration
            })
            .map_err(AppError::Git)?;

        Ok(tags)
    }

    /// List remotes
    #[instrument(skip(self))]
    pub fn list_remotes(&self) -> AppResult<Vec<RemoteInfo>> {
        let remote_names = self.repo.remotes().map_err(AppError::Git)?;
        let mut remotes = Vec::new();

        for remote_name in remote_names.iter() {
            if let Some(name) = remote_name {
                if let Ok(remote) = self.repo.find_remote(name) {
                    let remote_info = RemoteInfo {
                        name: name.to_string(),
                        fetch_url: remote.url().unwrap_or("").to_string(),
                        push_url: remote.pushurl().or(remote.url()).unwrap_or("").to_string(),
                        is_connected: false, // TODO: Implement connection check
                    };
                    remotes.push(remote_info);
                }
            }
        }

        Ok(remotes)
    }

    /// List stash entries
    #[instrument(skip(self))]
    pub fn list_stash(&mut self) -> AppResult<Vec<StashInfo>> {
        let mut stash_entries = Vec::new();

        self.repo
            .stash_foreach(|index, message, _oid| {
                let stash_info = StashInfo {
                    index,
                    message: message.to_string(),
                    date: Utc::now(), // TODO: Get actual stash creation date
                    branch: "unknown".to_string(), // TODO: Get branch from stash
                };
                stash_entries.push(stash_info);
                true // Continue iteration
            })
            .map_err(AppError::Git)?;

        Ok(stash_entries)
    }

    /// Create a stash
    #[instrument(skip(self))]
    pub fn stash_save(&mut self, message: Option<&str>) -> AppResult<()> {
        let operation_start = Instant::now();

        let msg = message.unwrap_or("WIP stash");
        info!("Creating stash: {}", msg);

        let signature = self.repo.signature().map_err(AppError::Git)?;

        self.repo
            .stash_save(&signature, msg, Some(git2::StashFlags::DEFAULT))
            .map_err(AppError::Git)?;

        let duration = operation_start.elapsed();
        debug!("Created stash in {:?}", duration);

        Ok(())
    }

    /// Apply a stash
    #[instrument(skip(self))]
    pub fn stash_pop(&mut self, index: usize) -> AppResult<()> {
        let operation_start = Instant::now();

        info!("Applying stash at index: {}", index);

        let mut options = git2::StashApplyOptions::new();
        options.progress_cb(|_| true);

        self.repo
            .stash_apply(index, Some(&mut options))
            .map_err(AppError::Git)?;

        self.repo.stash_drop(index).map_err(AppError::Git)?;

        let duration = operation_start.elapsed();
        debug!("Applied and dropped stash {} in {:?}", index, duration);

        Ok(())
    }

    /// Merge a branch
    #[instrument(skip(self))]
    pub fn merge(&self, branch_name: &str) -> AppResult<()> {
        let operation_start = Instant::now();

        info!("Merging branch: {}", branch_name);

        // Find the branch to merge
        let branch = self
            .repo
            .find_branch(branch_name, BranchType::Local)
            .map_err(AppError::Git)?;

        let branch_commit = branch.get().peel_to_commit().map_err(AppError::Git)?;

        // Get HEAD commit
        let head_commit = self
            .repo
            .head()
            .map_err(AppError::Git)?
            .peel_to_commit()
            .map_err(AppError::Git)?;

        // Perform merge analysis
        let analysis = self.repo.merge_analysis(&[]).map_err(AppError::Git)?;

        if analysis.0.is_up_to_date() {
            info!("Already up to date");
        } else if analysis.0.is_fast_forward() {
            // Fast-forward merge
            info!("Fast-forward merge");
            let refname = format!(
                "refs/heads/{}",
                self.repo.head().unwrap().shorthand().unwrap_or("main")
            );

            let mut reference = self.repo.find_reference(&refname).map_err(AppError::Git)?;
            reference
                .set_target(branch_commit.id(), "Fast-forward merge")
                .map_err(AppError::Git)?;

            // Update working directory
            self.repo.set_head(&refname).map_err(AppError::Git)?;
            let tree = branch_commit.tree().map_err(AppError::Git)?;
            let mut checkout_options = git2::build::CheckoutBuilder::new();
            checkout_options.safe();
            self.repo
                .checkout_tree(tree.as_object(), Some(&mut checkout_options))
                .map_err(AppError::Git)?;
        } else if analysis.0.is_normal() {
            // Normal merge - requires merge commit
            info!("Normal merge (merge commit required)");

            let mut merge_options = git2::MergeOptions::new();
            merge_options.file_favor(git2::FileFavor::Normal);

            let mut checkout_options = git2::build::CheckoutBuilder::new();
            checkout_options.safe();

            self.repo
                .merge(&[], Some(&mut merge_options), Some(&mut checkout_options))
                .map_err(AppError::Git)?;

            // Check if merge resulted in conflicts
            let mut index = self.repo.index().map_err(AppError::Git)?;
            if index.has_conflicts() {
                return Err(AppError::Git(git2::Error::from_str(
                    "Merge conflicts detected",
                )));
            }

            // Create merge commit
            let signature = self.repo.signature().map_err(AppError::Git)?;
            let tree_id = index.write_tree().map_err(AppError::Git)?;
            let tree = self.repo.find_tree(tree_id).map_err(AppError::Git)?;

            let merge_message = format!("Merge branch '{}'", branch_name);

            self.repo
                .commit(
                    Some("HEAD"),
                    &signature,
                    &signature,
                    &merge_message,
                    &tree,
                    &[&head_commit, &branch_commit],
                )
                .map_err(AppError::Git)?;

            // Clean up merge state
            self.repo.cleanup_state().map_err(AppError::Git)?;
        }

        let duration = operation_start.elapsed();
        debug!("Merged branch {} in {:?}", branch_name, duration);

        Ok(())
    }
}

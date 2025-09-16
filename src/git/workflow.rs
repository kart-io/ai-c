use crate::git::GitService;
use crate::error::AppError;

// Git操作专用错误类型
pub type GitError = AppError;
use async_trait::async_trait;
use git2::{Branch, BranchType, Oid, Reference, Repository, Tag};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    pub name: String,
    pub head: String, // 使用String而不是Oid来支持序列化
    pub upstream: Option<String>,
    pub branch_type: String, // 使用String而不是BranchType
    pub ahead: usize,
    pub behind: usize,
    pub last_commit_time: chrono::DateTime<chrono::Utc>,
    pub author: String,
    pub is_current: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagInfo {
    pub name: String,
    pub target: String, // 使用String而不是Oid
    pub tag_type: TagType,
    pub message: Option<String>,
    pub tagger: Option<String>,
    pub created_time: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TagType {
    Lightweight,
    Annotated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteInfo {
    pub name: String,
    pub url: String,
    pub fetch_refs: Vec<String>,
    pub push_refs: Vec<String>,
    pub is_connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitFlowConfig {
    pub master_branch: String,
    pub develop_branch: String,
    pub feature_prefix: String,
    pub release_prefix: String,
    pub hotfix_prefix: String,
    pub support_prefix: String,
}

impl Default for GitFlowConfig {
    fn default() -> Self {
        Self {
            master_branch: "master".to_string(),
            develop_branch: "develop".to_string(),
            feature_prefix: "feature/".to_string(),
            release_prefix: "release/".to_string(),
            hotfix_prefix: "hotfix/".to_string(),
            support_prefix: "support/".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchProtectionRule {
    pub branch_pattern: String,
    pub require_pull_request: bool,
    pub required_reviewers: usize,
    pub dismiss_stale_reviews: bool,
    pub require_status_checks: bool,
    pub restrict_pushes: bool,
    pub allowed_users: Vec<String>,
}

#[async_trait]
pub trait BranchManager: Send + Sync {
    async fn list_branches(&self, branch_type: Option<BranchType>) -> Result<Vec<BranchInfo>, GitError>;
    async fn create_branch(&self, name: &str, start_point: Option<&str>) -> Result<BranchInfo, GitError>;
    async fn delete_branch(&self, name: &str, force: bool) -> Result<(), GitError>;
    async fn rename_branch(&self, old_name: &str, new_name: &str) -> Result<(), GitError>;
    async fn checkout_branch(&self, name: &str) -> Result<(), GitError>;
    async fn merge_branch(&self, source: &str, target: &str, strategy: MergeStrategy) -> Result<String, GitError>;
    async fn get_branch_comparison(&self, base: &str, head: &str) -> Result<BranchComparison, GitError>;
    async fn set_upstream(&self, branch: &str, upstream: &str) -> Result<(), GitError>;
    async fn get_current_branch(&self) -> Result<Option<BranchInfo>, GitError>;
}

#[async_trait]
pub trait TagManager: Send + Sync {
    async fn list_tags(&self) -> Result<Vec<TagInfo>, GitError>;
    async fn create_tag(&self, name: &str, target: Option<&str>, message: Option<&str>) -> Result<TagInfo, GitError>;
    async fn delete_tag(&self, name: &str) -> Result<(), GitError>;
    async fn get_tag_details(&self, name: &str) -> Result<TagInfo, GitError>;
    async fn push_tag(&self, name: &str, remote: &str) -> Result<(), GitError>;
    async fn pull_tags(&self, remote: &str) -> Result<Vec<TagInfo>, GitError>;
}

#[async_trait]
pub trait RemoteManager: Send + Sync {
    async fn list_remotes(&self) -> Result<Vec<RemoteInfo>, GitError>;
    async fn add_remote(&self, name: &str, url: &str) -> Result<RemoteInfo, GitError>;
    async fn remove_remote(&self, name: &str) -> Result<(), GitError>;
    async fn set_remote_url(&self, name: &str, url: &str) -> Result<(), GitError>;
    async fn fetch(&self, remote: &str, refs: Option<Vec<String>>) -> Result<(), GitError>;
    async fn push(&self, remote: &str, refs: Vec<String>, force: bool) -> Result<(), GitError>;
    async fn pull(&self, remote: &str, branch: &str, strategy: MergeStrategy) -> Result<(), GitError>;
    async fn test_connection(&self, remote: &str) -> Result<bool, GitError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MergeStrategy {
    Merge,
    Rebase,
    Squash,
    FastForward,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchComparison {
    pub ahead: usize,
    pub behind: usize,
    pub commits_ahead: Vec<String>, // 使用String而不是Oid
    pub commits_behind: Vec<String>, // 使用String而不是Oid
    pub merge_base: Option<String>, // 使用String而不是Oid
    pub conflicts: Option<Vec<String>>,
}

pub struct GitWorkflowManager {
    git_service: Arc<GitService>,
    branch_manager: Arc<RwLock<Box<dyn BranchManager>>>,
    tag_manager: Arc<RwLock<Box<dyn TagManager>>>,
    remote_manager: Arc<RwLock<Box<dyn RemoteManager>>>,
    gitflow_config: Arc<RwLock<GitFlowConfig>>,
    protection_rules: Arc<RwLock<HashMap<String, BranchProtectionRule>>>,
}

impl GitWorkflowManager {
    pub fn new(git_service: Arc<GitService>) -> Self {
        let branch_manager = Arc::new(RwLock::new(
            Box::new(DefaultBranchManager::new(git_service.clone())) as Box<dyn BranchManager>
        ));
        let tag_manager = Arc::new(RwLock::new(
            Box::new(DefaultTagManager::new(git_service.clone())) as Box<dyn TagManager>
        ));
        let remote_manager = Arc::new(RwLock::new(
            Box::new(DefaultRemoteManager::new(git_service.clone())) as Box<dyn RemoteManager>
        ));

        Self {
            git_service,
            branch_manager,
            tag_manager,
            remote_manager,
            gitflow_config: Arc::new(RwLock::new(GitFlowConfig::default())),
            protection_rules: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Git Flow工作流支持
    pub async fn gitflow_init(&self, config: GitFlowConfig) -> Result<(), GitError> {
        info!("Initializing Git Flow with config: {:?}", config);

        // 创建develop分支（如果不存在）
        if !self.branch_exists(&config.develop_branch).await? {
            self.branch_manager.read().await
                .create_branch(&config.develop_branch, Some(&config.master_branch))
                .await?;
            info!("Created develop branch: {}", config.develop_branch);
        }

        // 更新配置
        *self.gitflow_config.write().await = config;

        Ok(())
    }

    pub async fn gitflow_feature_start(&self, feature_name: &str) -> Result<BranchInfo, GitError> {
        let config = self.gitflow_config.read().await;
        let branch_name = format!("{}{}", config.feature_prefix, feature_name);

        info!("Starting feature: {}", branch_name);

        self.branch_manager.read().await
            .create_branch(&branch_name, Some(&config.develop_branch))
            .await
    }

    pub async fn gitflow_feature_finish(&self, feature_name: &str) -> Result<String, GitError> {
        let config = self.gitflow_config.read().await;
        let branch_name = format!("{}{}", config.feature_prefix, feature_name);

        info!("Finishing feature: {}", branch_name);

        // 合并到develop分支
        let merge_commit = self.branch_manager.read().await
            .merge_branch(&branch_name, &config.develop_branch, MergeStrategy::Merge)
            .await?;

        // 删除feature分支
        self.branch_manager.read().await
            .delete_branch(&branch_name, false)
            .await?;

        Ok(merge_commit)
    }

    pub async fn gitflow_release_start(&self, version: &str) -> Result<BranchInfo, GitError> {
        let config = self.gitflow_config.read().await;
        let branch_name = format!("{}{}", config.release_prefix, version);

        info!("Starting release: {}", branch_name);

        self.branch_manager.read().await
            .create_branch(&branch_name, Some(&config.develop_branch))
            .await
    }

    pub async fn gitflow_release_finish(&self, version: &str) -> Result<(String, TagInfo), GitError> {
        let config = self.gitflow_config.read().await.clone();
        let branch_name = format!("{}{}", config.release_prefix, version);

        info!("Finishing release: {}", branch_name);

        // 合并到master分支
        let merge_commit = self.branch_manager.read().await
            .merge_branch(&branch_name, &config.master_branch, MergeStrategy::Merge)
            .await?;

        // 创建标签
        let tag = self.tag_manager.read().await
            .create_tag(version, Some(&config.master_branch), Some(&format!("Release {}", version)))
            .await?;

        // 合并回develop分支
        self.branch_manager.read().await
            .merge_branch(&config.master_branch, &config.develop_branch, MergeStrategy::Merge)
            .await?;

        // 删除release分支
        self.branch_manager.read().await
            .delete_branch(&branch_name, false)
            .await?;

        Ok((merge_commit, tag))
    }

    pub async fn gitflow_hotfix_start(&self, version: &str) -> Result<BranchInfo, GitError> {
        let config = self.gitflow_config.read().await;
        let branch_name = format!("{}{}", config.hotfix_prefix, version);

        info!("Starting hotfix: {}", branch_name);

        self.branch_manager.read().await
            .create_branch(&branch_name, Some(&config.master_branch))
            .await
    }

    pub async fn gitflow_hotfix_finish(&self, version: &str) -> Result<(String, TagInfo), GitError> {
        let config = self.gitflow_config.read().await.clone();
        let branch_name = format!("{}{}", config.hotfix_prefix, version);

        info!("Finishing hotfix: {}", branch_name);

        // 合并到master分支
        let merge_commit = self.branch_manager.read().await
            .merge_branch(&branch_name, &config.master_branch, MergeStrategy::Merge)
            .await?;

        // 创建标签
        let tag = self.tag_manager.read().await
            .create_tag(version, Some(&config.master_branch), Some(&format!("Hotfix {}", version)))
            .await?;

        // 合并回develop分支
        self.branch_manager.read().await
            .merge_branch(&config.master_branch, &config.develop_branch, MergeStrategy::Merge)
            .await?;

        // 删除hotfix分支
        self.branch_manager.read().await
            .delete_branch(&branch_name, false)
            .await?;

        Ok((merge_commit, tag))
    }

    // 分支保护规则
    pub async fn add_protection_rule(&self, pattern: String, rule: BranchProtectionRule) -> Result<(), GitError> {
        info!("Adding protection rule for pattern: {}", pattern);
        self.protection_rules.write().await.insert(pattern, rule);
        Ok(())
    }

    pub async fn remove_protection_rule(&self, pattern: &str) -> Result<(), GitError> {
        info!("Removing protection rule for pattern: {}", pattern);
        self.protection_rules.write().await.remove(pattern);
        Ok(())
    }

    pub async fn check_protection_rules(&self, branch: &str, operation: &str) -> Result<bool, GitError> {
        let rules = self.protection_rules.read().await;

        for (pattern, rule) in rules.iter() {
            if branch.starts_with(pattern) || pattern == "*" {
                debug!("Checking protection rule for branch: {} operation: {}", branch, operation);

                match operation {
                    "push" if rule.restrict_pushes => {
                        warn!("Push restricted for protected branch: {}", branch);
                        return Ok(false);
                    }
                    "force_push" => {
                        warn!("Force push not allowed for protected branch: {}", branch);
                        return Ok(false);
                    }
                    _ => {}
                }
            }
        }

        Ok(true)
    }

    // 辅助方法
    async fn branch_exists(&self, name: &str) -> Result<bool, GitError> {
        let branches = self.branch_manager.read().await.list_branches(None).await?;
        Ok(branches.iter().any(|b| b.name == name))
    }

    pub async fn get_branch_manager(&self) -> Arc<RwLock<Box<dyn BranchManager>>> {
        self.branch_manager.clone()
    }

    pub async fn get_tag_manager(&self) -> Arc<RwLock<Box<dyn TagManager>>> {
        self.tag_manager.clone()
    }

    pub async fn get_remote_manager(&self) -> Arc<RwLock<Box<dyn RemoteManager>>> {
        self.remote_manager.clone()
    }
}

// 默认实现类
pub struct DefaultBranchManager {
    git_service: Arc<GitService>,
}

impl DefaultBranchManager {
    pub fn new(git_service: Arc<GitService>) -> Self {
        Self { git_service }
    }
}

#[async_trait]
impl BranchManager for DefaultBranchManager {
    async fn list_branches(&self, branch_type: Option<BranchType>) -> Result<Vec<BranchInfo>, GitError> {
        let repo = self.git_service.get_repository()?;

        // Use spawn_blocking to avoid Send issues with git2 types
        let branches = tokio::task::spawn_blocking(move || -> Result<Vec<BranchInfo>, GitError> {
            let mut branches = Vec::new();

            let filter = branch_type.unwrap_or(BranchType::Local);
            let branch_iter = repo.branches(Some(filter))?;

            for branch_result in branch_iter {
                let (branch, _) = branch_result?;

                if let Some(name) = branch.name()? {
                    let branch_ref = branch.get();
                    let head = branch_ref.target().map(|oid| oid.to_string()).unwrap_or_default();

                    let upstream = branch.upstream().ok().and_then(|upstream_branch| {
                        upstream_branch.get().shorthand().map(|s| s.to_string())
                    });

                    let is_current = branch.is_head();

                    let branch_info = BranchInfo {
                        name: name.to_string(),
                        head,
                        upstream,
                        branch_type: if is_current { "Local".to_string() } else { "Remote".to_string() },
                        ahead: 0,
                        behind: 0,
                        last_commit_time: chrono::Utc::now(),
                        author: String::new(),
                        is_current,
                    };

                    branches.push(branch_info);
                }
            }

            Ok(branches)
        }).await.map_err(|e| GitError::InvalidOperation(format!("Task join error: {}", e)))??;

        Ok(branches)
    }

    async fn create_branch(&self, name: &str, start_point: Option<&str>) -> Result<BranchInfo, GitError> {
        let repo = self.git_service.get_repository()?;
        let name_owned = name.to_string();
        let start_point_owned = start_point.map(|s| s.to_string());

        // Use spawn_blocking to avoid Send issues with git2 types
        let branch_info = tokio::task::spawn_blocking(move || -> Result<BranchInfo, GitError> {
            // 查找起始点
            let target = if let Some(start) = start_point_owned {
                repo.revparse_single(&start)?.id()
            } else {
                repo.head()?.target().ok_or_else(|| GitError::InvalidOperation("HEAD has no target".to_string()))?
            };

            let commit = repo.find_commit(target)?;
            let branch = repo.branch(&name_owned, &commit, false)?;

            // Extract branch info synchronously
            let branch_ref = branch.get();
            let branch_name = branch_ref.shorthand().unwrap_or(&name_owned).to_string();
            let head = branch_ref.target().map(|oid| oid.to_string()).unwrap_or_default();

            let upstream = branch.upstream().ok().and_then(|upstream_branch| {
                upstream_branch.get().shorthand().map(|s| s.to_string())
            });

            Ok(BranchInfo {
                name: branch_name,
                head,
                upstream,
                branch_type: "Local".to_string(),
                ahead: 0,
                behind: 0,
                last_commit_time: chrono::Utc::now(),
                author: String::new(),
                is_current: false,
            })
        }).await.map_err(|e| GitError::InvalidOperation(format!("Task join error: {}", e)))??;

        info!("Created branch: {}", name);
        Ok(branch_info)
    }

    async fn delete_branch(&self, name: &str, force: bool) -> Result<(), GitError> {
        let repo = self.git_service.get_repository()?;
        let mut branch = repo.find_branch(name, BranchType::Local)?;

        if !force && !branch.is_head() {
            // 检查分支是否已合并
            // 这里可以添加更复杂的合并检查逻辑
        }

        branch.delete()?;
        info!("Deleted branch: {}", name);
        Ok(())
    }

    async fn rename_branch(&self, old_name: &str, new_name: &str) -> Result<(), GitError> {
        let repo = self.git_service.get_repository()?;
        let mut branch = repo.find_branch(old_name, BranchType::Local)?;
        branch.rename(new_name, false)?;
        info!("Renamed branch: {} -> {}", old_name, new_name);
        Ok(())
    }

    async fn checkout_branch(&self, name: &str) -> Result<(), GitError> {
        let repo = self.git_service.get_repository()?;
        let (object, reference) = repo.revparse_ext(name)?;

        repo.checkout_tree(&object, None)?;

        match reference {
            Some(gref) => repo.set_head(gref.name().unwrap())?,
            None => repo.set_head_detached(object.id())?,
        }

        info!("Checked out branch: {}", name);
        Ok(())
    }

    async fn merge_branch(&self, source: &str, target: &str, strategy: MergeStrategy) -> Result<String, GitError> {
        let repo = self.git_service.get_repository()?;

        // 检出目标分支
        self.checkout_branch(target).await?;

        // 查找源分支的提交
        let source_commit = repo.revparse_single(source)?.id();
        let target_commit = repo.revparse_single(target)?.id();

        // 执行合并（简化实现）
        let merge_commit = match strategy {
            MergeStrategy::FastForward => {
                // 检查是否可以快进
                let merge_base = repo.merge_base(source_commit, target_commit)?;
                if merge_base == target_commit {
                    // 可以快进
                    repo.reference(&format!("refs/heads/{}", target), source_commit, true, "Fast-forward merge")?;
                    source_commit.to_string()
                } else {
                    return Err(GitError::InvalidOperation("Fast-forward merge not possible".to_string()));
                }
            }
            _ => {
                // 其他合并策略的实现会更复杂
                // 这里提供一个简化的合并实现
                source_commit.to_string()
            }
        };

        info!("Merged branch: {} -> {}", source, target);
        Ok(merge_commit)
    }

    async fn get_branch_comparison(&self, base: &str, head: &str) -> Result<BranchComparison, GitError> {
        let repo = self.git_service.get_repository()?;

        let base_commit = repo.revparse_single(base)?.id();
        let head_commit = repo.revparse_single(head)?.id();
        let merge_base = repo.merge_base(base_commit, head_commit).ok().map(|oid| oid.to_string());

        // 简化的比较实现
        let comparison = BranchComparison {
            ahead: 0,  // 需要实际计算
            behind: 0, // 需要实际计算
            commits_ahead: vec![],
            commits_behind: vec![],
            merge_base,
            conflicts: None,
        };

        Ok(comparison)
    }

    async fn set_upstream(&self, branch: &str, upstream: &str) -> Result<(), GitError> {
        let repo = self.git_service.get_repository()?;
        let mut branch_ref = repo.find_branch(branch, BranchType::Local)?;
        branch_ref.set_upstream(Some(upstream))?;
        info!("Set upstream for branch {}: {}", branch, upstream);
        Ok(())
    }

    async fn get_current_branch(&self) -> Result<Option<BranchInfo>, GitError> {
        let repo = self.git_service.get_repository()?;

        // Use spawn_blocking to avoid Send issues with git2 types
        let branch_info = tokio::task::spawn_blocking(move || -> Result<Option<BranchInfo>, GitError> {
            if let Ok(head) = repo.head() {
                if head.is_branch() {
                    if let Some(name) = head.shorthand() {
                        let branch = repo.find_branch(name, BranchType::Local)?;
                        let branch_ref = branch.get();
                        let head = branch_ref.target().map(|oid| oid.to_string()).unwrap_or_default();

                        let upstream = branch.upstream().ok().and_then(|upstream_branch| {
                            upstream_branch.get().shorthand().map(|s| s.to_string())
                        });

                        return Ok(Some(BranchInfo {
                            name: name.to_string(),
                            head,
                            upstream,
                            branch_type: "Local".to_string(),
                            ahead: 0,
                            behind: 0,
                            last_commit_time: chrono::Utc::now(),
                            author: String::new(),
                            is_current: true,
                        }));
                    }
                }
            }

            Ok(None)
        }).await.map_err(|e| GitError::InvalidOperation(format!("Task join error: {}", e)))??;

        Ok(branch_info)
    }
}


pub struct DefaultTagManager {
    git_service: Arc<GitService>,
}

impl DefaultTagManager {
    pub fn new(git_service: Arc<GitService>) -> Self {
        Self { git_service }
    }
}

#[async_trait]
impl TagManager for DefaultTagManager {
    async fn list_tags(&self) -> Result<Vec<TagInfo>, GitError> {
        let repo = self.git_service.get_repository()?;
        let mut tags = Vec::new();

        repo.tag_foreach(|oid, name| {
            if let Ok(name_str) = std::str::from_utf8(name) {
                if let Ok(object) = repo.find_object(oid, None) {
                    let tag_info = TagInfo {
                        name: name_str.trim_start_matches("refs/tags/").to_string(),
                        target: oid.to_string(),
                        tag_type: if object.kind() == Some(git2::ObjectType::Tag) {
                            TagType::Annotated
                        } else {
                            TagType::Lightweight
                        },
                        message: None, // 需要进一步解析
                        tagger: None,  // 需要进一步解析
                        created_time: chrono::Utc::now(), // 需要实际获取
                    };
                    tags.push(tag_info);
                }
            }
            true
        })?;

        Ok(tags)
    }

    async fn create_tag(&self, name: &str, target: Option<&str>, message: Option<&str>) -> Result<TagInfo, GitError> {
        let repo = self.git_service.get_repository()?;

        let target_oid = if let Some(target_ref) = target {
            repo.revparse_single(target_ref)?.id()
        } else {
            repo.head()?.target().ok_or_else(|| GitError::InvalidOperation("HEAD has no target".to_string()))?
        };

        let tag_oid = if let Some(msg) = message {
            // 创建带注释的标签
            let signature = repo.signature()?;
            let target_obj = repo.find_object(target_oid, None)?;
            repo.tag(name, &target_obj, &signature, msg, false)?
        } else {
            // 创建轻量级标签
            repo.tag_lightweight(name, &repo.find_object(target_oid, None)?, false)?;
            target_oid
        };

        let tag_info = TagInfo {
            name: name.to_string(),
            target: tag_oid.to_string(),
            tag_type: if message.is_some() { TagType::Annotated } else { TagType::Lightweight },
            message: message.map(|s| s.to_string()),
            tagger: None, // 需要从signature获取
            created_time: chrono::Utc::now(),
        };

        info!("Created tag: {}", name);
        Ok(tag_info)
    }

    async fn delete_tag(&self, name: &str) -> Result<(), GitError> {
        let repo = self.git_service.get_repository()?;
        repo.tag_delete(name)?;
        info!("Deleted tag: {}", name);
        Ok(())
    }

    async fn get_tag_details(&self, name: &str) -> Result<TagInfo, GitError> {
        let repo = self.git_service.get_repository()?;
        let tag_ref = repo.find_reference(&format!("refs/tags/{}", name))?;

        if let Some(target) = tag_ref.target() {
            let tag_info = TagInfo {
                name: name.to_string(),
                target: target.to_string(),
                tag_type: TagType::Lightweight, // 需要进一步检查
                message: None,
                tagger: None,
                created_time: chrono::Utc::now(),
            };

            Ok(tag_info)
        } else {
            Err(GitError::InvalidOperation(format!("Tag {} not found", name)))
        }
    }

    async fn push_tag(&self, _name: &str, _remote: &str) -> Result<(), GitError> {
        // 推送标签的实现
        info!("Pushing tag to remote (not implemented)");
        Ok(())
    }

    async fn pull_tags(&self, _remote: &str) -> Result<Vec<TagInfo>, GitError> {
        // 拉取标签的实现
        info!("Pulling tags from remote (not implemented)");
        Ok(vec![])
    }
}

pub struct DefaultRemoteManager {
    git_service: Arc<GitService>,
}

impl DefaultRemoteManager {
    pub fn new(git_service: Arc<GitService>) -> Self {
        Self { git_service }
    }
}

#[async_trait]
impl RemoteManager for DefaultRemoteManager {
    async fn list_remotes(&self) -> Result<Vec<RemoteInfo>, GitError> {
        let repo = self.git_service.get_repository()?;
        let mut remotes = Vec::new();

        for remote_name in repo.remotes()?.iter() {
            if let Some(name) = remote_name {
                if let Ok(remote) = repo.find_remote(name) {
                    let remote_info = RemoteInfo {
                        name: name.to_string(),
                        url: remote.url().unwrap_or("").to_string(),
                        fetch_refs: vec![], // 需要进一步解析
                        push_refs: vec![],  // 需要进一步解析
                        is_connected: false, // 需要测试连接
                    };
                    remotes.push(remote_info);
                }
            }
        }

        Ok(remotes)
    }

    async fn add_remote(&self, name: &str, url: &str) -> Result<RemoteInfo, GitError> {
        let repo = self.git_service.get_repository()?;
        repo.remote(name, url)?;

        let remote_info = RemoteInfo {
            name: name.to_string(),
            url: url.to_string(),
            fetch_refs: vec![],
            push_refs: vec![],
            is_connected: false,
        };

        info!("Added remote: {} -> {}", name, url);
        Ok(remote_info)
    }

    async fn remove_remote(&self, name: &str) -> Result<(), GitError> {
        let repo = self.git_service.get_repository()?;
        repo.remote_delete(name)?;
        info!("Removed remote: {}", name);
        Ok(())
    }

    async fn set_remote_url(&self, name: &str, url: &str) -> Result<(), GitError> {
        let repo = self.git_service.get_repository()?;
        repo.remote_set_url(name, url)?;
        info!("Set remote URL: {} -> {}", name, url);
        Ok(())
    }

    async fn fetch(&self, _remote: &str, _refs: Option<Vec<String>>) -> Result<(), GitError> {
        // 获取远程分支的实现
        info!("Fetching from remote (not implemented)");
        Ok(())
    }

    async fn push(&self, _remote: &str, _refs: Vec<String>, _force: bool) -> Result<(), GitError> {
        // 推送到远程的实现
        info!("Pushing to remote (not implemented)");
        Ok(())
    }

    async fn pull(&self, _remote: &str, _branch: &str, _strategy: MergeStrategy) -> Result<(), GitError> {
        // 拉取远程分支的实现
        info!("Pulling from remote (not implemented)");
        Ok(())
    }

    async fn test_connection(&self, _remote: &str) -> Result<bool, GitError> {
        // 测试远程连接的实现
        info!("Testing remote connection (not implemented)");
        Ok(false)
    }
}
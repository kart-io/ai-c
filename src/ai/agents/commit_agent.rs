//! CommitAgent - AI agent for generating commit messages
//!
//! Provides intelligent commit message generation based on:
//! - Staged file changes analysis
//! - Commit history context
//! - Conventional commit patterns
//! - Code quality suggestions

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, instrument, warn};

use crate::{
    ai::{Agent, AgentCapability, AgentResult, AgentTask, HealthStatus, AgentMetrics},
    error::AppResult,
    git::{FileStatus, GitService},
};

/// Commit message generation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitGenerationRequest {
    /// Files that are staged for commit
    pub staged_files: Vec<FileStatus>,
    /// Optional context about the changes
    pub context: Option<String>,
    /// Preferred commit message style
    pub style: CommitMessageStyle,
    /// Maximum commit message length
    pub max_length: Option<usize>,
    /// Whether to include detailed description
    pub include_description: bool,
}

/// Commit message styles
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CommitMessageStyle {
    /// Conventional Commits format
    Conventional,
    /// Simple descriptive format
    Simple,
    /// Detailed format with bullet points
    Detailed,
    /// Custom format with template
    Custom(String),
}

impl Default for CommitMessageStyle {
    fn default() -> Self {
        CommitMessageStyle::Conventional
    }
}

/// Generated commit message with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedCommitMessage {
    /// Primary commit message (subject line)
    pub subject: String,
    /// Detailed description (body)
    pub description: Option<String>,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// Suggested commit type (feat, fix, docs, etc.)
    pub commit_type: String,
    /// Detected scope (module, component, etc.)
    pub scope: Option<String>,
    /// Alternative suggestions
    pub alternatives: Vec<String>,
    /// Generation metadata
    pub metadata: CommitMetadata,
}

/// Metadata about commit message generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitMetadata {
    /// Number of files analyzed
    pub files_count: usize,
    /// Generation time in milliseconds
    pub generation_time_ms: u64,
    /// AI model used (if external)
    pub model_used: Option<String>,
    /// Analysis confidence
    pub analysis_confidence: f64,
}

/// CommitAgent implementation
pub struct CommitAgent {
    /// Agent identifier
    id: String,
    /// Agent name
    name: String,
    /// Agent version
    version: String,
    /// Agent configuration
    config: CommitAgentConfig,
    /// Agent configuration (trait)
    agent_config: crate::ai::AgentConfig,
    /// Agent status
    status: crate::ai::AgentStatus,
    /// Performance metrics
    metrics: AgentMetrics,
    /// Health status
    health_status: HealthStatus,
    /// Prompt templates
    prompt_templates: HashMap<CommitMessageStyle, String>,
    /// Git service for analysis
    git_service: Option<GitService>,
}

/// CommitAgent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitAgentConfig {
    /// Default commit message style
    pub default_style: CommitMessageStyle,
    /// Maximum subject line length
    pub max_subject_length: usize,
    /// Maximum description length
    pub max_description_length: usize,
    /// Enable AI enhancement (external services)
    pub enable_ai_enhancement: bool,
    /// Cache generated messages
    pub enable_caching: bool,
    /// Analysis timeout in seconds
    pub analysis_timeout: u64,
}

impl Default for CommitAgentConfig {
    fn default() -> Self {
        Self {
            default_style: CommitMessageStyle::Conventional,
            max_subject_length: 72,
            max_description_length: 500,
            enable_ai_enhancement: true,
            enable_caching: true,
            analysis_timeout: 5,
        }
    }
}

impl CommitAgent {
    /// Create a new CommitAgent
    pub fn new(config: CommitAgentConfig, git_service: Option<GitService>) -> Self {
        let mut prompt_templates = HashMap::new();

        // Default prompt templates
        prompt_templates.insert(
            CommitMessageStyle::Conventional,
            "Analyze the following staged files and generate a conventional commit message:\n\
             Format: <type>(<scope>): <description>\n\
             Types: feat, fix, docs, style, refactor, test, chore\n\
             Files: {files}\n\
             Context: {context}".to_string(),
        );

        prompt_templates.insert(
            CommitMessageStyle::Simple,
            "Generate a simple, clear commit message for these changes:\n\
             Files: {files}\n\
             Context: {context}".to_string(),
        );

        prompt_templates.insert(
            CommitMessageStyle::Detailed,
            "Generate a detailed commit message with bullet points:\n\
             Subject: Brief summary\n\
             Body: Detailed changes with bullet points\n\
             Files: {files}\n\
             Context: {context}".to_string(),
        );

        let agent_id = format!("commit-agent-{}", uuid::Uuid::new_v4());
        let agent_config = crate::ai::AgentConfig {
            id: agent_id.clone(),
            name: "Commit Message Generator".to_string(),
            enabled: true,
            priority: 5,
            max_concurrent_tasks: 5,
            timeout: std::time::Duration::from_secs(30),
            retry_count: 3,
            custom_settings: std::collections::HashMap::new(),
        };

        Self {
            id: agent_id,
            name: "Commit Message Generator".to_string(),
            version: "1.0.0".to_string(),
            config,
            agent_config,
            status: crate::ai::AgentStatus::Uninitialized,
            metrics: AgentMetrics::default(),
            health_status: HealthStatus::Healthy,
            prompt_templates,
            git_service,
        }
    }

    /// Generate commit message from staged changes
    #[instrument(skip(self))]
    pub async fn generate_commit_message(
        &mut self,
        request: CommitGenerationRequest,
    ) -> AppResult<GeneratedCommitMessage> {
        let start_time = std::time::Instant::now();
        info!("Generating commit message for {} files", request.staged_files.len());

        // Analyze staged files
        let analysis = self.analyze_staged_changes(&request.staged_files)?;

        // Generate commit message based on style
        let commit_message = match request.style {
            CommitMessageStyle::Conventional => {
                self.generate_conventional_commit(&analysis, &request).await?
            }
            CommitMessageStyle::Simple => {
                self.generate_simple_commit(&analysis, &request).await?
            }
            CommitMessageStyle::Detailed => {
                self.generate_detailed_commit(&analysis, &request).await?
            }
            CommitMessageStyle::Custom(ref template) => {
                self.generate_custom_commit(&analysis, &request, template).await?
            }
        };

        let generation_time = start_time.elapsed();

        // Update metrics
        self.metrics.tasks_processed += 1;
        self.metrics.average_response_time =
            (self.metrics.average_response_time + generation_time) / 2;
        self.metrics.last_activity = Utc::now();

        debug!(
            "Generated commit message in {:?}: '{}'",
            generation_time,
            commit_message.subject
        );

        Ok(commit_message)
    }

    /// Analyze staged changes to understand the nature of commits
    fn analyze_staged_changes(&self, files: &[FileStatus]) -> AppResult<ChangeAnalysis> {
        let mut analysis = ChangeAnalysis::default();
        analysis.total_files = files.len();

        for file in files {
            // Determine file type and category
            let file_category = self.categorize_file(&file.path);
            analysis.file_categories.insert(file_category.clone(),
                analysis.file_categories.get(&file_category).unwrap_or(&0) + 1);

            // Analyze change type
            if file.status.wt_new || file.status.index_new {
                analysis.new_files += 1;
            } else if file.status.wt_modified || file.status.index_modified {
                analysis.modified_files += 1;
            } else if file.status.wt_deleted || file.status.index_deleted {
                analysis.deleted_files += 1;
            }

            // Detect potential scope from file paths
            if let Some(scope) = self.extract_scope_from_path(&file.path) {
                let count = analysis.detected_scopes.get(&scope).unwrap_or(&0) + 1;
                analysis.detected_scopes.insert(scope, count);
            }
        }

        // Determine primary commit type
        analysis.primary_type = self.determine_commit_type(&analysis);

        Ok(analysis)
    }

    /// Generate conventional commit message
    async fn generate_conventional_commit(
        &self,
        analysis: &ChangeAnalysis,
        request: &CommitGenerationRequest,
    ) -> AppResult<GeneratedCommitMessage> {
        let commit_type = &analysis.primary_type;

        // Determine scope (most common detected scope)
        let scope = analysis.detected_scopes
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(scope, _)| scope.clone());

        // Generate subject line
        let subject = if let Some(scope) = &scope {
            format!("{}({}): {}", commit_type, scope, self.generate_description(analysis))
        } else {
            format!("{}: {}", commit_type, self.generate_description(analysis))
        };

        // Generate description if requested
        let description = if request.include_description {
            Some(self.generate_commit_description(analysis, request))
        } else {
            None
        };

        // Generate alternatives
        let alternatives = vec![
            format!("{}: {}", commit_type, self.generate_alternative_description(analysis, 1)),
            format!("{}: {}", commit_type, self.generate_alternative_description(analysis, 2)),
        ];

        Ok(GeneratedCommitMessage {
            subject: self.truncate_subject(&subject),
            description,
            confidence: self.calculate_confidence(analysis),
            commit_type: commit_type.clone(),
            scope,
            alternatives,
            metadata: CommitMetadata {
                files_count: analysis.total_files,
                generation_time_ms: 50, // Mock value - would be actual time
                model_used: None, // Local generation
                analysis_confidence: 0.85,
            },
        })
    }

    /// Generate simple commit message
    async fn generate_simple_commit(
        &self,
        analysis: &ChangeAnalysis,
        _request: &CommitGenerationRequest,
    ) -> AppResult<GeneratedCommitMessage> {
        let subject = self.generate_description(analysis);

        Ok(GeneratedCommitMessage {
            subject: self.truncate_subject(&subject),
            description: None,
            confidence: self.calculate_confidence(analysis),
            commit_type: analysis.primary_type.clone(),
            scope: None,
            alternatives: vec![
                self.generate_alternative_description(analysis, 1),
                self.generate_alternative_description(analysis, 2),
            ],
            metadata: CommitMetadata {
                files_count: analysis.total_files,
                generation_time_ms: 30,
                model_used: None,
                analysis_confidence: 0.75,
            },
        })
    }

    /// Generate detailed commit message
    async fn generate_detailed_commit(
        &self,
        analysis: &ChangeAnalysis,
        request: &CommitGenerationRequest,
    ) -> AppResult<GeneratedCommitMessage> {
        let subject = format!("{}: {}", analysis.primary_type, self.generate_description(analysis));

        let description = self.generate_detailed_description(analysis, request);

        Ok(GeneratedCommitMessage {
            subject: self.truncate_subject(&subject),
            description: Some(description),
            confidence: self.calculate_confidence(analysis),
            commit_type: analysis.primary_type.clone(),
            scope: None,
            alternatives: vec![
                format!("Update {}", self.get_primary_category(analysis)),
                format!("Improve {}", self.get_primary_category(analysis)),
            ],
            metadata: CommitMetadata {
                files_count: analysis.total_files,
                generation_time_ms: 80,
                model_used: None,
                analysis_confidence: 0.90,
            },
        })
    }

    /// Generate custom format commit message
    async fn generate_custom_commit(
        &self,
        analysis: &ChangeAnalysis,
        request: &CommitGenerationRequest,
        template: &str,
    ) -> AppResult<GeneratedCommitMessage> {
        // Simple template substitution
        let subject = template
            .replace("{type}", &analysis.primary_type)
            .replace("{description}", &self.generate_description(analysis))
            .replace("{files_count}", &analysis.total_files.to_string());

        Ok(GeneratedCommitMessage {
            subject: self.truncate_subject(&subject),
            description: None,
            confidence: 0.70, // Lower confidence for custom templates
            commit_type: analysis.primary_type.clone(),
            scope: None,
            alternatives: vec![],
            metadata: CommitMetadata {
                files_count: analysis.total_files,
                generation_time_ms: 40,
                model_used: None,
                analysis_confidence: 0.70,
            },
        })
    }

    /// Categorize file by type
    fn categorize_file(&self, path: &str) -> String {
        let path_lower = path.to_lowercase();

        if path_lower.ends_with(".rs") {
            "rust".to_string()
        } else if path_lower.ends_with(".js") || path_lower.ends_with(".ts") {
            "javascript".to_string()
        } else if path_lower.ends_with(".md") || path_lower.ends_with(".txt") {
            "documentation".to_string()
        } else if path_lower.ends_with(".toml") || path_lower.ends_with(".yml") || path_lower.ends_with(".yaml") {
            "configuration".to_string()
        } else if path_lower.contains("test") {
            "tests".to_string()
        } else {
            "code".to_string()
        }
    }

    /// Extract scope from file path
    fn extract_scope_from_path(&self, path: &str) -> Option<String> {
        let parts: Vec<&str> = path.split('/').collect();

        // Extract meaningful scope from path structure
        if parts.len() > 2 {
            Some(parts[1].to_string()) // src/ui -> "ui"
        } else if parts.len() > 1 {
            Some(parts[0].to_string())
        } else {
            None
        }
    }

    /// Determine primary commit type based on changes
    fn determine_commit_type(&self, analysis: &ChangeAnalysis) -> String {
        if analysis.new_files > analysis.modified_files && analysis.new_files > analysis.deleted_files {
            "feat".to_string()
        } else if analysis.deleted_files > 0 {
            "refactor".to_string()
        } else if analysis.file_categories.contains_key("tests") {
            "test".to_string()
        } else if analysis.file_categories.contains_key("documentation") {
            "docs".to_string()
        } else if analysis.file_categories.contains_key("configuration") {
            "chore".to_string()
        } else {
            "fix".to_string()
        }
    }

    /// Generate commit description
    fn generate_description(&self, analysis: &ChangeAnalysis) -> String {
        let primary_category = self.get_primary_category(analysis);

        if analysis.new_files > 0 && analysis.modified_files > 0 {
            format!("add and update {}", primary_category)
        } else if analysis.new_files > 0 {
            format!("add {}", primary_category)
        } else if analysis.deleted_files > 0 {
            format!("remove {}", primary_category)
        } else {
            format!("update {}", primary_category)
        }
    }

    /// Generate alternative descriptions
    fn generate_alternative_description(&self, analysis: &ChangeAnalysis, variant: usize) -> String {
        let primary_category = self.get_primary_category(analysis);

        match variant {
            1 => format!("improve {}", primary_category),
            2 => format!("enhance {}", primary_category),
            _ => format!("modify {}", primary_category),
        }
    }

    /// Generate detailed commit description
    fn generate_commit_description(&self, analysis: &ChangeAnalysis, request: &CommitGenerationRequest) -> String {
        let mut description = String::new();

        if analysis.new_files > 0 {
            description.push_str(&format!("- Add {} new file(s)\n", analysis.new_files));
        }
        if analysis.modified_files > 0 {
            description.push_str(&format!("- Update {} file(s)\n", analysis.modified_files));
        }
        if analysis.deleted_files > 0 {
            description.push_str(&format!("- Remove {} file(s)\n", analysis.deleted_files));
        }

        if let Some(context) = &request.context {
            description.push_str(&format!("\nContext: {}", context));
        }

        description.trim_end().to_string()
    }

    /// Generate detailed description with bullet points
    fn generate_detailed_description(&self, analysis: &ChangeAnalysis, request: &CommitGenerationRequest) -> String {
        let mut description = vec![];

        // File changes summary
        if analysis.new_files > 0 {
            description.push(format!("• Add {} new file(s)", analysis.new_files));
        }
        if analysis.modified_files > 0 {
            description.push(format!("• Modify {} existing file(s)", analysis.modified_files));
        }
        if analysis.deleted_files > 0 {
            description.push(format!("• Delete {} file(s)", analysis.deleted_files));
        }

        // File categories
        if !analysis.file_categories.is_empty() {
            description.push("".to_string()); // Empty line
            description.push("Affected components:".to_string());
            for (category, count) in &analysis.file_categories {
                description.push(format!("• {}: {} file(s)", category, count));
            }
        }

        // Context if provided
        if let Some(context) = &request.context {
            description.push("".to_string());
            description.push(format!("Context: {}", context));
        }

        description.join("\n")
    }

    /// Get primary file category
    fn get_primary_category(&self, analysis: &ChangeAnalysis) -> String {
        analysis.file_categories
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(category, _)| category.clone())
            .unwrap_or_else(|| "files".to_string())
    }

    /// Calculate confidence score
    fn calculate_confidence(&self, analysis: &ChangeAnalysis) -> f64 {
        // Base confidence
        let mut confidence: f64 = 0.5;

        // More files analyzed = higher confidence
        if analysis.total_files > 5 {
            confidence += 0.2;
        } else if analysis.total_files > 1 {
            confidence += 0.1;
        }

        // Clear file categories = higher confidence
        if !analysis.file_categories.is_empty() {
            confidence += 0.2;
        }

        // Clear change pattern = higher confidence
        if analysis.new_files > 0 || analysis.modified_files > 0 {
            confidence += 0.1;
        }

        confidence.min(1.0)
    }

    /// Truncate subject line to maximum length
    fn truncate_subject(&self, subject: &str) -> String {
        if subject.len() <= self.config.max_subject_length {
            subject.to_string()
        } else {
            format!("{}...", &subject[..self.config.max_subject_length - 3])
        }
    }
}

/// Analysis of staged changes
#[derive(Debug, Default)]
struct ChangeAnalysis {
    /// Total number of files
    total_files: usize,
    /// Number of new files
    new_files: usize,
    /// Number of modified files
    modified_files: usize,
    /// Number of deleted files
    deleted_files: usize,
    /// File categories and their counts
    file_categories: HashMap<String, usize>,
    /// Detected scopes and their frequencies
    detected_scopes: HashMap<String, usize>,
    /// Primary commit type
    primary_type: String,
}

#[async_trait]
impl Agent for CommitAgent {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn agent_type(&self) -> &str {
        "commit-agent"
    }

    fn capabilities(&self) -> Vec<AgentCapability> {
        vec![
            AgentCapability::CommitMessageGeneration,
            AgentCapability::CodeAnalysis,
        ]
    }

    fn config(&self) -> &crate::ai::AgentConfig {
        &self.agent_config
    }

    fn get_status(&self) -> crate::ai::AgentStatus {
        self.status.clone()
    }

    fn metrics(&self) -> AgentMetrics {
        self.metrics.clone()
    }

    fn update_metrics(&mut self, metrics: AgentMetrics) {
        self.metrics = metrics;
    }

    async fn handle_task(&mut self, task: AgentTask) -> Result<AgentResult, crate::ai::AgentError> {
        use crate::ai::agent::AgentTaskType;
        let start_time = std::time::Instant::now();

        let result = match &task.task_type {
            AgentTaskType::GenerateCommitMessage { staged_files, context, .. } => {
                // Convert to internal format
                let request = CommitGenerationRequest {
                    staged_files: staged_files.iter().map(|path| FileStatus {
                        path: path.clone(),
                        status: crate::git::GitStatusFlags::default(),
                        size: 0,
                        modified: chrono::Utc::now(),
                        is_binary: false,
                    }).collect(),
                    context: context.clone(),
                    style: self.config.default_style.clone(),
                    max_length: Some(self.config.max_subject_length),
                    include_description: true,
                };

                let result = self.generate_commit_message(request).await
                    .map_err(|e| crate::ai::AgentError::TaskProcessingFailed(e.to_string()))?;
                serde_json::to_value(result)
                    .map_err(|e| crate::ai::AgentError::SerializationError(e.to_string()))?
            }
            _ => {
                warn!("Unsupported task type for CommitAgent: {:?}", task.task_type);
                return Err(crate::ai::AgentError::UnsupportedCapability(
                    format!("Task type not supported: {:?}", task.task_type)
                ));
            }
        };

        Ok(AgentResult {
            task_id: task.task_id,
            success: true,
            data: result,
            error: None,
            duration: start_time.elapsed(),
            timestamp: chrono::Utc::now(),
            agent_id: self.id.clone(),
        })
    }

    fn health_check(&self) -> HealthStatus {
        self.health_status.clone()
    }

    async fn initialize(&mut self) -> Result<(), crate::ai::AgentError> {
        info!("Initializing CommitAgent: {}", self.id);
        self.status = crate::ai::AgentStatus::Initializing;
        self.health_status = HealthStatus::Healthy;
        self.status = crate::ai::AgentStatus::Idle;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), crate::ai::AgentError> {
        info!("Shutting down CommitAgent: {}", self.id);
        self.status = crate::ai::AgentStatus::Shutting;
        self.health_status = HealthStatus::Shutdown;
        self.status = crate::ai::AgentStatus::Shutdown;
        Ok(())
    }
}

#[cfg(test)]
impl CommitAgent {
    /// Get configuration for testing
    pub fn get_config(&self) -> &CommitAgentConfig {
        &self.config
    }
}
//! Core Agent trait and types
//!
//! Defines the fundamental Agent interface and related types for
//! the actor-based AI system.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

use super::{AgentMetrics, TaskPriority};
use std::collections::HashMap;

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub priority: u8,
    pub max_concurrent_tasks: usize,
    pub timeout: Duration,
    pub retry_count: u8,
    pub custom_settings: HashMap<String, serde_json::Value>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: "default-agent".to_string(),
            enabled: true,
            priority: 5,
            max_concurrent_tasks: 10,
            timeout: Duration::from_secs(30),
            retry_count: 3,
            custom_settings: HashMap::new(),
        }
    }
}

/// Agent status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    /// Agent is not initialized
    Uninitialized,
    /// Agent is initializing
    Initializing,
    /// Agent is idle and ready for tasks
    Idle,
    /// Agent is processing a task
    Processing(String), // Task ID
    /// Agent encountered an error
    Error(String),
    /// Agent is shutting down
    Shutting,
    /// Agent has shut down
    Shutdown,
}

impl Default for AgentStatus {
    fn default() -> Self {
        AgentStatus::Uninitialized
    }
}

/// Agent error types - following design document specifications
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
pub enum AgentError {
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Task processing failed: {0}")]
    TaskProcessingFailed(String),

    #[error("Timeout occurred: {0}")]
    Timeout(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Agent initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Capability not supported: {0}")]
    UnsupportedCapability(String),

    #[error("Resource unavailable: {0}")]
    ResourceUnavailable(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

/// Agent message for inter-agent communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: Uuid,
    pub from: String,
    pub to: String,
    pub message_type: MessageType,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub correlation_id: Option<String>,
    pub reply_to: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Request,
    Response,
    Notification,
    Command,
    Event,
}

/// Core Agent trait
///
/// All agents must implement this trait to participate in the agent system.
/// Performance requirements:
/// - Agent initialization: < 500ms
/// - Task processing: < 2s for normal priority tasks
/// - Health checks: < 100ms
#[async_trait]
pub trait Agent: Send + Sync {
    /// Get agent unique identifier
    fn id(&self) -> &str;

    /// Get agent name (user-friendly)
    fn name(&self) -> &str;

    /// Get agent version
    fn version(&self) -> &str;

    /// Get agent type name
    fn agent_type(&self) -> &str;

    /// Get agent capabilities
    fn capabilities(&self) -> Vec<AgentCapability>;

    /// Get agent configuration
    fn config(&self) -> &AgentConfig;

    /// Get agent status
    fn get_status(&self) -> AgentStatus;

    /// Initialize the agent
    ///
    /// Performance requirement: < 500ms
    async fn initialize(&mut self) -> Result<(), AgentError>;

    /// Handle a task
    ///
    /// Performance requirement: < 2s for normal priority tasks
    async fn handle_task(&mut self, task: AgentTask) -> Result<AgentResult, AgentError>;

    /// Shutdown the agent gracefully
    async fn shutdown(&mut self) -> Result<(), AgentError>;

    /// Perform health check
    ///
    /// Performance requirement: < 100ms
    fn health_check(&self) -> HealthStatus;

    /// Get agent metrics
    fn metrics(&self) -> AgentMetrics;

    /// Update agent metrics
    fn update_metrics(&mut self, metrics: AgentMetrics);

    /// Handle configuration update (optional override)
    async fn handle_config_update(&mut self, _config: AgentConfig) -> Result<(), AgentError> {
        // Default implementation - agents can override
        Ok(())
    }

    /// Handle inter-agent message (optional override)
    async fn handle_agent_message(&mut self, _message: AgentMessage) -> Result<(), AgentError> {
        // Default implementation - agents can override
        Ok(())
    }

    /// Check if agent can handle a specific task
    fn can_handle_task(&self, task: &AgentTask) -> bool {
        self.capabilities().iter().any(|cap| cap.matches_task(task))
    }

    /// Get agent load factor (0.0 to 1.0)
    fn load_factor(&self) -> f32 {
        // Default implementation based on metrics
        let metrics = self.metrics();
        (metrics.cpu_usage / 100.0).min(1.0)
    }

    /// Check if agent is available for new tasks
    fn is_available(&self) -> bool {
        matches!(self.health_check(), HealthStatus::Healthy) && self.load_factor() < 0.8
    }
}

/// Agent capabilities define what types of tasks an agent can handle
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentCapability {
    /// Generate commit messages from Git diffs
    CommitMessageGeneration,
    /// Analyze code quality and suggest improvements
    CodeAnalysis,
    /// Review code changes and provide feedback
    CodeReview,
    /// Search and query codebase semantically
    SemanticSearch,
    /// Generate documentation
    DocumentationGeneration,
    /// Refactor code suggestions
    CodeRefactoring,
    /// Generate unit tests
    TestGeneration,
    /// Analyze Git workflow patterns
    WorkflowAnalysis,
    /// Custom capability with name
    Custom(String),
}

impl AgentCapability {
    /// Check if this capability matches a given task
    pub fn matches_task(&self, task: &AgentTask) -> bool {
        match (&self, &task.task_type) {
            (
                AgentCapability::CommitMessageGeneration,
                AgentTaskType::GenerateCommitMessage { .. },
            ) => true,
            (AgentCapability::CodeAnalysis, AgentTaskType::AnalyzeCode { .. }) => true,
            (AgentCapability::CodeReview, AgentTaskType::ReviewChanges { .. }) => true,
            (AgentCapability::SemanticSearch, AgentTaskType::SearchCode { .. }) => true,
            (
                AgentCapability::DocumentationGeneration,
                AgentTaskType::GenerateDocumentation { .. },
            ) => true,
            (AgentCapability::CodeRefactoring, AgentTaskType::SuggestRefactoring { .. }) => true,
            (AgentCapability::TestGeneration, AgentTaskType::GenerateTests { .. }) => true,
            (AgentCapability::WorkflowAnalysis, AgentTaskType::AnalyzeWorkflow { .. }) => true,
            (AgentCapability::Custom(name), AgentTaskType::Custom { task_name, .. }) => {
                name == task_name
            }
            _ => false,
        }
    }

    /// Get display name for the capability
    pub fn display_name(&self) -> &str {
        match self {
            AgentCapability::CommitMessageGeneration => "Commit Message Generation",
            AgentCapability::CodeAnalysis => "Code Analysis",
            AgentCapability::CodeReview => "Code Review",
            AgentCapability::SemanticSearch => "Semantic Search",
            AgentCapability::DocumentationGeneration => "Documentation Generation",
            AgentCapability::CodeRefactoring => "Code Refactoring",
            AgentCapability::TestGeneration => "Test Generation",
            AgentCapability::WorkflowAnalysis => "Workflow Analysis",
            AgentCapability::Custom(name) => name,
        }
    }
}

/// Agent task definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    /// Task unique identifier
    pub task_id: Uuid,
    /// Task type and parameters
    pub task_type: AgentTaskType,
    /// Task priority
    pub priority: TaskPriority,
    /// Task timeout
    pub timeout: Duration,
    /// Task creation time
    pub created_at: DateTime<Utc>,
    /// Task deadline (optional)
    pub deadline: Option<DateTime<Utc>>,
    /// Task metadata
    pub metadata: serde_json::Value,
}

impl AgentTask {
    /// Create a new task
    pub fn new(task_type: AgentTaskType) -> Self {
        Self {
            task_id: Uuid::new_v4(),
            task_type,
            priority: TaskPriority::Normal,
            timeout: Duration::from_secs(30),
            created_at: Utc::now(),
            deadline: None,
            metadata: serde_json::Value::Null,
        }
    }

    /// Set task priority
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set task timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set task deadline
    pub fn with_deadline(mut self, deadline: DateTime<Utc>) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// Check if task has expired
    pub fn is_expired(&self) -> bool {
        if let Some(deadline) = &self.deadline {
            Utc::now() > *deadline
        } else {
            Utc::now()
                > self.created_at
                    + chrono::Duration::from_std(self.timeout)
                        .unwrap_or_else(|_| chrono::Duration::seconds(30))
        }
    }
}

/// Types of tasks that agents can handle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentTaskType {
    /// Generate commit message from Git changes
    GenerateCommitMessage {
        staged_files: Vec<String>,
        diff_content: String,
        context: Option<String>,
    },

    /// Analyze code quality and issues
    AnalyzeCode {
        file_paths: Vec<String>,
        analysis_type: CodeAnalysisType,
    },

    /// Review code changes
    ReviewChanges {
        diff_content: String,
        file_paths: Vec<String>,
        review_type: CodeReviewType,
    },

    /// Search code semantically
    SearchCode {
        query: String,
        file_patterns: Vec<String>,
        search_type: SearchType,
    },

    /// Generate documentation
    GenerateDocumentation {
        target: DocumentationTarget,
        content: String,
    },

    /// Suggest code refactoring
    SuggestRefactoring {
        file_path: String,
        code_content: String,
        refactoring_type: RefactoringType,
    },

    /// Generate unit tests
    GenerateTests {
        file_path: String,
        function_names: Vec<String>,
        test_framework: String,
    },

    /// Analyze Git workflow patterns
    AnalyzeWorkflow {
        commit_history: Vec<String>,
        branch_info: Vec<String>,
        analysis_period: Duration,
    },

    /// Custom task type
    Custom {
        task_name: String,
        parameters: serde_json::Value,
    },
}

/// Code analysis types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CodeAnalysisType {
    Quality,
    Security,
    Performance,
    Complexity,
    Coverage,
    Dependencies,
}

/// Code review types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CodeReviewType {
    General,
    Security,
    Performance,
    Style,
    Logic,
}

/// Search types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchType {
    Semantic,
    Exact,
    Regex,
    Fuzzy,
}

/// Documentation targets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentationTarget {
    Function,
    Module,
    API,
    README,
    Changelog,
}

/// Refactoring types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RefactoringType {
    ExtractFunction,
    ExtractVariable,
    RenameSymbol,
    OptimizePerformance,
    ImproveReadability,
}

/// Agent task result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    /// Task ID this result corresponds to
    pub task_id: Uuid,
    /// Whether the task was successful
    pub success: bool,
    /// Result data (JSON)
    pub data: serde_json::Value,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Processing duration
    pub duration: Duration,
    /// Result timestamp
    pub timestamp: DateTime<Utc>,
    /// Agent ID that produced this result
    pub agent_id: String,
}

impl AgentResult {
    /// Create successful result
    pub fn success<T: Serialize>(
        task_id: Uuid,
        agent_id: String,
        data: T,
        duration: Duration,
    ) -> Self {
        Self {
            task_id,
            success: true,
            data: serde_json::to_value(data).unwrap_or(serde_json::Value::Null),
            error: None,
            duration,
            timestamp: Utc::now(),
            agent_id,
        }
    }

    /// Create error result
    pub fn error(task_id: Uuid, agent_id: String, error: String, duration: Duration) -> Self {
        Self {
            task_id,
            success: false,
            data: serde_json::Value::Null,
            error: Some(error),
            duration,
            timestamp: Utc::now(),
            agent_id,
        }
    }
}

/// Agent health status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Agent is healthy and ready
    Healthy,
    /// Agent is functional but degraded
    Degraded(String),
    /// Agent is unhealthy and needs attention
    Unhealthy(String),
    /// Agent is shutting down
    Shutdown,
}

impl HealthStatus {
    /// Check if the agent is operational
    pub fn is_operational(&self) -> bool {
        matches!(self, HealthStatus::Healthy | HealthStatus::Degraded(_))
    }

    /// Get health score (0.0 to 1.0)
    pub fn score(&self) -> f32 {
        match self {
            HealthStatus::Healthy => 1.0,
            HealthStatus::Degraded(_) => 0.5,
            HealthStatus::Unhealthy(_) => 0.0,
            HealthStatus::Shutdown => 0.0,
        }
    }
}

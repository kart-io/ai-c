//! Application event system
//!
//! Provides async event handling for background tasks, Git operations,
//! Agent communications, and MCP protocol messages.

use chrono::{DateTime, Utc};
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{
    ai::AgentResult,
    error::{AppError, AppResult},
    git::FileStatus,
};

/// Event handler for async operations
///
/// Manages background tasks and inter-component communication using
/// tokio channels for high-performance message passing.
pub struct EventHandler {
    /// Sender for application events
    event_sender: mpsc::UnboundedSender<AppEvent>,
    /// Receiver for application events
    event_receiver: mpsc::UnboundedReceiver<AppEvent>,
}

impl EventHandler {
    /// Create a new event handler
    pub async fn new() -> AppResult<Self> {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        Ok(Self {
            event_sender,
            event_receiver,
        })
    }

    /// Send an event to the application
    pub fn send_event(&self, event: AppEvent) -> AppResult<()> {
        self.event_sender
            .send(event)
            .map_err(|_| AppError::state("Failed to send application event"))?;
        Ok(())
    }

    /// Try to receive an event (non-blocking)
    pub async fn try_receive_event(&mut self) -> Option<AppEvent> {
        self.event_receiver.try_recv().ok()
    }

    /// Get a cloned sender for background tasks
    pub fn get_sender(&self) -> mpsc::UnboundedSender<AppEvent> {
        self.event_sender.clone()
    }
}

/// Application events for async communication
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// Git repository status was updated
    GitStatusUpdated(Vec<FileStatus>),

    /// Git operation completed
    GitOperationCompleted {
        operation: GitOperation,
        result: Result<String, String>,
        duration_ms: u64,
    },

    /// Agent task completed
    AgentTaskCompleted { task_id: Uuid, result: AgentResult },

    /// Agent task failed
    AgentTaskFailed {
        task_id: Uuid,
        error: String,
        retry_count: u32,
    },

    /// MCP protocol message received
    McpMessageReceived {
        message_id: String,
        content: String,
        timestamp: DateTime<Utc>,
    },

    /// MCP protocol error
    McpProtocolError {
        error: String,
        connection_id: String,
    },

    /// Performance warning
    PerformanceWarning {
        operation: String,
        expected_duration_ms: u64,
        actual_duration_ms: u64,
    },

    /// Configuration changed
    ConfigurationChanged {
        section: String,
        key: String,
        value: String,
    },

    /// UI state changed
    UIStateChanged { component: String, state: String },

    /// Background task started
    BackgroundTaskStarted {
        task_id: Uuid,
        task_type: BackgroundTaskType,
        estimated_duration: Option<Duration>,
    },

    /// Background task completed
    BackgroundTaskCompleted {
        task_id: Uuid,
        result: Result<String, String>,
        actual_duration: Duration,
    },

    /// Application error
    Error(String),

    /// Application shutdown requested
    Shutdown,
}

/// Git operations for event tracking
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitOperation {
    StatusRefresh,
    StageFile(String),
    UnstageFile(String),
    StageAll,
    UnstageAll,
    Commit(String),
    BranchCreate(String),
    BranchSwitch(String),
    BranchDelete(String),
    TagCreate(String),
    TagDelete(String),
    Fetch,
    Pull,
    Push,
    Stash,
    StashPop,
    Merge(String),
    Rebase(String),
}

impl GitOperation {
    /// Get the display name for the operation
    pub fn display_name(&self) -> &str {
        match self {
            GitOperation::StatusRefresh => "Status Refresh",
            GitOperation::StageFile(_) => "Stage File",
            GitOperation::UnstageFile(_) => "Unstage File",
            GitOperation::StageAll => "Stage All",
            GitOperation::UnstageAll => "Unstage All",
            GitOperation::Commit(_) => "Commit",
            GitOperation::BranchCreate(_) => "Create Branch",
            GitOperation::BranchSwitch(_) => "Switch Branch",
            GitOperation::BranchDelete(_) => "Delete Branch",
            GitOperation::TagCreate(_) => "Create Tag",
            GitOperation::TagDelete(_) => "Delete Tag",
            GitOperation::Fetch => "Fetch",
            GitOperation::Pull => "Pull",
            GitOperation::Push => "Push",
            GitOperation::Stash => "Stash",
            GitOperation::StashPop => "Stash Pop",
            GitOperation::Merge(_) => "Merge",
            GitOperation::Rebase(_) => "Rebase",
        }
    }

    /// Get the expected duration for performance monitoring
    pub fn expected_duration_ms(&self) -> u64 {
        match self {
            GitOperation::StatusRefresh => 200,
            GitOperation::StageFile(_) => 100,
            GitOperation::UnstageFile(_) => 100,
            GitOperation::StageAll => 500,
            GitOperation::UnstageAll => 300,
            GitOperation::Commit(_) => 1000,
            GitOperation::BranchCreate(_) => 200,
            GitOperation::BranchSwitch(_) => 300,
            GitOperation::BranchDelete(_) => 200,
            GitOperation::TagCreate(_) => 200,
            GitOperation::TagDelete(_) => 200,
            GitOperation::Fetch => 5000,
            GitOperation::Pull => 10000,
            GitOperation::Push => 10000,
            GitOperation::Stash => 500,
            GitOperation::StashPop => 500,
            GitOperation::Merge(_) => 2000,
            GitOperation::Rebase(_) => 5000,
        }
    }
}

/// Background task types for monitoring
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackgroundTaskType {
    GitStatusRefresh,
    AgentTaskExecution,
    McpProtocolCommunication,
    ConfigurationReload,
    PerformanceMonitoring,
    CacheCleanup,
}

impl BackgroundTaskType {
    /// Get the display name for the task type
    pub fn display_name(&self) -> &str {
        match self {
            BackgroundTaskType::GitStatusRefresh => "Git Status Refresh",
            BackgroundTaskType::AgentTaskExecution => "Agent Task Execution",
            BackgroundTaskType::McpProtocolCommunication => "MCP Communication",
            BackgroundTaskType::ConfigurationReload => "Configuration Reload",
            BackgroundTaskType::PerformanceMonitoring => "Performance Monitoring",
            BackgroundTaskType::CacheCleanup => "Cache Cleanup",
        }
    }
}

/// Event statistics for monitoring
#[derive(Debug, Clone)]
pub struct EventStatistics {
    pub total_events_processed: u64,
    pub events_per_second: f64,
    pub average_processing_time_ms: f64,
    pub error_rate: f64,
    pub last_reset: DateTime<Utc>,
}

impl Default for EventStatistics {
    fn default() -> Self {
        Self {
            total_events_processed: 0,
            events_per_second: 0.0,
            average_processing_time_ms: 0.0,
            error_rate: 0.0,
            last_reset: Utc::now(),
        }
    }
}

//! Application state management
//!
//! Centralized state management for the entire application following
//! the single source of truth principle.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{
    ai::AgentResult,
    git::{BranchInfo, FileStatus, GitService},
};

/// Central application state
///
/// Manages all application state including:
/// - Git repository information
/// - UI state and navigation
/// - Agent tasks and results
/// - Error states and notifications
/// - Performance metrics
#[derive(Debug, Clone)]
pub struct AppState {
    /// Application lifecycle state
    pub app_state: AppLifecycleState,

    /// Git repository state
    pub git_state: GitState,

    /// Git service for repository operations
    pub git_service: Option<GitService>,

    /// UI state
    pub ui_state: UIState,

    /// Agent system state
    pub agent_state: AgentState,

    /// Error and notification state
    pub notification_state: NotificationState,

    /// Performance monitoring state
    pub performance_state: PerformanceState,
}

impl AppState {
    /// Create a new application state
    pub fn new() -> Self {
        Self {
            app_state: AppLifecycleState::default(),
            git_state: GitState::default(),
            git_service: None,
            ui_state: UIState::default(),
            agent_state: AgentState::default(),
            notification_state: NotificationState::default(),
            performance_state: PerformanceState::default(),
        }
    }

    /// Set the Git service
    pub fn set_git_service(&mut self, git_service: GitService) {
        self.git_service = Some(git_service);
        self.git_state.is_repository = true;
    }

    /// Check if the application should quit
    pub fn should_quit(&self) -> bool {
        matches!(self.app_state.lifecycle, LifecyclePhase::Quitting)
    }

    /// Set the quit flag
    pub fn set_should_quit(&mut self, should_quit: bool) {
        if should_quit {
            self.app_state.lifecycle = LifecyclePhase::Quitting;
            self.app_state.quit_requested_at = Some(Utc::now());
        }
    }

    /// Update Git status
    pub fn update_git_status(&mut self, status: Vec<FileStatus>) {
        self.git_state.file_status = status;
        self.git_state.last_status_update = Utc::now();
    }

    /// Update agent task result
    pub fn update_agent_result(&mut self, task_id: Uuid, result: AgentResult) {
        self.agent_state.task_results.insert(task_id, result);
    }

    /// Add an error to the notification system
    pub fn add_error(&mut self, error: String) {
        self.notification_state.errors.push(ErrorNotification {
            id: Uuid::new_v4(),
            message: error,
            timestamp: Utc::now(),
            acknowledged: false,
        });
    }

    /// Get current active tab
    pub fn current_tab(&self) -> TabType {
        self.ui_state.current_tab
    }

    /// Set active tab
    pub fn set_current_tab(&mut self, tab: TabType) {
        self.ui_state.current_tab = tab;
        self.ui_state.tab_changed_at = Utc::now();
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Application lifecycle state
#[derive(Debug, Clone)]
pub struct AppLifecycleState {
    pub lifecycle: LifecyclePhase,
    pub started_at: DateTime<Utc>,
    pub quit_requested_at: Option<DateTime<Utc>>,
}

impl Default for AppLifecycleState {
    fn default() -> Self {
        Self {
            lifecycle: LifecyclePhase::Starting,
            started_at: Utc::now(),
            quit_requested_at: None,
        }
    }
}

/// Application lifecycle phases
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecyclePhase {
    Starting,
    Running,
    Quitting,
}

/// Git repository state
#[derive(Debug, Clone)]
pub struct GitState {
    pub repository_path: Option<String>,
    pub current_branch: Option<BranchInfo>,
    pub file_status: Vec<FileStatus>,
    pub last_status_update: DateTime<Utc>,
    pub is_repository: bool,
}

impl Default for GitState {
    fn default() -> Self {
        Self {
            repository_path: None,
            current_branch: None,
            file_status: Vec::new(),
            last_status_update: Utc::now(),
            is_repository: false,
        }
    }
}

/// UI state management
#[derive(Debug, Clone)]
pub struct UIState {
    pub current_tab: TabType,
    pub tab_changed_at: DateTime<Utc>,
    pub sidebar_width: u16,
    pub is_sidebar_visible: bool,
    pub terminal_size: (u16, u16),
    pub scroll_offset: usize,
    pub selected_item_index: usize,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            current_tab: TabType::Status,
            tab_changed_at: Utc::now(),
            sidebar_width: 25,
            is_sidebar_visible: true,
            terminal_size: (80, 24),
            scroll_offset: 0,
            selected_item_index: 0,
        }
    }
}

/// Available UI tabs matching the 6-tab design
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabType {
    Branches,
    Tags,
    Stash,
    Status,
    Remotes,
    GitFlow,
}

impl TabType {
    /// Get all available tabs
    pub fn all() -> &'static [TabType] {
        &[
            TabType::Branches,
            TabType::Tags,
            TabType::Stash,
            TabType::Status,
            TabType::Remotes,
            TabType::GitFlow,
        ]
    }

    /// Get tab display name
    pub fn name(&self) -> &'static str {
        match self {
            TabType::Branches => "Branches",
            TabType::Tags => "Tags",
            TabType::Stash => "Stash",
            TabType::Status => "Status",
            TabType::Remotes => "Remotes",
            TabType::GitFlow => "Git工作流",
        }
    }
}

/// Agent system state
#[derive(Debug, Clone)]
pub struct AgentState {
    pub active_agents: Vec<String>,
    pub task_results: HashMap<Uuid, AgentResult>,
    pub last_agent_activity: DateTime<Utc>,
}

impl Default for AgentState {
    fn default() -> Self {
        Self {
            active_agents: Vec::new(),
            task_results: HashMap::new(),
            last_agent_activity: Utc::now(),
        }
    }
}

/// Notification state for errors and messages
#[derive(Debug, Clone)]
pub struct NotificationState {
    pub errors: Vec<ErrorNotification>,
    pub info_messages: Vec<InfoNotification>,
}

impl Default for NotificationState {
    fn default() -> Self {
        Self {
            errors: Vec::new(),
            info_messages: Vec::new(),
        }
    }
}

/// Error notification
#[derive(Debug, Clone)]
pub struct ErrorNotification {
    pub id: Uuid,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub acknowledged: bool,
}

/// Info notification
#[derive(Debug, Clone)]
pub struct InfoNotification {
    pub id: Uuid,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub acknowledged: bool,
}

/// Performance monitoring state
#[derive(Debug, Clone)]
pub struct PerformanceState {
    pub startup_time_ms: u64,
    pub last_render_time_ms: u64,
    pub last_git_operation_time_ms: u64,
    pub memory_usage_mb: u64,
    pub performance_warnings: Vec<PerformanceWarning>,
}

impl Default for PerformanceState {
    fn default() -> Self {
        Self {
            startup_time_ms: 0,
            last_render_time_ms: 0,
            last_git_operation_time_ms: 0,
            memory_usage_mb: 0,
            performance_warnings: Vec::new(),
        }
    }
}

/// Performance warning
#[derive(Debug, Clone)]
pub struct PerformanceWarning {
    pub id: Uuid,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub severity: PerformanceWarningSeverity,
}

/// Performance warning severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerformanceWarningSeverity {
    Low,
    Medium,
    High,
}

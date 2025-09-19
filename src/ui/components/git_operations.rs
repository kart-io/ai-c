//! Advanced Git operations interface

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap},
    Frame,
};
use std::collections::HashMap;

use crate::{
    app::state::AppState,
    error::AppResult,
    git::CommitInfo,
    ui::theme::Theme,
};

use super::{Component, InputModal, ConfirmationModal, ProgressModal, Modal, ModalResult};

/// Safe UTF-8 string truncation utility
fn safe_truncate_string(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars - 3).collect();
        format!("{}...", truncated)
    }
}

/// Git operation types
#[derive(Debug, Clone, PartialEq)]
pub enum GitOperation {
    Rebase,
    CherryPick,
    Merge,
    Reset,
    Stash,
    Tag,
    Remote,
    Submodule,
    Worktree,
    Bisect,
    Reflog,
    Hooks,
}

impl GitOperation {
    pub fn as_str(&self) -> &'static str {
        match self {
            GitOperation::Rebase => "Rebase",
            GitOperation::CherryPick => "Cherry Pick",
            GitOperation::Merge => "Merge",
            GitOperation::Reset => "Reset",
            GitOperation::Stash => "Stash",
            GitOperation::Tag => "Tags",
            GitOperation::Remote => "Remotes",
            GitOperation::Submodule => "Submodules",
            GitOperation::Worktree => "Worktrees",
            GitOperation::Bisect => "Bisect",
            GitOperation::Reflog => "Reflog",
            GitOperation::Hooks => "Hooks",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            GitOperation::Rebase => "Interactive rebasing and history rewriting",
            GitOperation::CherryPick => "Apply commits from other branches",
            GitOperation::Merge => "Merge branches and resolve conflicts",
            GitOperation::Reset => "Reset to specific commits (soft/mixed/hard)",
            GitOperation::Stash => "Stash and manage work-in-progress changes",
            GitOperation::Tag => "Create and manage repository tags",
            GitOperation::Remote => "Manage remote repositories",
            GitOperation::Submodule => "Manage Git submodules",
            GitOperation::Worktree => "Manage Git worktrees",
            GitOperation::Bisect => "Binary search for bug introduction",
            GitOperation::Reflog => "View and recover from reflog",
            GitOperation::Hooks => "Manage Git hooks",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            GitOperation::Rebase => "🔄",
            GitOperation::CherryPick => "🍒",
            GitOperation::Merge => "🔀",
            GitOperation::Reset => "↩️",
            GitOperation::Stash => "📦",
            GitOperation::Tag => "🏷️",
            GitOperation::Remote => "🌐",
            GitOperation::Submodule => "📁",
            GitOperation::Worktree => "🌳",
            GitOperation::Bisect => "🔍",
            GitOperation::Reflog => "📜",
            GitOperation::Hooks => "🪝",
        }
    }
}

/// Focus state for operations tab
#[derive(Debug, Clone, PartialEq)]
pub enum OperationsFocus {
    OperationList,
    HistoryList,
}

/// Shortcut manager for conflict detection and help
#[derive(Debug, Clone)]
pub struct ShortcutManager {
    /// Last warning message about conflicts
    last_warning: Option<String>,
    /// Timestamp of last warning to auto-clear after delay
    warning_timestamp: Option<std::time::Instant>,
}

impl ShortcutManager {
    pub fn new() -> Self {
        Self {
            last_warning: None,
            warning_timestamp: None,
        }
    }

    /// Check if a key conflicts with main app shortcuts
    pub fn check_conflict(&mut self, key: KeyEvent) -> Option<String> {
        match key.code {
            KeyCode::Tab if !key.modifiers.contains(KeyModifiers::CONTROL | KeyModifiers::SHIFT) => {
                Some("Tab reserved for main app navigation. Use Ctrl+Tab for focus switching.".to_string())
            }
            KeyCode::Char(' ') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some("Space reserved for main app focus. Use Ctrl+Space for tab switching.".to_string())
            }
            _ => None
        }
    }

    /// Set a warning message
    pub fn set_warning(&mut self, message: String) {
        self.last_warning = Some(message);
        self.warning_timestamp = Some(std::time::Instant::now());
    }

    /// Get current warning, clearing it if expired
    pub fn get_warning(&mut self) -> Option<String> {
        if let Some(timestamp) = self.warning_timestamp {
            if timestamp.elapsed() > std::time::Duration::from_secs(3) {
                self.last_warning = None;
                self.warning_timestamp = None;
                return None;
            }
        }
        self.last_warning.clone()
    }

    /// Clear current warning
    pub fn clear_warning(&mut self) {
        self.last_warning = None;
        self.warning_timestamp = None;
    }
}

/// Git operations interface component
pub struct GitOperationsComponent {
    is_open: bool,
    selected_operation: usize,
    operations: Vec<GitOperation>,
    operation_state: ListState,
    current_tab: usize,
    tab_names: Vec<String>,

    // Focus state for operations tab
    operations_focus: OperationsFocus,

    // Modals
    input_modal: InputModal,
    confirmation_modal: ConfirmationModal,
    progress_modal: ProgressModal,

    // Operation-specific data
    rebase_data: RebaseData,
    merge_data: MergeData,
    stash_data: StashData,
    tag_data: TagData,
    remote_data: RemoteData,

    // History tab data
    history_commits: Vec<CommitInfo>,
    history_state: ListState,
    history_selected: usize,

    // Status
    current_operation: Option<GitOperation>,
    operation_progress: Option<String>,
    operation_error: Option<String>,

    // Shortcut conflict detection
    shortcut_manager: ShortcutManager,
}

/// Rebase operation data
#[derive(Debug, Clone, Default)]
pub struct RebaseData {
    pub target_branch: String,
    pub commits: Vec<RebaseCommit>,
    pub is_interactive: bool,
    pub current_step: usize,
    pub total_steps: usize,
}

#[derive(Debug, Clone)]
pub struct RebaseCommit {
    pub hash: String,
    pub message: String,
    pub action: RebaseAction,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RebaseAction {
    Pick,
    Reword,
    Edit,
    Squash,
    Fixup,
    Drop,
}

impl RebaseAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            RebaseAction::Pick => "pick",
            RebaseAction::Reword => "reword",
            RebaseAction::Edit => "edit",
            RebaseAction::Squash => "squash",
            RebaseAction::Fixup => "fixup",
            RebaseAction::Drop => "drop",
        }
    }
}

/// Merge operation data
#[derive(Debug, Clone, Default)]
pub struct MergeData {
    pub source_branch: String,
    pub target_branch: String,
    pub conflicts: Vec<MergeConflict>,
    pub is_fast_forward: bool,
    pub strategy: MergeStrategy,
}

#[derive(Debug, Clone)]
pub struct MergeConflict {
    pub file_path: String,
    pub status: ConflictStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConflictStatus {
    Unresolved,
    Resolved,
    Modified,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MergeStrategy {
    Recursive,
    Octopus,
    Ours,
    Subtree,
}

impl Default for MergeStrategy {
    fn default() -> Self {
        MergeStrategy::Recursive
    }
}

impl MergeStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            MergeStrategy::Recursive => "recursive",
            MergeStrategy::Octopus => "octopus",
            MergeStrategy::Ours => "ours",
            MergeStrategy::Subtree => "subtree",
        }
    }
}

/// Stash operation data
#[derive(Debug, Clone, Default)]
pub struct StashData {
    pub stashes: Vec<StashEntry>,
    pub selected_stash: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct StashEntry {
    pub index: usize,
    pub message: String,
    pub branch: String,
    pub timestamp: String,
}

/// Tag operation data
#[derive(Debug, Clone, Default)]
pub struct TagData {
    pub tags: Vec<TagEntry>,
    pub selected_tag: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct TagEntry {
    pub name: String,
    pub target: String,
    pub message: Option<String>,
    pub is_annotated: bool,
}

/// Remote operation data
#[derive(Debug, Clone, Default)]
pub struct RemoteData {
    pub remotes: Vec<RemoteEntry>,
    pub selected_remote: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct RemoteEntry {
    pub name: String,
    pub url: String,
    pub fetch_url: Option<String>,
}

impl GitOperationsComponent {
    pub fn new() -> Self {
        let operations = vec![
            GitOperation::Rebase,
            GitOperation::CherryPick,
            GitOperation::Merge,
            GitOperation::Reset,
            GitOperation::Stash,
            GitOperation::Tag,
            GitOperation::Remote,
            GitOperation::Submodule,
            GitOperation::Worktree,
            GitOperation::Bisect,
            GitOperation::Reflog,
            GitOperation::Hooks,
        ];

        let mut operation_state = ListState::default();
        operation_state.select(Some(0));

        let tab_names = vec![
            "Operations".to_string(),
            "Status".to_string(),
            "History".to_string(),
            "Help".to_string(),
        ];

        let mut history_state = ListState::default();
        history_state.select(Some(0));

        Self {
            is_open: false,
            selected_operation: 0,
            operations,
            operation_state,
            current_tab: 0,
            tab_names,
            operations_focus: OperationsFocus::OperationList, // Start with operations list focused
            input_modal: InputModal::new(),
            confirmation_modal: ConfirmationModal::new(),
            progress_modal: ProgressModal::new(),
            rebase_data: RebaseData::default(),
            merge_data: MergeData::default(),
            stash_data: StashData::default(),
            tag_data: TagData::default(),
            remote_data: RemoteData::default(),
            history_commits: Vec::new(),
            history_state,
            history_selected: 0,
            current_operation: None,
            operation_progress: None,
            operation_error: None,
            shortcut_manager: ShortcutManager::new(),
        }
    }

    /// Open Git operations interface
    pub fn open(&mut self) {
        self.is_open = true;
        self.current_tab = 0;
        self.operation_state.select(Some(0));
        self.operations_focus = OperationsFocus::OperationList; // Reset focus to operations list

        // Initialize history state if we have commits
        if !self.history_commits.is_empty() {
            self.history_state.select(Some(0));
            self.history_selected = 0;
        }
    }

    /// Load commit history for History tab
    pub async fn load_commit_history(&mut self, state: &AppState) {
        if let Some(git_service) = &state.git_service {
            if let Ok(commits) = git_service.get_commit_history(50).await {
                self.history_commits = commits;
                if !self.history_commits.is_empty() {
                    self.history_state.select(Some(0));
                    self.history_selected = 0;
                }
            }
        }
    }

    /// Close Git operations interface
    pub fn close(&mut self) {
        self.is_open = false;
        self.current_operation = None;
        self.operation_progress = None;
        self.operation_error = None;
        self.input_modal.close();
        self.confirmation_modal.close();
        self.progress_modal.close();
    }

    /// Check if interface is open
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    /// Switch to next tab
    pub fn next_tab(&mut self) {
        self.current_tab = (self.current_tab + 1) % self.tab_names.len();
    }

    /// Switch to previous tab
    pub fn previous_tab(&mut self) {
        self.current_tab = if self.current_tab == 0 {
            self.tab_names.len() - 1
        } else {
            self.current_tab - 1
        };
    }

    /// Move selection up with boundary handling
    pub fn move_up(&mut self) {
        let len = self.operations.len();
        if len == 0 {
            // Clear selection if no operations
            self.operation_state.select(None);
            return;
        }

        let selected = self.operation_state.selected().unwrap_or(0);
        let new_selected = if selected == 0 { len - 1 } else { selected - 1 };
        self.operation_state.select(Some(new_selected));
        self.selected_operation = new_selected;
    }

    /// Move selection down with boundary handling
    pub fn move_down(&mut self) {
        let len = self.operations.len();
        if len == 0 {
            // Clear selection if no operations
            self.operation_state.select(None);
            return;
        }

        let selected = self.operation_state.selected().unwrap_or(0);
        let new_selected = (selected + 1) % len;
        self.operation_state.select(Some(new_selected));
        self.selected_operation = new_selected;
    }

    /// Move history selection up with bounds checking
    pub fn move_history_up(&mut self) {
        let len = self.history_commits.len();
        if len == 0 {
            // Clear selection if no history
            self.history_state.select(None);
            self.history_selected = 0;
            return;
        }

        let old_selected = self.history_selected;
        self.history_selected = if self.history_selected == 0 {
            len - 1
        } else {
            self.history_selected - 1
        };

        // Only update state if actually changed
        if old_selected != self.history_selected {
            self.history_state.select(Some(self.history_selected));
        }
    }

    /// Move history selection down with bounds checking
    pub fn move_history_down(&mut self) {
        let len = self.history_commits.len();
        if len == 0 {
            // Clear selection if no history
            self.history_state.select(None);
            self.history_selected = 0;
            return;
        }

        let old_selected = self.history_selected;
        self.history_selected = (self.history_selected + 1) % len;

        // Only update state if actually changed
        if old_selected != self.history_selected {
            self.history_state.select(Some(self.history_selected));
        }
    }

    /// Execute selected operation
    pub fn execute_selected_operation(&mut self) -> AppResult<()> {
        if let Some(operation) = self.operations.get(self.selected_operation) {
            self.start_operation(operation.clone())?;
        }
        Ok(())
    }

    /// Start a Git operation
    fn start_operation(&mut self, operation: GitOperation) -> AppResult<()> {
        self.current_operation = Some(operation.clone());
        self.operation_error = None;

        match operation {
            GitOperation::Rebase => self.start_rebase_operation(),
            GitOperation::CherryPick => self.start_cherry_pick_operation(),
            GitOperation::Merge => self.start_merge_operation(),
            GitOperation::Reset => self.start_reset_operation(),
            GitOperation::Stash => self.start_stash_operation(),
            GitOperation::Tag => self.start_tag_operation(),
            GitOperation::Remote => self.start_remote_operation(),
            GitOperation::Hooks => self.start_hooks_operation(),
            _ => {
                self.operation_error = Some(format!("{} operation not yet implemented", operation.as_str()));
                Ok(())
            }
        }
    }

    /// Start rebase operation
    fn start_rebase_operation(&mut self) -> AppResult<()> {
        self.input_modal.open("Interactive Rebase", "Enter target branch:");
        Ok(())
    }

    /// Start cherry-pick operation
    fn start_cherry_pick_operation(&mut self) -> AppResult<()> {
        self.input_modal.open("Cherry Pick", "Enter commit hash(es):");
        Ok(())
    }

    /// Start merge operation
    fn start_merge_operation(&mut self) -> AppResult<()> {
        self.input_modal.open("Merge Branch", "Enter branch to merge:");
        Ok(())
    }

    /// Start reset operation
    fn start_reset_operation(&mut self) -> AppResult<()> {
        self.input_modal.open("Reset", "Enter commit hash or HEAD~n:");
        Ok(())
    }

    /// Start stash operation
    fn start_stash_operation(&mut self) -> AppResult<()> {
        // Load current stashes
        self.load_stashes()?;
        self.current_tab = 0; // Switch to operations tab
        Ok(())
    }

    /// Start tag operation
    fn start_tag_operation(&mut self) -> AppResult<()> {
        // Load current tags
        self.load_tags()?;
        self.current_tab = 0; // Switch to operations tab
        Ok(())
    }

    /// Start remote operation
    fn start_remote_operation(&mut self) -> AppResult<()> {
        // Load current remotes
        self.load_remotes()?;
        self.current_tab = 0; // Switch to operations tab
        Ok(())
    }

    /// Start hooks operation
    fn start_hooks_operation(&mut self) -> AppResult<()> {
        self.operation_progress = Some("Managing Git hooks...".to_string());
        Ok(())
    }

    /// Load stash entries
    fn load_stashes(&mut self) -> AppResult<()> {
        // TODO: Load actual stash data from Git service
        self.stash_data.stashes = vec![
            StashEntry {
                index: 0,
                message: "WIP: working on feature".to_string(),
                branch: "feature/new-ui".to_string(),
                timestamp: "2024-01-15 14:30:00".to_string(),
            },
            StashEntry {
                index: 1,
                message: "experimental changes".to_string(),
                branch: "main".to_string(),
                timestamp: "2024-01-14 09:15:00".to_string(),
            },
        ];
        Ok(())
    }

    /// Load tag entries
    fn load_tags(&mut self) -> AppResult<()> {
        // TODO: Load actual tag data from Git service
        self.tag_data.tags = vec![
            TagEntry {
                name: "v1.0.0".to_string(),
                target: "a1b2c3d4".to_string(),
                message: Some("Release version 1.0.0".to_string()),
                is_annotated: true,
            },
            TagEntry {
                name: "v0.9.0".to_string(),
                target: "e5f6g7h8".to_string(),
                message: None,
                is_annotated: false,
            },
        ];
        Ok(())
    }

    /// Load remote entries
    fn load_remotes(&mut self) -> AppResult<()> {
        // TODO: Load actual remote data from Git service
        self.remote_data.remotes = vec![
            RemoteEntry {
                name: "origin".to_string(),
                url: "git@github.com:user/repo.git".to_string(),
                fetch_url: None,
            },
            RemoteEntry {
                name: "upstream".to_string(),
                url: "git@github.com:upstream/repo.git".to_string(),
                fetch_url: None,
            },
        ];
        Ok(())
    }

    /// Render main interface
    fn render_main_interface(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Calculate centered area
        let modal_area = Self::centered_rect(80, 70, area);

        // Clear background
        frame.render_widget(Clear, modal_area);

        // Split into tabs, content, and help bar
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tabs
                Constraint::Min(0),    // Content
                Constraint::Length(2), // Help bar
            ])
            .split(modal_area);

        // Render tabs
        let tab_titles: Vec<Line> = self.tab_names.iter()
            .map(|name| Line::from(name.as_str()))
            .collect();

        let tabs = Tabs::new(tab_titles)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Git Operations"))
            .select(self.current_tab)
            .style(Style::default())
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

        frame.render_widget(tabs, chunks[0]);

        // Render content based on current tab
        match self.current_tab {
            0 => self.render_operations_tab(frame, chunks[1], theme),
            1 => self.render_status_tab(frame, chunks[1], theme),
            2 => self.render_history_tab(frame, chunks[1], theme),
            3 => self.render_help_tab(frame, chunks[1], theme),
            _ => {}
        }

        // Render help bar at bottom
        self.render_help_bar(frame, chunks[2], theme);
    }

    /// Render help bar with current shortcuts
    fn render_help_bar(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let help_text = if self.current_tab == 0 {
            match self.operations_focus {
                OperationsFocus::OperationList => {
                    " Operations Panel | Ctrl+Tab: ⇄ History | →: Focus History | ↑↓: Navigate | Enter: Execute | Ctrl+Space: Next Tab | Esc: Exit"
                }
                OperationsFocus::HistoryList => {
                    " History Panel | Ctrl+Tab: ⇄ Operations | ←: Focus Operations | ↑↓: Navigate | PgUp/Dn: ±5 | Ctrl+Space: Next Tab | Esc: Exit"
                }
            }
        } else {
            match self.current_tab {
                1 => " Status Tab | Ctrl+Space: Next Tab | Shift+Tab: Previous Tab | Esc: Exit",
                2 => " History Tab | ↑↓: Navigate | PgUp/Dn: Quick Scroll | Ctrl+Space: Next Tab | Shift+Tab: Previous Tab | Esc: Exit",
                3 => " Help Tab | Ctrl+Space: Next Tab | Shift+Tab: Previous Tab | Esc: Exit",
                _ => " Ctrl+Space: Next Tab | Shift+Tab: Previous Tab | Esc: Exit"
            }
        };

        let help_bar = Paragraph::new(Line::from(help_text))
            .block(Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::DarkGray))
                .title("Git Operations Shortcuts"))
            .style(Style::default().fg(Color::Gray));

        frame.render_widget(help_bar, area);
    }

    /// Render operations tab
    fn render_operations_tab(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40), // Operation list
                Constraint::Percentage(60), // History commits list (previously operation details)
            ])
            .split(area);

        // Render operation list with focus indication
        let items: Vec<ListItem> = self.operations.iter().map(|op| {
            let line = Line::from(vec![
                Span::raw(op.icon()),
                Span::raw(" "),
                Span::styled(op.as_str(), Style::default().add_modifier(Modifier::BOLD)),
            ]);
            ListItem::new(line)
        }).collect();

        let operation_title = if self.operations_focus == OperationsFocus::OperationList {
            "🎯 Operations [FOCUSED]"
        } else {
            "Operations"
        };

        let operation_list = List::new(items)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(operation_title)
                .border_style(if self.operations_focus == OperationsFocus::OperationList {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                }))
            .highlight_style(if self.operations_focus == OperationsFocus::OperationList {
                Style::default()
                    .bg(theme.selection_color())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            })
            .highlight_symbol(if self.operations_focus == OperationsFocus::OperationList { "▶ " } else { "  " });

        frame.render_stateful_widget(operation_list, chunks[0], &mut self.operation_state);

        // Render history commits in the right panel
        self.render_history_panel(frame, chunks[1], theme);
    }

    /// Render history panel in the operations tab
    fn render_history_panel(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if self.history_commits.is_empty() {
            let lines = vec![
                Line::from("Git Commit History"),
                Line::from(""),
                Line::from("Loading commit history..."),
                Line::from(""),
                Line::from("Shortcuts are shown in the bottom bar."),
            ];

            let history_widget = Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title("History"))
                .style(theme.text_style())
                .wrap(Wrap { trim: true });

            frame.render_widget(history_widget, area);
            return;
        }

        // Split area for commit list and details
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(60), // Commit list
                Constraint::Percentage(40), // Commit details
            ])
            .split(area);

        // Render commit list
        let items: Vec<ListItem> = self.history_commits.iter().take(15).map(|commit| {
            let short_hash = &commit.hash[..std::cmp::min(8, commit.hash.len())];
            let short_message = safe_truncate_string(&commit.message, 40);

            let line = Line::from(vec![
                Span::styled(
                    format!("{} ", short_hash),
                    Style::default().fg(Color::Yellow)
                ),
                Span::raw(short_message),
            ]);
            ListItem::new(line)
        }).collect();

        let history_title = if self.operations_focus == OperationsFocus::HistoryList {
            format!("🎯 History ({}) [FOCUSED]", self.history_commits.len())
        } else {
            format!("History ({})", self.history_commits.len())
        };

        let commit_list = List::new(items)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(history_title)
                .border_style(if self.operations_focus == OperationsFocus::HistoryList {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                }))
            .highlight_style(if self.operations_focus == OperationsFocus::HistoryList {
                Style::default()
                    .bg(theme.selection_color())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            })
            .highlight_symbol(if self.operations_focus == OperationsFocus::HistoryList { "▶ " } else { "  " });

        frame.render_stateful_widget(commit_list, chunks[0], &mut self.history_state);

        // Render selected commit details
        if let Some(commit) = self.history_commits.get(self.history_selected) {
            let lines = vec![
                Line::from(vec![
                    Span::styled("Hash: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(&commit.hash[..std::cmp::min(16, commit.hash.len())]),
                ]),
                Line::from(vec![
                    Span::styled("Author: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(&commit.author),
                ]),
                Line::from(vec![
                    Span::styled("Date: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(commit.date.format("%Y-%m-%d %H:%M").to_string()),
                ]),
                Line::from(""),
                Line::from(commit.message.as_str()),
            ];

            let details_widget = Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title("Commit Details"))
                .style(theme.text_style())
                .wrap(Wrap { trim: true });

            frame.render_widget(details_widget, chunks[1]);
        }
    }

    /// Render operation details
    fn render_operation_details(&self, frame: &mut Frame, area: Rect, operation: &GitOperation, theme: &Theme) {
        let details = match operation {
            GitOperation::Rebase => self.render_rebase_details(area, theme),
            GitOperation::Merge => self.render_merge_details(area, theme),
            GitOperation::Stash => self.render_stash_details(area, theme),
            GitOperation::Tag => self.render_tag_details(area, theme),
            GitOperation::Remote => self.render_remote_details(area, theme),
            _ => {
                let text = vec![
                    Line::from(operation.description()),
                    Line::from(""),
                    Line::from("Press Enter to start this operation."),
                ];

                Paragraph::new(text)
                    .block(Block::default().borders(Borders::ALL).title("Details"))
                    .style(theme.text_style())
                    .wrap(Wrap { trim: true })
            }
        };

        frame.render_widget(details, area);
    }

    /// Render rebase details
    fn render_rebase_details(&self, area: Rect, theme: &Theme) -> Paragraph {
        let text = vec![
            Line::from("Interactive Rebase"),
            Line::from(""),
            Line::from("Available actions:"),
            Line::from("• pick - use commit"),
            Line::from("• reword - use commit, but edit message"),
            Line::from("• edit - use commit, but stop for amending"),
            Line::from("• squash - use commit, meld into previous"),
            Line::from("• fixup - like squash, discard message"),
            Line::from("• drop - remove commit"),
            Line::from(""),
            Line::from("Press Enter to start interactive rebase."),
        ];

        Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Rebase Details"))
            .style(theme.text_style())
            .wrap(Wrap { trim: true })
    }

    /// Render merge details
    fn render_merge_details(&self, area: Rect, theme: &Theme) -> Paragraph {
        let text = if self.merge_data.conflicts.is_empty() {
            vec![
                Line::from("Merge Branch"),
                Line::from(""),
                Line::from("Merge strategies:"),
                Line::from("• recursive (default)"),
                Line::from("• octopus (multiple heads)"),
                Line::from("• ours (favor our changes)"),
                Line::from("• subtree (subtree merge)"),
                Line::from(""),
                Line::from("Press Enter to start merge."),
            ]
        } else {
            let mut lines = vec![
                Line::from("Merge Conflicts Detected"),
                Line::from(""),
                Line::from("Conflicted files:"),
            ];

            for conflict in &self.merge_data.conflicts {
                let status_icon = match conflict.status {
                    ConflictStatus::Unresolved => "❌",
                    ConflictStatus::Resolved => "✅",
                    ConflictStatus::Modified => "⚠️",
                };
                lines.push(Line::from(format!("{} {}", status_icon, conflict.file_path)));
            }

            lines.extend(vec![
                Line::from(""),
                Line::from("Resolve conflicts manually, then continue merge."),
            ]);

            lines
        };

        Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Merge Details"))
            .style(theme.text_style())
            .wrap(Wrap { trim: true })
    }

    /// Render stash details
    fn render_stash_details(&self, area: Rect, theme: &Theme) -> Paragraph {
        let text = if self.stash_data.stashes.is_empty() {
            vec![
                Line::from("No stashes found"),
                Line::from(""),
                Line::from("Create a new stash:"),
                Line::from("• Stash current changes"),
                Line::from("• Stash with message"),
                Line::from("• Stash including untracked files"),
            ]
        } else {
            let mut lines = vec![
                Line::from("Available Stashes:"),
                Line::from(""),
            ];

            for stash in &self.stash_data.stashes {
                lines.push(Line::from(format!("stash@{{{}}}: {}", stash.index, stash.message)));
                lines.push(Line::from(format!("  Branch: {} | {}", stash.branch, stash.timestamp)));
                lines.push(Line::from(""));
            }

            lines.extend(vec![
                Line::from("Actions:"),
                Line::from("• Apply stash"),
                Line::from("• Pop stash"),
                Line::from("• Drop stash"),
                Line::from("• Create branch from stash"),
            ]);

            lines
        };

        Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Stash Details"))
            .style(theme.text_style())
            .wrap(Wrap { trim: true })
    }

    /// Render tag details
    fn render_tag_details(&self, area: Rect, theme: &Theme) -> Paragraph {
        let text = if self.tag_data.tags.is_empty() {
            vec![
                Line::from("No tags found"),
                Line::from(""),
                Line::from("Create a new tag:"),
                Line::from("• Lightweight tag"),
                Line::from("• Annotated tag"),
                Line::from("• Signed tag"),
            ]
        } else {
            let mut lines = vec![
                Line::from("Available Tags:"),
                Line::from(""),
            ];

            for tag in &self.tag_data.tags {
                let tag_type = if tag.is_annotated { "annotated" } else { "lightweight" };
                lines.push(Line::from(format!("{} ({})", tag.name, tag_type)));
                lines.push(Line::from(format!("  Target: {}", tag.target)));
                if let Some(ref message) = tag.message {
                    lines.push(Line::from(format!("  Message: {}", message)));
                }
                lines.push(Line::from(""));
            }

            lines.extend(vec![
                Line::from("Actions:"),
                Line::from("• Create tag"),
                Line::from("• Delete tag"),
                Line::from("• Push tag"),
                Line::from("• Checkout tag"),
            ]);

            lines
        };

        Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Tag Details"))
            .style(theme.text_style())
            .wrap(Wrap { trim: true })
    }

    /// Render remote details
    fn render_remote_details(&self, area: Rect, theme: &Theme) -> Paragraph {
        let text = if self.remote_data.remotes.is_empty() {
            vec![
                Line::from("No remotes configured"),
                Line::from(""),
                Line::from("Add a remote:"),
                Line::from("• Add origin remote"),
                Line::from("• Add upstream remote"),
                Line::from("• Add custom remote"),
            ]
        } else {
            let mut lines = vec![
                Line::from("Configured Remotes:"),
                Line::from(""),
            ];

            for remote in &self.remote_data.remotes {
                lines.push(Line::from(format!("{}", remote.name)));
                lines.push(Line::from(format!("  URL: {}", remote.url)));
                if let Some(ref fetch_url) = remote.fetch_url {
                    lines.push(Line::from(format!("  Fetch: {}", fetch_url)));
                }
                lines.push(Line::from(""));
            }

            lines.extend(vec![
                Line::from("Actions:"),
                Line::from("• Add remote"),
                Line::from("• Remove remote"),
                Line::from("• Change URL"),
                Line::from("• Fetch from remote"),
                Line::from("• Push to remote"),
            ]);

            lines
        };

        Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Remote Details"))
            .style(theme.text_style())
            .wrap(Wrap { trim: true })
    }

    /// Render status tab
    fn render_status_tab(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let mut lines = vec![
            Line::from("Git Operations Status"),
            Line::from(""),
        ];

        // Show shortcut conflict warning if any
        if let Some(warning) = self.shortcut_manager.get_warning() {
            lines.push(Line::from(Span::styled(
                format!("⚠️ Shortcut Conflict: {}", warning),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            )));
            lines.push(Line::from(""));
        }

        if let Some(ref operation) = self.current_operation {
            lines.push(Line::from(format!("Current Operation: {}", operation.as_str())));

            if let Some(ref progress) = self.operation_progress {
                lines.push(Line::from(format!("Progress: {}", progress)));
            }

            if let Some(ref error) = self.operation_error {
                lines.push(Line::from(Span::styled(
                    format!("Error: {}", error),
                    Style::default().fg(Color::Red)
                )));
            }
        } else {
            lines.push(Line::from("No operation in progress"));
        }

        lines.extend(vec![
            Line::from(""),
            Line::from("💡 Shortcut Tips:"),
            Line::from("• Use Ctrl+Tab for focus switching (not Tab)"),
            Line::from("• Use Ctrl+Space for tab switching (not Space)"),
            Line::from("• Regular Tab/Space are reserved for main app"),
            Line::from(""),
            Line::from("Recent Operations:"),
            Line::from("• Rebase completed successfully"),
            Line::from("• Merge with conflicts resolved"),
            Line::from("• Stash created: 'WIP: feature work'"),
        ]);

        let status_widget = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Status & Shortcuts"))
            .style(theme.text_style())
            .wrap(Wrap { trim: true });

        frame.render_widget(status_widget, area);
    }

    /// Render history tab
    fn render_history_tab(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if self.history_commits.is_empty() {
            let lines = vec![
                Line::from("Git Commit History"),
                Line::from(""),
                Line::from("No commits found or repository not initialized."),
                Line::from(""),
                Line::from("Use ↑/↓ to navigate commits when available."),
            ];

            let history_widget = Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title("History"))
                .style(theme.text_style())
                .wrap(Wrap { trim: true });

            frame.render_widget(history_widget, area);
            return;
        }

        // Create chunks for commit list and details
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(60), // Commit list
                Constraint::Percentage(40), // Commit details
            ])
            .split(area);

        // Render commit list
        let items: Vec<ListItem> = self.history_commits.iter().map(|commit| {
            let short_hash = &commit.hash[..std::cmp::min(8, commit.hash.len())];
            let short_message = safe_truncate_string(&commit.message, 50);

            let line = Line::from(vec![
                Span::styled(
                    format!("{} ", short_hash),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                ),
                Span::styled(
                    short_message,
                    Style::default().fg(Color::White)
                ),
            ]);
            ListItem::new(line)
        }).collect();

        let commit_list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Commits"))
            .highlight_style(
                Style::default()
                    .bg(theme.selection_color())
                    .add_modifier(Modifier::BOLD)
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(commit_list, chunks[0], &mut self.history_state);

        // Render commit details
        if let Some(commit) = self.history_commits.get(self.history_selected) {
            let lines = vec![
                Line::from(vec![
                    Span::styled("Hash: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(&commit.hash),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Author: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(&commit.author),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Date: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(commit.date.format("%Y-%m-%d %H:%M:%S").to_string()),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Message: ", Style::default().add_modifier(Modifier::BOLD)),
                ]),
                Line::from(commit.message.as_str()),
            ];

            let details_widget = Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title("Commit Details"))
                .style(theme.text_style())
                .wrap(Wrap { trim: true });

            frame.render_widget(details_widget, chunks[1]);
        }
    }

    /// Render help tab
    fn render_help_tab(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let lines = vec![
            Line::from("Git Operations Help & Shortcuts Reference"),
            Line::from(""),
            Line::from("🎯 Context-Aware Navigation:"),
            Line::from(""),
            Line::from("Operations Tab (Dual Panel):"),
            Line::from("• Ctrl+Tab   - Switch focus between Operations ⇄ History panels"),
            Line::from("• ←/→        - Quick focus jump: ← Operations | → History"),
            Line::from("• ↑/↓        - Navigate in currently focused panel"),
            Line::from("• PageUp/Dn  - Quick scroll history (±5 commits at once)"),
            Line::from("• Enter      - Execute selected Git operation"),
            Line::from(""),
            Line::from("Other Tabs (Status/History/Help):"),
            Line::from("• ↑/↓        - Navigate items in the active list"),
            Line::from("• PageUp/Dn  - Quick scroll (where applicable)"),
            Line::from(""),
            Line::from("🏷️ Tab Management:"),
            Line::from("• Ctrl+Space - Next tab (avoids main app Tab conflict)"),
            Line::from("• Shift+Tab  - Previous tab"),
            Line::from("• 1-4        - Jump directly to Operations/Status/History/Help"),
            Line::from(""),
            Line::from("🎨 Visual Indicators:"),
            Line::from("• ◆ marker   - Shows active panel in Operations tab"),
            Line::from("• Yellow border - Indicates focused panel"),
            Line::from("• ▶ symbol   - Selected item in lists"),
            Line::from(""),
            Line::from("⚠️ Key Conflict Avoidance:"),
            Line::from("• Tab/Shift+Tab reserved for main app navigation"),
            Line::from("• Space reserved for main app focus switching"),
            Line::from("• Git Operations uses Ctrl+ modifiers to avoid conflicts"),
            Line::from(""),
            Line::from("💡 Tips:"),
            Line::from("• Bottom bar shows context-sensitive shortcuts"),
            Line::from("• Use Esc anytime to exit Git Operations interface"),
            Line::from("• All shortcuts work immediately without confirmation"),
        ];

        let help_widget = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("📚 Complete Shortcuts Guide"))
            .style(theme.text_style())
            .wrap(Wrap { trim: true });

        frame.render_widget(help_widget, area);
    }

    /// Calculate centered rectangle
    fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }
}

impl Component for GitOperationsComponent {
    fn render(&mut self, frame: &mut Frame, area: Rect, _state: &AppState, theme: &Theme) {
        if !self.is_open {
            return;
        }

        // Render main interface
        self.render_main_interface(frame, area, theme);

        // Render modals on top if open
        if self.input_modal.is_open() {
            self.input_modal.render(frame, area, theme);
        }

        if self.confirmation_modal.is_open() {
            self.confirmation_modal.render(frame, area, theme);
        }

        if self.progress_modal.is_open() {
            self.progress_modal.render(frame, area, theme);
        }
    }

    fn handle_key_event(&mut self, key: KeyEvent, _state: &mut AppState) -> AppResult<()> {
        if !self.is_open {
            return Ok(());
        }

        // Check for shortcut conflicts first
        if let Some(warning) = self.shortcut_manager.check_conflict(key) {
            self.shortcut_manager.set_warning(warning);
            return Ok(()); // Don't process conflicting keys
        }

        // Handle modal events first
        if self.input_modal.is_open() {
            match self.input_modal.handle_key_event(key)? {
                ModalResult::Input(input) => {
                    // Process input based on current operation
                    if let Some(ref operation) = self.current_operation {
                        match operation {
                            GitOperation::Rebase => {
                                self.rebase_data.target_branch = input;
                                self.operation_progress = Some("Starting interactive rebase...".to_string());
                            }
                            GitOperation::CherryPick => {
                                self.operation_progress = Some(format!("Cherry-picking commit(s): {}", input));
                            }
                            GitOperation::Merge => {
                                self.merge_data.source_branch = input;
                                self.operation_progress = Some("Starting merge...".to_string());
                            }
                            GitOperation::Reset => {
                                self.operation_progress = Some(format!("Resetting to: {}", input));
                            }
                            _ => {}
                        }
                    }
                    self.current_tab = 1; // Switch to status tab
                }
                ModalResult::Cancelled => {
                    self.current_operation = None;
                }
                ModalResult::None => {}
                _ => {}
            }
            return Ok(());
        }

        if self.confirmation_modal.is_open() {
            match self.confirmation_modal.handle_key_event(key)? {
                ModalResult::Confirmed => {
                    // Execute confirmed operation
                }
                ModalResult::Cancelled => {
                    self.current_operation = None;
                }
                ModalResult::None => {}
                _ => {}
            }
            return Ok(());
        }

        // Handle main interface events
        match key.code {
            KeyCode::Esc => self.close(),
            // Direct tab navigation with number keys (1-4)
            KeyCode::Char(c @ '1'..='4') => {
                let tab_index = (c as u8 - b'1') as usize;
                if tab_index < self.tab_names.len() {
                    self.current_tab = tab_index;
                    // Reset focus to operations list when switching to operations tab
                    if tab_index == 0 {
                        self.operations_focus = OperationsFocus::OperationList;
                    }
                }
            }
            // Use Ctrl+Tab instead of Tab to avoid conflict with main tab navigation
            KeyCode::Tab if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+Tab to switch focus between panels in operations tab
                if self.current_tab == 0 {
                    if self.history_commits.is_empty() {
                        // If no history, show helpful message and stay on operations
                        self.operation_error = Some("No commit history available. Focus remains on operations list.".to_string());
                        // Clear error after 3 seconds would be nice, but we'll just keep focus on operations
                    } else {
                        // Normal focus switching
                        self.operations_focus = match self.operations_focus {
                            OperationsFocus::OperationList => OperationsFocus::HistoryList,
                            OperationsFocus::HistoryList => OperationsFocus::OperationList,
                        };
                        // Clear any previous error
                        self.operation_error = None;
                    }
                }
            }
            // Use Ctrl+Space instead of Space to switch tabs (avoid conflicts)
            KeyCode::Char(' ') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.next_tab();
            }
            // Use Shift+Tab for previous tab
            KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.previous_tab();
            }
            KeyCode::Up => {
                if self.current_tab == 0 {
                    // In operations tab, navigate based on focus
                    match self.operations_focus {
                        OperationsFocus::OperationList => self.move_up(),
                        OperationsFocus::HistoryList => self.move_history_up(),
                    }
                } else if self.current_tab == 2 {
                    // History tab
                    self.move_history_up();
                }
            }
            KeyCode::Down => {
                if self.current_tab == 0 {
                    // In operations tab, navigate based on focus
                    match self.operations_focus {
                        OperationsFocus::OperationList => self.move_down(),
                        OperationsFocus::HistoryList => self.move_history_down(),
                    }
                } else if self.current_tab == 2 {
                    // History tab
                    self.move_history_down();
                }
            }
            KeyCode::Left => {
                if self.current_tab == 0 {
                    // Switch focus to operations list
                    self.operations_focus = OperationsFocus::OperationList;
                }
            }
            KeyCode::Right => {
                if self.current_tab == 0 {
                    if !self.history_commits.is_empty() {
                        // Switch focus to history panel
                        self.operations_focus = OperationsFocus::HistoryList;
                        self.operation_error = None; // Clear any error
                    } else {
                        // Show helpful message when no history is available
                        self.operation_error = Some("No commit history available to focus on.".to_string());
                    }
                }
            }
            KeyCode::PageUp => {
                // Enhanced quick navigation with feedback
                match self.current_tab {
                    0 => {
                        // Operations tab - only works when history panel is focused
                        if self.operations_focus == OperationsFocus::HistoryList && !self.history_commits.is_empty() {
                            let len = self.history_commits.len();
                            let old_selected = self.history_selected;
                            self.history_selected = self.history_selected.saturating_sub(5);
                            if old_selected != self.history_selected {
                                self.history_state.select(Some(self.history_selected));
                            }
                        }
                    }
                    2 => {
                        // History tab - always works
                        if !self.history_commits.is_empty() {
                            let len = self.history_commits.len();
                            let old_selected = self.history_selected;
                            self.history_selected = self.history_selected.saturating_sub(5);
                            if old_selected != self.history_selected {
                                self.history_state.select(Some(self.history_selected));
                            }
                        }
                    }
                    _ => {} // No action for other tabs
                }
            }
            KeyCode::PageDown => {
                // Enhanced quick navigation with feedback
                match self.current_tab {
                    0 => {
                        // Operations tab - only works when history panel is focused
                        if self.operations_focus == OperationsFocus::HistoryList && !self.history_commits.is_empty() {
                            let len = self.history_commits.len();
                            let old_selected = self.history_selected;
                            self.history_selected = (self.history_selected + 5).min(len - 1);
                            if old_selected != self.history_selected {
                                self.history_state.select(Some(self.history_selected));
                            }
                        }
                    }
                    2 => {
                        // History tab - always works
                        if !self.history_commits.is_empty() {
                            let len = self.history_commits.len();
                            let old_selected = self.history_selected;
                            self.history_selected = (self.history_selected + 5).min(len - 1);
                            if old_selected != self.history_selected {
                                self.history_state.select(Some(self.history_selected));
                            }
                        }
                    }
                    _ => {} // No action for other tabs
                }
            }
            KeyCode::Enter => {
                if self.current_tab == 0 && self.operations_focus == OperationsFocus::OperationList {
                    self.execute_selected_operation()?;
                }
                // Could add functionality for history list Enter key here
            }
            _ => {}
        }

        Ok(())
    }
}
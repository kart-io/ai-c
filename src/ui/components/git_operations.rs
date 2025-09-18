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
    ui::theme::Theme,
};

use super::{Component, InputModal, ConfirmationModal, ProgressModal, Modal, ModalResult};

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

/// Git operations interface component
pub struct GitOperationsComponent {
    is_open: bool,
    selected_operation: usize,
    operations: Vec<GitOperation>,
    operation_state: ListState,
    current_tab: usize,
    tab_names: Vec<String>,

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

    // Status
    current_operation: Option<GitOperation>,
    operation_progress: Option<String>,
    operation_error: Option<String>,
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

        Self {
            is_open: false,
            selected_operation: 0,
            operations,
            operation_state,
            current_tab: 0,
            tab_names,
            input_modal: InputModal::new(),
            confirmation_modal: ConfirmationModal::new(),
            progress_modal: ProgressModal::new(),
            rebase_data: RebaseData::default(),
            merge_data: MergeData::default(),
            stash_data: StashData::default(),
            tag_data: TagData::default(),
            remote_data: RemoteData::default(),
            current_operation: None,
            operation_progress: None,
            operation_error: None,
        }
    }

    /// Open Git operations interface
    pub fn open(&mut self) {
        self.is_open = true;
        self.current_tab = 0;
        self.operation_state.select(Some(0));
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

    /// Move selection up
    pub fn move_up(&mut self) {
        let len = self.operations.len();
        if len > 0 {
            let selected = self.operation_state.selected().unwrap_or(0);
            let new_selected = if selected == 0 { len - 1 } else { selected - 1 };
            self.operation_state.select(Some(new_selected));
            self.selected_operation = new_selected;
        }
    }

    /// Move selection down
    pub fn move_down(&mut self) {
        let len = self.operations.len();
        if len > 0 {
            let selected = self.operation_state.selected().unwrap_or(0);
            let new_selected = (selected + 1) % len;
            self.operation_state.select(Some(new_selected));
            self.selected_operation = new_selected;
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

        // Split into tabs and content
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tabs
                Constraint::Min(0),    // Content
            ])
            .split(modal_area);

        // Render tabs
        let tab_titles: Vec<Line> = self.tab_names.iter()
            .map(|name| Line::from(name.as_str()))
            .collect();

        let tabs = Tabs::new(tab_titles)
            .block(Block::default().borders(Borders::ALL).title("Git Operations"))
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
    }

    /// Render operations tab
    fn render_operations_tab(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40), // Operation list
                Constraint::Percentage(60), // Operation details
            ])
            .split(area);

        // Render operation list
        let items: Vec<ListItem> = self.operations.iter().map(|op| {
            let line = Line::from(vec![
                Span::raw(op.icon()),
                Span::raw(" "),
                Span::styled(op.as_str(), Style::default().add_modifier(Modifier::BOLD)),
            ]);
            ListItem::new(line)
        }).collect();

        let operation_list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Operations"))
            .highlight_style(
                Style::default()
                    .bg(theme.selection_color())
                    .add_modifier(Modifier::BOLD)
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(operation_list, chunks[0], &mut self.operation_state);

        // Render operation details
        if let Some(operation) = self.operations.get(self.selected_operation) {
            self.render_operation_details(frame, chunks[1], operation, theme);
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
    fn render_status_tab(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let mut lines = vec![
            Line::from("Git Operations Status"),
            Line::from(""),
        ];

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
            Line::from("Recent Operations:"),
            Line::from("• Rebase completed successfully"),
            Line::from("• Merge with conflicts resolved"),
            Line::from("• Stash created: 'WIP: feature work'"),
        ]);

        let status_widget = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Status"))
            .style(theme.text_style())
            .wrap(Wrap { trim: true });

        frame.render_widget(status_widget, area);
    }

    /// Render history tab
    fn render_history_tab(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let lines = vec![
            Line::from("Operation History"),
            Line::from(""),
            Line::from("Recent operations will be displayed here."),
            Line::from(""),
            Line::from("• 2024-01-15 14:30 - Interactive rebase completed"),
            Line::from("• 2024-01-15 13:45 - Cherry-pick applied: a1b2c3d"),
            Line::from("• 2024-01-15 12:15 - Merge branch 'feature' into main"),
            Line::from("• 2024-01-15 11:30 - Reset HEAD~2 (soft)"),
            Line::from("• 2024-01-15 10:45 - Stash created: 'WIP changes'"),
        ];

        let history_widget = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("History"))
            .style(theme.text_style())
            .wrap(Wrap { trim: true });

        frame.render_widget(history_widget, area);
    }

    /// Render help tab
    fn render_help_tab(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let lines = vec![
            Line::from("Git Operations Help"),
            Line::from(""),
            Line::from("Keyboard Shortcuts:"),
            Line::from("• ↑/↓   - Navigate operations"),
            Line::from("• Space - Switch between tabs"),
            Line::from("• Enter - Execute selected operation"),
            Line::from("• Esc   - Close Git operations"),
            Line::from(""),
            Line::from("Operation Descriptions:"),
            Line::from(""),
            Line::from("Rebase: Rewrite commit history by moving commits"),
            Line::from("Cherry Pick: Apply specific commits from other branches"),
            Line::from("Merge: Combine changes from different branches"),
            Line::from("Reset: Move HEAD to a specific commit"),
            Line::from("Stash: Temporarily save work-in-progress changes"),
            Line::from("Tags: Create, delete, and manage repository tags"),
            Line::from("Remotes: Manage remote repository connections"),
            Line::from("Hooks: Configure Git hooks for automation"),
        ];

        let help_widget = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Help"))
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
            KeyCode::Char(' ') => self.next_tab(),
            KeyCode::BackTab => self.previous_tab(),
            KeyCode::Up => {
                if self.current_tab == 0 {
                    self.move_up();
                }
            }
            KeyCode::Down => {
                if self.current_tab == 0 {
                    self.move_down();
                }
            }
            KeyCode::Enter => {
                if self.current_tab == 0 {
                    self.execute_selected_operation()?;
                }
            }
            _ => {}
        }

        Ok(())
    }
}
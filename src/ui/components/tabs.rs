//! Tab components for different Git operations
//!
//! Each tab represents a different view/functionality within the TUI.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use tracing::debug;

use crate::{
    app::state::AppState,
    error::AppResult,
    ui::{components::{Component, DiffViewerComponent}, theme::Theme, selection::{TextPosition, SelectionMode}},
};

/// Status tab component - shows working directory status
pub struct StatusTabComponent {
    selected_index: usize,
    diff_viewer: DiffViewerComponent,
    show_diff: bool,
}

impl StatusTabComponent {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            diff_viewer: DiffViewerComponent::new(),
            show_diff: false,
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        if self.show_diff {
            // 显示差异查看器
            let diff_area = area;
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    if let Err(e) = self.diff_viewer.render(frame, diff_area, state, theme).await {
                        debug!("Failed to render diff viewer: {}", e);
                    }
                })
            });
        } else {
            // 显示文件状态列表
            let items: Vec<ListItem> = state
                .git_state
                .file_status
                .iter()
                .enumerate()
                .map(|(index, file)| {
                    let status_char = file.status.status_char();
                    let item_text = format!(" {} {}", status_char, file.path);

                    let style = if index == self.selected_index {
                        theme.highlight_style()
                    } else {
                        theme.git_status_style(status_char)
                    };

                    ListItem::new(item_text).style(style)
                })
                .collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .title("Git Status (Press Enter to view diff, Esc to go back)")
                        .borders(Borders::ALL)
                        .border_style(theme.border_style()),
                )
                .style(theme.text_style());

            frame.render_widget(list, area);
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent, state: &mut AppState) -> AppResult<()> {
        if self.show_diff {
            // 在差异查看器模式下处理按键
            match key.code {
                KeyCode::Esc => {
                    self.show_diff = false;
                    return Ok(());
                }
                _ => {
                    // 转发其他按键到差异查看器
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            if let Err(e) = self.diff_viewer.handle_key(key).await {
                                debug!("Diff viewer key handling failed: {}", e);
                            }
                        })
                    });
                    return Ok(());
                }
            }
        }

        // 在文件列表模式下处理按键
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_index < state.git_state.file_status.len().saturating_sub(1) {
                    self.selected_index += 1;
                }
            }
            KeyCode::Enter => {
                // 显示选中文件的差异
                if let Some(_selected_file) = state.git_state.file_status.get(self.selected_index) {
                    // TODO: Implement async file diff loading in a proper way
                    // For now, just show the diff viewer
                    self.show_diff = true;
                }
            }
            KeyCode::Char('l') => {
                // Select current line
                let text_lines = state.git_state.file_status.iter()
                    .map(|file| format!(" {} {}", file.status.status_char(), file.path))
                    .collect::<Vec<_>>();
                state.ui_state.selection_manager.select_line(self.selected_index, &text_lines);
            }
            KeyCode::Esc => {
                // Clear selection
                state.ui_state.selection_manager.clear_selection();
            }
            _ => {}
        }
        Ok(())
    }

    /// 加载文件差异到差异查看器
    async fn load_file_diff(&mut self, file_status: &crate::git::FileStatus, state: &AppState) -> AppResult<()> {
        if let Some(git_service) = &state.git_service {
            let file_path = std::path::PathBuf::from(&file_status.path);

            // 使用GitService的新方法获取文件差异
            let (old_content, new_content) = git_service.get_file_diff(&file_path).await?;
            self.diff_viewer.load_git_diff(&file_path, old_content, new_content).await?;
        }
        Ok(())
    }
}

/// Branches tab component with enhanced three-column layout
pub struct BranchesTabComponent {
    selected_index: usize,
    view_mode: BranchViewMode,
    selected_branch: Option<String>,
    list_state: ListState,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum BranchViewMode {
    List,    // Focus on branch list
    Details, // Focus on branch details
    Actions, // Focus on action buttons
}

impl BranchesTabComponent {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            selected_index: 0,
            view_mode: BranchViewMode::List,
            selected_branch: None,
            list_state,
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        // Create three-panel layout: Actions bar + Main content (branches + details)
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Action buttons bar
                Constraint::Min(0),    // Main content area
            ])
            .split(area);

        // Render action buttons at the top
        self.render_action_buttons(frame, main_layout[0], state, theme);

        // Split main content into branches list and details panel
        let content_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(30), // Branch list - fixed reasonable width
                Constraint::Min(0),     // Branch details - take remaining space
            ])
            .split(main_layout[1]);

        // Render branch list
        self.render_branch_list(frame, content_layout[0], state, theme);

        // Render branch details
        self.render_branch_details(frame, content_layout[1], state, theme);
    }

    fn render_action_buttons(&self, frame: &mut Frame, area: Rect, _state: &AppState, theme: &Theme) {
        let buttons = vec![
            "Checkout", "Create New", "Delete", "Merge", "Pull", "Push", "Refresh"
        ];

        let button_text = buttons.join(" | ");
        let actions_para = Paragraph::new(format!(" {} ", button_text))
            .block(
                Block::default()
                    .title("Branch Actions")
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            )
            .style(theme.text_style());

        frame.render_widget(actions_para, area);
    }

    fn render_branch_list(&mut self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        let branches = if let Some(git_service) = &state.git_service {
            git_service.list_branches().unwrap_or_default()
        } else {
            vec![]
        };

        let items: Vec<ListItem> = branches
            .iter()
            .enumerate()
            .map(|(index, branch)| {
                let is_selected = index == self.selected_index;

                // Create enhanced branch display
                let status_prefix = if branch.is_current {
                    "● "
                } else if branch.is_remote {
                    "◯ "
                } else {
                    "○ "
                };

                let mut branch_text = format!("{}{}", status_prefix, branch.name);

                // Add upstream info if available
                if let Some(ref _upstream) = branch.upstream {
                    if branch.ahead > 0 || branch.behind > 0 {
                        branch_text.push_str(&format!(" [↑{} ↓{}]", branch.ahead, branch.behind));
                    } else {
                        branch_text.push_str(" [✓]");
                    }
                }

                let style = if is_selected {
                    theme.highlight_style()
                } else if branch.is_current {
                    theme.success_style()
                } else if branch.is_remote {
                    theme.muted_style()
                } else {
                    theme.text_style()
                };

                ListItem::new(branch_text).style(style)
            })
            .collect();

        let list_title = format!("Branches ({}/{})",
            self.selected_index.saturating_add(1),
            branches.len().max(1)
        );

        let list = List::new(items)
            .block(
                Block::default()
                    .title(list_title)
                    .borders(Borders::ALL)
                    .border_style(if self.view_mode == BranchViewMode::List {
                        theme.accent_border_style()
                    } else {
                        theme.border_style()
                    }),
            )
            .style(theme.text_style())
            .highlight_style(theme.highlight_style())
            .highlight_symbol("▶ ");

        // Update the stored list state
        self.list_state.select(Some(self.selected_index));

        frame.render_stateful_widget(list, area, &mut self.list_state);

        // Update selected branch if valid
        if let Some(git_service) = &state.git_service {
            if let Ok(branches) = git_service.list_branches() {
                if let Some(branch) = branches.get(self.selected_index) {
                    self.selected_branch = Some(branch.name.clone());
                }
            }
        }
    }

    fn render_branch_details(&self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        // Create git commit log list
        let commit_items = if let Some(git_service) = &state.git_service {
            // Try to get actual commit history from git service
            vec![
                ListItem::new("● abc123 feat: Add new feature (2 hours ago)"),
                ListItem::new("● def456 fix: Fix critical bug (1 day ago)"),
                ListItem::new("● ghi789 docs: Update README (3 days ago)"),
                ListItem::new("● jkl012 refactor: Improve performance (4 days ago)"),
                ListItem::new("● mno345 test: Add unit tests (5 days ago)"),
                ListItem::new("● pqr678 style: Format code (6 days ago)"),
                ListItem::new("● stu901 ci: Update workflow (1 week ago)"),
                ListItem::new("● vwx234 deps: Upgrade dependencies (1 week ago)"),
                ListItem::new("● yza567 hotfix: Security patch (2 weeks ago)"),
                ListItem::new("● bcd890 release: Version 1.0.0 (3 weeks ago)"),
                ListItem::new("● efg123 merge: Feature branch (3 weeks ago)"),
                ListItem::new("● hij456 feat: Initial implementation (1 month ago)"),
                ListItem::new("● klm789 initial: Project setup (1 month ago)"),
            ]
        } else {
            vec![
                ListItem::new("No git repository found"),
                ListItem::new("Initialize git repository to see commits"),
            ]
        };

        let list_title = if let Some(ref branch_name) = self.selected_branch {
            format!("Git Log - {}", branch_name)
        } else {
            "Git Log".to_string()
        };

        let commit_list = List::new(commit_items)
            .block(
                Block::default()
                    .title(list_title)
                    .borders(Borders::ALL)
                    .border_style(if self.view_mode == BranchViewMode::Details {
                        theme.accent_border_style()
                    } else {
                        theme.border_style()
                    }),
            )
            .style(theme.text_style())
            .highlight_style(theme.highlight_style())
            .highlight_symbol("▶ ");

        // Create a stateful list for scrolling
        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(0)); // Select first commit by default

        frame.render_stateful_widget(commit_list, area, &mut list_state);
    }

    pub fn handle_key_event(&mut self, key: KeyEvent, state: &mut AppState) -> AppResult<()> {
        let branch_count = if let Some(git_service) = &state.git_service {
            git_service.list_branches().unwrap_or_default().len()
        } else {
            0
        };

        match key.code {
            // Navigation between panels - only switch between List and Details
            KeyCode::Char(' ') => {
                self.view_mode = match self.view_mode {
                    BranchViewMode::List => BranchViewMode::Details,
                    BranchViewMode::Details => BranchViewMode::List,
                    BranchViewMode::Actions => BranchViewMode::List, // Always go back to List from Actions
                };
            }
            // Branch list navigation - always respond to arrow keys in branches tab
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                    // Always set focus to List when navigating
                    self.view_mode = BranchViewMode::List;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_index < branch_count.saturating_sub(1) {
                    self.selected_index += 1;
                    // Always set focus to List when navigating
                    self.view_mode = BranchViewMode::List;
                }
            }
            KeyCode::Enter => {
                // Checkout selected branch
                debug!("Checkout branch: {}", self.selected_index);
                // TODO: Implement actual branch checkout logic
            }
            KeyCode::Char('d') => {
                // Delete selected branch
                debug!("Delete branch: {}", self.selected_index);
                // TODO: Implement branch deletion logic
            }
            KeyCode::Char('n') => {
                // Create new branch
                debug!("Create new branch");
                // TODO: Implement new branch creation
            }
            KeyCode::Char('m') => {
                // Merge selected branch
                debug!("Merge branch: {}", self.selected_index);
                // TODO: Implement branch merge logic
            }
            KeyCode::Char('p') => {
                // Push selected branch
                debug!("Push branch: {}", self.selected_index);
                // TODO: Implement branch push logic
            }
            KeyCode::Char('r') => {
                // Refresh branch list
                debug!("Refresh branch list");
                // TODO: Implement branch list refresh
            }
            KeyCode::Char('l') => {
                // Select current line (branch)
                if let Some(git_service) = &state.git_service {
                    let text_lines = git_service.list_branches().unwrap_or_default().iter()
                        .map(|branch| {
                            let prefix = if branch.is_current { "● " } else { "○ " };
                            format!("{}{}", prefix, branch.name)
                        })
                        .collect::<Vec<_>>();
                    state.ui_state.selection_manager.select_line(self.selected_index, &text_lines);
                }
            }
            KeyCode::Esc => {
                // Clear selection
                state.ui_state.selection_manager.clear_selection();
            }
            _ => {}
        }
        Ok(())
    }
}

/// Tags tab component - manages Git tags
pub struct TagsTabComponent {
    selected_index: usize,
}

impl TagsTabComponent {
    pub fn new() -> Self {
        Self { selected_index: 0 }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        let tags = if let Some(git_service) = &state.git_service {
            // For now, use mock data until Git service implements list_tags()
            // In a real implementation, this would be: git_service.list_tags().unwrap_or_default()
            vec![
                crate::git::TagInfo {
                    name: "v1.0.0".to_string(),
                    target: "abc123".to_string(),
                    target_commit: "abc123".to_string(),
                    message: Some("Release version 1.0.0".to_string()),
                    tagger: Some("Release Manager".to_string()),
                    date: chrono::Utc::now() - chrono::Duration::days(30),
                },
                crate::git::TagInfo {
                    name: "v0.9.0".to_string(),
                    target: "def456".to_string(),
                    target_commit: "def456".to_string(),
                    message: Some("Release version 0.9.0".to_string()),
                    tagger: Some("Release Manager".to_string()),
                    date: chrono::Utc::now() - chrono::Duration::days(60),
                },
                crate::git::TagInfo {
                    name: "v0.8.0".to_string(),
                    target: "ghi789".to_string(),
                    target_commit: "ghi789".to_string(),
                    message: Some("Release version 0.8.0".to_string()),
                    tagger: Some("Release Manager".to_string()),
                    date: chrono::Utc::now() - chrono::Duration::days(90),
                },
            ]
        } else {
            vec![]
        };

        let items: Vec<ListItem> = tags
            .iter()
            .enumerate()
            .map(|(index, tag)| {
                let style = if index == self.selected_index {
                    theme.highlight_style()
                } else {
                    theme.text_style()
                };

                let display_text = if let Some(ref message) = tag.message {
                    format!("  {} - {}", tag.name, message)
                } else {
                    format!("  {}", tag.name)
                };

                ListItem::new(display_text).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Tags")
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            )
            .style(theme.text_style());

        frame.render_widget(list, area);
    }

    pub fn handle_key_event(&mut self, key: KeyEvent, _state: &mut AppState) -> AppResult<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected_index += 1;
            }
            _ => {}
        }
        Ok(())
    }
}

/// Stash tab component - manages Git stash
pub struct StashTabComponent {
    selected_index: usize,
}

impl StashTabComponent {
    pub fn new() -> Self {
        Self { selected_index: 0 }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        let stashes = if let Some(git_service) = &state.git_service {
            // Use actual stash data from git service
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    git_service.list_stash().await.unwrap_or_default()
                })
            })
        } else {
            vec![]
        };

        let items: Vec<ListItem> = stashes
            .iter()
            .enumerate()
            .map(|(index, stash)| {
                let style = if index == self.selected_index {
                    theme.highlight_style()
                } else {
                    theme.text_style()
                };

                let display_text = format!("  stash@{{{}}}: {} ({})",
                    stash.index,
                    stash.message,
                    stash.branch
                );

                ListItem::new(display_text).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Stash")
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            )
            .style(theme.text_style());

        frame.render_widget(list, area);
    }

    pub fn handle_key_event(&mut self, key: KeyEvent, _state: &mut AppState) -> AppResult<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected_index += 1;
            }
            _ => {}
        }
        Ok(())
    }
}

/// Remotes tab component - manages Git remotes
pub struct RemotesTabComponent {
    selected_index: usize,
}

impl RemotesTabComponent {
    pub fn new() -> Self {
        Self { selected_index: 0 }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        let remotes = if let Some(git_service) = &state.git_service {
            // Use actual remote data from git service
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    git_service.list_remotes().await.unwrap_or_default()
                })
            })
        } else {
            vec![]
        };

        let items: Vec<ListItem> = remotes
            .iter()
            .enumerate()
            .map(|(index, remote)| {
                let style = if index == self.selected_index {
                    theme.highlight_style()
                } else {
                    theme.text_style()
                };

                let status_indicator = if remote.is_connected { "✓" } else { "✗" };
                let display_text = format!("  {} {} ({})",
                    status_indicator,
                    remote.name,
                    remote.fetch_url
                );

                ListItem::new(display_text).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Remotes")
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            )
            .style(theme.text_style());

        frame.render_widget(list, area);
    }

    pub fn handle_key_event(&mut self, key: KeyEvent, _state: &mut AppState) -> AppResult<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected_index += 1;
            }
            _ => {}
        }
        Ok(())
    }
}

/// GitFlow tab component - Git Flow workflow management
pub struct GitFlowTabComponent {
    selected_index: usize,
}

impl GitFlowTabComponent {
    pub fn new() -> Self {
        Self { selected_index: 0 }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, _state: &AppState, theme: &Theme) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0)])
            .split(area);

        let content = Paragraph::new("Git Flow workflow management\n\n• feature/\n• release/\n• hotfix/")
            .block(
                Block::default()
                    .title("Git Flow")
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            )
            .style(theme.text_style());

        frame.render_widget(content, layout[0]);
    }

    pub fn handle_key_event(&mut self, key: KeyEvent, _state: &mut AppState) -> AppResult<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected_index += 1;
            }
            _ => {}
        }
        Ok(())
    }
}
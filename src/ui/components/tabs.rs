//! Tab components for different Git operations
//!
//! Each tab represents a different view/functionality within the TUI.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use tracing::{debug, error};
use chrono::Utc;

use crate::{
    app::state::AppState,
    error::AppResult,
    git::CommitInfo,
    ui::{
        components::{Component, DiffViewerComponent, modals::{InputModal, Modal, ModalResult}},
        theme::Theme,
        selection::{TextPosition, SelectionMode},
        keyboard::{ShortcutManager, NavigationKey, ActionKey, NavigationHandler},
    },
};

/// 提交分页管理结构
#[derive(Debug, Clone)]
struct CommitsPagination {
    /// 每页显示的提交数量
    page_size: usize,
    /// 当前页码（从0开始）
    current_page: usize,
    /// 总提交数量
    total_commits: usize,
    /// 已加载的提交列表
    loaded_commits: Vec<CommitInfo>,
    /// 是否还有更多提交可以加载
    has_more: bool,
    /// 正在加载标志
    is_loading: bool,
}

impl CommitsPagination {
    fn new() -> Self {
        Self {
            page_size: 50, // 每页50个提交
            current_page: 0,
            total_commits: 0,
            loaded_commits: Vec::new(),
            has_more: true,
            is_loading: false,
        }
    }

    /// 重置分页状态（切换分支时使用）
    fn reset(&mut self) {
        self.current_page = 0;
        self.total_commits = 0;
        self.loaded_commits.clear();
        self.has_more = true;
        self.is_loading = false;
    }

    /// 获取当前已加载的提交数量
    fn loaded_count(&self) -> usize {
        self.loaded_commits.len()
    }

    /// 检查是否需要加载更多提交
    fn should_load_more(&self, current_index: usize) -> bool {
        !self.is_loading &&
        self.has_more &&
        current_index + 10 >= self.loaded_commits.len() // 提前10个位置开始预加载
    }
}

/// Safe UTF-8 string truncation utility
fn safe_truncate_string(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars - 3).collect();
        format!("{}...", truncated)
    }
}

/// Status tab component - shows working directory status
pub struct StatusTabComponent {
    selected_index: usize,
    diff_viewer: DiffViewerComponent,
    show_diff: bool,
    shortcut_manager: ShortcutManager,
}

impl StatusTabComponent {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            diff_viewer: DiffViewerComponent::new(),
            show_diff: false,
            shortcut_manager: ShortcutManager::new(),
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

        // 使用统一的快捷键管理器处理导航键
        if let Some(nav_key) = self.shortcut_manager.is_navigation_key(&key) {
            // 创建一个临时的导航处理器，传入当前的 item_count
            let mut nav_handler = StatusTabNavigationHandler {
                component: self,
                item_count: state.git_state.file_status.len(),
            };
            nav_handler.handle_navigation(nav_key);
            return Ok(());
        }

        // 使用统一的快捷键管理器处理动作键
        if let Some(action_key) = self.shortcut_manager.is_action_key(&key) {
            match action_key {
                ActionKey::Confirm => {
                    // 显示选中文件的差异
                    if let Some(_selected_file) = state.git_state.file_status.get(self.selected_index) {
                        // TODO: Implement async file diff loading in a proper way
                        // For now, just show the diff viewer
                        self.show_diff = true;
                    }
                }
                ActionKey::SelectLine => {
                    // Select current line
                    let text_lines = state.git_state.file_status.iter()
                        .map(|file| format!(" {} {}", file.status.status_char(), file.path))
                        .collect::<Vec<_>>();
                    state.ui_state.selection_manager.select_line(self.selected_index, &text_lines);
                }
                ActionKey::Cancel => {
                    // Clear selection
                    state.ui_state.selection_manager.clear_selection();
                }
                ActionKey::Add => {
                    // Stage selected file
                    if let Some(selected_file) = state.git_state.file_status.get(self.selected_index) {
                        if let Some(git_service) = &state.git_service {
                            debug!("Staging file: {}", selected_file.path);
                            tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current().block_on(async {
                                    match git_service.stage_file(&selected_file.path).await {
                                        Ok(()) => {
                                            debug!("Successfully staged file: {}", selected_file.path);
                                        }
                                        Err(e) => {
                                            debug!("Failed to stage file {}: {:?}", selected_file.path, e);
                                        }
                                    }
                                })
                            });
                        }
                    }
                }
                ActionKey::Remove => {
                    // Unstage selected file
                    if let Some(selected_file) = state.git_state.file_status.get(self.selected_index) {
                        if let Some(git_service) = &state.git_service {
                            debug!("Unstaging file: {}", selected_file.path);
                            tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current().block_on(async {
                                    match git_service.unstage_file(&selected_file.path).await {
                                        Ok(()) => {
                                            debug!("Successfully unstaged file: {}", selected_file.path);
                                        }
                                        Err(e) => {
                                            debug!("Failed to unstage file {}: {:?}", selected_file.path, e);
                                        }
                                    }
                                })
                            });
                        }
                    }
                }
                ActionKey::Refresh => {
                    // Refresh file status
                    if let Some(git_service) = &state.git_service {
                        debug!("Refreshing file status");
                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                match git_service.get_status().await {
                                    Ok(_status) => {
                                        debug!("Successfully refreshed file status");
                                    }
                                    Err(e) => {
                                        debug!("Failed to refresh file status: {:?}", e);
                                    }
                                }
                            })
                        });
                    }
                }
                _ => {
                    // 其他动作键暂时忽略
                }
            }
        }

        Ok(())
    }
}

/// Helper structure for navigation handling with dynamic item count
struct StatusTabNavigationHandler<'a> {
    component: &'a mut StatusTabComponent,
    item_count: usize,
}

impl<'a> NavigationHandler for StatusTabNavigationHandler<'a> {
    fn selected_index(&self) -> usize {
        self.component.selected_index
    }

    fn set_selected_index(&mut self, index: usize) {
        self.component.selected_index = index;
    }

    fn item_count(&self) -> usize {
        self.item_count
    }
}

impl StatusTabComponent {
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
    commit_list_state: ListState,  // Git Log 区域的列表状态
    shortcut_manager: ShortcutManager,
    input_modal: InputModal,
    // 分页相关字段
    commits_pagination: CommitsPagination,
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
        let mut commit_list_state = ListState::default();
        commit_list_state.select(Some(0));
        Self {
            selected_index: 0,
            view_mode: BranchViewMode::List,
            selected_branch: None,
            list_state,
            commit_list_state,
            shortcut_manager: ShortcutManager::new(),
            input_modal: InputModal::new(),
            commits_pagination: CommitsPagination::new(),
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

        // Render modal on top if open
        if self.input_modal.is_open() {
            self.input_modal.render(frame, area, theme);
        }
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
            match git_service.list_branches() {
                Ok(branches) => branches,
                Err(e) => {
                    // Log error but don't modify state in render method
                    error!("Failed to load branch list: {}", e);
                    vec![]
                }
            }
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
                    let new_branch_name = branch.name.clone();
                    // 如果切换到不同分支，重置 Git Log 的滚动位置和分页状态
                    if self.selected_branch.as_ref() != Some(&new_branch_name) {
                        self.commit_list_state.select(Some(0));
                        self.commits_pagination.reset();
                    }
                    self.selected_branch = Some(new_branch_name);
                }
            }
        }
    }

    fn render_branch_details(&mut self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        // 初始加载提交数据（如果还没有加载）
        if let Some(ref branch_name) = self.selected_branch.clone() {
            if self.commits_pagination.loaded_commits.is_empty() && !self.commits_pagination.is_loading {
                self.load_more_commits(state, branch_name);
            }
        }

        // 使用分页数据创建提交列表
        let commit_items = if self.commits_pagination.loaded_commits.is_empty() {
            if let Some(ref branch_name) = self.selected_branch {
                if self.commits_pagination.is_loading {
                    vec![ListItem::new("Loading commits...")]
                } else {
                    vec![ListItem::new(format!("No commits found for branch '{}'", branch_name))]
                }
            } else {
                vec![ListItem::new("Select a branch to view commit history")]
            }
        } else {
            let mut items: Vec<ListItem> = self.commits_pagination.loaded_commits
                .iter()
.map(|commit| {
                    // Truncate message if too long (safe UTF-8 character boundary)
                    let message = safe_truncate_string(&commit.message, 60);
                    let hash_short = &commit.hash[..8];
                    let author_text = format!("({})", commit.author);
                    let date_text = commit.date.format("%Y-%m-%d %H:%M").to_string();

                    // 使用与 History 组件相同的 Linear View 样式
                    let commit_line = Line::from(vec![
                        Span::styled("●", Style::default().fg(Color::Yellow)),
                        Span::raw(" "),
                        Span::styled(
                            hash_short.to_string(),
                            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" "),
                        Span::styled(message, theme.text_style()),
                        Span::raw(" "),
                        Span::styled(
                            author_text,
                            Style::default().fg(Color::Gray),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            date_text,
                            Style::default().fg(Color::Blue),
                        ),
                    ]);

                    ListItem::new(commit_line)
                })
                .collect();

            // Add loading indicator if there are more commits to load
            if self.commits_pagination.has_more {
                let load_more_line = Line::from(vec![
                    Span::styled("📄", Style::default().fg(Color::Cyan)),
                    Span::raw(" "),
                    Span::styled(
                        "[Load More Commits...]",
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::ITALIC),
                    ),
                ]);
                items.push(ListItem::new(load_more_line));
            }

            items
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

        // 使用组件的持久化列表状态来保持滚动位置
        frame.render_stateful_widget(commit_list, area, &mut self.commit_list_state);
    }

    /// 加载更多提交数据
    fn load_more_commits(&mut self, state: &AppState, branch_name: &str) {
        if self.commits_pagination.is_loading {
            return; // 已经在加载中
        }

        self.commits_pagination.is_loading = true;

        if let Some(git_service) = &state.git_service {
            let skip_count = self.commits_pagination.loaded_commits.len();
            let limit = self.commits_pagination.page_size;

            match tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    // 为分页添加新的方法，支持 skip 和 limit
                    self.get_branch_commits_with_pagination(git_service, branch_name, skip_count, limit).await
                })
            }) {
                Ok(commits) => {
                    if commits.is_empty() {
                        // 没有更多提交了
                        self.commits_pagination.has_more = false;
                    } else {
                        // 将新提交追加到已有列表中
                        self.commits_pagination.loaded_commits.extend(commits);
                        self.commits_pagination.current_page += 1;

                        // 如果返回的提交数少于请求的数量，说明没有更多了
                        if self.commits_pagination.loaded_commits.len() - skip_count < limit {
                            self.commits_pagination.has_more = false;
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to load more commits: {:?}", e);
                    self.commits_pagination.has_more = false;
                }
            }
        }

        self.commits_pagination.is_loading = false;
    }

    /// 使用分页方式获取分支提交
    async fn get_branch_commits_with_pagination(
        &self,
        git_service: &crate::git::GitService,
        branch_name: &str,
        skip: usize,
        limit: usize,
    ) -> AppResult<Vec<CommitInfo>> {
        // 获取更大的数量然后手动分页，因为当前的 get_branch_commits 不支持 skip
        let total_limit = skip + limit;
        let all_commits = git_service.get_branch_commits(branch_name, total_limit).await?;

        // 手动分页：跳过前面已加载的提交
        let paginated_commits = all_commits
            .into_iter()
            .skip(skip)
            .take(limit)
            .collect();

        Ok(paginated_commits)
    }

    /// Handle navigation in details mode (Git Log area)
    fn handle_details_navigation(&mut self, nav_key: NavigationKey, state: &mut AppState) {
        // 使用分页系统的已加载提交数量（包括加载更多按钮）
        let mut commit_count = self.commits_pagination.loaded_commits.len();
        if self.commits_pagination.has_more {
            commit_count += 1; // 为“加载更多”项目留出空间
        }

        if commit_count == 0 {
            return;
        }

        // 获取当前选中的提交索引
        let current_selected = self.commit_list_state.selected().unwrap_or(0);

        match nav_key {
            NavigationKey::Up => {
                if current_selected > 0 {
                    self.commit_list_state.select(Some(current_selected - 1));
                }
            }
            NavigationKey::Down => {
                if current_selected + 1 < commit_count {
                    let new_index = current_selected + 1;
                    self.commit_list_state.select(Some(new_index));

                    // 如果用户导航到了“加载更多”项目，就自动加载更多提交
                    if self.commits_pagination.has_more &&
                       new_index == self.commits_pagination.loaded_commits.len() {
                        if let Some(ref branch_name) = self.selected_branch.clone() {
                            self.load_more_commits(state, branch_name);
                        }
                    }
                }
            }
            NavigationKey::Home => {
                self.commit_list_state.select(Some(0));
            }
            NavigationKey::End => {
                if commit_count > 0 {
                    self.commit_list_state.select(Some(commit_count - 1));
                }
            }
            NavigationKey::PageUp => {
                let new_index = current_selected.saturating_sub(5);
                self.commit_list_state.select(Some(new_index));
            }
            NavigationKey::PageDown => {
                if commit_count > 0 {
                    let new_index = std::cmp::min(current_selected + 5, commit_count - 1);
                    self.commit_list_state.select(Some(new_index));
                }
            }
            NavigationKey::Left | NavigationKey::Right => {
                // Left/Right navigation not used in commit list, ignore
            }
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent, state: &mut AppState) -> AppResult<()> {
        // Handle modal input first if modal is open
        if self.input_modal.is_open() {
            match self.input_modal.handle_key_event(key)? {
                ModalResult::Input(branch_name) => {
                    // Create new branch
                    if !branch_name.trim().is_empty() {
                        if let Some(git_service) = &state.git_service {
                            let result = tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current().block_on(async {
                                    git_service.create_branch(&branch_name, None).await
                                })
                            });

                            match result {
                                Ok(_) => {
                                    debug!("Successfully created branch: {}", branch_name);
                                    let success_msg = format!("Successfully created branch '{}'", branch_name);
                                    state.add_info(success_msg);
                                }
                                Err(e) => {
                                    let error_msg = format!("Failed to create branch '{}': {}", branch_name, e);
                                    error!("{}", error_msg);
                                    state.add_error(error_msg);
                                }
                            }
                        }
                    }
                    return Ok(());
                }
                ModalResult::Cancelled => {
                    return Ok(());
                }
                _ => {} // Continue handling other events
            }
            return Ok(()); // Modal is open, consume all other events
        }

        let branch_count = if let Some(git_service) = &state.git_service {
            match git_service.list_branches() {
                Ok(branches) => branches.len(),
                Err(e) => {
                    error!("Failed to get branch count: {}", e);
                    0
                }
            }
        } else {
            0
        };

        // 处理特殊的面板切换键（保留原有功能）
        match key.code {
            KeyCode::Char(' ') => {
                // Navigation between panels - only switch between List and Details
                self.view_mode = match self.view_mode {
                    BranchViewMode::List => BranchViewMode::Details,
                    BranchViewMode::Details => BranchViewMode::List,
                    BranchViewMode::Actions => BranchViewMode::List, // Always go back to List from Actions
                };
                return Ok(());
            }
            _ => {} // 继续处理其他键
        }

        // 使用统一的快捷键管理器处理导航键
        if let Some(nav_key) = self.shortcut_manager.is_navigation_key(&key) {
            match self.view_mode {
                BranchViewMode::List => {
                    // 在分支列表模式下处理导航
                    let mut nav_handler = BranchesTabNavigationHandler {
                        component: self,
                        item_count: branch_count,
                    };
                    nav_handler.handle_navigation(nav_key);
                }
                BranchViewMode::Details => {
                    // 在详情模式下处理 Git Log 区域的导航
                    self.handle_details_navigation(nav_key, state);
                }
                BranchViewMode::Actions => {
                    // 在动作模式下，导航切换回列表模式
                    self.view_mode = BranchViewMode::List;
                    let mut nav_handler = BranchesTabNavigationHandler {
                        component: self,
                        item_count: branch_count,
                    };
                    nav_handler.handle_navigation(nav_key);
                }
            }
            return Ok(());
        }

        // 使用统一的快捷键管理器处理动作键
        if let Some(action_key) = self.shortcut_manager.is_action_key(&key) {
            match action_key {
                ActionKey::Confirm => {
                    // Checkout selected branch
                    if let Some(git_service) = &state.git_service {
                        let branches = match git_service.list_branches() {
                            Ok(branches) => branches,
                            Err(e) => {
                                let error_msg = format!("Failed to load branches for checkout: {}", e);
                                error!("{}", error_msg);
                                state.add_error(error_msg);
                                return Ok(());
                            }
                        };
                        if let Some(branch) = branches.get(self.selected_index) {
                            debug!("Checkout branch: {}", branch.name);
                            let branch_name = branch.name.clone();
                            let result = tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current().block_on(async {
                                    git_service.switch_branch(&branch_name).await
                                })
                            });

                            match result {
                                Ok(()) => {
                                    debug!("Successfully checked out branch: {}", branch_name);
                                    let success_msg = format!("Successfully switched to branch '{}'", branch_name);
                                    state.add_info(success_msg);
                                }
                                Err(e) => {
                                    let error_msg = format!("Failed to checkout branch '{}': {}", branch_name, e);
                                    error!("{}", error_msg);
                                    state.add_error(error_msg);
                                }
                            }
                        }
                    }
                }
                ActionKey::Delete => {
                    // Delete selected branch
                    if let Some(git_service) = &state.git_service {
                        let branches = match git_service.list_branches() {
                            Ok(branches) => branches,
                            Err(e) => {
                                let error_msg = format!("Failed to load branches for delete: {}", e);
                                error!("{}", error_msg);
                                state.add_error(error_msg);
                                return Ok(());
                            }
                        };
                        if let Some(branch) = branches.get(self.selected_index) {
                            // Don't allow deleting current branch
                            if !branch.is_current {
                                debug!("Delete branch: {}", branch.name);
                                let branch_name = branch.name.clone();
                                let result = tokio::task::block_in_place(|| {
                                    tokio::runtime::Handle::current().block_on(async {
                                        git_service.delete_branch(&branch_name).await
                                    })
                                });

                                match result {
                                    Ok(()) => {
                                        debug!("Successfully deleted branch: {}", branch_name);
                                        // Adjust selected_index if needed
                                        if self.selected_index > 0 && self.selected_index >= branches.len() - 1 {
                                            self.selected_index -= 1;
                                        }
                                        let success_msg = format!("Successfully deleted branch '{}'", branch_name);
                                        state.add_info(success_msg);
                                    }
                                    Err(e) => {
                                        let error_msg = format!("Failed to delete branch '{}': {}", branch_name, e);
                                        error!("{}", error_msg);
                                        state.add_error(error_msg);
                                    }
                                }
                            } else {
                                let error_msg = format!("Cannot delete current branch '{}'", branch.name);
                                debug!("{}", error_msg);
                                state.add_error(error_msg);
                            }
                        }
                    }
                }
                ActionKey::New => {
                    // Open input dialog for new branch name
                    self.input_modal.open_with_placeholder(
                        "Create New Branch",
                        "Enter branch name:",
                        "feature/my-new-feature"
                    );
                }
                ActionKey::Merge => {
                    // Merge selected branch
                    if let Some(git_service) = &state.git_service {
                        let branches = match git_service.list_branches() {
                            Ok(branches) => branches,
                            Err(e) => {
                                let error_msg = format!("Failed to load branches for merge: {}", e);
                                error!("{}", error_msg);
                                state.add_error(error_msg);
                                return Ok(());
                            }
                        };
                        if let Some(branch) = branches.get(self.selected_index) {
                            // Don't allow merging current branch into itself
                            if !branch.is_current {
                                debug!("Merge branch: {}", branch.name);
                                let branch_name = branch.name.clone();
                                let result = tokio::task::block_in_place(|| {
                                    tokio::runtime::Handle::current().block_on(async {
                                        git_service.merge_branch(&branch_name).await
                                    })
                                });

                                match result {
                                    Ok(()) => {
                                        debug!("Successfully merged branch: {}", branch_name);
                                        // TODO: Add success notification if needed
                                    }
                                    Err(e) => {
                                        let error_msg = format!("Failed to merge branch '{}': {}", branch_name, e);
                                        error!("{}", error_msg);
                                        state.add_error(error_msg);
                                    }
                                }
                            } else {
                                let error_msg = format!("Cannot merge current branch '{}' into itself", branch.name);
                                debug!("{}", error_msg);
                                state.add_error(error_msg);
                            }
                        }
                    }
                }
                ActionKey::Push => {
                    if let Some(git_service) = &state.git_service {
                        if let Ok(branches) = git_service.list_branches() {
                            if let Some(branch) = branches.get(self.selected_index) {
                                debug!("Push branch: {}", branch.name);
                                let branch_name = branch.name.clone();
                                let result = tokio::task::block_in_place(|| {
                                    tokio::runtime::Handle::current().block_on(async {
                                        git_service.push_branch(&branch_name).await
                                    })
                                });

                                match result {
                                    Ok(()) => {
                                        debug!("Successfully pushed branch: {}", branch_name);
                                        // TODO: Add success notification if needed
                                    }
                                    Err(e) => {
                                        let error_msg = format!("Failed to push branch '{}': {}", branch_name, e);
                                        error!("{}", error_msg);
                                        state.add_error(error_msg);
                                    }
                                }
                            }
                        }
                    }
                }
                ActionKey::Pull => {
                    if let Some(git_service) = &state.git_service {
                        if let Ok(branches) = git_service.list_branches() {
                            if let Some(branch) = branches.get(self.selected_index) {
                                debug!("Pull changes for branch: {}", branch.name);
                                let branch_name = branch.name.clone();
                                let result = tokio::task::block_in_place(|| {
                                    tokio::runtime::Handle::current().block_on(async {
                                        git_service.pull_branch(&branch_name).await
                                    })
                                });

                                match result {
                                    Ok(()) => {
                                        debug!("Successfully pulled changes for branch: {}", branch_name);
                                        // TODO: Add success notification if needed
                                    }
                                    Err(e) => {
                                        let error_msg = format!("Failed to pull changes for branch '{}': {}", branch_name, e);
                                        error!("{}", error_msg);
                                        state.add_error(error_msg);
                                    }
                                }
                            }
                        }
                    }
                }
                ActionKey::Refresh => {
                    if let Some(git_service) = &state.git_service {
                        debug!("Refreshing branch list");
                        let result = tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                git_service.get_status().await
                            })
                        });

                        match result {
                            Ok(_status) => {
                                debug!("Successfully refreshed branch list");
                                // TODO: Add success notification if needed
                            }
                            Err(e) => {
                                let error_msg = format!("Failed to refresh branch list: {}", e);
                                error!("{}", error_msg);
                                state.add_error(error_msg);
                            }
                        }
                    }
                }
                ActionKey::SelectLine => {
                    // Select current line (branch)
                    if let Some(git_service) = &state.git_service {
                        let branches = match git_service.list_branches() {
                            Ok(branches) => branches,
                            Err(e) => {
                                let error_msg = format!("Failed to load branches for selection: {}", e);
                                error!("{}", error_msg);
                                state.add_error(error_msg);
                                return Ok(());
                            }
                        };
                        let text_lines = branches.iter()
                            .map(|branch| {
                                let prefix = if branch.is_current { "● " } else { "○ " };
                                format!("{}{}", prefix, branch.name)
                            })
                            .collect::<Vec<_>>();
                        state.ui_state.selection_manager.select_line(self.selected_index, &text_lines);
                    }
                }
                ActionKey::Cancel => {
                    // Clear selection
                    state.ui_state.selection_manager.clear_selection();
                }
                _ => {
                    // 其他动作键暂时忽略
                }
            }
        }

        Ok(())
    }
}

/// Helper structure for navigation handling with dynamic item count
struct BranchesTabNavigationHandler<'a> {
    component: &'a mut BranchesTabComponent,
    item_count: usize,
}

impl<'a> NavigationHandler for BranchesTabNavigationHandler<'a> {
    fn selected_index(&self) -> usize {
        self.component.selected_index
    }

    fn set_selected_index(&mut self, index: usize) {
        self.component.selected_index = index;
    }

    fn item_count(&self) -> usize {
        self.item_count
    }
}

/// Tags tab component - manages Git tags
pub struct TagsTabComponent {
    selected_index: usize,
    shortcut_manager: ShortcutManager,
}

impl TagsTabComponent {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            shortcut_manager: ShortcutManager::new(),
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        let tags = if let Some(git_service) = &state.git_service {
            // Use real Git service to fetch tags
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    git_service.list_tags().await.unwrap_or_default()
                })
            })
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

    pub fn handle_key_event(&mut self, key: KeyEvent, state: &mut AppState) -> AppResult<()> {
        // Get tag count for boundary checking using real Git service
        let tag_count = if let Some(git_service) = &state.git_service {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    git_service.list_tags().await.unwrap_or_default().len()
                })
            })
        } else {
            0
        };

        // 使用统一的快捷键管理器处理导航键
        if let Some(nav_key) = self.shortcut_manager.is_navigation_key(&key) {
            let mut nav_handler = TagsTabNavigationHandler {
                component: self,
                item_count: tag_count,
            };
            nav_handler.handle_navigation(nav_key);
            return Ok(());
        }

        // 使用统一的快捷键管理器处理动作键
        if let Some(action_key) = self.shortcut_manager.is_action_key(&key) {
            match action_key {
                ActionKey::Confirm => {
                    // View tag details or checkout tag
                    if let Some(git_service) = &state.git_service {
                        if let Ok(tags) = tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                git_service.list_tags().await
                            })
                        }) {
                            if let Some(tag) = tags.get(self.selected_index) {
                                debug!("View tag details: {}", tag.name);
                                // For now, just show tag information in debug
                                debug!("Tag {} created at {} by {:?}", tag.name, tag.date, tag.tagger);
                            }
                        }
                    }
                }
                ActionKey::Delete => {
                    // Delete selected tag with confirmation
                    if let Some(git_service) = &state.git_service {
                        if let Ok(tags) = tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                git_service.list_tags().await
                            })
                        }) {
                            if let Some(tag) = tags.get(self.selected_index) {
                                debug!("Delete tag: {}", tag.name);
                                tokio::task::block_in_place(|| {
                                    tokio::runtime::Handle::current().block_on(async {
                                        match git_service.delete_tag(&tag.name).await {
                                            Ok(()) => {
                                                debug!("Successfully deleted tag: {}", tag.name);
                                                // Adjust selected index if we deleted the last item
                                                if self.selected_index >= tags.len().saturating_sub(1) && self.selected_index > 0 {
                                                    self.selected_index = self.selected_index.saturating_sub(1);
                                                }
                                            }
                                            Err(e) => {
                                                debug!("Failed to delete tag {}: {:?}", tag.name, e);
                                            }
                                        }
                                    })
                                });
                            }
                        }
                    }
                }
                ActionKey::New => {
                    // Create new tag at current HEAD
                    if let Some(git_service) = &state.git_service {
                        // Auto-generate tag name with timestamp for now
                        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
                        let tag_name = format!("tag_{}", timestamp);
                        debug!("Create new tag: {}", tag_name);

                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                match git_service.create_tag(&tag_name, None, Some("Automated tag creation")).await {
                                    Ok(tag_info) => {
                                        debug!("Successfully created tag: {} ({})", tag_info.name, tag_info.target);
                                    }
                                    Err(e) => {
                                        debug!("Failed to create tag {}: {:?}", tag_name, e);
                                    }
                                }
                            })
                        });
                    }
                }
                ActionKey::Refresh => {
                    // Refresh tag list
                    if let Some(git_service) = &state.git_service {
                        debug!("Refreshing tag list");
                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                match git_service.get_status().await {
                                    Ok(_status) => {
                                        debug!("Successfully refreshed tag list");
                                    }
                                    Err(e) => {
                                        debug!("Failed to refresh tag list: {:?}", e);
                                    }
                                }
                            })
                        });
                    }
                }
                ActionKey::SelectLine => {
                    // Select current line with actual tag data
                    if tag_count > 0 {
                        if let Some(git_service) = &state.git_service {
                            if let Ok(tags) = tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current().block_on(async {
                                    git_service.list_tags().await
                                })
                            }) {
                                if let Some(tag) = tags.get(self.selected_index) {
                                    debug!("Select tag line: {} ({})", tag.name, tag.target);
                                    let tag_info = format!("{} {} {}", tag.name, tag.target, tag.date);
                                    state.ui_state.selection_manager.select_line(self.selected_index, &[tag_info]);
                                }
                            }
                        }
                    }
                }
                ActionKey::Cancel => {
                    // Clear selection
                    debug!("Clear tag selection");
                    state.ui_state.selection_manager.clear_selection();
                }
                ActionKey::Show => {
                    // Show detailed tag information
                    if tag_count > 0 {
                        if let Some(git_service) = &state.git_service {
                            if let Ok(tags) = tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current().block_on(async {
                                    git_service.list_tags().await
                                })
                            }) {
                                if let Some(tag) = tags.get(self.selected_index) {
                                    debug!("Show tag details: {} (commit: {}, date: {}, tagger: {:?})",
                                           tag.name, tag.target, tag.date, tag.tagger);
                                    // In a real implementation, this could open a detail view or modal
                                }
                            }
                        }
                    }
                }
                _ => {
                    // 其他动作键暂时忽略
                }
            }
        }
        Ok(())
    }
}

/// Helper structure for navigation handling with dynamic item count
struct TagsTabNavigationHandler<'a> {
    component: &'a mut TagsTabComponent,
    item_count: usize,
}

impl<'a> NavigationHandler for TagsTabNavigationHandler<'a> {
    fn selected_index(&self) -> usize {
        self.component.selected_index
    }

    fn set_selected_index(&mut self, index: usize) {
        self.component.selected_index = index;
    }

    fn item_count(&self) -> usize {
        self.item_count
    }
}

/// Stash tab component - manages Git stash
pub struct StashTabComponent {
    selected_index: usize,
    shortcut_manager: ShortcutManager,
}

impl StashTabComponent {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            shortcut_manager: ShortcutManager::new(),
        }
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

    pub fn handle_key_event(&mut self, key: KeyEvent, state: &mut AppState) -> AppResult<()> {
        // Get stash count for boundary checking
        let stash_count = if let Some(git_service) = &state.git_service {
            // Use actual stash count from git service
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    git_service.list_stash().await.unwrap_or_default().len()
                })
            })
        } else {
            0
        };

        // 使用统一的快捷键管理器处理导航键
        if let Some(nav_key) = self.shortcut_manager.is_navigation_key(&key) {
            let mut nav_handler = StashTabNavigationHandler {
                component: self,
                item_count: stash_count,
            };
            nav_handler.handle_navigation(nav_key);
            return Ok(());
        }

        // 使用统一的快捷键管理器处理动作键
        if let Some(action_key) = self.shortcut_manager.is_action_key(&key) {
            if let Some(git_service) = &state.git_service {
                match action_key {
                    ActionKey::Confirm => {
                        // Apply (pop) selected stash
                        debug!("Apply stash: index {}", self.selected_index);
                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                if let Err(e) = git_service.stash_pop(self.selected_index).await {
                                    debug!("Failed to apply stash: {}", e);
                                } else {
                                    debug!("Successfully applied and removed stash {}", self.selected_index);
                                }
                            })
                        });
                    }
                    ActionKey::Apply => {
                        // Apply selected stash (keep in stash list)
                        debug!("Apply stash (keep): index {}", self.selected_index);
                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                if let Err(e) = git_service.stash_apply(self.selected_index).await {
                                    debug!("Failed to apply stash: {}", e);
                                } else {
                                    debug!("Successfully applied stash {} (kept in list)", self.selected_index);
                                }
                            })
                        });
                    }
                    ActionKey::Delete => {
                        // Drop (delete) selected stash
                        debug!("Drop stash: index {}", self.selected_index);
                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                if let Err(e) = git_service.stash_drop(self.selected_index).await {
                                    debug!("Failed to drop stash: {}", e);
                                } else {
                                    debug!("Successfully dropped stash {}", self.selected_index);
                                    // Adjust selected_index if needed
                                    if self.selected_index > 0 {
                                        self.selected_index -= 1;
                                    }
                                }
                            })
                        });
                    }
                    ActionKey::Show => {
                        // Show stash diff
                        if stash_count > 0 {
                            if let Some(git_service) = &state.git_service {
                                let stash_index = self.selected_index;
                                match tokio::task::block_in_place(|| {
                                    tokio::runtime::Handle::current().block_on(async {
                                        git_service.get_stash_diff(stash_index).await
                                    })
                                }) {
                                    Ok(diff_content) => {
                                        debug!("Got stash diff for index {}: {} lines",
                                               stash_index, diff_content.lines().count());
                                        // For now, just log the diff preview (first few lines)
                                        let preview: String = diff_content
                                            .lines()
                                            .take(5)
                                            .collect::<Vec<_>>()
                                            .join("\n");
                                        debug!("Stash diff preview:\n{}", preview);
                                        // TODO: Display diff in a dedicated viewer component
                                        // This could be implemented as a popup or separate tab
                                    }
                                    Err(e) => {
                                        debug!("Failed to get stash diff: {:?}", e);
                                    }
                                }
                            }
                        } else {
                            debug!("No stashes available to show");
                        }
                    }
                    ActionKey::New => {
                        // Create new stash
                        debug!("Create new stash");
                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                if let Err(e) = git_service.stash_save(Some("WIP stash from TUI")).await {
                                    debug!("Failed to create stash: {}", e);
                                } else {
                                    debug!("Successfully created new stash");
                                }
                            })
                        });
                    }
                    ActionKey::Refresh => {
                        // Refresh stash list (this happens automatically on next render)
                        debug!("Refresh stash list");
                        // The list will be refreshed on the next render cycle
                    }
                    ActionKey::SelectLine => {
                        // Select current line with actual stash data
                        if stash_count > 0 {
                            tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current().block_on(async {
                                    if let Ok(stashes) = git_service.list_stash().await {
                                        if let Some(stash) = stashes.get(self.selected_index) {
                                            let text_lines = vec![format!("{}: {}", stash.index, stash.message)];
                                            state.ui_state.selection_manager.select_line(self.selected_index, &text_lines);
                                            debug!("Selected stash line: {}", stash.message);
                                        }
                                    }
                                })
                            });
                        }
                    }
                    ActionKey::Cancel => {
                        // Clear selection
                        debug!("Clear stash selection");
                        state.ui_state.selection_manager.clear_selection();
                    }
                    _ => {
                        // 其他动作键暂时忽略
                    }
                }
            }
        }
        Ok(())
    }
}

/// Helper structure for navigation handling with dynamic item count
struct StashTabNavigationHandler<'a> {
    component: &'a mut StashTabComponent,
    item_count: usize,
}

impl<'a> NavigationHandler for StashTabNavigationHandler<'a> {
    fn selected_index(&self) -> usize {
        self.component.selected_index
    }

    fn set_selected_index(&mut self, index: usize) {
        self.component.selected_index = index;
    }

    fn item_count(&self) -> usize {
        self.item_count
    }
}

/// Remotes tab component - manages Git remotes
pub struct RemotesTabComponent {
    selected_index: usize,
    shortcut_manager: ShortcutManager,
}

impl RemotesTabComponent {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            shortcut_manager: ShortcutManager::new(),
        }
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

    pub fn handle_key_event(&mut self, key: KeyEvent, state: &mut AppState) -> AppResult<()> {
        // Get remote count for safe boundary checking
        let remote_count = if let Some(git_service) = &state.git_service {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    git_service.list_remotes().await.unwrap_or_default().len()
                })
            })
        } else {
            0
        };

        // 使用统一的快捷键管理器处理导航键
        if let Some(nav_key) = self.shortcut_manager.is_navigation_key(&key) {
            let mut nav_handler = RemotesTabNavigationHandler {
                component: self,
                item_count: remote_count,
            };
            nav_handler.handle_navigation(nav_key);
            return Ok(());
        }

        // 使用统一的快捷键管理器处理动作键
        if let Some(action_key) = self.shortcut_manager.is_action_key(&key) {
            match action_key {
                ActionKey::Confirm => {
                    // Connect to or inspect selected remote
                    if let Some(git_service) = &state.git_service {
                        if let Ok(remotes) = tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                git_service.list_remotes().await
                            })
                        }) {
                            if let Some(remote) = remotes.get(self.selected_index) {
                                debug!("Inspect remote: {} ({})", remote.name, remote.fetch_url);
                                debug!("Remote connected: {}", remote.is_connected);
                                debug!("Push URL: {:?}", remote.push_url);
                            }
                        }
                    }
                }
                ActionKey::New => {
                    // Add new remote (placeholder - would require user input dialog)
                    debug!("Add new remote operation requested");
                    // NOTE: Real implementation would need a dialog to get remote name and URL
                    // For now, just log that the operation was requested
                    debug!("Remote addition would require user dialog for name and URL input");
                }
                ActionKey::Delete => {
                    // Remove selected remote (placeholder - GitService doesn't have remove_remote method yet)
                    if let Some(git_service) = &state.git_service {
                        if let Ok(remotes) = tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                git_service.list_remotes().await
                            })
                        }) {
                            if let Some(remote) = remotes.get(self.selected_index) {
                                debug!("Remove remote requested: {}", remote.name);
                                debug!("NOTE: Remote removal not yet implemented in GitService");
                                // In real implementation, would call git_service.remove_remote(&remote.name)
                            }
                        }
                    }
                }
                ActionKey::Push => {
                    // Push to selected remote
                    if let Some(git_service) = &state.git_service {
                        if let Ok(remotes) = tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                git_service.list_remotes().await
                            })
                        }) {
                            if let Some(remote) = remotes.get(self.selected_index) {
                                debug!("Push to remote: {}", remote.name);
                                // Use the existing push_branch method with current branch
                                if let Ok(Some(current_branch)) = git_service.get_current_branch() {
                                    tokio::task::block_in_place(|| {
                                        tokio::runtime::Handle::current().block_on(async {
                                            match git_service.push_branch(&current_branch.name).await {
                                                Ok(()) => {
                                                    debug!("Successfully pushed {} to remote {}", current_branch.name, remote.name);
                                                }
                                                Err(e) => {
                                                    debug!("Failed to push {} to remote {}: {:?}", current_branch.name, remote.name, e);
                                                }
                                            }
                                        })
                                    });
                                } else {
                                    debug!("No current branch to push");
                                }
                            }
                        }
                    }
                }
                ActionKey::Refresh => {
                    // Refresh remote list
                    if let Some(git_service) = &state.git_service {
                        debug!("Refreshing remote list");
                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                match git_service.get_status().await {
                                    Ok(_status) => {
                                        debug!("Successfully refreshed remote list");
                                    }
                                    Err(e) => {
                                        debug!("Failed to refresh remote list: {:?}", e);
                                    }
                                }
                            })
                        });
                    }
                }
                ActionKey::SelectLine => {
                    // Select current line with actual remote data
                    if remote_count > 0 {
                        if let Some(git_service) = &state.git_service {
                            if let Ok(remotes) = tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current().block_on(async {
                                    git_service.list_remotes().await
                                })
                            }) {
                                if let Some(remote) = remotes.get(self.selected_index) {
                                    debug!("Select remote line: {} ({})", remote.name, remote.fetch_url);
                                    let remote_info = format!("{} {} {}", remote.name, remote.fetch_url, remote.is_connected);
                                    state.ui_state.selection_manager.select_line(self.selected_index, &[remote_info]);
                                }
                            }
                        }
                    }
                }
                ActionKey::Cancel => {
                    // Clear selection
                    debug!("Clear remote selection");
                    state.ui_state.selection_manager.clear_selection();
                }
                _ => {
                    // 其他动作键暂时忽略
                }
            }
        }

        Ok(())
    }
}

/// Helper structure for navigation handling with dynamic item count
struct RemotesTabNavigationHandler<'a> {
    component: &'a mut RemotesTabComponent,
    item_count: usize,
}

impl<'a> NavigationHandler for RemotesTabNavigationHandler<'a> {
    fn selected_index(&self) -> usize {
        self.component.selected_index
    }

    fn set_selected_index(&mut self, index: usize) {
        self.component.selected_index = index;
    }

    fn item_count(&self) -> usize {
        self.item_count
    }
}

/// GitFlow branch types
#[derive(Debug, Clone, PartialEq)]
enum GitFlowBranchType {
    Feature,
    Release,
    Hotfix,
    Support,
}

impl GitFlowBranchType {
    fn as_str(&self) -> &'static str {
        match self {
            GitFlowBranchType::Feature => "feature/",
            GitFlowBranchType::Release => "release/",
            GitFlowBranchType::Hotfix => "hotfix/",
            GitFlowBranchType::Support => "support/",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            GitFlowBranchType::Feature => "New feature development",
            GitFlowBranchType::Release => "Release preparation",
            GitFlowBranchType::Hotfix => "Critical bug fixes",
            GitFlowBranchType::Support => "Long-term support",
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            GitFlowBranchType::Feature => "✨",
            GitFlowBranchType::Release => "🚀",
            GitFlowBranchType::Hotfix => "🚨",
            GitFlowBranchType::Support => "🛠️",
        }
    }
}

/// GitFlow tab component - Git Flow workflow management
pub struct GitFlowTabComponent {
    selected_index: usize,
    shortcut_manager: ShortcutManager,
    gitflow_branches: Vec<GitFlowBranchType>,
}

impl GitFlowTabComponent {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            shortcut_manager: ShortcutManager::new(),
            gitflow_branches: vec![
                GitFlowBranchType::Feature,
                GitFlowBranchType::Release,
                GitFlowBranchType::Hotfix,
                GitFlowBranchType::Support,
            ],
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        // Create two-column layout: branch types + actions
        let main_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(40), // Branch types list
                Constraint::Min(0),     // Actions and details
            ])
            .split(area);

        // Render GitFlow branch types list
        self.render_gitflow_list(frame, main_layout[0], state, theme);

        // Render selected branch type details
        self.render_gitflow_details(frame, main_layout[1], state, theme);
    }

    fn render_gitflow_list(&mut self, frame: &mut Frame, area: Rect, _state: &AppState, theme: &Theme) {
        let items: Vec<ListItem> = self.gitflow_branches
            .iter()
            .enumerate()
            .map(|(index, branch_type)| {
                let style = if index == self.selected_index {
                    theme.highlight_style()
                } else {
                    theme.text_style()
                };

                let display_text = format!("  {} {} {}",
                    branch_type.icon(),
                    branch_type.as_str(),
                    branch_type.description()
                );

                ListItem::new(display_text).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("GitFlow Branch Types")
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            )
            .style(theme.text_style());

        frame.render_widget(list, area);
    }

    fn render_gitflow_details(&self, frame: &mut Frame, area: Rect, _state: &AppState, theme: &Theme) {
        if let Some(selected_type) = self.gitflow_branches.get(self.selected_index) {
            let details = match selected_type {
                GitFlowBranchType::Feature => {
                    "Feature Branch Workflow\n\n\
                    • Create: n - Start new feature\n\
                    • Finish: f - Merge feature to develop\n\
                    • List: l - Show all feature branches\n\
                    • Checkout: Enter - Switch to feature\n\n\
                    Features are for developing new functionality.\n\
                    They branch from 'develop' and merge back."
                }
                GitFlowBranchType::Release => {
                    "Release Branch Workflow\n\n\
                    • Create: n - Start new release\n\
                    • Finish: f - Merge to main and develop\n\
                    • List: l - Show all release branches\n\
                    • Checkout: Enter - Switch to release\n\n\
                    Releases prepare for production deployment.\n\
                    They branch from 'develop' and merge to both 'main' and 'develop'."
                }
                GitFlowBranchType::Hotfix => {
                    "Hotfix Branch Workflow\n\n\
                    • Create: n - Start new hotfix\n\
                    • Finish: f - Merge to main and develop\n\
                    • List: l - Show all hotfix branches\n\
                    • Checkout: Enter - Switch to hotfix\n\n\
                    Hotfixes address critical production issues.\n\
                    They branch from 'main' and merge to both 'main' and 'develop'."
                }
                GitFlowBranchType::Support => {
                    "Support Branch Workflow\n\n\
                    • Create: n - Start new support branch\n\
                    • List: l - Show all support branches\n\
                    • Checkout: Enter - Switch to support\n\n\
                    Support branches maintain old releases.\n\
                    They branch from 'main' at specific versions."
                }
            };

            let content = Paragraph::new(details)
                .block(
                    Block::default()
                        .title(format!("{} {} Details", selected_type.icon(), selected_type.as_str()))
                        .borders(Borders::ALL)
                        .border_style(theme.border_style()),
                )
                .style(theme.text_style())
                .wrap(Wrap { trim: true });

            frame.render_widget(content, area);
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent, state: &mut AppState) -> AppResult<()> {
        // 使用统一的快捷键管理器处理导航键
        if let Some(nav_key) = self.shortcut_manager.is_navigation_key(&key) {
            let item_count = self.gitflow_branches.len();
            let mut nav_handler = GitFlowTabNavigationHandler {
                component: self,
                item_count,
            };
            nav_handler.handle_navigation(nav_key);
            return Ok(());
        }

        // 使用统一的快捷键管理器处理动作键
        if let Some(action_key) = self.shortcut_manager.is_action_key(&key) {
            if let Some(selected_type) = self.gitflow_branches.get(self.selected_index) {
                match action_key {
                    ActionKey::Confirm => {
                        // Show existing branches of selected type
                        if let Some(git_service) = &state.git_service {
                            let flow_type = selected_type.as_str().trim_end_matches('/');
                            match tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current().block_on(async {
                                    git_service.list_gitflow_branches(flow_type).await
                                })
                            }) {
                                Ok(branches) => {
                                    debug!("Found {} {} branches", branches.len(), flow_type);
                                    // In a real implementation, this would switch to a branch list view
                                }
                                Err(e) => {
                                    debug!("Failed to list {} branches: {:?}", flow_type, e);
                                }
                            }
                        }
                    }
                    ActionKey::New => {
                        // Create new branch of selected type
                        if let Some(git_service) = &state.git_service {
                            let flow_type = selected_type.as_str().trim_end_matches('/');
                            // For now, create a sample branch name
                            let branch_name = format!("new-{}-{}", flow_type, chrono::Utc::now().timestamp() % 1000);
                            match tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current().block_on(async {
                                    git_service.create_gitflow_branch(flow_type, &branch_name).await
                                })
                            }) {
                                Ok(branch) => {
                                    debug!("Created new {} branch: {}", flow_type, branch.name);
                                    let success_msg = format!("Successfully created {} branch '{}'", flow_type, branch.name);
                                    state.add_info(success_msg);
                                }
                                Err(e) => {
                                    debug!("Failed to create {} branch: {:?}", flow_type, e);
                                    let error_msg = format!("Failed to create {} branch: {}", flow_type, e);
                                    state.add_error(error_msg);
                                }
                            }
                        }
                    }
                    ActionKey::SelectLine => {
                        // List and select specific branch of selected type
                        if let Some(git_service) = &state.git_service {
                            let flow_type = selected_type.as_str().trim_end_matches('/');
                            match tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current().block_on(async {
                                    git_service.list_gitflow_branches(flow_type).await
                                })
                            }) {
                                Ok(branches) => {
                                    debug!("Selected line - {} {} branches available", branches.len(), flow_type);
                                    // In a real implementation, this would open a selection dialog
                                }
                                Err(e) => {
                                    debug!("Failed to list {} branches: {:?}", flow_type, e);
                                }
                            }
                        }
                    }
                    ActionKey::Refresh => {
                        // Refresh GitFlow status
                        if let Some(git_service) = &state.git_service {
                            match tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current().block_on(async {
                                    git_service.get_gitflow_status().await
                                })
                            }) {
                                Ok(status) => {
                                    debug!("GitFlow status refreshed: {} feature, {} release, {} hotfix, {} support branches",
                                           status.feature_branches, status.release_branches,
                                           status.hotfix_branches, status.support_branches);
                                }
                                Err(e) => {
                                    debug!("Failed to refresh GitFlow status: {:?}", e);
                                }
                            }
                        }
                    }
                    ActionKey::Cancel => {
                        // Return to main interface
                        debug!("Cancel GitFlow operation");
                        // In a real implementation, this would return to the main tabs view
                    }
                    _ => {
                        // Handle other keys if needed
                    }
                }
            }
        }

        // Handle GitFlow-specific keys
        match key.code {
            KeyCode::Char('f') => {
                // Finish current branch of selected type
                if let Some(selected_type) = self.gitflow_branches.get(self.selected_index) {
                    if let Some(git_service) = &state.git_service {
                        let flow_type = selected_type.as_str().trim_end_matches('/');

                        // First, get the current branch to see if it matches the selected type
                        if let Ok(Some(current_branch)) = git_service.get_current_branch() {
                            if current_branch.name.starts_with(selected_type.as_str()) {
                                // Finish the current branch if it matches the selected type
                                match tokio::task::block_in_place(|| {
                                    tokio::runtime::Handle::current().block_on(async {
                                        git_service.finish_gitflow_branch(flow_type, &current_branch.name).await
                                    })
                                }) {
                                    Ok(()) => {
                                        debug!("Successfully finished {} branch: {}", flow_type, current_branch.name);
                                    }
                                    Err(e) => {
                                        debug!("Failed to finish {} branch: {:?}", flow_type, e);
                                    }
                                }
                            } else {
                                debug!("Current branch '{}' is not a {} branch", current_branch.name, flow_type);
                            }
                        } else {
                            debug!("Could not determine current branch for finishing {} workflow", flow_type);
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }
}

/// Helper structure for navigation handling with dynamic item count
struct GitFlowTabNavigationHandler<'a> {
    component: &'a mut GitFlowTabComponent,
    item_count: usize,
}

impl<'a> NavigationHandler for GitFlowTabNavigationHandler<'a> {
    fn selected_index(&self) -> usize {
        self.component.selected_index
    }

    fn set_selected_index(&mut self, index: usize) {
        self.component.selected_index = index;
    }

    fn item_count(&self) -> usize {
        self.item_count
    }
}
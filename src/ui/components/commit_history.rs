//! Git commit history component with advanced features

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use std::collections::HashMap;

use crate::{
    app::state::AppState,
    error::AppResult,
    git::CommitInfo,
    ui::{
        components::DiffViewerComponent,
        keyboard::{ShortcutManager, NavigationHandler, ActionKey},
        theme::Theme,
    },
};

/// Commit history display modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HistoryDisplayMode {
    /// Linear commit list
    Linear,
    /// Graph with branches
    Graph,
    /// Compact view
    Compact,
}

/// Commit history component
pub struct CommitHistoryComponent {
    selected_index: usize,
    display_mode: HistoryDisplayMode,
    show_commit_details: bool,
    show_file_list: bool,
    diff_viewer: DiffViewerComponent,
    commits_per_page: usize,
    current_page: usize,
    search_filter: String,
    author_filter: Option<String>,
    branch_colors: HashMap<String, Color>,
    shortcut_manager: ShortcutManager,
}

impl CommitHistoryComponent {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            display_mode: HistoryDisplayMode::Linear,
            show_commit_details: false,
            show_file_list: false,
            diff_viewer: DiffViewerComponent::new(),
            commits_per_page: 50,
            current_page: 0,
            search_filter: String::new(),
            author_filter: None,
            branch_colors: Self::init_branch_colors(),
            shortcut_manager: ShortcutManager::new(),
        }
    }

    fn init_branch_colors() -> HashMap<String, Color> {
        let mut colors = HashMap::new();
        colors.insert("main".to_string(), Color::Green);
        colors.insert("master".to_string(), Color::Green);
        colors.insert("develop".to_string(), Color::Blue);
        colors.insert("feature".to_string(), Color::Yellow);
        colors.insert("hotfix".to_string(), Color::Red);
        colors.insert("release".to_string(), Color::Magenta);
        colors
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        if self.show_commit_details {
            self.render_commit_details_view(frame, area, state, theme);
        } else {
            self.render_commit_list_view(frame, area, state, theme);
        }
    }

    fn render_commit_list_view(&mut self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        // Split into header, list, and footer
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),   // Header with controls
                Constraint::Min(0),      // Commit list
                Constraint::Length(2),   // Footer with pagination
            ])
            .split(area);

        // Render header with controls
        self.render_header(frame, chunks[0], state, theme);

        // Get commits from git service
        let commits = self.get_filtered_commits(state);

        // Render commit list based on display mode
        match self.display_mode {
            HistoryDisplayMode::Linear => self.render_linear_view(frame, chunks[1], &commits, theme),
            HistoryDisplayMode::Graph => self.render_graph_view(frame, chunks[1], &commits, theme),
            HistoryDisplayMode::Compact => self.render_compact_view(frame, chunks[1], &commits, theme),
        }

        // Render footer with pagination and stats
        self.render_footer(frame, chunks[2], &commits, theme);
    }

    fn render_header(&self, frame: &mut Frame, area: Rect, _state: &AppState, theme: &Theme) {
        let header_text = format!(
            "📈 History [{}] | Filter: {} | Author: {} | Mode: {:?}",
            self.commits_per_page,
            if self.search_filter.is_empty() { "none" } else { &self.search_filter },
            self.author_filter.as_deref().unwrap_or("all"),
            self.display_mode
        );

        let header = Paragraph::new(header_text)
            .block(
                Block::default()
                    .title("Commit History Controls")
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            )
            .style(theme.text_style())
            .wrap(Wrap { trim: true });

        frame.render_widget(header, area);
    }

    fn render_linear_view(&mut self, frame: &mut Frame, area: Rect, commits: &[CommitInfo], theme: &Theme) {
        let items: Vec<ListItem> = commits
            .iter()
            .enumerate()
            .map(|(index, commit)| {
                let style = if index == self.selected_index {
                    theme.highlight_style()
                } else {
                    theme.text_style()
                };

                let commit_line = Line::from(vec![
                    Span::styled("●", Style::default().fg(Color::Yellow)),
                    Span::raw(" "),
                    Span::styled(
                        &commit.hash[..8],
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(&commit.message, style),
                    Span::raw(" "),
                    Span::styled(
                        format!("({})", commit.author),
                        Style::default().fg(Color::Gray),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        commit.date.format("%Y-%m-%d %H:%M").to_string(),
                        Style::default().fg(Color::Blue),
                    ),
                ]);

                ListItem::new(commit_line)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Linear History")
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            )
            .style(theme.text_style());

        frame.render_widget(list, area);
    }

    fn render_graph_view(&mut self, frame: &mut Frame, area: Rect, commits: &[CommitInfo], theme: &Theme) {
        let items: Vec<ListItem> = commits
            .iter()
            .enumerate()
            .map(|(index, commit)| {
                let style = if index == self.selected_index {
                    theme.highlight_style()
                } else {
                    theme.text_style()
                };

                // Simple graph representation
                let graph_prefix = if index == 0 {
                    "│ ● "
                } else if index == commits.len() - 1 {
                    "└─● "
                } else {
                    "├─● "
                };

                let branch_color = self.get_branch_color(&commit.hash);

                let commit_line = Line::from(vec![
                    Span::styled(graph_prefix, Style::default().fg(branch_color)),
                    Span::styled(
                        &commit.hash[..8],
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(&commit.message, style),
                    Span::raw(" "),
                    Span::styled(
                        format!("({})", commit.author),
                        Style::default().fg(Color::Gray),
                    ),
                ]);

                ListItem::new(commit_line)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Graph History")
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            )
            .style(theme.text_style());

        frame.render_widget(list, area);
    }

    fn render_compact_view(&mut self, frame: &mut Frame, area: Rect, commits: &[CommitInfo], theme: &Theme) {
        let items: Vec<ListItem> = commits
            .iter()
            .enumerate()
            .map(|(index, commit)| {
                let style = if index == self.selected_index {
                    theme.highlight_style()
                } else {
                    theme.text_style()
                };

                let short_message = commit.message.chars().take(60).collect::<String>();
                let formatted_date = commit.date.format("%m-%d").to_string();

                let commit_line = Line::from(vec![
                    Span::styled(&commit.hash[..8], Style::default().fg(Color::Yellow)),
                    Span::raw(" "),
                    Span::styled(short_message, style),
                    Span::raw(" "),
                    Span::styled(formatted_date, Style::default().fg(Color::Blue)),
                ]);

                ListItem::new(commit_line)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Compact History")
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            )
            .style(theme.text_style());

        frame.render_widget(list, area);
    }

    fn render_commit_details_view(&mut self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        let commits = self.get_filtered_commits(state);
        if let Some(commit) = commits.get(self.selected_index) {
            if self.show_file_list {
                // Split into commit info and file list
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(8),   // Commit info
                        Constraint::Min(0),      // File list
                    ])
                    .split(area);

                self.render_commit_info(frame, chunks[0], commit, theme);
                self.render_commit_files(frame, chunks[1], commit, theme);
            } else {
                // Show diff viewer
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(8),   // Commit info
                        Constraint::Min(0),      // Diff viewer
                    ])
                    .split(area);

                self.render_commit_info(frame, chunks[0], commit, theme);

                // Render diff viewer
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        if let Err(e) = self.diff_viewer.render(frame, chunks[1], state, theme).await {
                            tracing::debug!("Failed to render diff viewer: {}", e);
                        }
                    })
                });
            }
        }
    }

    fn render_commit_info(&self, frame: &mut Frame, area: Rect, commit: &CommitInfo, theme: &Theme) {
        let info_text = vec![
            Line::from(vec![
                Span::styled("Commit: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(&commit.hash, Style::default().fg(Color::Yellow)),
            ]),
            Line::from(vec![
                Span::styled("Author: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&commit.author),
                Span::raw(" <"),
                Span::styled(&commit.author_email, Style::default().fg(Color::Blue)),
                Span::raw(">"),
            ]),
            Line::from(vec![
                Span::styled("Date: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(commit.date.format("%Y-%m-%d %H:%M:%S %z").to_string()),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Message: ", Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(commit.message.clone()),
        ];

        let info_paragraph = Paragraph::new(info_text)
            .block(
                Block::default()
                    .title("Commit Details")
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(info_paragraph, area);
    }

    fn render_commit_files(&self, frame: &mut Frame, area: Rect, commit: &CommitInfo, theme: &Theme) {
        // Mock file list for now - in real implementation, get from git service
        let files = vec![
            "src/main.rs",
            "src/lib.rs",
            "Cargo.toml",
            "README.md",
        ];

        let items: Vec<ListItem> = files
            .iter()
            .map(|file| {
                ListItem::new(format!("M  {}", file))
                    .style(theme.text_style())
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Changed Files")
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            );

        frame.render_widget(list, area);
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect, commits: &[CommitInfo], theme: &Theme) {
        let total_pages = (commits.len() + self.commits_per_page - 1) / self.commits_per_page;
        let footer_text = format!(
            "Page {}/{} | {} commits | [M]ode [Enter]Details [F]iles [/]Search [A]uthor [G]raph",
            self.current_page + 1,
            total_pages.max(1),
            commits.len()
        );

        let footer = Paragraph::new(footer_text)
            .style(Style::default().fg(Color::Gray))
            .wrap(Wrap { trim: true });

        frame.render_widget(footer, area);
    }

    fn get_filtered_commits(&self, state: &AppState) -> Vec<CommitInfo> {
        // Get commits from git service (mock data for now)
        let all_commits = if let Some(git_service) = &state.git_service {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    git_service.get_commit_history(self.commits_per_page).await.unwrap_or_default()
                })
            })
        } else {
            vec![]
        };

        // Apply filters
        let mut filtered: Vec<CommitInfo> = all_commits.into_iter()
            .filter(|commit| {
                // Search filter
                if !self.search_filter.is_empty() {
                    if !commit.message.to_lowercase().contains(&self.search_filter.to_lowercase())
                        && !commit.hash.to_lowercase().contains(&self.search_filter.to_lowercase()) {
                        return false;
                    }
                }

                // Author filter
                if let Some(ref author_filter) = self.author_filter {
                    if !commit.author.to_lowercase().contains(&author_filter.to_lowercase()) {
                        return false;
                    }
                }

                true
            })
            .collect();

        // Pagination
        let start_idx = self.current_page * self.commits_per_page;
        let end_idx = (start_idx + self.commits_per_page).min(filtered.len());

        if start_idx < filtered.len() {
            filtered[start_idx..end_idx].to_vec()
        } else {
            vec![]
        }
    }

    fn get_branch_color(&self, _commit_hash: &str) -> Color {
        // Simple color assignment based on hash
        // In real implementation, would get branch info from git
        Color::Green
    }

    pub fn handle_key_event(&mut self, key: KeyEvent, state: &mut AppState) -> AppResult<()> {
        if self.show_commit_details {
            return self.handle_details_key_event(key, state);
        }

        let commits = self.get_filtered_commits(state);

        // Reset selected_index if no commits available
        if commits.is_empty() {
            self.selected_index = 0;
            return Ok(());
        }

        // Ensure selected_index is within bounds
        if self.selected_index >= commits.len() {
            self.selected_index = commits.len() - 1;
        }

        // Handle History-specific keys first (preserve existing functionality)
        match key.code {
            KeyCode::Char('m') => {
                // Toggle display mode (History-specific)
                self.display_mode = match self.display_mode {
                    HistoryDisplayMode::Linear => HistoryDisplayMode::Graph,
                    HistoryDisplayMode::Graph => HistoryDisplayMode::Compact,
                    HistoryDisplayMode::Compact => HistoryDisplayMode::Linear,
                };
                return Ok(());
            }
            KeyCode::Char('f') => {
                // Toggle file list in details view (History-specific)
                self.show_file_list = !self.show_file_list;
                return Ok(());
            }
            KeyCode::Char('/') => {
                // Start search (History-specific)
                tracing::info!("Search mode activated");
                return Ok(());
            }
            KeyCode::Char('a') => {
                // Filter by author (History-specific)
                tracing::info!("Author filter activated");
                return Ok(());
            }
            KeyCode::PageUp => {
                // Page navigation (History-specific)
                if self.current_page > 0 {
                    self.current_page -= 1;
                    self.selected_index = 0;
                }
                return Ok(());
            }
            KeyCode::PageDown => {
                // Page navigation (History-specific)
                let total_pages = (commits.len() + self.commits_per_page - 1) / self.commits_per_page;
                if self.current_page < total_pages.saturating_sub(1) {
                    self.current_page += 1;
                    self.selected_index = 0;
                }
                return Ok(());
            }
            _ => {} // Continue to unified shortcut processing
        }

        // 使用统一的快捷键管理器处理导航键
        if let Some(nav_key) = self.shortcut_manager.is_navigation_key(&key) {
            let item_count = commits.len();
            let mut nav_handler = CommitHistoryNavigationHandler {
                component: self,
                item_count,
            };
            nav_handler.handle_navigation(nav_key);
            return Ok(());
        }

        // 使用统一的快捷键管理器处理动作键
        if let Some(action_key) = self.shortcut_manager.is_action_key(&key) {
            match action_key {
                ActionKey::Confirm => {
                    // Show commit details
                    if let Some(commit) = commits.get(self.selected_index) {
                        self.show_commit_details = true;
                        self.load_commit_diff(commit, state)?;
                    }
                }
                ActionKey::Cancel => {
                    // Clear filters or exit details
                    if self.show_commit_details {
                        self.show_commit_details = false;
                    } else {
                        self.search_filter.clear();
                        self.author_filter = None;
                    }
                }
                _ => {
                    // 其他动作键暂时忽略
                }
            }
        }

        Ok(())
    }

    fn handle_details_key_event(&mut self, key: KeyEvent, state: &mut AppState) -> AppResult<()> {
        match key.code {
            KeyCode::Esc => {
                self.show_commit_details = false;
            }
            KeyCode::Char('f') => {
                self.show_file_list = !self.show_file_list;
            }
            _ => {
                // Forward to diff viewer when not showing file list
                if !self.show_file_list {
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            if let Err(e) = self.diff_viewer.handle_key(key).await {
                                tracing::debug!("Diff viewer key handling failed: {}", e);
                            }
                        })
                    });
                }
            }
        }
        Ok(())
    }

    fn load_commit_diff(&mut self, commit: &CommitInfo, _state: &AppState) -> AppResult<()> {
        // In real implementation, would get commit diff from git service
        let old_content = "// Original version\nfn main() {\n    println!(\"Hello\");\n}".to_string();
        let new_content = "// Updated version\nfn main() {\n    println!(\"Hello, World!\");\n    println!(\"New feature added\");\n}".to_string();

        let file_path = std::path::PathBuf::from("src/main.rs");

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                if let Err(e) = self.diff_viewer.load_git_diff(&file_path, old_content, new_content).await {
                    tracing::error!("Failed to load commit diff: {}", e);
                }
            })
        });

        Ok(())
    }
}

/// Helper structure for navigation handling with dynamic item count
struct CommitHistoryNavigationHandler<'a> {
    component: &'a mut CommitHistoryComponent,
    item_count: usize,
}

impl<'a> NavigationHandler for CommitHistoryNavigationHandler<'a> {
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
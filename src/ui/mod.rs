//! User Interface module
//!
//! Provides Terminal User Interface components using ratatui with
//! responsive design and keyboard navigation.

pub mod components;
pub mod keyboard;
pub mod layout;
pub mod theme;
pub mod diff;
pub mod selection;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Tabs, Paragraph},
    Frame,
};
use tracing::debug;

use crate::{
    app::state::{AppState, TabType, FocusArea},
    config::UIConfig,
    error::AppResult,
};
use selection::{TextPosition, SelectionMode};
use components::*;
use theme::Theme;

/// Main UI renderer
pub struct UI {
    /// Current theme
    theme: Theme,
    /// UI configuration
    config: UIConfig,
    /// Component instances
    components: UIComponents,
}

impl UI {
    /// Create a new UI instance
    pub fn new(config: &UIConfig) -> AppResult<Self> {
        debug!("Initializing UI with theme: {}", config.theme);

        let theme = Theme::load(&config.theme)?;
        let components = UIComponents::new(&theme);

        Ok(Self {
            theme,
            config: config.clone(),
            components,
        })
    }

    /// Render the entire UI with four-layer layout: Tabs + Toolbar + Content + Help
    pub fn render(&mut self, frame: &mut Frame, state: &AppState) {
        let size = frame.size();

        // Four-layer layout: Top tabs + Toolbar + Main content + Help bar
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Top navigation tabs (Branches, Tags, etc.)
                Constraint::Length(3), // Git operations toolbar
                Constraint::Min(0),    // Main content area
                Constraint::Length(2), // Bottom help/shortcuts bar
            ])
            .split(size);

        // Render top navigation tabs
        self.render_navigation_tabs(frame, main_chunks[0], state);

        // Render Git operations toolbar
        self.render_git_toolbar(frame, main_chunks[1], state);

        // Render main content based on current active tab
        self.render_tab_content(frame, main_chunks[2], state);

        // Render bottom help bar
        self.render_help_bar(frame, main_chunks[3], state);

        // Render help overlay if visible (rendered last to appear on top)
        self.components.help.render(frame, size, state, &self.theme);
    }

    /// Handle key events
    pub fn handle_key_event(&mut self, key: KeyEvent, state: &mut AppState) -> AppResult<()> {
        // Handle help system first (highest priority)
        if self.components.help.handle_key_event(key, state)? {
            return Ok(());
        }

        // Handle global help toggle
        if matches!(key.code, KeyCode::Char('?')) {
            self.components.help.toggle();
            return Ok(());
        }

        // Handle global selection and copy shortcuts
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('a') => {
                    // Select all - get current content based on active tab
                    let text_lines = self.get_current_text_content(state);
                    state.ui_state.selection_manager.select_all(&text_lines);
                    return Ok(());
                }
                KeyCode::Char('c') => {
                    // Copy selection
                    let text_lines = self.get_current_text_content(state);
                    if let Err(e) = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            state.ui_state.selection_manager.copy_selection(&text_lines).await
                        })
                    }) {
                        debug!("Failed to copy selection: {}", e);
                    }
                    return Ok(());
                }
                _ => {}
            }
        }

        match key.code {
            // Tab navigation with number keys
            KeyCode::Char(c @ '1'..='7') if !key.modifiers.contains(KeyModifiers::ALT) => {
                let tab_index = (c as u8 - b'1') as usize;
                if tab_index < TabType::all().len() {
                    state.set_current_tab(TabType::all()[tab_index]);
                }
            }
            // Tab navigation with Tab/Shift+Tab
            KeyCode::Tab if !key.modifiers.contains(KeyModifiers::SHIFT) => {
                let current_index = TabType::all()
                    .iter()
                    .position(|&tab| tab == state.current_tab())
                    .unwrap_or(0);

                let next_index = (current_index + 1) % TabType::all().len();
                state.set_current_tab(TabType::all()[next_index]);
            }
            KeyCode::BackTab | KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => {
                let current_index = TabType::all()
                    .iter()
                    .position(|&tab| tab == state.current_tab())
                    .unwrap_or(0);

                let prev_index = if current_index == 0 {
                    TabType::all().len() - 1
                } else {
                    current_index - 1
                };
                state.set_current_tab(TabType::all()[prev_index]);
            }
            // Modern Git client keyboard shortcuts
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::ALT) => {
                // Alt+C: Checkout branch
                debug!("Alt+C: Checkout branch requested");
            }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::ALT) => {
                // Alt+N: Create new branch
                debug!("Alt+N: Create new branch requested");
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::ALT) => {
                // Alt+D: Delete branch
                debug!("Alt+D: Delete branch requested");
            }
            KeyCode::Char('m') if key.modifiers.contains(KeyModifiers::ALT) => {
                // Alt+M: Merge branch
                debug!("Alt+M: Merge branch requested");
            }
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::ALT) => {
                // Alt+P: Push changes
                debug!("Alt+P: Push changes requested");
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::ALT) => {
                // Alt+U: Pull changes
                debug!("Alt+U: Pull changes requested");
            }
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+R: Refresh
                debug!("Refresh requested");
            }
            // Forward navigation and action keys to the active tab component
            // Note: Tab key is reserved for tab switching, use Space for panel switching
            KeyCode::Up | KeyCode::Char('k') |
            KeyCode::Down | KeyCode::Char('j') |
            KeyCode::Enter |
            KeyCode::Char(' ') |
            KeyCode::PageUp | KeyCode::PageDown |
            KeyCode::Char('m') | KeyCode::Char('f') | KeyCode::Char('a') |
            KeyCode::Char('r') | KeyCode::Char('l') | KeyCode::Char('s') |
            KeyCode::Char('p') | KeyCode::Char('d') | KeyCode::Char('c') |
            KeyCode::Char('n') | KeyCode::Home | KeyCode::End => {
                // Forward to the current active tab component
                match state.current_tab() {
                    TabType::Branches => self.components.branches_tab.handle_key_event(key, state)?,
                    TabType::Tags => self.components.tags_tab.handle_key_event(key, state)?,
                    TabType::Stash => self.components.stash_tab.handle_key_event(key, state)?,
                    TabType::Status => self.components.status_tab.handle_key_event(key, state)?,
                    TabType::Remotes => self.components.remotes_tab.handle_key_event(key, state)?,
                    TabType::History => self.components.history_tab.handle_key_event(key, state)?,
                    TabType::GitFlow => self.components.gitflow_tab.handle_key_event(key, state)?,
                }
            }
            // Forward other keys to the active component
            _ => {
                self.components.handle_key_event(key, state)?;
            }
        }

        Ok(())
    }

    /// Handle terminal resize
    pub fn handle_resize(&mut self, width: u16, height: u16) {
        debug!("Terminal resized to {}x{}", width, height);
        // Components will adapt automatically to new size
    }

    /// Get current text content based on the active tab
    fn get_current_text_content(&self, state: &AppState) -> Vec<String> {
        match state.current_tab() {
            TabType::Status => {
                // Get file status as text lines
                state.git_state.file_status.iter()
                    .map(|file| format!(" {} {}", file.status.status_char(), file.path))
                    .collect()
            }
            TabType::Branches => {
                // Get branches as text lines
                if let Some(git_service) = &state.git_service {
                    git_service.list_branches().unwrap_or_default().iter()
                        .map(|branch| {
                            let prefix = if branch.is_current { "* " } else { "  " };
                            format!("{}{}", prefix, branch.name)
                        })
                        .collect()
                } else {
                    vec![]
                }
            }
            TabType::Tags => {
                // Mock tags data
                vec!["  v1.0.0".to_string(), "  v0.9.0".to_string(), "  v0.8.0".to_string()]
            }
            TabType::Stash => {
                // Mock stash data
                vec![
                    "  stash@{0}: WIP on main".to_string(),
                    "  stash@{1}: feature work".to_string()
                ]
            }
            TabType::Remotes => {
                // Mock remotes data
                vec!["  origin".to_string(), "  upstream".to_string(), "  fork".to_string()]
            }
            TabType::History => {
                // Mock commit history
                vec![
                    "  abc123 feat: Add new feature".to_string(),
                    "  def456 fix: Fix critical bug".to_string(),
                    "  ghi789 docs: Update README".to_string(),
                ]
            }
            TabType::GitFlow => {
                // Git flow information
                vec![
                    "Git Flow workflow management".to_string(),
                    "".to_string(),
                    "• feature/".to_string(),
                    "• release/".to_string(),
                    "• hotfix/".to_string(),
                ]
            }
        }
    }

    /// Render top navigation tabs (Branches, Tags, Stash, etc.)
    fn render_navigation_tabs(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let tab_titles: Vec<String> = TabType::all()
            .iter()
            .map(|tab| tab.name().to_string())
            .collect();

        let current_tab_index = TabType::all()
            .iter()
            .position(|&tab| tab == state.current_tab())
            .unwrap_or(0);

        // Split area: tabs on left, status info on right
        let header_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(0),      // Tabs area - auto-fit content
                Constraint::Length(25),  // Status area - fixed width
            ])
            .split(area);

        let tabs = Tabs::new(tab_titles)
            .block(
                Block::default()
                    .title("🌟 AI-Commit TUI")
                    .borders(Borders::ALL)
                    .border_style(self.theme.accent_border_style())
                    .title_style(self.theme.success_style()),
            )
            .style(self.theme.text_style())
            .highlight_style(self.theme.tab_highlight_style())
            .select(current_tab_index);

        frame.render_widget(tabs, header_layout[0]);

        // Render status info in the right area
        self.render_status_info(frame, header_layout[1], state);
    }

    /// Render Git operations toolbar (second row)
    fn render_git_toolbar(&self, frame: &mut Frame, area: Rect, _state: &AppState) {
        let toolbar_buttons = vec![
            "Checkout", "Create New", "Delete", "Merge", "Push", "Pull", "Refresh", "Settings"
        ];

        let button_text = toolbar_buttons.join(" │ ");
        let toolbar_content = format!(" ⚡ Git Operations: {} ", button_text);

        let toolbar = Paragraph::new(toolbar_content)
            .block(
                Block::default()
                    .title("🔧 Actions")
                    .borders(Borders::ALL)
                    .border_style(self.theme.border_style()),
            )
            .style(self.theme.text_style())
            .wrap(ratatui::widgets::Wrap { trim: true });

        frame.render_widget(toolbar, area);
    }

    /// Render status info (sync status and repository info)
    fn render_status_info(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let status_text = if let Some(branch_info) = &state.git_state.current_branch {
            format!(
                " {} ↑{} ↓{} ",
                branch_info.name,
                branch_info.ahead,
                branch_info.behind
            )
        } else {
            " No repo ".to_string()
        };

        let status = Paragraph::new(status_text)
            .block(
                Block::default()
                    .title("📊 Status")
                    .borders(Borders::ALL)
                    .border_style(self.theme.border_style()),
            )
            .style(self.theme.status_style());

        frame.render_widget(status, area);
    }

    /// Render bottom help/shortcuts bar
    fn render_help_bar(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let help_text = match state.current_tab() {
            TabType::Branches => "↑/↓: Select branch | Space: Switch panel | Enter: Checkout | ?: Help | 1-7: Switch tabs",
            TabType::Tags => "↑/↓: Select tag | Space: Switch panel | Enter: View tag | ?: Help | 1-7: Switch tabs",
            TabType::Stash => "↑/↓: Select stash | Space: Switch panel | Enter: Apply | ?: Help | 1-7: Switch tabs",
            TabType::Status => "↑/↓: Select file | Space: Switch panel | Enter: Stage | ?: Help | 1-7: Switch tabs",
            TabType::Remotes => "↑/↓: Select remote | Space: Switch panel | Enter: Fetch | ?: Help | 1-7: Switch tabs",
            TabType::History => "↑/↓: Select commit | Space: Switch panel | Enter: View | ?: Help | 1-7: Switch tabs",
            TabType::GitFlow => "↑/↓: Navigate | Space: Switch panel | Enter: Execute | ?: Help | 1-7: Switch tabs",
        };

        let help_para = Paragraph::new(help_text)
            .block(
                Block::default()
                    .title("Quick Help")
                    .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                    .border_style(self.theme.border_style()),
            )
            .style(self.theme.text_style())
            .wrap(ratatui::widgets::Wrap { trim: true });

        frame.render_widget(help_para, area);
    }

    /// Render main-content (sidebar + content area)
    fn render_main_content(&mut self, frame: &mut Frame, area: Rect, state: &AppState) {
        // Fixed sidebar width of 250px to match tui-demo.html
        let sidebar_width = 250;

        let content_layout = if state.ui_state.is_sidebar_visible {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(sidebar_width),
                    Constraint::Min(0),
                ])
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(0)])
                .split(area)
        };

        // Render sidebar if visible
        if state.ui_state.is_sidebar_visible && content_layout.len() > 1 {
            self.components
                .sidebar
                .render(frame, content_layout[0], state, &self.theme);
        }

        // Render content area
        let content_area = if state.ui_state.is_sidebar_visible && content_layout.len() > 1 {
            content_layout[1]
        } else {
            content_layout[0]
        };

        // Render content based on active tab
        match state.current_tab() {
            TabType::Status => {
                self.components
                    .status_tab
                    .render(frame, content_area, state, &self.theme);
            }
            TabType::Branches => {
                self.components
                    .branches_tab
                    .render(frame, content_area, state, &self.theme);
            }
            TabType::Tags => {
                self.components
                    .tags_tab
                    .render(frame, content_area, state, &self.theme);
            }
            TabType::Stash => {
                self.components
                    .stash_tab
                    .render(frame, content_area, state, &self.theme);
            }
            TabType::Remotes => {
                self.components
                    .remotes_tab
                    .render(frame, content_area, state, &self.theme);
            }
            TabType::History => {
                self.components
                    .history_tab
                    .render(frame, content_area, state, &self.theme);
            }
            TabType::GitFlow => {
                self.components
                    .gitflow_tab
                    .render(frame, content_area, state, &self.theme);
            }
        }
    }

    /// Render sync status (replaces the old status bar)
    fn render_sync_status(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        // Create sync status widget showing repository status
        let status_text = if let Some(branch_info) = &state.git_state.current_branch {
            format!(
                " {} | {} ahead, {} behind ",
                branch_info.name,
                branch_info.ahead,
                branch_info.behind
            )
        } else {
            " No repository ".to_string()
        };

        let sync_status = ratatui::widgets::Paragraph::new(status_text)
            .block(
                Block::default()
                    .title("Status")
                    .borders(Borders::ALL)
                    .border_style(self.theme.border_style())
                    .title_style(self.theme.text_style()),
            )
            .style(self.theme.status_style());

        frame.render_widget(sync_status, area);
    }

    /// Render left sidebar with file tree and branch information
    fn render_left_sidebar(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        // Split sidebar into sections: File tree + Branch info
        let sidebar_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(60), // File tree area
                Constraint::Percentage(40), // Branch info area
            ])
            .split(area);

        // Render file tree section
        self.render_file_tree_section(frame, sidebar_chunks[0], state);

        // Render branch info section
        self.render_branch_info_section(frame, sidebar_chunks[1], state);
    }

    /// Render file tree section (top part of sidebar)
    fn render_file_tree_section(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let files = vec![
            "📁 src/",
            "  📄 main.rs",
            "  📄 lib.rs",
            "  📁 ui/",
            "    📄 mod.rs",
            "    📄 components.rs",
            "📁 tests/",
            "📄 Cargo.toml",
            "📄 README.md",
        ];

        let file_items: Vec<ratatui::widgets::ListItem> = files
            .iter()
            .map(|&file| ratatui::widgets::ListItem::new(file))
            .collect();

        let file_list = ratatui::widgets::List::new(file_items)
            .block(
                Block::default()
                    .title("📂 Repository Files")
                    .borders(Borders::ALL)
                    .border_style(self.theme.border_style()),
            )
            .style(self.theme.text_style());

        frame.render_widget(file_list, area);
    }

    /// Render branch info section (bottom part of sidebar)
    fn render_branch_info_section(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let branches = if let Some(git_service) = &state.git_service {
            git_service.list_branches().unwrap_or_default()
        } else {
            vec![]
        };

        let branch_items: Vec<ratatui::widgets::ListItem> = branches
            .iter()
            .take(5) // Show only first 5 branches in sidebar
            .map(|branch| {
                let prefix = if branch.is_current { "🌟 " } else { "🌿 " };
                let display_text = format!("{}{}", prefix, branch.name);
                ratatui::widgets::ListItem::new(display_text)
                    .style(if branch.is_current {
                        self.theme.success_style()
                    } else {
                        self.theme.text_style()
                    })
            })
            .collect();

        let branch_list = ratatui::widgets::List::new(branch_items)
            .block(
                Block::default()
                    .title("🔀 Branches")
                    .borders(Borders::ALL)
                    .border_style(self.theme.border_style()),
            )
            .style(self.theme.text_style());

        frame.render_widget(branch_list, area);
    }

    /// Render center content area (Git log, commits, diff view)
    fn render_center_content(&mut self, frame: &mut Frame, area: Rect, state: &AppState) {
        // Split center area: Commit log + Details panel
        let center_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(70), // Commit log
                Constraint::Percentage(30), // Details/diff panel
            ])
            .split(area);

        // Render commit log
        self.render_commit_log(frame, center_chunks[0], state);

        // Render details panel
        self.render_details_panel(frame, center_chunks[1], state);
    }

    /// Render commit log (main center area)
    fn render_commit_log(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let commits = vec![
            "🔵 abc123 feat: Add new UI components (2 hours ago) - John Doe",
            "🟢 def456 fix: Fix layout issues in branch view (1 day ago) - Jane Smith",
            "🔴 ghi789 refactor: Improve code structure (2 days ago) - John Doe",
            "🟡 jkl012 docs: Update README with examples (3 days ago) - Alice Johnson",
            "🟢 mno345 test: Add comprehensive tests (4 days ago) - Bob Wilson",
            "🔵 pqr678 feat: Implement search functionality (5 days ago) - Carol Davis",
        ];

        let commit_items: Vec<ratatui::widgets::ListItem> = commits
            .iter()
            .map(|&commit| ratatui::widgets::ListItem::new(commit))
            .collect();

        let commit_list = ratatui::widgets::List::new(commit_items)
            .block(
                Block::default()
                    .title("📜 Git Log")
                    .borders(Borders::ALL)
                    .border_style(self.theme.accent_border_style()),
            )
            .style(self.theme.text_style())
            .highlight_style(self.theme.highlight_style())
            .highlight_symbol("▶ ");

        frame.render_widget(commit_list, area);
    }

    /// Render details panel (bottom center area)
    fn render_details_panel(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let details_text = vec![
            "Commit Details:",
            "",
            "📝 Message: feat: Add new UI components",
            "👤 Author: John Doe <john@example.com>",
            "📅 Date: 2024-01-15 14:30:22",
            "🔗 Hash: abc123def456ghi789",
            "",
            "📊 Changes:",
            "  +15 -3   src/ui/mod.rs",
            "  +8  -0   src/ui/components.rs",
            "  +2  -1   README.md",
        ].join("\n");

        let details = Paragraph::new(details_text)
            .block(
                Block::default()
                    .title("🔍 Commit Details")
                    .borders(Borders::ALL)
                    .border_style(self.theme.border_style()),
            )
            .style(self.theme.text_style())
            .wrap(ratatui::widgets::Wrap { trim: true });

        frame.render_widget(details, area);
    }

    /// Render tab content based on current active tab
    fn render_tab_content(&mut self, frame: &mut Frame, area: Rect, state: &AppState) {
        match state.current_tab() {
            TabType::Branches => {
                // For Branches tab, show enhanced branch management interface
                self.components
                    .branches_tab
                    .render(frame, area, state, &self.theme);
            }
            TabType::Tags => {
                // For Tags tab, show tag management
                self.components
                    .tags_tab
                    .render(frame, area, state, &self.theme);
            }
            TabType::Stash => {
                // For Stash tab, show stash management
                self.components
                    .stash_tab
                    .render(frame, area, state, &self.theme);
            }
            TabType::Status => {
                // For Status tab, show file status and staging area
                self.components
                    .status_tab
                    .render(frame, area, state, &self.theme);
            }
            TabType::Remotes => {
                // For Remotes tab, show remote repositories
                self.components
                    .remotes_tab
                    .render(frame, area, state, &self.theme);
            }
            TabType::History => {
                // For History tab, show commit history with file tree
                let content_layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Length(30), // Left sidebar (file tree/branches)
                        Constraint::Min(0),     // Center content area (commits)
                    ])
                    .split(area);

                // Render left sidebar with file tree and branches
                self.render_left_sidebar(frame, content_layout[0], state);

                // Render commit history in center
                self.components
                    .history_tab
                    .render(frame, content_layout[1], state, &self.theme);
            }
            TabType::GitFlow => {
                // For GitFlow tab, show git flow management
                self.components
                    .gitflow_tab
                    .render(frame, area, state, &self.theme);
            }
        }
    }
}

/// Container for all UI components
struct UIComponents {
    pub sidebar: SidebarComponent,
    pub status_tab: StatusTabComponent,
    pub branches_tab: BranchesTabComponent,
    pub tags_tab: TagsTabComponent,
    pub stash_tab: StashTabComponent,
    pub remotes_tab: RemotesTabComponent,
    pub history_tab: CommitHistoryComponent,
    pub gitflow_tab: GitFlowTabComponent,
    pub help: HelpComponent,
}

impl UIComponents {
    fn new(theme: &Theme) -> Self {
        Self {
            sidebar: SidebarComponent::new(),
            status_tab: StatusTabComponent::new(),
            branches_tab: BranchesTabComponent::new(),
            tags_tab: TagsTabComponent::new(),
            stash_tab: StashTabComponent::new(),
            remotes_tab: RemotesTabComponent::new(),
            history_tab: CommitHistoryComponent::new(),
            gitflow_tab: GitFlowTabComponent::new(),
            help: HelpComponent::new(),
        }
    }

    fn handle_key_event(&mut self, key: KeyEvent, state: &mut AppState) -> AppResult<()> {
        // Handle global focus switching with Space key
        if key.code == KeyCode::Char(' ') {
            state.ui_state.current_focus = match state.ui_state.current_focus {
                FocusArea::Sidebar => FocusArea::MainContent,
                FocusArea::MainContent => FocusArea::Sidebar,
            };
            return Ok(());
        }

        // Handle navigation based on current focus
        match state.ui_state.current_focus {
            FocusArea::Sidebar => {
                // Handle sidebar navigation (branches, tags, files, etc.)
                self.handle_sidebar_key_event(key, state)
            }
            FocusArea::MainContent => {
                // Forward to the active tab component for main content area
                match state.current_tab() {
                    TabType::Status => self.status_tab.handle_key_event(key, state),
                    TabType::Branches => self.branches_tab.handle_key_event(key, state),
                    TabType::Tags => self.tags_tab.handle_key_event(key, state),
                    TabType::Stash => self.stash_tab.handle_key_event(key, state),
                    TabType::Remotes => self.remotes_tab.handle_key_event(key, state),
                    TabType::History => self.history_tab.handle_key_event(key, state),
                    TabType::GitFlow => self.gitflow_tab.handle_key_event(key, state),
                }
            }
        }
    }

    fn handle_sidebar_key_event(&mut self, key: KeyEvent, state: &mut AppState) -> AppResult<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if state.ui_state.sidebar_selected_index > 0 {
                    state.ui_state.sidebar_selected_index -= 1;
                }
            },
            KeyCode::Down | KeyCode::Char('j') => {
                // Get the maximum index based on current tab content
                let max_index = self.get_sidebar_max_index(state);
                if state.ui_state.sidebar_selected_index < max_index.saturating_sub(1) {
                    state.ui_state.sidebar_selected_index += 1;
                }
            },
            KeyCode::Enter => {
                // Handle sidebar selection based on current tab
                self.handle_sidebar_selection(state)?
            },
            _ => {}
        }
        Ok(())
    }

    fn get_sidebar_max_index(&self, state: &AppState) -> usize {
        match state.current_tab() {
            TabType::Branches => {
                // Get number of branches from git service
                if let Some(git_service) = &state.git_service {
                    git_service.list_branches().unwrap_or_default().len()
                } else {
                    0
                }
            },
            TabType::Tags => 3, // Mock data for now
            TabType::Stash => 2, // Mock data for now
            TabType::Status => state.git_state.file_status.len(),
            TabType::Remotes => 3, // Mock data for now
            TabType::History => 3, // Mock data for now
            TabType::GitFlow => 3, // Mock data for now
        }
    }

    fn handle_sidebar_selection(&mut self, state: &mut AppState) -> AppResult<()> {
        match state.current_tab() {
            TabType::Branches => {
                // Handle branch switching
                let git_service_clone = state.git_service.clone();
                if let Some(git_service) = &git_service_clone {
                    if let Ok(branches) = git_service.list_branches() {
                        if let Some(selected_branch) = branches.get(state.ui_state.sidebar_selected_index) {
                            // Update current branch info immediately for UI feedback
                            let mut updated_branch = selected_branch.clone();

                            // If switching to a different branch, update the state
                            if !selected_branch.is_current {
                                // Mark the selected branch as current and others as not current
                                updated_branch.is_current = true;

                                // Update the current branch in AppState - this will generate commits automatically
                                let commits = Vec::new(); // Empty for now, will be generated by AppState
                                state.update_current_branch(updated_branch, commits);

                                tracing::info!("Switched UI to branch: {}", selected_branch.name);

                                // Also perform actual git operation in background (if needed)
                                let git_service = git_service.clone();
                                let branch_name = selected_branch.name.clone();
                                if let Ok(handle) = tokio::runtime::Handle::try_current() {
                                    handle.spawn(async move {
                                        if let Err(e) = git_service.switch_branch(&branch_name).await {
                                            tracing::error!("Failed to switch branch: {}", e);
                                        } else {
                                            tracing::info!("Successfully switched git branch to: {}", branch_name);
                                        }
                                    });
                                }
                            }
                        }
                    }
                }
            },
            // TODO: Add handlers for other tabs
            _ => {}
        }
        Ok(())
    }
}

//! User Interface module
//!
//! Provides Terminal User Interface components using ratatui with
//! responsive design and keyboard navigation.

pub mod components;
pub mod layout;
pub mod theme;
pub mod diff;
pub mod selection;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Tabs},
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

    /// Render the entire UI
    pub fn render(&mut self, frame: &mut Frame, state: &AppState) {
        let size = frame.size();

        // Main layout: Header-Nav + Main-Content (matches tui-demo.html structure)
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header-Nav (tabs + sync status)
                Constraint::Min(0),    // Main-Content (sidebar + content area)
            ])
            .split(size);

        // Render header-nav with navigation tabs and status
        self.render_header_nav(frame, main_chunks[0], state);

        // Render main-content (sidebar + content area)
        self.render_main_content(frame, main_chunks[1], state);
    }

    /// Handle key events
    pub fn handle_key_event(&mut self, key: KeyEvent, state: &mut AppState) -> AppResult<()> {
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
            // Tab navigation
            KeyCode::Tab => {
                let current_index = TabType::all()
                    .iter()
                    .position(|&tab| tab == state.current_tab())
                    .unwrap_or(0);

                let next_index = (current_index + 1) % TabType::all().len();
                state.set_current_tab(TabType::all()[next_index]);
            }
            KeyCode::BackTab => {
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
            // Number keys for direct tab navigation
            KeyCode::Char(c @ '1'..='6') if !key.modifiers.contains(KeyModifiers::ALT) => {
                let tab_index = (c as u8 - b'1') as usize;
                if tab_index < TabType::all().len() {
                    state.set_current_tab(TabType::all()[tab_index]);
                }
            }
            // Forward other keys to the active tab component
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

    /// Render header-nav with navigation tabs and sync status
    fn render_header_nav(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let tab_titles: Vec<String> = TabType::all()
            .iter()
            .map(|tab| tab.name().to_string())
            .collect();

        let current_tab_index = TabType::all()
            .iter()
            .position(|&tab| tab == state.current_tab())
            .unwrap_or(0);

        // Split header area: tabs on left, sync status on right
        let header_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(0),     // Tabs area
                Constraint::Length(30), // Sync status area
            ])
            .split(area);

        let tabs = Tabs::new(tab_titles)
            .block(
                Block::default()
                    .title("AI-Commit TUI")
                    .borders(Borders::ALL)
                    .border_style(self.theme.border_style())
                    .title_style(self.theme.text_style()),
            )
            .style(Style::default().bg(self.theme.colors.secondary).fg(self.theme.colors.foreground))
            .highlight_style(self.theme.tab_highlight_style())
            .select(current_tab_index);

        frame.render_widget(tabs, header_layout[0]);

        // Render sync status in the right area
        self.render_sync_status(frame, header_layout[1], state);
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
                    .borders(Borders::ALL)
                    .border_style(self.theme.border_style()),
            )
            .style(self.theme.status_style());

        frame.render_widget(sync_status, area);
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
    pub gitflow_tab: GitFlowTabComponent,
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
            gitflow_tab: GitFlowTabComponent::new(),
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

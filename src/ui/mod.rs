//! User Interface module
//!
//! Provides Terminal User Interface components using ratatui with
//! responsive design and keyboard navigation.

pub mod components;
pub mod layout;
pub mod theme;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Tabs},
    Frame,
};
use tracing::debug;

use crate::{
    app::state::{AppState, TabType},
    config::UIConfig,
    error::AppResult,
};
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

        // Main layout: Header, Body, Status Bar
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Body
                Constraint::Length(1), // Status Bar
            ])
            .split(size);

        // Render header with navigation tabs
        self.render_header(frame, main_chunks[0], state);

        // Render main body
        self.render_body(frame, main_chunks[1], state);

        // Render status bar
        self.render_status_bar(frame, main_chunks[2], state);
    }

    /// Handle key events
    pub fn handle_key_event(&mut self, key: KeyEvent, state: &mut AppState) -> AppResult<()> {
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

    /// Render header with navigation tabs
    fn render_header(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let tab_titles: Vec<String> = TabType::all()
            .iter()
            .map(|tab| tab.name().to_string())
            .collect();

        let current_tab_index = TabType::all()
            .iter()
            .position(|&tab| tab == state.current_tab())
            .unwrap_or(0);

        let tabs = Tabs::new(tab_titles)
            .block(
                Block::default()
                    .title("AI-Commit TUI")
                    .borders(Borders::ALL)
                    .border_style(self.theme.border_style()),
            )
            .style(self.theme.tab_style())
            .highlight_style(self.theme.tab_highlight_style())
            .select(current_tab_index);

        frame.render_widget(tabs, area);
    }

    /// Render main body content
    fn render_body(&mut self, frame: &mut Frame, area: Rect, state: &AppState) {
        let body_layout = if state.ui_state.is_sidebar_visible {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(state.ui_state.sidebar_width),
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
        if state.ui_state.is_sidebar_visible && body_layout.len() > 1 {
            self.components
                .sidebar
                .render(frame, body_layout[0], state, &self.theme);
        }

        // Render main content area
        let content_area = if state.ui_state.is_sidebar_visible && body_layout.len() > 1 {
            body_layout[1]
        } else {
            body_layout[0]
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

    /// Render status bar
    fn render_status_bar(&mut self, frame: &mut Frame, area: Rect, state: &AppState) {
        self.components
            .status_bar
            .render(frame, area, state, &self.theme);
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
    pub status_bar: StatusBarComponent,
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
            status_bar: StatusBarComponent::new(),
        }
    }

    fn handle_key_event(&mut self, key: KeyEvent, state: &mut AppState) -> AppResult<()> {
        // Forward key event to the active tab component
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

//! Sidebar component
//!
//! Displays repository information and quick actions.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::{
    app::state::AppState,
    error::AppResult,
    ui::{components::Component, theme::Theme},
};

/// Sidebar component showing repository info and quick actions
pub struct SidebarComponent {
    // Component state can be added here
}

impl SidebarComponent {
    pub fn new() -> Self {
        Self {}
    }
}

impl Component for SidebarComponent {
    fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        state: &AppState,
        theme: &Theme,
    ) {
        let block = Block::default()
            .title("Repository Info")
            .borders(Borders::ALL)
            .border_style(theme.border_style());

        // Split sidebar into sections
        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6), // Repository info
                Constraint::Length(4), // Branch info
                Constraint::Min(0),    // Quick actions
            ])
            .split(area);

        // Repository section
        self.render_repository_info(frame, sections[0], state, theme);

        // Branch section
        self.render_branch_info(frame, sections[1], state, theme);

        // Quick actions section
        self.render_quick_actions(frame, sections[2], state, theme);
    }

    fn handle_key_event(&mut self, _key: KeyEvent, _state: &mut AppState) -> AppResult<()> {
        // Sidebar doesn't handle keys directly in this implementation
        Ok(())
    }
}

impl SidebarComponent {
    fn render_repository_info(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &AppState,
        theme: &Theme,
    ) {
        let repo_info = if state.git_state.is_repository {
            vec![
                format!("📁 Repository: ✓"),
                format!(
                    "📍 Path: {}",
                    state
                        .git_state
                        .repository_path
                        .as_deref()
                        .unwrap_or("Unknown")
                ),
                format!("📊 Files: {}", state.git_state.file_status.len()),
                format!(
                    "🕒 Updated: {}s ago",
                    (chrono::Utc::now() - state.git_state.last_status_update).num_seconds()
                ),
            ]
        } else {
            vec![
                "📁 Repository: ✗".to_string(),
                "Not a Git repository".to_string(),
                "".to_string(),
                "".to_string(),
            ]
        };

        let items: Vec<ListItem> = repo_info
            .into_iter()
            .map(|info| ListItem::new(info))
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Repository")
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            )
            .style(theme.text_style());

        frame.render_widget(list, area);
    }

    fn render_branch_info(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &AppState,
        theme: &Theme,
    ) {
        let branch_info = if let Some(branch) = &state.git_state.current_branch {
            vec![
                format!("🌿 Branch: {}", branch.name),
                format!("📈 Ahead: {}", branch.ahead),
                format!("📉 Behind: {}", branch.behind),
            ]
        } else {
            vec![
                "🌿 Branch: None".to_string(),
                "No commits yet".to_string(),
                "".to_string(),
            ]
        };

        let items: Vec<ListItem> = branch_info
            .into_iter()
            .map(|info| ListItem::new(info))
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Branch")
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            )
            .style(theme.text_style());

        frame.render_widget(list, area);
    }

    fn render_quick_actions(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &AppState,
        theme: &Theme,
    ) {
        let actions = vec![
            "r - Refresh status".to_string(),
            "s - Stage all".to_string(),
            "c - Commit".to_string(),
            "p - Push".to_string(),
            "f - Fetch".to_string(),
            "? - Help".to_string(),
        ];

        let items: Vec<ListItem> = actions
            .into_iter()
            .map(|action| ListItem::new(action))
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Quick Actions")
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            )
            .style(theme.muted_style());

        frame.render_widget(list, area);
    }
}

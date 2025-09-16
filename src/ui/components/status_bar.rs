//! Status bar component
//!
//! Displays application status, performance info, and help hints.

use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    widgets::Paragraph,
    Frame,
};

use crate::{
    app::state::AppState,
    error::AppResult,
    ui::{components::Component, theme::Theme},
};

/// Status bar component at the bottom of the screen
pub struct StatusBarComponent {
    // Component state
}

impl StatusBarComponent {
    pub fn new() -> Self {
        Self {}
    }

    fn get_status_message(&self, state: &AppState) -> String {
        match state.app_state.lifecycle {
            crate::app::state::LifecyclePhase::Starting => "Starting...".to_string(),
            crate::app::state::LifecyclePhase::Running => {
                if state.git_state.is_repository {
                    format!("Ready - {} files", state.git_state.file_status.len())
                } else {
                    "Not a Git repository".to_string()
                }
            }
            crate::app::state::LifecyclePhase::Quitting => "Shutting down...".to_string(),
        }
    }

    fn get_performance_info(&self, state: &AppState) -> String {
        format!(
            "Startup: {}ms | Render: {}ms | Memory: {}MB",
            state.performance_state.startup_time_ms,
            state.performance_state.last_render_time_ms,
            state.performance_state.memory_usage_mb
        )
    }

    fn get_help_text(&self, state: &AppState) -> String {
        match state.current_tab() {
            crate::app::state::TabType::Status => {
                "↑↓: Navigate | Space: Stage | Enter: Commit | r: Refresh"
            }
            crate::app::state::TabType::Branches => {
                "↑↓: Navigate | Enter: Switch | n: New | d: Delete"
            }
            crate::app::state::TabType::Tags => "↑↓: Navigate | n: New tag | d: Delete",
            crate::app::state::TabType::Stash => "↑↓: Navigate | Enter: Apply | d: Drop | s: Save",
            crate::app::state::TabType::Remotes => "↑↓: Navigate | f: Fetch | p: Push",
            crate::app::state::TabType::GitFlow => "↑↓: Navigate | Enter: Action",
        }
        .to_string()
    }
}

impl Component for StatusBarComponent {
    fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        state: &AppState,
        theme: &Theme,
    ) {
        // Split status bar into three sections
        let sections = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(30), // Status message
                Constraint::Min(0),     // Help text (center)
                Constraint::Length(35), // Performance info
            ])
            .split(area);

        // Status message (left)
        let status_message = self.get_status_message(state);
        let status_paragraph = Paragraph::new(status_message)
            .style(theme.text_style())
            .alignment(Alignment::Left);
        frame.render_widget(status_paragraph, sections[0]);

        // Help text (center)
        let help_text = self.get_help_text(state);
        let help_paragraph = Paragraph::new(help_text)
            .style(theme.muted_style())
            .alignment(Alignment::Center);
        frame.render_widget(help_paragraph, sections[1]);

        // Performance info (right)
        let perf_info = self.get_performance_info(state);
        let perf_paragraph = Paragraph::new(perf_info)
            .style(theme.muted_style())
            .alignment(Alignment::Right);
        frame.render_widget(perf_paragraph, sections[2]);
    }

    fn handle_key_event(&mut self, _key: KeyEvent, _state: &mut AppState) -> AppResult<()> {
        // Status bar doesn't handle keys directly
        Ok(())
    }
}

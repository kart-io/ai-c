//! Branches tab component

use crate::{
    app::state::AppState,
    error::AppResult,
    git::BranchInfo,
    ui::{components::Component, theme::Theme},
};
use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub struct BranchesTabComponent {
    selected_index: usize,
}

impl BranchesTabComponent {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
        }
    }

    fn get_branch_items(&self, branches: &[BranchInfo]) -> Vec<ListItem> {
        branches
            .iter()
            .map(|branch| {
                let mut text = branch.name.clone();
                if branch.is_current {
                    text = format!("* {}", text);
                }

                // Show commit info if available
                if !branch.last_commit_message.is_empty() {
                    text = format!(
                        "{} ({})",
                        text,
                        if branch.last_commit_message.len() > 30 {
                            format!("{}...", &branch.last_commit_message[..30.min(branch.last_commit_message.len())])
                        } else {
                            branch.last_commit_message.clone()
                        }
                    );
                }

                let style = if branch.is_current {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::White)
                };

                ListItem::new(text).style(style)
            })
            .collect()
    }
}

impl Component for BranchesTabComponent {
    fn render(&mut self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        // Create layout with branches list and details panel
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        // Get branches from Git service
        let branches = if let Some(git_service) = &state.git_service {
            git_service.list_branches().unwrap_or_else(|_| {
                // Return mock data if there's an error
                vec![
                    BranchInfo {
                        name: "main".to_string(),
                        is_current: true,
                        is_remote: false,
                        upstream: None,
                        ahead: 0,
                        behind: 0,
                        last_commit: "abc1234".to_string(),
                        last_commit_message: "Initial commit".to_string(),
                        last_commit_author: "Developer".to_string(),
                        last_commit_date: chrono::Utc::now(),
                    },
                    BranchInfo {
                        name: "feature/ui-improvements".to_string(),
                        is_current: false,
                        is_remote: false,
                        upstream: None,
                        ahead: 2,
                        behind: 0,
                        last_commit: "def5678".to_string(),
                        last_commit_message: "Add branch management UI".to_string(),
                        last_commit_author: "Developer".to_string(),
                        last_commit_date: chrono::Utc::now(),
                    },
                ]
            })
        } else {
            vec![]
        };

        // Render branches list
        let branch_items = self.get_branch_items(&branches);
        let branches_list = List::new(branch_items)
            .block(
                Block::default()
                    .title("Git Branches")
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            )
            .style(theme.text_style());

        frame.render_widget(branches_list, chunks[0]);

        // Render branch details
        let details_text = if !branches.is_empty() && self.selected_index < branches.len() {
            let branch = &branches[self.selected_index];
            format!(
                "Branch: {}\n\nCurrent: {}\nRemote: {}\nUpstream: {}\nAhead: {} | Behind: {}\n\nLast Commit:\n{}\n\nAuthor: {}\nDate: {}",
                branch.name,
                if branch.is_current { "Yes" } else { "No" },
                if branch.is_remote { "Yes" } else { "No" },
                branch.upstream.as_ref().unwrap_or(&"None".to_string()),
                branch.ahead,
                branch.behind,
                branch.last_commit_message,
                branch.last_commit_author,
                branch.last_commit_date.format("%Y-%m-%d %H:%M:%S")
            )
        } else {
            "No branch selected\n\nUse ↑/↓ arrows to navigate\nPress Enter to switch to branch".to_string()
        };

        let details_paragraph = Paragraph::new(details_text)
            .block(
                Block::default()
                    .title("Branch Details")
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            )
            .style(theme.text_style())
            .wrap(ratatui::widgets::Wrap { trim: true });

        frame.render_widget(details_paragraph, chunks[1]);
    }

    fn handle_key_event(&mut self, key: KeyEvent, _state: &mut AppState) -> AppResult<()> {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Up => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            },
            KeyCode::Down => {
                // We'll implement proper bounds checking when we have access to branches
                self.selected_index = (self.selected_index + 1).min(10); // Arbitrary max for now
            },
            KeyCode::Enter => {
                // TODO: Implement branch switching
                // This would call git_service.switch_branch() in a real implementation
            },
            _ => {}
        }
        Ok(())
    }
}

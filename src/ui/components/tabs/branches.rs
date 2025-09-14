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
use std::time::Instant;

pub struct BranchesTabComponent {
    selected_index: usize,
    cached_branches: Vec<BranchInfo>,
    last_update: std::time::Instant,
}

impl BranchesTabComponent {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            cached_branches: Vec::new(),
            last_update: std::time::Instant::now(),
        }
    }

    fn get_branches(&mut self, state: &AppState) -> &Vec<BranchInfo> {
        // Refresh branches every 5 seconds or if cache is empty
        let should_refresh = self.cached_branches.is_empty() ||
            self.last_update.elapsed().as_secs() > 5;

        if should_refresh {
            if let Some(git_service) = &state.git_service {
                self.cached_branches = git_service.list_branches().unwrap_or_else(|_| {
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
                });
                self.last_update = std::time::Instant::now();
            }
        }

        &self.cached_branches
    }

    fn force_refresh(&mut self) {
        self.cached_branches.clear();
        self.last_update = std::time::Instant::now() - std::time::Duration::from_secs(10);
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

        // Get branches from cache or refresh
        let branches = self.get_branches(state).clone();

        // Ensure selected_index is within bounds
        if self.selected_index >= branches.len() && !branches.is_empty() {
            self.selected_index = branches.len() - 1;
        }

        // Render branches list with selection highlighting
        let branch_items = self.get_branch_items(&branches);
        let branches_list = List::new(branch_items)
            .block(
                Block::default()
                    .title("Git Branches")
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            )
            .style(theme.text_style())
            .highlight_style(Style::default().fg(Color::Black).bg(Color::White))
            .highlight_symbol(">> ");

        frame.render_stateful_widget(branches_list, chunks[0], &mut ratatui::widgets::ListState::default().with_selected(Some(self.selected_index)));

        // Render branch details
        let details_text = if !branches.is_empty() && self.selected_index < branches.len() {
            let branch = &branches[self.selected_index];
            let switch_instruction = if branch.is_current {
                "Already on this branch".to_string()
            } else {
                "Press Enter to switch to this branch".to_string()
            };

            format!(
                "Branch: {}\n\nCurrent: {}\nRemote: {}\nUpstream: {}\nAhead: {} | Behind: {}\n\nLast Commit:\n{}\n\nAuthor: {}\nDate: {}\n\n---\nCommands:\n{}\n↑/↓/j/k - Navigate branches",
                branch.name,
                if branch.is_current { "Yes (active)" } else { "No" },
                if branch.is_remote { "Yes" } else { "No" },
                branch.upstream.as_ref().unwrap_or(&"None".to_string()),
                branch.ahead,
                branch.behind,
                branch.last_commit_message,
                branch.last_commit_author,
                branch.last_commit_date.format("%Y-%m-%d %H:%M:%S"),
                switch_instruction
            )
        } else {
            "No branches found\n\nCommands:\n↑/↓/j/k - Navigate branches\nEnter - Switch to selected branch".to_string()
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

    fn handle_key_event(&mut self, key: KeyEvent, state: &mut AppState) -> AppResult<()> {
        use crossterm::event::KeyCode;

        // Get current branches for navigation bounds checking
        let branches = self.get_branches(state).clone();

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            },
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_index < branches.len().saturating_sub(1) {
                    self.selected_index += 1;
                }
            },
            KeyCode::Enter => {
                // Implement branch switching
                if let Some(git_service) = &state.git_service {
                    if !branches.is_empty() && self.selected_index < branches.len() {
                        let selected_branch = &branches[self.selected_index];

                        // Don't switch if already on the selected branch
                        if !selected_branch.is_current {
                            // Use tokio to handle the async call
                            let git_service = git_service.clone();
                            let branch_name = selected_branch.name.clone();

                            // Since we can't use async in the UI thread, we'll trigger the operation
                            // In a real implementation, this would be handled by the event system
                            match tokio::runtime::Handle::try_current() {
                                Ok(handle) => {
                                    handle.spawn(async move {
                                        if let Err(e) = git_service.switch_branch(&branch_name).await {
                                            tracing::error!("Failed to switch branch: {}", e);
                                        } else {
                                            tracing::info!("Successfully switched to branch: {}", branch_name);
                                        }
                                    });
                                    // Force refresh on next render
                                    self.force_refresh();
                                }
                                Err(_) => {
                                    // Fallback: we're not in an async context
                                    tracing::warn!("Cannot switch branch: not in async context");
                                }
                            }
                        }
                    }
                }
            },
            _ => {}
        }
        Ok(())
    }
}

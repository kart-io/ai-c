//! Status tab component
//!
//! Shows Git file status with staging/unstaging capabilities.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::{
    app::state::AppState,
    error::AppResult,
    ui::{components::Component, theme::Theme},
};

/// Status tab showing file changes
pub struct StatusTabComponent {
    /// List state for navigation
    list_state: ListState,
}

impl StatusTabComponent {
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
        }
    }

    fn get_file_status_items(&self, state: &AppState) -> Vec<ListItem> {
        if state.git_state.file_status.is_empty() {
            vec![ListItem::new("No changes detected")]
        } else {
            state
                .git_state
                .file_status
                .iter()
                .map(|file_status| {
                    let status_char = file_status.status.status_char();
                    let file_info = format!(
                        "{} {} ({})",
                        status_char,
                        file_status.path,
                        Self::format_file_size(file_status.size)
                    );
                    ListItem::new(file_info)
                })
                .collect()
        }
    }

    fn format_file_size(size: u64) -> String {
        if size < 1024 {
            format!("{}B", size)
        } else if size < 1024 * 1024 {
            format!("{:.1}KB", size as f64 / 1024.0)
        } else {
            format!("{:.1}MB", size as f64 / (1024.0 * 1024.0))
        }
    }

    fn get_selected_file<'a>(&self, state: &'a AppState) -> Option<&'a crate::git::FileStatus> {
        if let Some(selected) = self.list_state.selected() {
            state.git_state.file_status.get(selected)
        } else {
            None
        }
    }
}

impl Component for StatusTabComponent {
    fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        state: &AppState,
        theme: &Theme,
    ) {
        // Split into file list and details
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(60), // File list
                Constraint::Percentage(40), // File details
            ])
            .split(area);

        // Render file list
        self.render_file_list(frame, chunks[0], state, theme);

        // Render file details
        self.render_file_details(frame, chunks[1], state, theme);
    }

    fn handle_key_event(&mut self, key: KeyEvent, state: &mut AppState) -> AppResult<()> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                if !state.git_state.file_status.is_empty() {
                    let i = match self.list_state.selected() {
                        Some(i) => {
                            if i >= state.git_state.file_status.len() - 1 {
                                0
                            } else {
                                i + 1
                            }
                        }
                        None => 0,
                    };
                    self.list_state.select(Some(i));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if !state.git_state.file_status.is_empty() {
                    let i = match self.list_state.selected() {
                        Some(i) => {
                            if i == 0 {
                                state.git_state.file_status.len() - 1
                            } else {
                                i - 1
                            }
                        }
                        None => 0,
                    };
                    self.list_state.select(Some(i));
                }
            }
            KeyCode::Char(' ') => {
                // Toggle staging for selected file
                if let Some(selected_file) = self.get_selected_file(state) {
                    let file_path = selected_file.path.clone();
                    // TODO: Implement staging toggle via event system
                    tracing::info!("Toggle staging for: {}", file_path);
                }
            }
            KeyCode::Char('a') => {
                // Stage all files
                tracing::info!("Stage all files requested");
                // TODO: Implement via event system
            }
            _ => {}
        }

        Ok(())
    }
}

impl StatusTabComponent {
    fn render_file_list(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        state: &AppState,
        theme: &Theme,
    ) {
        let items = if state.git_state.file_status.is_empty() {
            vec![ListItem::new("No changes detected")]
        } else {
            state
                .git_state
                .file_status
                .iter()
                .map(|file_status| {
                    let status_char = file_status.status.status_char();
                    let file_info = format!(
                        "{} {} ({})",
                        status_char,
                        file_status.path,
                        Self::format_file_size(file_status.size)
                    );
                    ListItem::new(file_info)
                })
                .collect()
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .title("File Status")
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            )
            .style(theme.text_style())
            .highlight_style(theme.highlight_style())
            .highlight_symbol("→ ");

        frame.render_stateful_widget(list, area, &mut self.list_state);
    }

    fn render_file_details(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &AppState,
        theme: &Theme,
    ) {
        let details = if let Some(selected_file) = self.get_selected_file(state) {
            format!(
                "File: {}\n\
                 Size: {}\n\
                 Modified: {}\n\
                 Status: {}\n\
                 Staged: {}\n\
                 Working Tree: {}\n\
                 Binary: {}",
                selected_file.path,
                Self::format_file_size(selected_file.size),
                selected_file.modified.format("%Y-%m-%d %H:%M:%S"),
                selected_file.status.status_char(),
                if selected_file.status.is_staged() {
                    "Yes"
                } else {
                    "No"
                },
                if selected_file.status.is_modified() {
                    "Modified"
                } else {
                    "Clean"
                },
                if selected_file.is_binary { "Yes" } else { "No" }
            )
        } else {
            "Select a file to view details".to_string()
        };

        let paragraph = Paragraph::new(details)
            .block(
                Block::default()
                    .title("File Details")
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            )
            .style(theme.text_style())
            .wrap(ratatui::widgets::Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }
}

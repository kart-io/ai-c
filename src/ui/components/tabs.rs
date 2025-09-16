//! Tab components for different Git operations
//!
//! Each tab represents a different view/functionality within the TUI.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use tracing::debug;

use crate::{
    app::state::AppState,
    error::AppResult,
    ui::{components::Component, theme::Theme, selection::{TextPosition, SelectionMode}},
};

/// Status tab component - shows working directory status
pub struct StatusTabComponent {
    selected_index: usize,
}

impl StatusTabComponent {
    pub fn new() -> Self {
        Self { selected_index: 0 }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        let items: Vec<ListItem> = state
            .git_state
            .file_status
            .iter()
            .map(|file| {
                let status_char = file.status.status_char();

                ListItem::new(format!(" {} {}", status_char, file.path))
                    .style(theme.git_status_style(status_char))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Git Status")
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            )
            .style(theme.text_style())
            .highlight_style(theme.highlight_style());

        frame.render_stateful_widget(list, area, &mut ratatui::widgets::ListState::default());
    }

    pub fn handle_key_event(&mut self, key: KeyEvent, state: &mut AppState) -> AppResult<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_index < state.git_state.file_status.len().saturating_sub(1) {
                    self.selected_index += 1;
                }
            }
            KeyCode::Enter => {
                debug!("Enter pressed on status file: {}", self.selected_index);
            }
            KeyCode::Char(' ') if key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT) => {
                // Start/extend selection with Shift+Space
                let pos = TextPosition::new(self.selected_index, 0);
                if state.ui_state.selection_manager.is_selecting() {
                    state.ui_state.selection_manager.update_selection(pos);
                } else {
                    state.ui_state.selection_manager.start_selection(pos);
                }
            }
            KeyCode::Char('l') => {
                // Select current line
                let text_lines = state.git_state.file_status.iter()
                    .map(|file| format!(" {} {}", file.status.status_char(), file.path))
                    .collect::<Vec<_>>();
                state.ui_state.selection_manager.select_line(self.selected_index, &text_lines);
            }
            KeyCode::Esc => {
                // Clear selection
                state.ui_state.selection_manager.clear_selection();
            }
            _ => {}
        }
        Ok(())
    }
}

/// Branches tab component - manages Git branches
pub struct BranchesTabComponent {
    selected_index: usize,
}

impl BranchesTabComponent {
    pub fn new() -> Self {
        Self { selected_index: 0 }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        let branches = if let Some(git_service) = &state.git_service {
            git_service.list_branches().unwrap_or_default()
        } else {
            vec![]
        };

        let items: Vec<ListItem> = branches
            .iter()
            .map(|branch| {
                let prefix = if branch.is_current { "* " } else { "  " };
                let item = ListItem::new(format!("{}{}", prefix, branch.name));
                if branch.is_current {
                    item.style(theme.highlight_style())
                } else {
                    item.style(theme.text_style())
                }
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Branches")
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            )
            .style(theme.text_style());

        frame.render_widget(list, area);
    }

    pub fn handle_key_event(&mut self, key: KeyEvent, state: &mut AppState) -> AppResult<()> {
        let branch_count = if let Some(git_service) = &state.git_service {
            git_service.list_branches().unwrap_or_default().len()
        } else {
            0
        };

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_index < branch_count.saturating_sub(1) {
                    self.selected_index += 1;
                }
            }
            KeyCode::Enter => {
                debug!("Enter pressed on branch: {}", self.selected_index);
            }
            KeyCode::Char(' ') if key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT) => {
                // Start/extend selection with Shift+Space
                let pos = TextPosition::new(self.selected_index, 0);
                if state.ui_state.selection_manager.is_selecting() {
                    state.ui_state.selection_manager.update_selection(pos);
                } else {
                    state.ui_state.selection_manager.start_selection(pos);
                }
            }
            KeyCode::Char('l') => {
                // Select current line (branch)
                if let Some(git_service) = &state.git_service {
                    let text_lines = git_service.list_branches().unwrap_or_default().iter()
                        .map(|branch| {
                            let prefix = if branch.is_current { "* " } else { "  " };
                            format!("{}{}", prefix, branch.name)
                        })
                        .collect::<Vec<_>>();
                    state.ui_state.selection_manager.select_line(self.selected_index, &text_lines);
                }
            }
            KeyCode::Esc => {
                // Clear selection
                state.ui_state.selection_manager.clear_selection();
            }
            _ => {}
        }
        Ok(())
    }
}

/// Tags tab component - manages Git tags
pub struct TagsTabComponent {
    selected_index: usize,
}

impl TagsTabComponent {
    pub fn new() -> Self {
        Self { selected_index: 0 }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, _state: &AppState, theme: &Theme) {
        // Mock data for now
        let tags = vec!["v1.0.0", "v0.9.0", "v0.8.0"];

        let items: Vec<ListItem> = tags
            .iter()
            .map(|tag| ListItem::new(format!("  {}", tag)).style(theme.text_style()))
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

    pub fn handle_key_event(&mut self, key: KeyEvent, _state: &mut AppState) -> AppResult<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected_index += 1;
            }
            _ => {}
        }
        Ok(())
    }
}

/// Stash tab component - manages Git stash
pub struct StashTabComponent {
    selected_index: usize,
}

impl StashTabComponent {
    pub fn new() -> Self {
        Self { selected_index: 0 }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, _state: &AppState, theme: &Theme) {
        // Mock data for now
        let stashes = vec!["stash@{0}: WIP on main", "stash@{1}: feature work"];

        let items: Vec<ListItem> = stashes
            .iter()
            .map(|stash| ListItem::new(format!("  {}", stash)).style(theme.text_style()))
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

    pub fn handle_key_event(&mut self, key: KeyEvent, _state: &mut AppState) -> AppResult<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected_index += 1;
            }
            _ => {}
        }
        Ok(())
    }
}

/// Remotes tab component - manages Git remotes
pub struct RemotesTabComponent {
    selected_index: usize,
}

impl RemotesTabComponent {
    pub fn new() -> Self {
        Self { selected_index: 0 }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, _state: &AppState, theme: &Theme) {
        // Mock data for now
        let remotes = vec!["origin", "upstream", "fork"];

        let items: Vec<ListItem> = remotes
            .iter()
            .map(|remote| ListItem::new(format!("  {}", remote)).style(theme.text_style()))
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

    pub fn handle_key_event(&mut self, key: KeyEvent, _state: &mut AppState) -> AppResult<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected_index += 1;
            }
            _ => {}
        }
        Ok(())
    }
}

/// GitFlow tab component - Git Flow workflow management
pub struct GitFlowTabComponent {
    selected_index: usize,
}

impl GitFlowTabComponent {
    pub fn new() -> Self {
        Self { selected_index: 0 }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, _state: &AppState, theme: &Theme) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0)])
            .split(area);

        let content = Paragraph::new("Git Flow workflow management\n\n• feature/\n• release/\n• hotfix/")
            .block(
                Block::default()
                    .title("Git Flow")
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            )
            .style(theme.text_style());

        frame.render_widget(content, layout[0]);
    }

    pub fn handle_key_event(&mut self, key: KeyEvent, _state: &mut AppState) -> AppResult<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected_index += 1;
            }
            _ => {}
        }
        Ok(())
    }
}
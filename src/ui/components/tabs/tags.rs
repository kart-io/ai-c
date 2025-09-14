//! Tags tab component

use crate::{
    app::state::AppState,
    error::AppResult,
    git::TagInfo,
    ui::{components::Component, theme::Theme},
};
use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub struct TagsTabComponent {
    selected_index: usize,
    tags: Vec<TagInfo>,
}

impl TagsTabComponent {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            tags: vec![],
        }
    }

    fn get_tag_items(&self) -> Vec<ListItem> {
        self.tags
            .iter()
            .map(|tag| {
                let mut text = tag.name.clone();

                // Add message preview if available
                if let Some(ref message) = tag.message {
                    if !message.is_empty() {
                        let preview = if message.len() > 40 {
                            format!("{}...", &message[..40.min(message.len())])
                        } else {
                            message.clone()
                        };
                        text = format!("{} - {}", text, preview);
                    }
                }

                let style = if tag.message.is_some() {
                    Style::default().fg(Color::Yellow) // Annotated tag
                } else {
                    Style::default().fg(Color::White)  // Lightweight tag
                };

                ListItem::new(text).style(style)
            })
            .collect()
    }

    fn load_tags(&mut self, state: &AppState) {
        // Only load tags if we don't already have them or if forced
        if !self.tags.is_empty() {
            return;
        }

        if let Some(_git_service) = &state.git_service {
            // Use mock data for now to avoid async issues in UI thread
            // In a real implementation, we'd load this data asynchronously
            // and store it in the AppState, then update the UI
            self.tags = vec![
                TagInfo {
                    name: "v1.0.0".to_string(),
                    target_commit: "abc1234".to_string(),
                    message: Some("First stable release".to_string()),
                    tagger: Some("Developer <dev@example.com>".to_string()),
                    date: chrono::Utc::now(),
                },
                TagInfo {
                    name: "v0.9.0".to_string(),
                    target_commit: "def5678".to_string(),
                    message: None, // Lightweight tag
                    tagger: None,
                    date: chrono::Utc::now(),
                },
                TagInfo {
                    name: "v0.8.0".to_string(),
                    target_commit: "ghi9012".to_string(),
                    message: Some("Beta release with UI improvements".to_string()),
                    tagger: Some("Developer <dev@example.com>".to_string()),
                    date: chrono::Utc::now(),
                },
            ];
        } else {
            self.tags = vec![];
        }
    }
}

impl Component for TagsTabComponent {
    fn render(&mut self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        // Load tags data
        self.load_tags(state);

        // Create layout with tags list and details panel
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Render tags list
        let tag_items = self.get_tag_items();
        let tags_list = List::new(tag_items)
            .block(
                Block::default()
                    .title("Git Tags")
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            )
            .style(theme.text_style());

        frame.render_widget(tags_list, chunks[0]);

        // Render tag details
        let details_text = if !self.tags.is_empty() && self.selected_index < self.tags.len() {
            let tag = &self.tags[self.selected_index];
            let tag_type = if tag.message.is_some() { "Annotated" } else { "Lightweight" };

            format!(
                "Tag: {}\nType: {}\nTarget: {}\n\n{}{}Date: {}\n\n{}",
                tag.name,
                tag_type,
                if tag.target_commit.len() >= 8 {
                    &tag.target_commit[..8]
                } else {
                    &tag.target_commit
                },
                if let Some(ref message) = tag.message {
                    format!("Message:\n{}\n\n", message)
                } else {
                    String::new()
                },
                if let Some(ref tagger) = tag.tagger {
                    format!("Tagger: {}\n", tagger)
                } else {
                    String::new()
                },
                tag.date.format("%Y-%m-%d %H:%M:%S"),
                "Commands:\nD - Delete tag\nC - Create new tag"
            )
        } else if self.tags.is_empty() {
            "No tags found in repository\n\nPress 'C' to create a new tag".to_string()
        } else {
            "Use ↑/↓ arrows to navigate\nPress 'D' to delete selected tag\nPress 'C' to create new tag".to_string()
        };

        let details_paragraph = Paragraph::new(details_text)
            .block(
                Block::default()
                    .title("Tag Details")
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
                if self.selected_index < self.tags.len().saturating_sub(1) {
                    self.selected_index += 1;
                }
            },
            KeyCode::Char('d') | KeyCode::Char('D') => {
                // TODO: Implement tag deletion
                // This would call git_service.delete_tag() in a real implementation
            },
            KeyCode::Char('c') | KeyCode::Char('C') => {
                // TODO: Implement tag creation dialog
                // This would show a dialog to create a new tag
            },
            _ => {}
        }
        Ok(())
    }
}

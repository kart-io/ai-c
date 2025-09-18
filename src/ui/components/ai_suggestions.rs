//! AI suggestions panel component

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::{
    app::state::AppState,
    error::AppResult,
    ui::theme::Theme,
};

/// AI suggestions panel component
pub struct AISuggestionsComponent {
    selected_index: usize,
    show_preview: bool,
}

impl AISuggestionsComponent {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            show_preview: false,
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        // Split area into suggestions list and preview panel
        let chunks = if self.show_preview {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(50), // Suggestions list
                    Constraint::Percentage(50), // Preview panel
                ])
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(100)])
                .split(area)
        };

        // Render suggestions list
        self.render_suggestions_list(frame, chunks[0], state, theme);

        // Render preview panel if enabled
        if self.show_preview && chunks.len() > 1 {
            self.render_suggestion_preview(frame, chunks[1], state, theme);
        }
    }

    fn render_suggestions_list(&mut self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        let suggestions = &state.agent_state.ai_suggestions;

        let items: Vec<ListItem> = suggestions
            .iter()
            .enumerate()
            .map(|(index, suggestion)| {
                let confidence_color = if suggestion.confidence > 0.8 {
                    Color::Green
                } else if suggestion.confidence > 0.6 {
                    Color::Yellow
                } else {
                    Color::Red
                };

                let confidence_indicator = format!("{:.0}%", suggestion.confidence * 100.0);

                let type_icon = match suggestion.suggestion_type.as_str() {
                    "commit_message" => "💬",
                    "code_improvement" => "🔧",
                    "bug_fix" => "🐛",
                    "optimization" => "⚡",
                    _ => "💡",
                };

                let style = if index == self.selected_index {
                    theme.highlight_style()
                } else {
                    theme.text_style()
                };

                let line = Line::from(vec![
                    Span::raw(type_icon),
                    Span::raw(" "),
                    Span::styled(&suggestion.title, style),
                    Span::raw(" "),
                    Span::styled(
                        format!("({})", confidence_indicator),
                        Style::default().fg(confidence_color),
                    ),
                ]);

                ListItem::new(line)
            })
            .collect();

        let title = format!("AI Suggestions ({} available)", suggestions.len());
        let list = List::new(items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            )
            .style(theme.text_style());

        frame.render_widget(list, area);
    }

    fn render_suggestion_preview(&mut self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        let suggestions = &state.agent_state.ai_suggestions;

        if let Some(suggestion) = suggestions.get(self.selected_index) {
            // Split preview area into sections
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(6),   // Header info
                    Constraint::Min(0),      // Content preview
                    Constraint::Length(3),   // Actions
                ])
                .split(area);

            // Header info section
            let header_text = vec![
                Line::from(vec![
                    Span::styled("Type: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(&suggestion.suggestion_type),
                ]),
                Line::from(vec![
                    Span::styled("Confidence: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(
                        format!("{:.1}%", suggestion.confidence * 100.0),
                        if suggestion.confidence > 0.8 {
                            Style::default().fg(Color::Green)
                        } else if suggestion.confidence > 0.6 {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default().fg(Color::Red)
                        },
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Agent: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(suggestion.agent_id.to_string()),
                ]),
                Line::from(vec![
                    Span::styled("Generated: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(suggestion.timestamp.format("%H:%M:%S").to_string()),
                ]),
            ];

            let header_paragraph = Paragraph::new(header_text)
                .block(
                    Block::default()
                        .title(suggestion.title.as_str())
                        .borders(Borders::ALL)
                        .border_style(theme.border_style()),
                )
                .wrap(Wrap { trim: true });

            frame.render_widget(header_paragraph, chunks[0]);

            // Content preview section
            let content_paragraph = Paragraph::new(suggestion.content.clone())
                .block(
                    Block::default()
                        .title("Preview")
                        .borders(Borders::ALL)
                        .border_style(theme.border_style()),
                )
                .style(theme.text_style())
                .wrap(Wrap { trim: true });

            frame.render_widget(content_paragraph, chunks[1]);

            // Actions section
            let actions_text = "Enter: Apply | D: Dismiss | R: Regenerate | Esc: Close Preview";
            let actions_paragraph = Paragraph::new(actions_text)
                .block(
                    Block::default()
                        .title("Actions")
                        .borders(Borders::ALL)
                        .border_style(theme.border_style()),
                )
                .style(Style::default().fg(Color::Gray))
                .wrap(Wrap { trim: true });

            frame.render_widget(actions_paragraph, chunks[2]);
        } else {
            // No suggestion selected
            let placeholder = Paragraph::new("No suggestion selected")
                .block(
                    Block::default()
                        .title("Suggestion Preview")
                        .borders(Borders::ALL)
                        .border_style(theme.border_style()),
                )
                .style(Style::default().fg(Color::Gray))
                .wrap(Wrap { trim: true });

            frame.render_widget(placeholder, area);
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent, state: &mut AppState) -> AppResult<()> {
        let suggestion_count = state.agent_state.ai_suggestions.len();

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_index < suggestion_count.saturating_sub(1) {
                    self.selected_index += 1;
                }
            }
            KeyCode::Enter => {
                if self.show_preview {
                    // Apply selected suggestion
                    if let Some(suggestion) = state.agent_state.ai_suggestions.get(self.selected_index) {
                        tracing::info!("Applying suggestion: {}", suggestion.title);
                        // TODO: Implement suggestion application logic

                        // Remove applied suggestion
                        state.agent_state.ai_suggestions.remove(self.selected_index);
                        if self.selected_index >= state.agent_state.ai_suggestions.len() && self.selected_index > 0 {
                            self.selected_index -= 1;
                        }
                    }
                } else {
                    // Show preview
                    self.show_preview = true;
                }
            }
            KeyCode::Char(' ') => {
                // Toggle preview panel
                self.show_preview = !self.show_preview;
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                // Dismiss selected suggestion
                if suggestion_count > 0 {
                    state.agent_state.ai_suggestions.remove(self.selected_index);
                    if self.selected_index >= state.agent_state.ai_suggestions.len() && self.selected_index > 0 {
                        self.selected_index -= 1;
                    }
                    tracing::info!("Dismissed suggestion at index {}", self.selected_index);
                }
            }
            KeyCode::Char('r') => {
                // Regenerate suggestion
                if let Some(suggestion) = state.agent_state.ai_suggestions.get(self.selected_index) {
                    tracing::info!("Regenerating suggestion: {}", suggestion.title);
                    // TODO: Implement suggestion regeneration logic
                }
            }
            KeyCode::Esc => {
                // Close preview
                self.show_preview = false;
            }
            _ => {}
        }

        Ok(())
    }
}
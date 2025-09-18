//! Input modal component

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::{error::AppResult, ui::theme::Theme};

use super::{Modal, ModalResult};

/// Input modal for text entry
pub struct InputModal {
    is_open: bool,
    title: String,
    prompt: String,
    input: String,
    cursor_position: usize,
    placeholder: String,
}

impl InputModal {
    pub fn new() -> Self {
        Self {
            is_open: false,
            title: String::new(),
            prompt: String::new(),
            input: String::new(),
            cursor_position: 0,
            placeholder: String::new(),
        }
    }

    /// Open the modal with a prompt
    pub fn open(&mut self, title: &str, prompt: &str) {
        self.is_open = true;
        self.title = title.to_string();
        self.prompt = prompt.to_string();
        self.input.clear();
        self.cursor_position = 0;
        self.placeholder.clear();
    }

    /// Open with a placeholder text
    pub fn open_with_placeholder(&mut self, title: &str, prompt: &str, placeholder: &str) {
        self.open(title, prompt);
        self.placeholder = placeholder.to_string();
    }

    /// Get the current input value
    pub fn get_input(&self) -> &str {
        &self.input
    }

    /// Calculate centered rectangle for modal
    fn centered_rect(&self, percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }
}

impl Modal for InputModal {
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.is_open {
            return;
        }

        let modal_area = self.centered_rect(60, 25, area);

        // Clear the background
        frame.render_widget(Clear, modal_area);

        // Modal layout: title + prompt + input + help
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),    // Title
                Constraint::Length(3),    // Prompt
                Constraint::Length(3),    // Input field
                Constraint::Length(2),    // Help text
            ])
            .split(modal_area);

        // Title
        let title_block = Block::default()
            .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
            .title(self.title.as_str())
            .border_style(Style::default().fg(Color::Blue));
        frame.render_widget(title_block, chunks[0]);

        // Prompt
        let prompt = Paragraph::new(self.prompt.clone())
            .block(Block::default().borders(Borders::LEFT | Borders::RIGHT))
            .style(theme.text_style())
            .wrap(Wrap { trim: true });
        frame.render_widget(prompt, chunks[1]);

        // Input field
        let input_text = if self.input.is_empty() && !self.placeholder.is_empty() {
            self.placeholder.clone()
        } else {
            self.input.clone()
        };

        let input_style = if self.input.is_empty() && !self.placeholder.is_empty() {
            Style::default().fg(Color::DarkGray)
        } else {
            theme.text_style()
        };

        let input_field = Paragraph::new(input_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Green))
            )
            .style(input_style);
        frame.render_widget(input_field, chunks[2]);

        // Help text
        let help_text = "Enter: Submit | Esc: Cancel | Ctrl+U: Clear";
        let help = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT))
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        frame.render_widget(help, chunks[3]);
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> AppResult<ModalResult> {
        if !self.is_open {
            return Ok(ModalResult::None);
        }

        match key.code {
            KeyCode::Enter => {
                let input = self.input.clone();
                self.is_open = false;
                self.input.clear();
                self.cursor_position = 0;
                Ok(ModalResult::Input(input))
            }
            KeyCode::Esc => {
                self.is_open = false;
                self.input.clear();
                self.cursor_position = 0;
                Ok(ModalResult::Cancelled)
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Clear input (Ctrl+U)
                self.input.clear();
                self.cursor_position = 0;
                Ok(ModalResult::None)
            }
            KeyCode::Char(c) => {
                self.input.insert(self.cursor_position, c);
                self.cursor_position += 1;
                Ok(ModalResult::None)
            }
            KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                    self.input.remove(self.cursor_position);
                }
                Ok(ModalResult::None)
            }
            KeyCode::Delete => {
                if self.cursor_position < self.input.len() {
                    self.input.remove(self.cursor_position);
                }
                Ok(ModalResult::None)
            }
            KeyCode::Left => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
                Ok(ModalResult::None)
            }
            KeyCode::Right => {
                if self.cursor_position < self.input.len() {
                    self.cursor_position += 1;
                }
                Ok(ModalResult::None)
            }
            KeyCode::Home => {
                self.cursor_position = 0;
                Ok(ModalResult::None)
            }
            KeyCode::End => {
                self.cursor_position = self.input.len();
                Ok(ModalResult::None)
            }
            _ => Ok(ModalResult::None),
        }
    }

    fn is_open(&self) -> bool {
        self.is_open
    }

    fn close(&mut self) {
        self.is_open = false;
        self.input.clear();
        self.cursor_position = 0;
    }
}
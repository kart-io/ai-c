//! Confirmation modal component

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

/// Confirmation modal for yes/no decisions
pub struct ConfirmationModal {
    is_open: bool,
    title: String,
    message: String,
    confirm_text: String,
    cancel_text: String,
    selected_button: usize, // 0 = confirm, 1 = cancel
}

impl ConfirmationModal {
    pub fn new() -> Self {
        Self {
            is_open: false,
            title: String::new(),
            message: String::new(),
            confirm_text: "Yes".to_string(),
            cancel_text: "No".to_string(),
            selected_button: 0,
        }
    }

    /// Open the modal with a specific message
    pub fn open(&mut self, title: &str, message: &str) {
        self.is_open = true;
        self.title = title.to_string();
        self.message = message.to_string();
        self.selected_button = 0;
    }

    /// Open with custom button texts
    pub fn open_with_buttons(&mut self, title: &str, message: &str, confirm_text: &str, cancel_text: &str) {
        self.open(title, message);
        self.confirm_text = confirm_text.to_string();
        self.cancel_text = cancel_text.to_string();
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

impl Modal for ConfirmationModal {
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.is_open {
            return;
        }

        let modal_area = self.centered_rect(50, 30, area);

        // Clear the background
        frame.render_widget(Clear, modal_area);

        // Modal layout: title + message + buttons
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),    // Title area
                Constraint::Min(3),       // Message area
                Constraint::Length(3),    // Button area
            ])
            .split(modal_area);

        // Title
        let title_block = Block::default()
            .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
            .title(self.title.as_str())
            .border_style(Style::default().fg(Color::Blue));
        frame.render_widget(title_block, chunks[0]);

        // Message
        let message = Paragraph::new(self.message.clone())
            .block(Block::default().borders(Borders::LEFT | Borders::RIGHT))
            .style(theme.text_style())
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        frame.render_widget(message, chunks[1]);

        // Buttons
        let button_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(chunks[2]);

        // Confirm button
        let confirm_style = if self.selected_button == 0 {
            Style::default().bg(Color::Green).fg(Color::Black)
        } else {
            Style::default().fg(Color::Green)
        };

        let confirm_button = Paragraph::new(Line::from(vec![
            Span::styled(&self.confirm_text, confirm_style)
        ]))
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);
        frame.render_widget(confirm_button, button_layout[0]);

        // Cancel button
        let cancel_style = if self.selected_button == 1 {
            Style::default().bg(Color::Red).fg(Color::Black)
        } else {
            Style::default().fg(Color::Red)
        };

        let cancel_button = Paragraph::new(Line::from(vec![
            Span::styled(&self.cancel_text, cancel_style)
        ]))
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);
        frame.render_widget(cancel_button, button_layout[1]);
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> AppResult<ModalResult> {
        if !self.is_open {
            return Ok(ModalResult::None);
        }

        match key.code {
            KeyCode::Left | KeyCode::Char('h') => {
                self.selected_button = 0;
                Ok(ModalResult::None)
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.selected_button = 1;
                Ok(ModalResult::None)
            }
            KeyCode::Char(' ') => {
                self.selected_button = if self.selected_button == 0 { 1 } else { 0 };
                Ok(ModalResult::None)
            }
            KeyCode::Enter => {
                self.is_open = false;
                if self.selected_button == 0 {
                    Ok(ModalResult::Confirmed)
                } else {
                    Ok(ModalResult::Cancelled)
                }
            }
            KeyCode::Esc => {
                self.is_open = false;
                Ok(ModalResult::Cancelled)
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.is_open = false;
                Ok(ModalResult::Confirmed)
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.is_open = false;
                Ok(ModalResult::Cancelled)
            }
            _ => Ok(ModalResult::None),
        }
    }

    fn is_open(&self) -> bool {
        self.is_open
    }

    fn close(&mut self) {
        self.is_open = false;
    }
}
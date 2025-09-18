//! Progress modal component

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Gauge, Paragraph},
    Frame,
};

use crate::{error::AppResult, ui::theme::Theme};

use super::{Modal, ModalResult};

/// Progress modal for long-running operations
pub struct ProgressModal {
    is_open: bool,
    title: String,
    message: String,
    progress: u16, // 0-100
    is_indeterminate: bool,
    can_cancel: bool,
}

impl ProgressModal {
    pub fn new() -> Self {
        Self {
            is_open: false,
            title: String::new(),
            message: String::new(),
            progress: 0,
            is_indeterminate: false,
            can_cancel: false,
        }
    }

    /// Open with determinate progress
    pub fn open_determinate(&mut self, title: &str, message: &str, can_cancel: bool) {
        self.is_open = true;
        self.title = title.to_string();
        self.message = message.to_string();
        self.progress = 0;
        self.is_indeterminate = false;
        self.can_cancel = can_cancel;
    }

    /// Open with indeterminate progress
    pub fn open_indeterminate(&mut self, title: &str, message: &str, can_cancel: bool) {
        self.is_open = true;
        self.title = title.to_string();
        self.message = message.to_string();
        self.progress = 0;
        self.is_indeterminate = true;
        self.can_cancel = can_cancel;
    }

    /// Update progress (0-100)
    pub fn set_progress(&mut self, progress: u16) {
        self.progress = progress.min(100);
    }

    /// Update message
    pub fn set_message(&mut self, message: &str) {
        self.message = message.to_string();
    }

    /// Complete the operation and close modal
    pub fn complete(&mut self) {
        self.is_open = false;
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

impl Modal for ProgressModal {
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.is_open {
            return;
        }

        let modal_area = self.centered_rect(60, 20, area);

        // Clear the background
        frame.render_widget(Clear, modal_area);

        // Modal layout: title + message + progress bar + help
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),    // Title
                Constraint::Length(3),    // Message
                Constraint::Length(3),    // Progress bar
                Constraint::Length(2),    // Help text
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
            .alignment(Alignment::Center);
        frame.render_widget(message, chunks[1]);

        // Progress bar
        let progress = if self.is_indeterminate {
            // For indeterminate progress, show a pulsing effect
            // This is simplified - in a real implementation you might animate this
            50
        } else {
            self.progress
        };

        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Blue))
            .percent(progress)
            .label(if self.is_indeterminate {
                "Working...".to_string()
            } else {
                format!("{}%", self.progress)
            });
        frame.render_widget(gauge, chunks[2]);

        // Help text
        let help_text = if self.can_cancel {
            "Esc: Cancel"
        } else {
            "Please wait..."
        };

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
            KeyCode::Esc if self.can_cancel => {
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
//! Modal components for user interaction
//!
//! Provides various modal dialogs for user confirmation, input, and information display.

pub mod confirmation;
pub mod input;
pub mod progress;

pub use confirmation::ConfirmationModal;
pub use input::InputModal;
pub use progress::ProgressModal;

use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, Frame};

use crate::{app::state::AppState, error::AppResult, ui::theme::Theme};

/// Trait for modal components
pub trait Modal {
    /// Render the modal
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme);

    /// Handle key events
    fn handle_key_event(&mut self, key: KeyEvent) -> AppResult<ModalResult>;

    /// Check if modal is open
    fn is_open(&self) -> bool;

    /// Close the modal
    fn close(&mut self);
}

/// Result from modal interaction
#[derive(Debug, Clone, PartialEq)]
pub enum ModalResult {
    /// No action taken
    None,
    /// User confirmed the action
    Confirmed,
    /// User cancelled the action
    Cancelled,
    /// User provided input
    Input(String),
    /// Modal was closed
    Closed,
}
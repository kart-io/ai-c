//! UI Components module
//!
//! Contains all reusable UI components for the TUI interface.

pub mod sidebar;
pub mod status_bar;
pub mod tabs;

pub use sidebar::SidebarComponent;
pub use status_bar::StatusBarComponent;
pub use tabs::*;

use crossterm::event::KeyEvent;
use ratatui::{backend::Backend, layout::Rect, Frame};

use crate::{app::state::AppState, error::AppResult, ui::theme::Theme};

/// Trait for UI components that can render and handle events
pub trait Component {
    /// Render the component
    fn render(&mut self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme);

    /// Handle key events
    fn handle_key_event(&mut self, key: KeyEvent, state: &mut AppState) -> AppResult<()>;
}

//! UI Components module
//!
//! Contains all reusable UI components for the TUI interface.

pub mod sidebar;
pub mod status_bar;
pub mod tabs;
pub mod diff_viewer;
pub mod modals;
pub mod agent_panel;
pub mod ai_suggestions;
pub mod search;
pub mod filter;
pub mod global_search;
pub mod commit_history;
pub mod context_menu;
pub mod context_manager;
pub mod shortcuts;
pub mod search_cache;
pub mod search_index;
pub mod git_operations;
pub mod agent_manager;
pub mod help;

pub use sidebar::SidebarComponent;
pub use status_bar::StatusBarComponent;
pub use tabs::*;
pub use diff_viewer::DiffViewerComponent;
pub use modals::*;
pub use agent_panel::AgentPanelComponent;
pub use ai_suggestions::AISuggestionsComponent;
pub use search::SearchComponent;
pub use filter::FilterComponent;
pub use global_search::GlobalSearchManager;
pub use commit_history::CommitHistoryComponent;
pub use context_menu::{ContextMenuComponent, MenuItem, MenuAction};
pub use context_manager::{ContextMenuManager, ContextInfo, ContextType};
pub use shortcuts::{ShortcutsManager, Shortcut, ShortcutAction, ShortcutContext};
pub use search_cache::{SearchCache, CacheKey, CacheStats};
pub use search_index::{SearchIndex, SearchMatch, IndexStats};
pub use git_operations::{GitOperationsComponent, GitOperation};
pub use agent_manager::{AgentManagerComponent, AgentInfo, AgentStatus};
pub use help::{HelpComponent, HelpCategory};

use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, Frame};

use crate::{app::state::AppState, error::AppResult, ui::theme::Theme};

/// Trait for UI components that can render and handle events
pub trait Component {
    /// Render the component
    fn render(&mut self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme);

    /// Handle key events
    fn handle_key_event(&mut self, key: KeyEvent, state: &mut AppState) -> AppResult<()>;
}

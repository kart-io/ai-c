//! Help system component
//!
//! Provides comprehensive help information for all application features.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs, Wrap},
    Frame,
};
use tracing::debug;

use crate::{
    app::state::AppState,
    error::AppResult,
    ui::{
        theme::Theme,
        keyboard::{ActionKey, NavigationKey, ShortcutManager},
    },
};

/// Help system component
pub struct HelpComponent {
    /// Current help category
    current_category: HelpCategory,
    /// Whether the help overlay is visible
    is_visible: bool,
    /// Shortcut manager for consistent key handling
    shortcut_manager: ShortcutManager,
    /// Selected help item index
    selected_index: usize,
}

/// Help categories
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HelpCategory {
    General,
    Navigation,
    Branches,
    Tags,
    Stash,
    GitFlow,
    Shortcuts,
}

impl HelpCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            HelpCategory::General => "General",
            HelpCategory::Navigation => "Navigation",
            HelpCategory::Branches => "Branches",
            HelpCategory::Tags => "Tags",
            HelpCategory::Stash => "Stash",
            HelpCategory::GitFlow => "GitFlow",
            HelpCategory::Shortcuts => "Shortcuts",
        }
    }

    pub fn all() -> Vec<HelpCategory> {
        vec![
            HelpCategory::General,
            HelpCategory::Navigation,
            HelpCategory::Branches,
            HelpCategory::Tags,
            HelpCategory::Stash,
            HelpCategory::GitFlow,
            HelpCategory::Shortcuts,
        ]
    }
}

impl HelpComponent {
    pub fn new() -> Self {
        Self {
            current_category: HelpCategory::General,
            is_visible: false,
            shortcut_manager: ShortcutManager::new(),
            selected_index: 0,
        }
    }

    pub fn is_visible(&self) -> bool {
        self.is_visible
    }

    pub fn show(&mut self) {
        self.is_visible = true;
        debug!("Help system opened");
    }

    pub fn hide(&mut self) {
        self.is_visible = false;
        debug!("Help system closed");
    }

    pub fn toggle(&mut self) {
        self.is_visible = !self.is_visible;
        debug!("Help system toggled: {}", self.is_visible);
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, _state: &AppState, theme: &Theme) {
        if !self.is_visible {
            return;
        }

        // Create centered overlay
        let overlay_area = Self::centered_rect(80, 90, area);

        // Clear the background
        frame.render_widget(Clear, overlay_area);

        // Create main layout: tabs + content
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tabs
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Footer
            ])
            .split(overlay_area);

        // Render tabs
        self.render_tabs(frame, main_layout[0], theme);

        // Render content based on selected category
        self.render_content(frame, main_layout[1], theme);

        // Render footer with navigation help
        self.render_footer(frame, main_layout[2], theme);
    }

    fn render_tabs(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let categories = HelpCategory::all();
        let tab_titles: Vec<Line> = categories
            .iter()
            .map(|cat| Line::from(cat.as_str()))
            .collect();

        let selected_index = categories
            .iter()
            .position(|&cat| cat == self.current_category)
            .unwrap_or(0);

        let tabs = Tabs::new(tab_titles)
            .block(
                Block::default()
                    .title("Help - Press ? to close")
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            )
            .style(theme.text_style())
            .highlight_style(theme.highlight_style())
            .select(selected_index);

        frame.render_widget(tabs, area);
    }

    fn render_content(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let content = self.get_content_for_category(self.current_category);

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            )
            .style(theme.text_style())
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let footer_text = "Navigation: ←→/hl: Switch tabs | ↑↓/kj: Scroll | Esc/?: Close";

        let footer = Paragraph::new(footer_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            )
            .style(theme.text_style());

        frame.render_widget(footer, area);
    }

    fn get_content_for_category(&self, category: HelpCategory) -> String {
        match category {
            HelpCategory::General => {
                "GENERAL HELP\n\n\
                Welcome to AI-C TUI - An intelligent Git commit tool!\n\n\
                • Tab navigation: Use numbers 1-7 or Tab/Shift+Tab to switch between views\n\
                • Universal shortcuts: ? for help, Esc to exit dialogs\n\
                • All interfaces support vim-style navigation (hjkl) and arrow keys\n\
                • Most operations are performed with single key presses\n\
                • Status information is always shown at the bottom of the screen\n\n\
                MAIN SECTIONS:\n\
                1. Branches - Manage local and remote branches\n\
                2. Tags - Create, view, and manage Git tags\n\
                3. Stash - Save and restore working directory changes\n\
                4. Status - View current repository status and stage changes\n\
                5. Remotes - Manage remote repositories\n\
                6. History - Browse commit history and diffs\n\
                7. GitFlow - Git Flow workflow management\n\n\
                TIP: Each section has context-specific help available in the Shortcuts tab.".to_string()
            }
            HelpCategory::Navigation => {
                "NAVIGATION SHORTCUTS\n\n\
                UNIVERSAL MOVEMENT:\n\
                • ↑↓/kj: Move up/down in lists\n\
                • ←→/hl: Move left/right, switch tabs\n\
                • PgUp/PgDn: Quick scroll (5 items at a time)\n\
                • Home/End: Jump to start/end of list\n\
                • Space: Switch focus between panels\n\n\
                TAB NAVIGATION:\n\
                • 1-7: Jump directly to specific tabs\n\
                • Tab: Next tab\n\
                • Shift+Tab: Previous tab\n\n\
                UNIVERSAL ACTIONS:\n\
                • Enter: Confirm/Select current item\n\
                • Esc: Cancel/Back/Exit dialogs\n\
                • r: Refresh current view\n\
                • ?: Show help (this screen)\n\
                \n\
                SEARCH:\n\
                • /: Open search in current view\n\
                • Ctrl+F: Global search across all content".to_string()
            }
            HelpCategory::Branches => {
                "BRANCH MANAGEMENT\n\n\
                VIEWING BRANCHES:\n\
                • Enter: Checkout/switch to selected branch\n\
                • s: Show branch details and commit info\n\
                • d: Delete selected branch (with confirmation)\n\
                • n: Create new branch from current HEAD\n\
                • r: Refresh branch list\n\n\
                ADVANCED OPERATIONS:\n\
                • m: Merge selected branch into current branch\n\
                • p: Push branch to remote (if tracking is set up)\n\
                • l: Select specific commit on branch\n\n\
                BRANCH TYPES:\n\
                • Local branches: Branches that exist in your local repository\n\
                • Remote branches: Read-only references to remote repository state\n\
                • Tracking branches: Local branches that follow remote branches\n\n\
                SAFETY FEATURES:\n\
                • Cannot delete current branch\n\
                • Confirmation required for destructive operations\n\
                • Clear indication of current branch with highlighting\n\n\
                TIP: Branch names are color-coded to show their status and type.".to_string()
            }
            HelpCategory::Tags => {
                "TAG MANAGEMENT\n\n\
                VIEWING TAGS:\n\
                • Enter: Show detailed tag information\n\
                • s: Show tag details and target commit\n\
                • d: Delete selected tag (with confirmation)\n\
                • n: Create new tag at current HEAD\n\
                • r: Refresh tag list\n\n\
                TAG TYPES:\n\
                • Lightweight tags: Simple pointers to commits\n\
                • Annotated tags: Include metadata (author, date, message)\n\
                • Signed tags: Cryptographically signed for verification\n\n\
                TAG CREATION:\n\
                • When creating tags, you'll be prompted for:\n\
                  - Tag name (required)\n\
                  - Tag message (optional, creates annotated tag)\n\
                  - Target commit (defaults to current HEAD)\n\n\
                BEST PRACTICES:\n\
                • Use semantic versioning (v1.0.0, v2.1.3)\n\
                • Create annotated tags for releases\n\
                • Include meaningful messages for annotated tags\n\
                • Tag stable points in your project history\n\n\
                TIP: Tags are displayed in reverse chronological order.".to_string()
            }
            HelpCategory::Stash => {
                "STASH MANAGEMENT\n\n\
                STASH OPERATIONS:\n\
                • Enter: Apply selected stash (keeps stash in list)\n\
                • a: Apply selected stash (same as Enter)\n\
                • d: Drop (delete) selected stash permanently\n\
                • n: Create new stash from current working directory\n\
                • s: Show stash diff (what changes are included)\n\
                • r: Refresh stash list\n\n\
                STASH CREATION:\n\
                • Saves current working directory and index state\n\
                • Includes both staged and unstaged changes\n\
                • You'll be prompted for a stash message\n\
                • Working directory will be cleaned after stashing\n\n\
                STASH APPLICATION:\n\
                • Apply: Restores changes but keeps stash in list\n\
                • Pop: Restores changes and removes stash from list\n\
                • Can apply stashes to different branches\n\
                • May require conflict resolution\n\n\
                USE CASES:\n\
                • Save work in progress before switching branches\n\
                • Temporarily clean working directory\n\
                • Share changes between branches\n\
                • Backup uncommitted changes".to_string()
            }
            HelpCategory::GitFlow => {
                "GITFLOW WORKFLOW\n\n\
                BRANCH TYPES:\n\
                • Feature: New functionality development (feature/name)\n\
                • Release: Release preparation (release/version)\n\
                • Hotfix: Critical bug fixes (hotfix/name)\n\
                • Support: Long-term maintenance (support/version)\n\n\
                OPERATIONS:\n\
                • Enter: List existing branches of selected type\n\
                • n: Create new branch of selected type\n\
                • f: Finish current branch (merge and cleanup)\n\
                • l: Select specific branch to work with\n\
                • r: Refresh GitFlow status\n\n\
                WORKFLOW:\n\
                Feature branches:\n\
                  - Created from 'develop'\n\
                  - Merged back to 'develop'\n\
                \n\
                Release branches:\n\
                  - Created from 'develop'\n\
                  - Merged to both 'main' and 'develop'\n\
                  - Tagged on 'main'\n\
                \n\
                Hotfix branches:\n\
                  - Created from 'main'\n\
                  - Merged to both 'main' and 'develop'\n\
                  - Tagged on 'main'\n\
                \n\
                Support branches:\n\
                  - Created from specific points in 'main'\n\
                  - Never merged back\n\n\
                TIP: Ensure you have 'main' and 'develop' branches before using GitFlow.".to_string()
            }
            HelpCategory::Shortcuts => {
                "KEYBOARD SHORTCUTS REFERENCE\n\n\
                GLOBAL SHORTCUTS (work everywhere):\n\
                • ?: Show/hide help\n\
                • Esc: Cancel, back, or close dialogs\n\
                • Tab/Shift+Tab: Navigate between tabs\n\
                • 1-7: Jump to specific tab\n\
                • Ctrl+C: Quit application\n\
                • r: Refresh current view\n\n\
                NAVIGATION (all list views):\n\
                • ↑↓ or kj: Move selection up/down\n\
                • ←→ or hl: Move left/right, switch panels\n\
                • PgUp/PgDn: Scroll by page\n\
                • Home/End: Jump to start/end\n\
                • Space: Switch focus between panels\n\n\
                COMMON ACTIONS:\n\
                • Enter: Select/confirm/execute\n\
                • d: Delete item\n\
                • n: Create new item\n\
                • s: Show details/diff\n\
                • l: Select line/item\n\
                • a: Apply changes (stash, patches)\n\
                • m: Merge (branches)\n\
                • p: Push (branches)\n\
                • f: Finish (GitFlow)\n\n\
                SEARCH & FILTER:\n\
                • /: Search in current view\n\
                • Ctrl+F: Global search\n\
                • Ctrl+G: Filter current view\n\n\
                VIM USERS:\n\
                All vim navigation keys (hjkl) are supported throughout the interface.".to_string()
            }
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent, _state: &mut AppState) -> AppResult<bool> {
        if !self.is_visible {
            return Ok(false);
        }

        // Handle help-specific keys
        match key.code {
            KeyCode::Char('?') | KeyCode::Esc => {
                self.hide();
                return Ok(true);
            }
            KeyCode::Left | KeyCode::Char('h') => {
                let categories = HelpCategory::all();
                let current_index = categories
                    .iter()
                    .position(|&cat| cat == self.current_category)
                    .unwrap_or(0);
                if current_index > 0 {
                    self.current_category = categories[current_index - 1];
                } else {
                    self.current_category = categories[categories.len() - 1];
                }
                return Ok(true);
            }
            KeyCode::Right | KeyCode::Char('l') => {
                let categories = HelpCategory::all();
                let current_index = categories
                    .iter()
                    .position(|&cat| cat == self.current_category)
                    .unwrap_or(0);
                self.current_category = categories[(current_index + 1) % categories.len()];
                return Ok(true);
            }
            KeyCode::Char('1') => self.current_category = HelpCategory::General,
            KeyCode::Char('2') => self.current_category = HelpCategory::Navigation,
            KeyCode::Char('3') => self.current_category = HelpCategory::Branches,
            KeyCode::Char('4') => self.current_category = HelpCategory::Tags,
            KeyCode::Char('5') => self.current_category = HelpCategory::Stash,
            KeyCode::Char('6') => self.current_category = HelpCategory::GitFlow,
            KeyCode::Char('7') => self.current_category = HelpCategory::Shortcuts,
            _ => return Ok(false),
        }

        Ok(true)
    }

    fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

impl Default for HelpComponent {
    fn default() -> Self {
        Self::new()
    }
}
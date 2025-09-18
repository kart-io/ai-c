//! Context menu system for right-click actions

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};
use std::collections::HashMap;

use crate::{
    app::state::AppState,
    error::AppResult,
    ui::theme::Theme,
};

use super::Component;

/// Context menu item
#[derive(Debug, Clone)]
pub struct MenuItem {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
    pub hotkey: Option<String>,
    pub enabled: bool,
    pub icon: Option<String>,
    pub action: MenuAction,
    pub submenu: Option<Vec<MenuItem>>,
}

/// Context menu actions
#[derive(Debug, Clone)]
pub enum MenuAction {
    // Git operations
    GitAdd,
    GitRemove,
    GitDiscard,
    GitStage,
    GitUnstage,
    GitCommit,
    GitDiff,
    GitBlame,
    GitHistory,

    // File operations
    FileOpen,
    FileEdit,
    FileDelete,
    FileRename,
    FileCopy,
    FileProperties,

    // Branch operations
    BranchCheckout,
    BranchMerge,
    BranchDelete,
    BranchRename,
    BranchPush,
    BranchPull,

    // Commit operations
    CommitShow,
    CommitCherryPick,
    CommitRevert,
    CommitReset,
    CommitAmend,

    // Search operations
    SearchInFiles,
    SearchCommits,
    SearchBranches,

    // Custom action with data
    Custom(String),

    // Separator (no action)
    Separator,
}

impl MenuItem {
    pub fn new(id: &str, label: &str, action: MenuAction) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            description: None,
            hotkey: None,
            enabled: true,
            icon: None,
            action,
            submenu: None,
        }
    }

    pub fn with_description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    pub fn with_hotkey(mut self, hotkey: &str) -> Self {
        self.hotkey = Some(hotkey.to_string());
        self
    }

    pub fn with_icon(mut self, icon: &str) -> Self {
        self.icon = Some(icon.to_string());
        self
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub fn with_submenu(mut self, submenu: Vec<MenuItem>) -> Self {
        self.submenu = Some(submenu);
        self
    }

    pub fn separator(id: &str) -> Self {
        Self {
            id: id.to_string(),
            label: "─".repeat(20),
            description: None,
            hotkey: None,
            enabled: false,
            icon: None,
            action: MenuAction::Separator,
            submenu: None,
        }
    }
}

/// Context menu component
pub struct ContextMenuComponent {
    is_open: bool,
    position: (u16, u16),
    items: Vec<MenuItem>,
    selected_index: usize,
    list_state: ListState,
    context_data: HashMap<String, String>,
    menu_stack: Vec<(Vec<MenuItem>, usize)>, // For submenu navigation
}

impl ContextMenuComponent {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            is_open: false,
            position: (0, 0),
            items: Vec::new(),
            selected_index: 0,
            list_state,
            context_data: HashMap::new(),
            menu_stack: Vec::new(),
        }
    }

    /// Open context menu at position with items
    pub fn open(&mut self, position: (u16, u16), items: Vec<MenuItem>) {
        self.is_open = true;
        self.position = position;
        self.items = items;
        self.selected_index = 0;
        self.list_state.select(Some(0));
        self.menu_stack.clear();
        self.find_first_enabled_item();
    }

    /// Close context menu
    pub fn close(&mut self) {
        self.is_open = false;
        self.items.clear();
        self.context_data.clear();
        self.menu_stack.clear();
    }

    /// Check if context menu is open
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    /// Set context data for menu actions
    pub fn set_context_data(&mut self, data: HashMap<String, String>) {
        self.context_data = data;
    }

    /// Get context data
    pub fn get_context_data(&self) -> &HashMap<String, String> {
        &self.context_data
    }

    /// Handle mouse click to open context menu
    pub fn handle_mouse_event(&mut self, mouse: MouseEvent, area: Rect) -> AppResult<Option<MenuAction>> {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Right) => {
                // Open context menu at mouse position
                let position = (mouse.column, mouse.row);

                // Get context-appropriate menu items
                let items = self.get_context_menu_items_for_position(position, area);

                if !items.is_empty() {
                    self.open(position, items);
                }
                Ok(None)
            }
            MouseEventKind::Down(MouseButton::Left) if self.is_open => {
                // Check if click is on menu
                let menu_area = self.calculate_menu_area(area);

                if mouse.column >= menu_area.x && mouse.column < menu_area.x + menu_area.width &&
                   mouse.row >= menu_area.y && mouse.row < menu_area.y + menu_area.height {
                    // Click on menu - select item
                    let item_index = (mouse.row - menu_area.y) as usize;
                    if item_index < self.items.len() {
                        self.selected_index = item_index;
                        self.list_state.select(Some(item_index));
                        return self.execute_selected_action();
                    }
                } else {
                    // Click outside menu - close it
                    self.close();
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    /// Get context menu items based on position and current state
    fn get_context_menu_items_for_position(&self, position: (u16, u16), area: Rect) -> Vec<MenuItem> {
        // This would be implemented based on what's under the cursor
        // For now, return a default set of items
        self.get_default_context_menu()
    }

    /// Get default context menu items
    fn get_default_context_menu(&self) -> Vec<MenuItem> {
        vec![
            MenuItem::new("git_status", "Git Status", MenuAction::Custom("git_status".to_string()))
                .with_icon("📋")
                .with_hotkey("S"),

            MenuItem::new("git_diff", "Show Diff", MenuAction::GitDiff)
                .with_icon("📄")
                .with_hotkey("D"),

            MenuItem::separator("sep1"),

            MenuItem::new("search", "Search", MenuAction::Custom("search".to_string()))
                .with_icon("🔍")
                .with_hotkey("Ctrl+F")
                .with_submenu(vec![
                    MenuItem::new("search_files", "Search Files", MenuAction::SearchInFiles),
                    MenuItem::new("search_commits", "Search Commits", MenuAction::SearchCommits),
                    MenuItem::new("search_branches", "Search Branches", MenuAction::SearchBranches),
                ]),

            MenuItem::separator("sep2"),

            MenuItem::new("git_operations", "Git Operations", MenuAction::Custom("git_ops".to_string()))
                .with_icon("🌿")
                .with_submenu(vec![
                    MenuItem::new("git_add", "Add", MenuAction::GitAdd),
                    MenuItem::new("git_commit", "Commit", MenuAction::GitCommit),
                    MenuItem::new("git_push", "Push", MenuAction::BranchPush),
                    MenuItem::new("git_pull", "Pull", MenuAction::BranchPull),
                ]),
        ]
    }

    /// Get file-specific context menu
    pub fn get_file_context_menu(&self, file_path: &str) -> Vec<MenuItem> {
        vec![
            MenuItem::new("file_open", "Open", MenuAction::FileOpen)
                .with_icon("📂")
                .with_hotkey("Enter"),

            MenuItem::new("file_edit", "Edit", MenuAction::FileEdit)
                .with_icon("✏️")
                .with_hotkey("E"),

            MenuItem::separator("sep1"),

            MenuItem::new("git_add", "Stage File", MenuAction::GitAdd)
                .with_icon("➕"),

            MenuItem::new("git_discard", "Discard Changes", MenuAction::GitDiscard)
                .with_icon("🗑️"),

            MenuItem::new("git_diff", "Show Diff", MenuAction::GitDiff)
                .with_icon("📄"),

            MenuItem::new("git_blame", "Git Blame", MenuAction::GitBlame)
                .with_icon("👤"),

            MenuItem::new("git_history", "File History", MenuAction::GitHistory)
                .with_icon("📜"),

            MenuItem::separator("sep2"),

            MenuItem::new("file_copy", "Copy Path", MenuAction::FileCopy)
                .with_icon("📋")
                .with_hotkey("Ctrl+C"),

            MenuItem::new("file_rename", "Rename", MenuAction::FileRename)
                .with_icon("✏️")
                .with_hotkey("F2"),

            MenuItem::new("file_delete", "Delete", MenuAction::FileDelete)
                .with_icon("🗑️")
                .with_hotkey("Del"),

            MenuItem::separator("sep3"),

            MenuItem::new("file_properties", "Properties", MenuAction::FileProperties)
                .with_icon("ℹ️")
                .with_hotkey("Alt+Enter"),
        ]
    }

    /// Get commit-specific context menu
    pub fn get_commit_context_menu(&self, commit_hash: &str) -> Vec<MenuItem> {
        vec![
            MenuItem::new("commit_show", "Show Commit", MenuAction::CommitShow)
                .with_icon("👁️")
                .with_hotkey("Enter"),

            MenuItem::new("commit_diff", "Show Diff", MenuAction::GitDiff)
                .with_icon("📄"),

            MenuItem::separator("sep1"),

            MenuItem::new("commit_cherry_pick", "Cherry Pick", MenuAction::CommitCherryPick)
                .with_icon("🍒"),

            MenuItem::new("commit_revert", "Revert", MenuAction::CommitRevert)
                .with_icon("↩️"),

            MenuItem::new("commit_reset", "Reset to Here", MenuAction::CommitReset)
                .with_icon("🔄")
                .with_submenu(vec![
                    MenuItem::new("reset_soft", "Soft Reset", MenuAction::Custom("reset_soft".to_string())),
                    MenuItem::new("reset_mixed", "Mixed Reset", MenuAction::Custom("reset_mixed".to_string())),
                    MenuItem::new("reset_hard", "Hard Reset", MenuAction::Custom("reset_hard".to_string())),
                ]),

            MenuItem::separator("sep2"),

            MenuItem::new("commit_amend", "Amend", MenuAction::CommitAmend)
                .with_icon("✏️"),
        ]
    }

    /// Get branch-specific context menu
    pub fn get_branch_context_menu(&self, branch_name: &str, is_current: bool) -> Vec<MenuItem> {
        let mut items = vec![
            MenuItem::new("branch_checkout", "Checkout", MenuAction::BranchCheckout)
                .with_icon("🔄")
                .with_hotkey("Enter"),
        ];

        if !is_current {
            items.extend(vec![
                MenuItem::new("branch_merge", "Merge", MenuAction::BranchMerge)
                    .with_icon("🔀"),

                MenuItem::separator("sep1"),

                MenuItem::new("branch_delete", "Delete", MenuAction::BranchDelete)
                    .with_icon("🗑️")
                    .with_hotkey("Del"),
            ]);
        }

        items.extend(vec![
            MenuItem::new("branch_rename", "Rename", MenuAction::BranchRename)
                .with_icon("✏️")
                .with_hotkey("F2"),

            MenuItem::separator("sep2"),

            MenuItem::new("branch_push", "Push", MenuAction::BranchPush)
                .with_icon("⬆️"),

            MenuItem::new("branch_pull", "Pull", MenuAction::BranchPull)
                .with_icon("⬇️"),
        ]);

        items
    }

    /// Move selection up
    pub fn move_up(&mut self) {
        let len = self.items.len();
        if len == 0 {
            return;
        }

        loop {
            self.selected_index = if self.selected_index == 0 {
                len - 1
            } else {
                self.selected_index - 1
            };

            // Skip disabled items and separators
            if self.items[self.selected_index].enabled &&
               !matches!(self.items[self.selected_index].action, MenuAction::Separator) {
                break;
            }

            // Prevent infinite loop
            if self.selected_index == self.list_state.selected().unwrap_or(0) {
                break;
            }
        }

        self.list_state.select(Some(self.selected_index));
    }

    /// Move selection down
    pub fn move_down(&mut self) {
        let len = self.items.len();
        if len == 0 {
            return;
        }

        loop {
            self.selected_index = (self.selected_index + 1) % len;

            // Skip disabled items and separators
            if self.items[self.selected_index].enabled &&
               !matches!(self.items[self.selected_index].action, MenuAction::Separator) {
                break;
            }

            // Prevent infinite loop
            if self.selected_index == self.list_state.selected().unwrap_or(0) {
                break;
            }
        }

        self.list_state.select(Some(self.selected_index));
    }

    /// Enter submenu or execute action
    pub fn enter_or_execute(&mut self) -> AppResult<Option<MenuAction>> {
        if self.selected_index < self.items.len() {
            let item = &self.items[self.selected_index];

            if let Some(ref submenu) = item.submenu {
                // Enter submenu
                self.menu_stack.push((self.items.clone(), self.selected_index));
                self.items = submenu.clone();
                self.selected_index = 0;
                self.list_state.select(Some(0));
                self.find_first_enabled_item();
                Ok(None)
            } else {
                // Execute action
                self.execute_selected_action()
            }
        } else {
            Ok(None)
        }
    }

    /// Go back to parent menu
    pub fn go_back(&mut self) -> bool {
        if let Some((parent_items, parent_index)) = self.menu_stack.pop() {
            self.items = parent_items;
            self.selected_index = parent_index;
            self.list_state.select(Some(parent_index));
            true
        } else {
            false
        }
    }

    /// Execute selected action
    fn execute_selected_action(&mut self) -> AppResult<Option<MenuAction>> {
        if self.selected_index < self.items.len() {
            let action = self.items[self.selected_index].action.clone();
            self.close();
            Ok(Some(action))
        } else {
            Ok(None)
        }
    }

    /// Find first enabled item and select it
    fn find_first_enabled_item(&mut self) {
        for (index, item) in self.items.iter().enumerate() {
            if item.enabled && !matches!(item.action, MenuAction::Separator) {
                self.selected_index = index;
                self.list_state.select(Some(index));
                break;
            }
        }
    }

    /// Calculate menu area based on position and content
    fn calculate_menu_area(&self, container_area: Rect) -> Rect {
        let width = self.calculate_menu_width();
        let height = self.items.len() as u16 + 2; // +2 for borders

        let x = if self.position.0 + width > container_area.x + container_area.width {
            container_area.x + container_area.width - width
        } else {
            self.position.0
        }.max(container_area.x);

        let y = if self.position.1 + height > container_area.y + container_area.height {
            if self.position.1 > height {
                self.position.1 - height
            } else {
                container_area.y + container_area.height - height
            }
        } else {
            self.position.1
        }.max(container_area.y);

        Rect::new(x, y, width, height)
    }

    /// Calculate menu width based on content
    fn calculate_menu_width(&self) -> u16 {
        let mut max_width = 20; // Minimum width

        for item in &self.items {
            let mut item_width = item.label.len();

            if let Some(ref icon) = item.icon {
                item_width += icon.len() + 1; // +1 for space
            }

            if let Some(ref hotkey) = item.hotkey {
                item_width += hotkey.len() + 3; // +3 for " | "
            }

            if item.submenu.is_some() {
                item_width += 2; // " ▶"
            }

            max_width = max_width.max(item_width + 4); // +4 for padding
        }

        max_width as u16
    }
}

impl Component for ContextMenuComponent {
    fn render(&mut self, frame: &mut Frame, area: Rect, _state: &AppState, theme: &Theme) {
        if !self.is_open {
            return;
        }

        let menu_area = self.calculate_menu_area(area);

        // Clear background
        frame.render_widget(Clear, menu_area);

        // Render menu items
        let items: Vec<ListItem> = self.items.iter().map(|item| {
            if matches!(item.action, MenuAction::Separator) {
                ListItem::new(Line::from("─".repeat(menu_area.width as usize - 2)))
                    .style(Style::default().fg(Color::DarkGray))
            } else {
                let mut spans = Vec::new();

                // Icon
                if let Some(ref icon) = item.icon {
                    spans.push(Span::raw(format!("{} ", icon)));
                }

                // Label
                let label_style = if item.enabled {
                    Style::default()
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                spans.push(Span::styled(&item.label, label_style));

                // Hotkey
                if let Some(ref hotkey) = item.hotkey {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(
                        hotkey,
                        Style::default().fg(Color::DarkGray)
                    ));
                }

                // Submenu indicator
                if item.submenu.is_some() {
                    spans.push(Span::raw(" ▶"));
                }

                ListItem::new(Line::from(spans))
            }
        }).collect();

        let menu_list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Gray))
            )
            .highlight_style(
                Style::default()
                    .bg(theme.selection_color())
                    .add_modifier(Modifier::BOLD)
            )
            .highlight_symbol("");

        frame.render_stateful_widget(menu_list, menu_area, &mut self.list_state);
    }

    fn handle_key_event(&mut self, key: KeyEvent, _state: &mut AppState) -> AppResult<()> {
        if !self.is_open {
            return Ok(());
        }

        match key.code {
            KeyCode::Esc => {
                if !self.go_back() {
                    self.close();
                }
            }
            KeyCode::Up => self.move_up(),
            KeyCode::Down => self.move_down(),
            KeyCode::Enter => {
                let _ = self.enter_or_execute()?;
            }
            KeyCode::Left => {
                self.go_back();
            }
            KeyCode::Right => {
                let _ = self.enter_or_execute()?;
            }
            _ => {}
        }

        Ok(())
    }
}
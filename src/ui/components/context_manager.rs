//! Context menu manager for coordinating context-sensitive menus

use crossterm::event::{MouseEvent, KeyEvent};
use ratatui::{layout::Rect, Frame};
use std::collections::HashMap;

use crate::{
    app::state::AppState,
    error::AppResult,
    ui::theme::Theme,
};

use super::{Component, ContextMenuComponent, MenuItem, MenuAction};

/// Context types for determining appropriate menus
#[derive(Debug, Clone, PartialEq)]
pub enum ContextType {
    File,
    Commit,
    Branch,
    Tag,
    Remote,
    Stash,
    Empty,
    Search,
    Diff,
}

/// Context information for menu generation
#[derive(Debug, Clone)]
pub struct ContextInfo {
    pub context_type: ContextType,
    pub data: HashMap<String, String>,
    pub position: (u16, u16),
}

impl ContextInfo {
    pub fn new(context_type: ContextType, position: (u16, u16)) -> Self {
        Self {
            context_type,
            data: HashMap::new(),
            position,
        }
    }

    pub fn with_data(mut self, key: &str, value: &str) -> Self {
        self.data.insert(key.to_string(), value.to_string());
        self
    }

    pub fn get_data(&self, key: &str) -> Option<&String> {
        self.data.get(key)
    }
}

/// Context menu manager
pub struct ContextMenuManager {
    menu_component: ContextMenuComponent,
    current_context: Option<ContextInfo>,
    last_action: Option<MenuAction>,
}

impl ContextMenuManager {
    pub fn new() -> Self {
        Self {
            menu_component: ContextMenuComponent::new(),
            current_context: None,
            last_action: None,
        }
    }

    /// Show context menu for specific context
    pub fn show_context_menu(&mut self, context: ContextInfo) -> AppResult<()> {
        let items = self.generate_menu_items(&context);

        if !items.is_empty() {
            self.menu_component.open(context.position, items);

            // Set context data for the menu
            self.menu_component.set_context_data(context.data.clone());
            self.current_context = Some(context);
        }

        Ok(())
    }

    /// Generate menu items based on context
    fn generate_menu_items(&self, context: &ContextInfo) -> Vec<MenuItem> {
        match context.context_type {
            ContextType::File => {
                let default_path = String::new();
                let file_path = context.get_data("file_path").unwrap_or(&default_path);
                self.menu_component.get_file_context_menu(file_path)
            }
            ContextType::Commit => {
                let default_hash = String::new();
                let commit_hash = context.get_data("commit_hash").unwrap_or(&default_hash);
                self.menu_component.get_commit_context_menu(commit_hash)
            }
            ContextType::Branch => {
                let default_name = String::new();
                let branch_name = context.get_data("branch_name").unwrap_or(&default_name);
                let is_current = context.get_data("is_current")
                    .map(|s| s == "true")
                    .unwrap_or(false);
                self.menu_component.get_branch_context_menu(branch_name, is_current)
            }
            ContextType::Tag => self.generate_tag_menu(context),
            ContextType::Remote => self.generate_remote_menu(context),
            ContextType::Stash => self.generate_stash_menu(context),
            ContextType::Search => self.generate_search_menu(context),
            ContextType::Diff => self.generate_diff_menu(context),
            ContextType::Empty => self.generate_general_menu(context),
        }
    }

    /// Generate tag-specific menu
    fn generate_tag_menu(&self, context: &ContextInfo) -> Vec<MenuItem> {
        let tag_name = context.get_data("tag_name").unwrap_or(&String::new());

        vec![
            MenuItem::new("tag_checkout", "Checkout Tag", MenuAction::BranchCheckout)
                .with_icon("🏷️")
                .with_hotkey("Enter"),

            MenuItem::new("tag_show", "Show Tag", MenuAction::CommitShow)
                .with_icon("👁️"),

            MenuItem::separator("sep1"),

            MenuItem::new("tag_delete", "Delete Tag", MenuAction::Custom("tag_delete".to_string()))
                .with_icon("🗑️")
                .with_hotkey("Del"),

            MenuItem::new("tag_push", "Push Tag", MenuAction::Custom("tag_push".to_string()))
                .with_icon("⬆️"),
        ]
    }

    /// Generate remote-specific menu
    fn generate_remote_menu(&self, context: &ContextInfo) -> Vec<MenuItem> {
        let remote_name = context.get_data("remote_name").unwrap_or(&String::new());

        vec![
            MenuItem::new("remote_fetch", "Fetch", MenuAction::Custom("remote_fetch".to_string()))
                .with_icon("⬇️")
                .with_hotkey("F"),

            MenuItem::new("remote_pull", "Pull", MenuAction::BranchPull)
                .with_icon("⬇️"),

            MenuItem::new("remote_push", "Push", MenuAction::BranchPush)
                .with_icon("⬆️"),

            MenuItem::separator("sep1"),

            MenuItem::new("remote_edit", "Edit URL", MenuAction::Custom("remote_edit".to_string()))
                .with_icon("✏️"),

            MenuItem::new("remote_remove", "Remove Remote", MenuAction::Custom("remote_remove".to_string()))
                .with_icon("🗑️"),

            MenuItem::separator("sep2"),

            MenuItem::new("remote_branches", "Show Branches", MenuAction::Custom("remote_branches".to_string()))
                .with_icon("🌿"),
        ]
    }

    /// Generate stash-specific menu
    fn generate_stash_menu(&self, context: &ContextInfo) -> Vec<MenuItem> {
        let stash_index = context.get_data("stash_index").unwrap_or(&String::new());

        vec![
            MenuItem::new("stash_apply", "Apply", MenuAction::Custom("stash_apply".to_string()))
                .with_icon("📦")
                .with_hotkey("Enter"),

            MenuItem::new("stash_pop", "Pop", MenuAction::Custom("stash_pop".to_string()))
                .with_icon("📤"),

            MenuItem::new("stash_show", "Show Changes", MenuAction::GitDiff)
                .with_icon("👁️"),

            MenuItem::separator("sep1"),

            MenuItem::new("stash_drop", "Drop", MenuAction::Custom("stash_drop".to_string()))
                .with_icon("🗑️")
                .with_hotkey("Del"),

            MenuItem::new("stash_branch", "Create Branch", MenuAction::Custom("stash_branch".to_string()))
                .with_icon("🌿"),
        ]
    }

    /// Generate search-specific menu
    fn generate_search_menu(&self, context: &ContextInfo) -> Vec<MenuItem> {
        vec![
            MenuItem::new("search_files", "Search in Files", MenuAction::SearchInFiles)
                .with_icon("📁")
                .with_hotkey("Ctrl+Shift+F"),

            MenuItem::new("search_commits", "Search Commits", MenuAction::SearchCommits)
                .with_icon("📝"),

            MenuItem::new("search_branches", "Search Branches", MenuAction::SearchBranches)
                .with_icon("🌿"),

            MenuItem::separator("sep1"),

            MenuItem::new("search_filter", "Advanced Filter", MenuAction::Custom("search_filter".to_string()))
                .with_icon("🔍"),

            MenuItem::new("search_save", "Save Search", MenuAction::Custom("search_save".to_string()))
                .with_icon("💾"),

            MenuItem::separator("sep2"),

            MenuItem::new("search_replace", "Replace", MenuAction::Custom("search_replace".to_string()))
                .with_icon("🔄")
                .with_hotkey("Ctrl+H"),
        ]
    }

    /// Generate diff-specific menu
    fn generate_diff_menu(&self, context: &ContextInfo) -> Vec<MenuItem> {
        vec![
            MenuItem::new("diff_stage_hunk", "Stage Hunk", MenuAction::Custom("diff_stage_hunk".to_string()))
                .with_icon("➕")
                .with_hotkey("S"),

            MenuItem::new("diff_unstage_hunk", "Unstage Hunk", MenuAction::Custom("diff_unstage_hunk".to_string()))
                .with_icon("➖")
                .with_hotkey("U"),

            MenuItem::new("diff_discard_hunk", "Discard Hunk", MenuAction::Custom("diff_discard_hunk".to_string()))
                .with_icon("🗑️")
                .with_hotkey("D"),

            MenuItem::separator("sep1"),

            MenuItem::new("diff_copy", "Copy", MenuAction::FileCopy)
                .with_icon("📋")
                .with_hotkey("Ctrl+C"),

            MenuItem::new("diff_external", "External Diff", MenuAction::Custom("diff_external".to_string()))
                .with_icon("🔗"),

            MenuItem::separator("sep2"),

            MenuItem::new("diff_whitespace", "Ignore Whitespace", MenuAction::Custom("diff_whitespace".to_string()))
                .with_icon("🔍"),

            MenuItem::new("diff_word", "Word Diff", MenuAction::Custom("diff_word".to_string()))
                .with_icon("📝"),
        ]
    }

    /// Generate general/empty context menu
    fn generate_general_menu(&self, context: &ContextInfo) -> Vec<MenuItem> {
        vec![
            MenuItem::new("refresh", "Refresh", MenuAction::Custom("refresh".to_string()))
                .with_icon("🔄")
                .with_hotkey("F5"),

            MenuItem::separator("sep1"),

            MenuItem::new("search", "Search", MenuAction::Custom("search".to_string()))
                .with_icon("🔍")
                .with_hotkey("Ctrl+F"),

            MenuItem::new("filter", "Filter", MenuAction::Custom("filter".to_string()))
                .with_icon("🔍")
                .with_hotkey("Ctrl+Shift+F"),

            MenuItem::separator("sep2"),

            MenuItem::new("settings", "Settings", MenuAction::Custom("settings".to_string()))
                .with_icon("⚙️"),

            MenuItem::new("help", "Help", MenuAction::Custom("help".to_string()))
                .with_icon("❓")
                .with_hotkey("F1"),
        ]
    }

    /// Handle mouse events for context menu
    pub fn handle_mouse_event(&mut self, mouse: MouseEvent, area: Rect) -> AppResult<Option<MenuAction>> {
        self.menu_component.handle_mouse_event(mouse, area)
    }

    /// Get the last executed action
    pub fn get_last_action(&self) -> Option<&MenuAction> {
        self.last_action.as_ref()
    }

    /// Clear the last action
    pub fn clear_last_action(&mut self) {
        self.last_action = None;
    }

    /// Check if context menu is open
    pub fn is_open(&self) -> bool {
        self.menu_component.is_open()
    }

    /// Close context menu
    pub fn close(&mut self) {
        self.menu_component.close();
        self.current_context = None;
    }

    /// Get current context
    pub fn get_current_context(&self) -> Option<&ContextInfo> {
        self.current_context.as_ref()
    }

    /// Handle context menu action execution
    pub fn execute_action(&mut self, action: MenuAction, state: &mut AppState) -> AppResult<()> {
        self.last_action = Some(action.clone());

        match action {
            MenuAction::GitAdd => {
                if let Some(context) = &self.current_context {
                    if let Some(file_path) = context.get_data("file_path") {
                        // Execute git add command
                        println!("Git add: {}", file_path);
                    }
                }
            }
            MenuAction::GitDiff => {
                if let Some(context) = &self.current_context {
                    if let Some(file_path) = context.get_data("file_path") {
                        // Show diff for file
                        println!("Show diff: {}", file_path);
                    }
                }
            }
            MenuAction::BranchCheckout => {
                if let Some(context) = &self.current_context {
                    if let Some(branch_name) = context.get_data("branch_name") {
                        // Checkout branch
                        println!("Checkout branch: {}", branch_name);
                    }
                }
            }
            MenuAction::SearchInFiles => {
                // Activate search with files scope
                println!("Search in files");
            }
            MenuAction::Custom(ref command) => {
                // Handle custom commands
                match command.as_str() {
                    "refresh" => {
                        // Refresh current view
                        println!("Refresh");
                    }
                    "settings" => {
                        // Open settings
                        println!("Open settings");
                    }
                    "help" => {
                        // Show help
                        println!("Show help");
                    }
                    _ => {
                        println!("Unknown command: {}", command);
                    }
                }
            }
            _ => {
                println!("Unhandled action: {:?}", action);
            }
        }

        Ok(())
    }

    /// Show context menu for file
    pub fn show_file_menu(&mut self, file_path: &str, position: (u16, u16)) -> AppResult<()> {
        let context = ContextInfo::new(ContextType::File, position)
            .with_data("file_path", file_path);
        self.show_context_menu(context)
    }

    /// Show context menu for commit
    pub fn show_commit_menu(&mut self, commit_hash: &str, position: (u16, u16)) -> AppResult<()> {
        let context = ContextInfo::new(ContextType::Commit, position)
            .with_data("commit_hash", commit_hash);
        self.show_context_menu(context)
    }

    /// Show context menu for branch
    pub fn show_branch_menu(&mut self, branch_name: &str, is_current: bool, position: (u16, u16)) -> AppResult<()> {
        let context = ContextInfo::new(ContextType::Branch, position)
            .with_data("branch_name", branch_name)
            .with_data("is_current", &is_current.to_string());
        self.show_context_menu(context)
    }

    /// Show general context menu
    pub fn show_general_menu(&mut self, position: (u16, u16)) -> AppResult<()> {
        let context = ContextInfo::new(ContextType::Empty, position);
        self.show_context_menu(context)
    }
}

impl Component for ContextMenuManager {
    fn render(&mut self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        self.menu_component.render(frame, area, state, theme);
    }

    fn handle_key_event(&mut self, key: KeyEvent, state: &mut AppState) -> AppResult<()> {
        self.menu_component.handle_key_event(key, state)
    }
}
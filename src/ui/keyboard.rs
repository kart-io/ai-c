//! Unified keyboard shortcuts and key mappings
//!
//! This module provides a centralized place for all keyboard shortcuts
//! used throughout the application, ensuring consistency and avoiding conflicts.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Common navigation shortcuts used across all components
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NavigationKey {
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Home,
    End,
}

impl NavigationKey {
    /// Check if a key event matches this navigation key
    pub fn matches(&self, key: &KeyEvent) -> bool {
        match self {
            NavigationKey::Up => matches!(key.code, KeyCode::Up | KeyCode::Char('k')),
            NavigationKey::Down => matches!(key.code, KeyCode::Down | KeyCode::Char('j')),
            NavigationKey::Left => matches!(key.code, KeyCode::Left | KeyCode::Char('h')),
            NavigationKey::Right => matches!(key.code, KeyCode::Right | KeyCode::Char('l')),
            NavigationKey::PageUp => matches!(key.code, KeyCode::PageUp),
            NavigationKey::PageDown => matches!(key.code, KeyCode::PageDown),
            NavigationKey::Home => matches!(key.code, KeyCode::Home),
            NavigationKey::End => matches!(key.code, KeyCode::End),
        }
    }

    /// Get the primary key representation as string
    pub fn as_str(&self) -> &'static str {
        match self {
            NavigationKey::Up => "↑/k",
            NavigationKey::Down => "↓/j",
            NavigationKey::Left => "←/h",
            NavigationKey::Right => "→/l",
            NavigationKey::PageUp => "PgUp",
            NavigationKey::PageDown => "PgDn",
            NavigationKey::Home => "Home",
            NavigationKey::End => "End",
        }
    }
}

/// Common action shortcuts used across components
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActionKey {
    Confirm,           // Enter
    Cancel,            // Esc
    Delete,            // d
    New,               // n
    Refresh,           // r
    SelectLine,        // l (selection manager)
    Apply,             // a (for stash apply)
    Show,              // s (for show details/diff)
    Merge,             // m (for branch merge)
    Push,              // p (for push)
    Pull,              // u (for pull)
    SwitchPanel,       // Space (for focus switching)
    Help,              // ? (for help system)
    Add,               // + (for staging files)
    Remove,            // - (for unstaging files)
}

impl ActionKey {
    /// Check if a key event matches this action key
    pub fn matches(&self, key: &KeyEvent) -> bool {
        match self {
            ActionKey::Confirm => matches!(key.code, KeyCode::Enter),
            ActionKey::Cancel => matches!(key.code, KeyCode::Esc),
            ActionKey::Delete => matches!(key.code, KeyCode::Char('d')),
            ActionKey::New => matches!(key.code, KeyCode::Char('n')),
            ActionKey::Refresh => matches!(key.code, KeyCode::Char('r')),
            ActionKey::SelectLine => matches!(key.code, KeyCode::Char('l')),
            ActionKey::Apply => matches!(key.code, KeyCode::Char('a')),
            ActionKey::Show => matches!(key.code, KeyCode::Char('s')),
            ActionKey::Merge => matches!(key.code, KeyCode::Char('m')),
            ActionKey::Push => matches!(key.code, KeyCode::Char('p')),
            ActionKey::Pull => matches!(key.code, KeyCode::Char('u')),
            ActionKey::SwitchPanel => matches!(key.code, KeyCode::Char(' ')),
            ActionKey::Help => matches!(key.code, KeyCode::Char('?')),
            ActionKey::Add => matches!(key.code, KeyCode::Char('+')),
            ActionKey::Remove => matches!(key.code, KeyCode::Char('-')),
        }
    }

    /// Get the key representation as string
    pub fn as_str(&self) -> &'static str {
        match self {
            ActionKey::Confirm => "Enter",
            ActionKey::Cancel => "Esc",
            ActionKey::Delete => "d",
            ActionKey::New => "n",
            ActionKey::Refresh => "r",
            ActionKey::SelectLine => "l",
            ActionKey::Apply => "a",
            ActionKey::Show => "s",
            ActionKey::Merge => "m",
            ActionKey::Push => "p",
            ActionKey::Pull => "u",
            ActionKey::SwitchPanel => "Space",
            ActionKey::Help => "?",
            ActionKey::Add => "+",
            ActionKey::Remove => "-",
        }
    }

    /// Get the description of what this key does
    pub fn description(&self) -> &'static str {
        match self {
            ActionKey::Confirm => "Confirm/Execute",
            ActionKey::Cancel => "Cancel/Back",
            ActionKey::Delete => "Delete item",
            ActionKey::New => "Create new",
            ActionKey::Refresh => "Refresh list",
            ActionKey::SelectLine => "Select line",
            ActionKey::Apply => "Apply changes",
            ActionKey::Show => "Show details",
            ActionKey::Merge => "Merge branch",
            ActionKey::Push => "Push to remote",
            ActionKey::Pull => "Pull from remote",
            ActionKey::SwitchPanel => "Switch focus",
            ActionKey::Help => "Show help",
            ActionKey::Add => "Stage file",
            ActionKey::Remove => "Unstage file",
        }
    }
}

/// Unified shortcut manager for consistent key handling
#[derive(Debug, Default)]
pub struct ShortcutManager {
    /// Whether to show vim-style shortcuts in help
    pub show_vim_keys: bool,
}

impl ShortcutManager {
    pub fn new() -> Self {
        Self {
            show_vim_keys: true,
        }
    }

    /// Generate help text for navigation keys
    pub fn navigation_help(&self) -> String {
        if self.show_vim_keys {
            "↑↓/kj: Navigate | PgUp/Dn: Quick scroll | Home/End: Jump to start/end"
        } else {
            "↑↓: Navigate | PgUp/Dn: Quick scroll | Home/End: Jump to start/end"
        }.to_string()
    }

    /// Generate help text for common actions
    pub fn common_actions_help(&self) -> String {
        "Enter: Select | Esc: Back | d: Delete | n: New | r: Refresh".to_string()
    }

    /// Generate context-specific help for different component types
    pub fn context_help(&self, context: &str) -> String {
        match context {
            "branches" => format!("{} | m: Merge | p: Push | ?: Help", self.common_actions_help()),
            "tags" => format!("{} | s: Show details | ?: Help", self.common_actions_help()),
            "stash" => format!("{} | a: Apply | s: Show diff | ?: Help", self.common_actions_help()),
            "status" => format!("{} | l: Select line | ?: Help", self.common_actions_help()),
            "gitflow" => format!("{} | f: Finish branch | ?: Help", self.common_actions_help()),
            "history" => format!("{} | s: Show commit | ?: Help", self.common_actions_help()),
            "remotes" => format!("{} | p: Push | ?: Help", self.common_actions_help()),
            _ => format!("{} | ?: Help", self.common_actions_help()),
        }
    }

    /// Check if a key event is a navigation key
    pub fn is_navigation_key(&self, key: &KeyEvent) -> Option<NavigationKey> {
        for nav_key in [
            NavigationKey::Up, NavigationKey::Down, NavigationKey::Left, NavigationKey::Right,
            NavigationKey::PageUp, NavigationKey::PageDown, NavigationKey::Home, NavigationKey::End
        ] {
            if nav_key.matches(key) {
                return Some(nav_key);
            }
        }
        None
    }

    /// Check if a key event is an action key
    pub fn is_action_key(&self, key: &KeyEvent) -> Option<ActionKey> {
        for action_key in [
            ActionKey::Confirm, ActionKey::Cancel, ActionKey::Delete, ActionKey::New,
            ActionKey::Refresh, ActionKey::SelectLine, ActionKey::Apply, ActionKey::Show,
            ActionKey::Merge, ActionKey::Push, ActionKey::Pull, ActionKey::SwitchPanel, ActionKey::Help,
            ActionKey::Add, ActionKey::Remove
        ] {
            if action_key.matches(key) {
                return Some(action_key);
            }
        }
        None
    }
}

/// Standard navigation behavior for list components
pub trait NavigationHandler {
    /// Get the current selected index
    fn selected_index(&self) -> usize;

    /// Set the selected index
    fn set_selected_index(&mut self, index: usize);

    /// Get the total number of items
    fn item_count(&self) -> usize;

    /// Handle navigation key with standard behavior
    fn handle_navigation(&mut self, nav_key: NavigationKey) -> bool {
        let current = self.selected_index();
        let count = self.item_count();

        if count == 0 {
            return false;
        }

        let new_index = match nav_key {
            NavigationKey::Up => {
                if current > 0 { current - 1 } else { count - 1 }
            }
            NavigationKey::Down => {
                (current + 1) % count
            }
            NavigationKey::PageUp => {
                current.saturating_sub(5)
            }
            NavigationKey::PageDown => {
                (current + 5).min(count - 1)
            }
            NavigationKey::Home => 0,
            NavigationKey::End => count - 1,
            _ => return false,
        };

        if new_index != current {
            self.set_selected_index(new_index);
            true
        } else {
            false
        }
    }
}

/// Focus management for components with multiple panels
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusDirection {
    Next,
    Previous,
    Left,
    Right,
}

pub trait FocusHandler {
    /// Switch focus in the specified direction
    fn switch_focus(&mut self, direction: FocusDirection) -> bool;

    /// Get the current focus state as a string for display
    fn focus_indicator(&self) -> String;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigation_key_matching() {
        let up_key = KeyEvent::from(KeyCode::Up);
        let k_key = KeyEvent::from(KeyCode::Char('k'));

        assert!(NavigationKey::Up.matches(&up_key));
        assert!(NavigationKey::Up.matches(&k_key));
        assert!(!NavigationKey::Down.matches(&up_key));
    }

    #[test]
    fn test_action_key_matching() {
        let enter_key = KeyEvent::from(KeyCode::Enter);
        let esc_key = KeyEvent::from(KeyCode::Esc);

        assert!(ActionKey::Confirm.matches(&enter_key));
        assert!(ActionKey::Cancel.matches(&esc_key));
        assert!(!ActionKey::Delete.matches(&enter_key));
    }

    #[test]
    fn test_shortcut_manager() {
        let manager = ShortcutManager::new();
        let up_key = KeyEvent::from(KeyCode::Up);
        let enter_key = KeyEvent::from(KeyCode::Enter);

        assert_eq!(manager.is_navigation_key(&up_key), Some(NavigationKey::Up));
        assert_eq!(manager.is_action_key(&enter_key), Some(ActionKey::Confirm));
    }
}
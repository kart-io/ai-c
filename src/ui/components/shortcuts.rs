//! Global keyboard shortcuts manager

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;

use crate::error::AppResult;

/// Keyboard shortcut definition
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Shortcut {
    pub key: KeyCode,
    pub modifiers: KeyModifiers,
}

impl Shortcut {
    pub fn new(key: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { key, modifiers }
    }

    /// Create shortcut from key event
    pub fn from_key_event(event: KeyEvent) -> Self {
        Self {
            key: event.code,
            modifiers: event.modifiers,
        }
    }

    /// Create shortcut from string description (e.g., "Ctrl+F", "Alt+Enter")
    pub fn from_str(desc: &str) -> Option<Self> {
        let parts: Vec<&str> = desc.split('+').collect();
        if parts.is_empty() {
            return None;
        }

        let mut modifiers = KeyModifiers::empty();
        let mut key_part = parts[parts.len() - 1];

        for part in &parts[..parts.len() - 1] {
            match part.to_lowercase().as_str() {
                "ctrl" | "control" => modifiers.insert(KeyModifiers::CONTROL),
                "alt" => modifiers.insert(KeyModifiers::ALT),
                "shift" => modifiers.insert(KeyModifiers::SHIFT),
                _ => {}
            }
        }

        let key = match key_part.to_lowercase().as_str() {
            "a" => KeyCode::Char('a'),
            "b" => KeyCode::Char('b'),
            "c" => KeyCode::Char('c'),
            "d" => KeyCode::Char('d'),
            "e" => KeyCode::Char('e'),
            "f" => KeyCode::Char('f'),
            "g" => KeyCode::Char('g'),
            "h" => KeyCode::Char('h'),
            "i" => KeyCode::Char('i'),
            "j" => KeyCode::Char('j'),
            "k" => KeyCode::Char('k'),
            "l" => KeyCode::Char('l'),
            "m" => KeyCode::Char('m'),
            "n" => KeyCode::Char('n'),
            "o" => KeyCode::Char('o'),
            "p" => KeyCode::Char('p'),
            "q" => KeyCode::Char('q'),
            "r" => KeyCode::Char('r'),
            "s" => KeyCode::Char('s'),
            "t" => KeyCode::Char('t'),
            "u" => KeyCode::Char('u'),
            "v" => KeyCode::Char('v'),
            "w" => KeyCode::Char('w'),
            "x" => KeyCode::Char('x'),
            "y" => KeyCode::Char('y'),
            "z" => KeyCode::Char('z'),
            "enter" => KeyCode::Enter,
            "space" => KeyCode::Char(' '),
            "tab" => KeyCode::Tab,
            "esc" | "escape" => KeyCode::Esc,
            "backspace" => KeyCode::Backspace,
            "delete" | "del" => KeyCode::Delete,
            "up" => KeyCode::Up,
            "down" => KeyCode::Down,
            "left" => KeyCode::Left,
            "right" => KeyCode::Right,
            "home" => KeyCode::Home,
            "end" => KeyCode::End,
            "pageup" => KeyCode::PageUp,
            "pagedown" => KeyCode::PageDown,
            "insert" => KeyCode::Insert,
            f if f.starts_with('f') => {
                if let Ok(num) = f[1..].parse::<u8>() {
                    KeyCode::F(num)
                } else {
                    return None;
                }
            }
            _ => return None,
        };

        Some(Self { key, modifiers })
    }

    /// Convert shortcut to display string
    pub fn to_string(&self) -> String {
        let mut parts = Vec::new();

        if self.modifiers.contains(KeyModifiers::CONTROL) {
            parts.push("Ctrl");
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            parts.push("Alt");
        }
        if self.modifiers.contains(KeyModifiers::SHIFT) {
            parts.push("Shift");
        }

        let key_str = match self.key {
            KeyCode::Char(c) => c.to_uppercase().to_string(),
            KeyCode::Enter => "Enter".to_string(),
            KeyCode::Tab => "Tab".to_string(),
            KeyCode::Esc => "Esc".to_string(),
            KeyCode::Backspace => "Backspace".to_string(),
            KeyCode::Delete => "Del".to_string(),
            KeyCode::Up => "↑".to_string(),
            KeyCode::Down => "↓".to_string(),
            KeyCode::Left => "←".to_string(),
            KeyCode::Right => "→".to_string(),
            KeyCode::Home => "Home".to_string(),
            KeyCode::End => "End".to_string(),
            KeyCode::PageUp => "PgUp".to_string(),
            KeyCode::PageDown => "PgDn".to_string(),
            KeyCode::Insert => "Ins".to_string(),
            KeyCode::F(n) => format!("F{}", n),
            _ => "?".to_string(),
        };

        parts.push(&key_str);
        parts.join("+")
    }

    /// Check if shortcut matches key event
    pub fn matches(&self, event: KeyEvent) -> bool {
        self.key == event.code && self.modifiers == event.modifiers
    }
}

/// Shortcut action types
#[derive(Debug, Clone, PartialEq)]
pub enum ShortcutAction {
    // Navigation
    NavigateUp,
    NavigateDown,
    NavigateLeft,
    NavigateRight,
    NavigateHome,
    NavigateEnd,
    NavigatePageUp,
    NavigatePageDown,

    // Application
    Quit,
    Refresh,
    Help,
    Settings,

    // Search
    Search,
    SearchNext,
    SearchPrevious,
    AdvancedSearch,
    ClearSearch,

    // Git operations
    GitStatus,
    GitDiff,
    GitAdd,
    GitCommit,
    GitPush,
    GitPull,
    GitStash,
    GitBranch,

    // File operations
    OpenFile,
    EditFile,
    DeleteFile,
    RenameFile,
    CopyPath,

    // View operations
    ToggleSidebar,
    ToggleStatusBar,
    SwitchTab,
    CloseTab,
    NewTab,

    // Custom action
    Custom(String),
}

/// Shortcut context for conditional activation
#[derive(Debug, Clone, PartialEq)]
pub enum ShortcutContext {
    Global,           // Always active
    FileList,         // Active in file list views
    CommitList,       // Active in commit list views
    BranchList,       // Active in branch list views
    DiffViewer,       // Active in diff viewer
    SearchMode,       // Active during search
    ModalOpen,        // Active when modal is open
    MenuOpen,         // Active when context menu is open
}

/// Shortcut definition with context and action
#[derive(Debug, Clone)]
pub struct ShortcutDefinition {
    pub shortcut: Shortcut,
    pub action: ShortcutAction,
    pub context: ShortcutContext,
    pub description: String,
    pub enabled: bool,
}

impl ShortcutDefinition {
    pub fn new(
        shortcut: Shortcut,
        action: ShortcutAction,
        context: ShortcutContext,
        description: &str,
    ) -> Self {
        Self {
            shortcut,
            action,
            context,
            description: description.to_string(),
            enabled: true,
        }
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

/// Global shortcuts manager
pub struct ShortcutsManager {
    shortcuts: HashMap<Shortcut, ShortcutDefinition>,
    context_stack: Vec<ShortcutContext>,
}

impl ShortcutsManager {
    pub fn new() -> Self {
        let mut manager = Self {
            shortcuts: HashMap::new(),
            context_stack: vec![ShortcutContext::Global],
        };

        manager.initialize_default_shortcuts();
        manager
    }

    /// Initialize default application shortcuts
    fn initialize_default_shortcuts(&mut self) {
        let shortcuts = vec![
            // Application shortcuts
            ShortcutDefinition::new(
                Shortcut::new(KeyCode::Char('q'), KeyModifiers::CONTROL),
                ShortcutAction::Quit,
                ShortcutContext::Global,
                "Quit application"
            ),
            ShortcutDefinition::new(
                Shortcut::new(KeyCode::F(5), KeyModifiers::empty()),
                ShortcutAction::Refresh,
                ShortcutContext::Global,
                "Refresh current view"
            ),
            ShortcutDefinition::new(
                Shortcut::new(KeyCode::Char('?'), KeyModifiers::empty()),
                ShortcutAction::Help,
                ShortcutContext::Global,
                "Show help"
            ),

            // Search shortcuts
            ShortcutDefinition::new(
                Shortcut::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
                ShortcutAction::Search,
                ShortcutContext::Global,
                "Open search"
            ),
            ShortcutDefinition::new(
                Shortcut::new(KeyCode::Char('f'), KeyModifiers::CONTROL | KeyModifiers::SHIFT),
                ShortcutAction::AdvancedSearch,
                ShortcutContext::Global,
                "Open advanced search"
            ),
            ShortcutDefinition::new(
                Shortcut::new(KeyCode::Char('g'), KeyModifiers::CONTROL),
                ShortcutAction::SearchNext,
                ShortcutContext::SearchMode,
                "Find next"
            ),
            ShortcutDefinition::new(
                Shortcut::new(KeyCode::Char('g'), KeyModifiers::CONTROL | KeyModifiers::SHIFT),
                ShortcutAction::SearchPrevious,
                ShortcutContext::SearchMode,
                "Find previous"
            ),

            // Git operation shortcuts
            ShortcutDefinition::new(
                Shortcut::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
                ShortcutAction::GitStatus,
                ShortcutContext::Global,
                "Show Git status"
            ),
            ShortcutDefinition::new(
                Shortcut::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
                ShortcutAction::GitDiff,
                ShortcutContext::FileList,
                "Show file diff"
            ),
            ShortcutDefinition::new(
                Shortcut::new(KeyCode::Char('a'), KeyModifiers::CONTROL),
                ShortcutAction::GitAdd,
                ShortcutContext::FileList,
                "Add file to staging"
            ),
            ShortcutDefinition::new(
                Shortcut::new(KeyCode::Char('c'), KeyModifiers::CONTROL | KeyModifiers::SHIFT),
                ShortcutAction::GitCommit,
                ShortcutContext::Global,
                "Create commit"
            ),

            // Navigation shortcuts
            ShortcutDefinition::new(
                Shortcut::new(KeyCode::Tab, KeyModifiers::empty()),
                ShortcutAction::SwitchTab,
                ShortcutContext::Global,
                "Switch to next tab"
            ),
            ShortcutDefinition::new(
                Shortcut::new(KeyCode::Tab, KeyModifiers::SHIFT),
                ShortcutAction::SwitchTab,
                ShortcutContext::Global,
                "Switch to previous tab"
            ),

            // File operation shortcuts
            ShortcutDefinition::new(
                Shortcut::new(KeyCode::Enter, KeyModifiers::empty()),
                ShortcutAction::OpenFile,
                ShortcutContext::FileList,
                "Open selected file"
            ),
            ShortcutDefinition::new(
                Shortcut::new(KeyCode::F(2), KeyModifiers::empty()),
                ShortcutAction::RenameFile,
                ShortcutContext::FileList,
                "Rename selected file"
            ),
            ShortcutDefinition::new(
                Shortcut::new(KeyCode::Delete, KeyModifiers::empty()),
                ShortcutAction::DeleteFile,
                ShortcutContext::FileList,
                "Delete selected file"
            ),
            ShortcutDefinition::new(
                Shortcut::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
                ShortcutAction::CopyPath,
                ShortcutContext::FileList,
                "Copy file path"
            ),

            // View toggles
            ShortcutDefinition::new(
                Shortcut::new(KeyCode::F(9), KeyModifiers::empty()),
                ShortcutAction::ToggleSidebar,
                ShortcutContext::Global,
                "Toggle sidebar"
            ),
        ];

        for shortcut_def in shortcuts {
            self.add_shortcut(shortcut_def);
        }
    }

    /// Add or update a shortcut
    pub fn add_shortcut(&mut self, definition: ShortcutDefinition) {
        self.shortcuts.insert(definition.shortcut.clone(), definition);
    }

    /// Remove a shortcut
    pub fn remove_shortcut(&mut self, shortcut: &Shortcut) {
        self.shortcuts.remove(shortcut);
    }

    /// Push a context onto the stack
    pub fn push_context(&mut self, context: ShortcutContext) {
        self.context_stack.push(context);
    }

    /// Pop a context from the stack
    pub fn pop_context(&mut self) -> Option<ShortcutContext> {
        if self.context_stack.len() > 1 {
            self.context_stack.pop()
        } else {
            None // Always keep at least Global context
        }
    }

    /// Get current context
    pub fn current_context(&self) -> &ShortcutContext {
        self.context_stack.last().unwrap_or(&ShortcutContext::Global)
    }

    /// Handle key event and return action if shortcut matches
    pub fn handle_key_event(&self, event: KeyEvent) -> Option<ShortcutAction> {
        let shortcut = Shortcut::from_key_event(event);

        if let Some(definition) = self.shortcuts.get(&shortcut) {
            if definition.enabled && self.is_context_active(&definition.context) {
                return Some(definition.action.clone());
            }
        }

        None
    }

    /// Check if a context is currently active
    fn is_context_active(&self, context: &ShortcutContext) -> bool {
        match context {
            ShortcutContext::Global => true,
            other => self.context_stack.contains(other),
        }
    }

    /// Get all shortcuts for a specific context
    pub fn get_shortcuts_for_context(&self, context: &ShortcutContext) -> Vec<&ShortcutDefinition> {
        self.shortcuts
            .values()
            .filter(|def| def.enabled && (&def.context == context || def.context == ShortcutContext::Global))
            .collect()
    }

    /// Get all enabled shortcuts
    pub fn get_all_shortcuts(&self) -> Vec<&ShortcutDefinition> {
        self.shortcuts
            .values()
            .filter(|def| def.enabled)
            .collect()
    }

    /// Get shortcuts formatted for help display
    pub fn get_help_text(&self, context: Option<&ShortcutContext>) -> Vec<(String, String)> {
        let context = context.unwrap_or(self.current_context());
        let mut shortcuts = self.get_shortcuts_for_context(context);

        shortcuts.sort_by(|a, b| a.shortcut.to_string().cmp(&b.shortcut.to_string()));

        shortcuts
            .into_iter()
            .map(|def| (def.shortcut.to_string(), def.description.clone()))
            .collect()
    }

    /// Enable or disable a shortcut
    pub fn set_shortcut_enabled(&mut self, shortcut: &Shortcut, enabled: bool) {
        if let Some(definition) = self.shortcuts.get_mut(shortcut) {
            definition.enabled = enabled;
        }
    }

    /// Check if a shortcut exists
    pub fn has_shortcut(&self, shortcut: &Shortcut) -> bool {
        self.shortcuts.contains_key(shortcut)
    }

    /// Get shortcut definition
    pub fn get_shortcut(&self, shortcut: &Shortcut) -> Option<&ShortcutDefinition> {
        self.shortcuts.get(shortcut)
    }

    /// Clear all custom shortcuts (keeps defaults)
    pub fn reset_to_defaults(&mut self) {
        self.shortcuts.clear();
        self.initialize_default_shortcuts();
    }

    /// Load shortcuts from configuration
    pub fn load_from_config(&mut self, _config: &HashMap<String, String>) -> AppResult<()> {
        // TODO: Implement configuration loading
        // This would parse shortcut definitions from configuration file
        Ok(())
    }

    /// Save shortcuts to configuration
    pub fn save_to_config(&self) -> HashMap<String, String> {
        // TODO: Implement configuration saving
        // This would serialize shortcut definitions to configuration format
        HashMap::new()
    }
}

impl Default for ShortcutsManager {
    fn default() -> Self {
        Self::new()
    }
}
//! Text selection and clipboard management module
//!
//! Provides functionality for text selection, copy operations, and clipboard management
//! across different UI components.

use arboard::Clipboard;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, warn};

use crate::error::{AppError, AppResult};

/// Position in text for selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextPosition {
    pub line: usize,
    pub column: usize,
}

impl TextPosition {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }

    /// Check if this position is before another position
    pub fn is_before(&self, other: &TextPosition) -> bool {
        self.line < other.line || (self.line == other.line && self.column < other.column)
    }

    /// Check if this position is after another position
    pub fn is_after(&self, other: &TextPosition) -> bool {
        self.line > other.line || (self.line == other.line && self.column > other.column)
    }
}

/// Text selection range
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSelection {
    pub start: TextPosition,
    pub end: TextPosition,
}

impl TextSelection {
    pub fn new(start: TextPosition, end: TextPosition) -> Self {
        // Ensure start is always before end
        if start.is_after(&end) {
            Self { start: end, end: start }
        } else {
            Self { start, end }
        }
    }

    /// Create a selection from current position
    pub fn from_position(pos: TextPosition) -> Self {
        Self { start: pos, end: pos }
    }

    /// Check if selection is empty (start == end)
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Extend selection to a new position
    pub fn extend_to(&mut self, pos: TextPosition) {
        self.end = pos;
        // Reorder if needed
        if self.start.is_after(&self.end) {
            std::mem::swap(&mut self.start, &mut self.end);
        }
    }

    /// Check if a position is within the selection
    pub fn contains(&self, pos: &TextPosition) -> bool {
        (self.start == *pos || self.start.is_before(pos))
            && (self.end == *pos || pos.is_before(&self.end))
    }

    /// Get the selection range as (start_line, start_col, end_line, end_col)
    pub fn as_range(&self) -> (usize, usize, usize, usize) {
        (self.start.line, self.start.column, self.end.line, self.end.column)
    }
}

/// Selection mode for different types of selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    /// Character-by-character selection
    Character,
    /// Word-by-word selection
    Word,
    /// Line-by-line selection
    Line,
    /// Block selection (rectangular)
    Block,
}

/// Clipboard manager for handling copy/paste operations
pub struct ClipboardManager {
    clipboard: Arc<Mutex<Option<Clipboard>>>,
}

impl std::fmt::Debug for ClipboardManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClipboardManager")
            .field("clipboard", &"<clipboard>")
            .finish()
    }
}

impl Default for ClipboardManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ClipboardManager {
    /// Create a new clipboard manager
    pub fn new() -> Self {
        let clipboard = match Clipboard::new() {
            Ok(cb) => Some(cb),
            Err(e) => {
                warn!("Failed to initialize clipboard: {}", e);
                None
            }
        };

        Self {
            clipboard: Arc::new(Mutex::new(clipboard)),
        }
    }

    /// Copy text to clipboard
    pub async fn copy_text(&self, text: &str) -> AppResult<()> {
        let mut clipboard_guard = self.clipboard.lock().await;

        if let Some(ref mut clipboard) = *clipboard_guard {
            clipboard.set_text(text.to_string())
                .map_err(|e| AppError::application(&format!("Failed to copy to clipboard: {}", e)))?;
            debug!("Copied {} characters to clipboard", text.len());
            Ok(())
        } else {
            Err(AppError::application("Clipboard not available"))
        }
    }

    /// Get text from clipboard
    pub async fn get_text(&self) -> AppResult<String> {
        let mut clipboard_guard = self.clipboard.lock().await;

        if let Some(ref mut clipboard) = *clipboard_guard {
            clipboard.get_text()
                .map_err(|e| AppError::application(&format!("Failed to get from clipboard: {}", e)))
        } else {
            Err(AppError::application("Clipboard not available"))
        }
    }

    /// Check if clipboard is available
    pub async fn is_available(&self) -> bool {
        let clipboard_guard = self.clipboard.lock().await;
        clipboard_guard.is_some()
    }
}

/// Selection manager for handling text selection across components
#[derive(Debug)]
pub struct SelectionManager {
    /// Current selection
    current_selection: Option<TextSelection>,
    /// Selection mode
    mode: SelectionMode,
    /// Clipboard manager
    pub clipboard: ClipboardManager,
    /// Whether selection is active
    is_selecting: bool,
    /// Original start position when selection began
    selection_start: Option<TextPosition>,
}

impl Default for SelectionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for SelectionManager {
    fn clone(&self) -> Self {
        Self {
            current_selection: self.current_selection.clone(),
            mode: self.mode,
            clipboard: ClipboardManager::new(), // Create new clipboard manager
            is_selecting: self.is_selecting,
            selection_start: self.selection_start,
        }
    }
}

impl SelectionManager {
    /// Create a new selection manager
    pub fn new() -> Self {
        Self {
            current_selection: None,
            mode: SelectionMode::Character,
            clipboard: ClipboardManager::new(),
            is_selecting: false,
            selection_start: None,
        }
    }

    /// Start selection at a position
    pub fn start_selection(&mut self, pos: TextPosition) {
        self.is_selecting = true;
        self.selection_start = Some(pos);
        self.current_selection = Some(TextSelection::from_position(pos));
        debug!("Started selection at {:?}", pos);
    }

    /// Update selection to a new position
    pub fn update_selection(&mut self, pos: TextPosition) {
        if self.is_selecting {
            if let Some(ref mut selection) = self.current_selection {
                selection.extend_to(pos);
                debug!("Updated selection to {:?}", selection);
            }
        }
    }

    /// End selection
    pub fn end_selection(&mut self) {
        self.is_selecting = false;
        debug!("Ended selection");
    }

    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.current_selection = None;
        self.is_selecting = false;
        self.selection_start = None;
        debug!("Cleared selection");
    }

    /// Get current selection
    pub fn get_selection(&self) -> Option<&TextSelection> {
        self.current_selection.as_ref()
    }

    /// Check if currently selecting
    pub fn is_selecting(&self) -> bool {
        self.is_selecting
    }

    /// Set selection mode
    pub fn set_mode(&mut self, mode: SelectionMode) {
        self.mode = mode;
        debug!("Changed selection mode to {:?}", mode);
    }

    /// Get current selection mode
    pub fn get_mode(&self) -> SelectionMode {
        self.mode
    }

    /// Copy selected text to clipboard
    pub async fn copy_selection(&self, text_lines: &[String]) -> AppResult<()> {
        if let Some(selection) = &self.current_selection {
            let selected_text = self.extract_selected_text(text_lines, selection)?;
            self.clipboard.copy_text(&selected_text).await?;
            debug!("Copied selection: {} characters", selected_text.len());
            Ok(())
        } else {
            Err(AppError::application("No selection to copy"))
        }
    }

    /// Extract selected text from text lines
    pub fn extract_selected_text(&self, text_lines: &[String], selection: &TextSelection) -> AppResult<String> {
        let (start_line, start_col, end_line, end_col) = selection.as_range();

        if start_line >= text_lines.len() {
            return Ok(String::new());
        }

        let mut result = String::new();

        if start_line == end_line {
            // Single line selection
            let line = &text_lines[start_line];
            let start_pos = start_col.min(line.len());
            let end_pos = end_col.min(line.len());
            if start_pos < end_pos {
                result.push_str(&line[start_pos..end_pos]);
            }
        } else {
            // Multi-line selection
            // First line
            let first_line = &text_lines[start_line];
            let start_pos = start_col.min(first_line.len());
            if start_pos < first_line.len() {
                result.push_str(&first_line[start_pos..]);
                result.push('\n');
            }

            // Middle lines
            for line_idx in (start_line + 1)..end_line {
                if line_idx < text_lines.len() {
                    result.push_str(&text_lines[line_idx]);
                    result.push('\n');
                }
            }

            // Last line
            if end_line < text_lines.len() {
                let last_line = &text_lines[end_line];
                let end_pos = end_col.min(last_line.len());
                if end_pos > 0 {
                    result.push_str(&last_line[..end_pos]);
                }
            }
        }

        Ok(result)
    }

    /// Select all text
    pub fn select_all(&mut self, text_lines: &[String]) {
        if text_lines.is_empty() {
            self.clear_selection();
            return;
        }

        let start = TextPosition::new(0, 0);
        let end = if text_lines.len() > 0 {
            let last_line_idx = text_lines.len() - 1;
            let last_line_len = text_lines[last_line_idx].len();
            TextPosition::new(last_line_idx, last_line_len)
        } else {
            TextPosition::new(0, 0)
        };

        self.current_selection = Some(TextSelection::new(start, end));
        self.is_selecting = false;
        debug!("Selected all text");
    }

    /// Select current line
    pub fn select_line(&mut self, line_idx: usize, text_lines: &[String]) {
        if line_idx >= text_lines.len() {
            return;
        }

        let start = TextPosition::new(line_idx, 0);
        let end = TextPosition::new(line_idx, text_lines[line_idx].len());

        self.current_selection = Some(TextSelection::new(start, end));
        self.is_selecting = false;
        debug!("Selected line {}", line_idx);
    }

    /// Select word at position
    pub fn select_word_at(&mut self, pos: TextPosition, text_lines: &[String]) {
        if pos.line >= text_lines.len() {
            return;
        }

        let line = &text_lines[pos.line];
        if pos.column >= line.len() {
            return;
        }

        // Find word boundaries
        let chars: Vec<char> = line.chars().collect();
        let mut start_col = pos.column;
        let mut end_col = pos.column;

        // Find start of word
        while start_col > 0 && chars[start_col - 1].is_alphanumeric() {
            start_col -= 1;
        }

        // Find end of word
        while end_col < chars.len() && chars[end_col].is_alphanumeric() {
            end_col += 1;
        }

        let start = TextPosition::new(pos.line, start_col);
        let end = TextPosition::new(pos.line, end_col);

        self.current_selection = Some(TextSelection::new(start, end));
        self.is_selecting = false;
        debug!("Selected word at {:?}", pos);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_position() {
        let pos1 = TextPosition::new(1, 5);
        let pos2 = TextPosition::new(1, 10);
        let pos3 = TextPosition::new(2, 3);

        assert!(pos1.is_before(&pos2));
        assert!(pos1.is_before(&pos3));
        assert!(pos2.is_before(&pos3));

        assert!(pos2.is_after(&pos1));
        assert!(pos3.is_after(&pos1));
        assert!(pos3.is_after(&pos2));
    }

    #[test]
    fn test_text_selection() {
        let pos1 = TextPosition::new(1, 5);
        let pos2 = TextPosition::new(1, 10);

        let selection = TextSelection::new(pos1, pos2);
        assert_eq!(selection.start, pos1);
        assert_eq!(selection.end, pos2);

        // Test reversed positions
        let selection_rev = TextSelection::new(pos2, pos1);
        assert_eq!(selection_rev.start, pos1);
        assert_eq!(selection_rev.end, pos2);

        // Test contains
        let mid_pos = TextPosition::new(1, 7);
        assert!(selection.contains(&mid_pos));
        assert!(selection.contains(&pos1));
        assert!(selection.contains(&pos2));

        let outside_pos = TextPosition::new(1, 15);
        assert!(!selection.contains(&outside_pos));
    }

    #[test]
    fn test_extract_selected_text() {
        let manager = SelectionManager::new();
        let text_lines = vec![
            "Hello world".to_string(),
            "This is line 2".to_string(),
            "Final line".to_string(),
        ];

        // Single line selection
        let selection = TextSelection::new(
            TextPosition::new(0, 6),
            TextPosition::new(0, 11),
        );
        let result = manager.extract_selected_text(&text_lines, &selection).unwrap();
        assert_eq!(result, "world");

        // Multi-line selection
        let selection = TextSelection::new(
            TextPosition::new(0, 6),
            TextPosition::new(2, 5),
        );
        let result = manager.extract_selected_text(&text_lines, &selection).unwrap();
        assert_eq!(result, "world\nThis is line 2\nFinal");
    }
}
//! Advanced filter system for search results

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use std::collections::HashMap;

use crate::{
    app::state::AppState,
    error::AppResult,
    ui::theme::Theme,
};

use super::{Component, modals::{InputModal, Modal, ModalResult}};

/// Filter type for different filter categories
#[derive(Debug, Clone, PartialEq)]
pub enum FilterType {
    Author,
    DateFrom,
    DateTo,
    FileExtension,
    FilePath,
    ContentType,
    Branch,
    Tag,
    FileSize,
    CommitMessage,
}

impl FilterType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FilterType::Author => "Author",
            FilterType::DateFrom => "Date From",
            FilterType::DateTo => "Date To",
            FilterType::FileExtension => "File Extension",
            FilterType::FilePath => "File Path",
            FilterType::ContentType => "Content Type",
            FilterType::Branch => "Branch",
            FilterType::Tag => "Tag",
            FilterType::FileSize => "File Size",
            FilterType::CommitMessage => "Commit Message",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            FilterType::Author => "Filter by commit author",
            FilterType::DateFrom => "Filter from date (YYYY-MM-DD)",
            FilterType::DateTo => "Filter to date (YYYY-MM-DD)",
            FilterType::FileExtension => "Filter by file extension (e.g., .rs, .js)",
            FilterType::FilePath => "Filter by file path pattern",
            FilterType::ContentType => "Filter by content type",
            FilterType::Branch => "Filter by branch name",
            FilterType::Tag => "Filter by tag name",
            FilterType::FileSize => "Filter by file size (e.g., >1MB, <100KB)",
            FilterType::CommitMessage => "Filter by commit message pattern",
        }
    }

    pub fn placeholder(&self) -> &'static str {
        match self {
            FilterType::Author => "e.g., john.doe@example.com",
            FilterType::DateFrom => "e.g., 2024-01-01",
            FilterType::DateTo => "e.g., 2024-12-31",
            FilterType::FileExtension => "e.g., .rs, .js, .py",
            FilterType::FilePath => "e.g., src/*, *.toml",
            FilterType::ContentType => "e.g., text, binary, image",
            FilterType::Branch => "e.g., main, feature/*",
            FilterType::Tag => "e.g., v1.*, release-*",
            FilterType::FileSize => "e.g., >1MB, <100KB, 50KB-2MB",
            FilterType::CommitMessage => "e.g., fix:, feat:, refactor",
        }
    }
}

/// Individual filter item
#[derive(Debug, Clone)]
pub struct FilterItem {
    pub filter_type: FilterType,
    pub value: String,
    pub is_active: bool,
    pub is_negated: bool, // Support for NOT filters
}

impl FilterItem {
    pub fn new(filter_type: FilterType, value: String) -> Self {
        Self {
            filter_type,
            value,
            is_active: true,
            is_negated: false,
        }
    }

    pub fn toggle_active(&mut self) {
        self.is_active = !self.is_active;
    }

    pub fn toggle_negated(&mut self) {
        self.is_negated = !self.is_negated;
    }

    pub fn display_text(&self) -> String {
        let prefix = if self.is_negated { "NOT " } else { "" };
        let status = if self.is_active { "✓" } else { "✗" };
        format!("{} {}{}: {}", status, prefix, self.filter_type.as_str(), self.value)
    }
}

/// Advanced filter component
pub struct FilterComponent {
    is_open: bool,
    filters: Vec<FilterItem>,
    filter_state: ListState,
    available_types: Vec<FilterType>,
    type_state: ListState,
    mode: FilterMode,
    input_modal: InputModal,
    current_editing_index: Option<usize>,
    preset_filters: HashMap<String, Vec<FilterItem>>,
}

/// Filter component modes
#[derive(Debug, Clone, PartialEq)]
enum FilterMode {
    ViewFilters,    // Viewing current filters
    SelectType,     // Selecting filter type to add
    EditingFilter,  // Editing specific filter
}

impl FilterComponent {
    pub fn new() -> Self {
        let mut filter_state = ListState::default();
        filter_state.select(Some(0));

        let mut type_state = ListState::default();
        type_state.select(Some(0));

        let available_types = vec![
            FilterType::Author,
            FilterType::DateFrom,
            FilterType::DateTo,
            FilterType::FileExtension,
            FilterType::FilePath,
            FilterType::ContentType,
            FilterType::Branch,
            FilterType::Tag,
            FilterType::FileSize,
            FilterType::CommitMessage,
        ];

        let mut preset_filters = HashMap::new();

        // Common preset filters
        preset_filters.insert("Recent Work".to_string(), vec![
            FilterItem::new(FilterType::DateFrom, "2024-01-01".to_string()),
            FilterItem::new(FilterType::Author, "current-user".to_string()),
        ]);

        preset_filters.insert("Source Files".to_string(), vec![
            FilterItem::new(FilterType::FileExtension, ".rs".to_string()),
            FilterItem::new(FilterType::FilePath, "src/*".to_string()),
        ]);

        preset_filters.insert("Documentation".to_string(), vec![
            FilterItem::new(FilterType::FileExtension, ".md".to_string()),
            FilterItem::new(FilterType::FilePath, "docs/*".to_string()),
        ]);

        Self {
            is_open: false,
            filters: Vec::new(),
            filter_state,
            available_types,
            type_state,
            mode: FilterMode::ViewFilters,
            input_modal: InputModal::new(),
            current_editing_index: None,
            preset_filters,
        }
    }

    /// Open filter component
    pub fn open(&mut self) {
        self.is_open = true;
        self.mode = FilterMode::ViewFilters;
        if self.filters.is_empty() {
            self.filter_state.select(None);
        } else {
            self.filter_state.select(Some(0));
        }
    }

    /// Close filter component
    pub fn close(&mut self) {
        self.is_open = false;
        self.mode = FilterMode::ViewFilters;
        self.input_modal.close();
    }

    /// Check if filter component is open
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    /// Add new filter
    pub fn add_filter(&mut self) {
        self.mode = FilterMode::SelectType;
        self.type_state.select(Some(0));
    }

    /// Remove selected filter
    pub fn remove_selected_filter(&mut self) {
        if let Some(index) = self.filter_state.selected() {
            if index < self.filters.len() {
                self.filters.remove(index);

                // Adjust selection
                if self.filters.is_empty() {
                    self.filter_state.select(None);
                } else if index >= self.filters.len() {
                    self.filter_state.select(Some(self.filters.len() - 1));
                }
            }
        }
    }

    /// Edit selected filter
    pub fn edit_selected_filter(&mut self) {
        if let Some(index) = self.filter_state.selected() {
            if let Some(filter) = self.filters.get(index) {
                self.current_editing_index = Some(index);
                self.input_modal.open_with_placeholder(
                    &format!("Edit {}", filter.filter_type.as_str()),
                    filter.filter_type.description(),
                    filter.filter_type.placeholder(),
                );
                self.mode = FilterMode::EditingFilter;
            }
        }
    }

    /// Toggle selected filter active state
    pub fn toggle_selected_filter(&mut self) {
        if let Some(index) = self.filter_state.selected() {
            if let Some(filter) = self.filters.get_mut(index) {
                filter.toggle_active();
            }
        }
    }

    /// Toggle selected filter negation
    pub fn toggle_selected_negation(&mut self) {
        if let Some(index) = self.filter_state.selected() {
            if let Some(filter) = self.filters.get_mut(index) {
                filter.toggle_negated();
            }
        }
    }

    /// Clear all filters
    pub fn clear_all_filters(&mut self) {
        self.filters.clear();
        self.filter_state.select(None);
    }

    /// Apply preset filters
    pub fn apply_preset(&mut self, preset_name: &str) {
        if let Some(preset) = self.preset_filters.get(preset_name).cloned() {
            self.filters.extend(preset);
            if !self.filters.is_empty() {
                self.filter_state.select(Some(0));
            }
        }
    }

    /// Get active filters
    pub fn get_active_filters(&self) -> Vec<&FilterItem> {
        self.filters.iter().filter(|f| f.is_active).collect()
    }

    /// Get all filters (for serialization/persistence)
    pub fn get_all_filters(&self) -> &[FilterItem] {
        &self.filters
    }

    /// Set filters (for deserialization/loading)
    pub fn set_filters(&mut self, filters: Vec<FilterItem>) {
        self.filters = filters;
        if !self.filters.is_empty() {
            self.filter_state.select(Some(0));
        } else {
            self.filter_state.select(None);
        }
    }

    /// Check if any filters match a given item
    pub fn matches_filters(&self, metadata: &HashMap<String, String>) -> bool {
        let active_filters = self.get_active_filters();

        if active_filters.is_empty() {
            return true; // No filters means everything matches
        }

        for filter in active_filters {
            let matches = self.filter_matches(filter, metadata);

            // If negated, we want the opposite
            let result = if filter.is_negated { !matches } else { matches };

            // For now, use AND logic (all filters must match)
            // TODO: Could be extended to support OR logic
            if !result {
                return false;
            }
        }

        true
    }

    /// Check if a single filter matches metadata
    fn filter_matches(&self, filter: &FilterItem, metadata: &HashMap<String, String>) -> bool {
        let value = &filter.value.to_lowercase();

        match filter.filter_type {
            FilterType::Author => {
                metadata.get("author")
                    .map(|a| a.to_lowercase().contains(value))
                    .unwrap_or(false)
            }
            FilterType::DateFrom => {
                metadata.get("date")
                    .map(|d| d >= &filter.value)
                    .unwrap_or(false)
            }
            FilterType::DateTo => {
                metadata.get("date")
                    .map(|d| d <= &filter.value)
                    .unwrap_or(false)
            }
            FilterType::FileExtension => {
                metadata.get("file_path")
                    .map(|p| p.to_lowercase().ends_with(value))
                    .unwrap_or(false)
            }
            FilterType::FilePath => {
                metadata.get("file_path")
                    .map(|p| self.matches_pattern(&p.to_lowercase(), value))
                    .unwrap_or(false)
            }
            FilterType::ContentType => {
                metadata.get("content_type")
                    .map(|c| c.to_lowercase().contains(value))
                    .unwrap_or(false)
            }
            FilterType::Branch => {
                metadata.get("branch")
                    .map(|b| self.matches_pattern(&b.to_lowercase(), value))
                    .unwrap_or(false)
            }
            FilterType::Tag => {
                metadata.get("tag")
                    .map(|t| self.matches_pattern(&t.to_lowercase(), value))
                    .unwrap_or(false)
            }
            FilterType::FileSize => {
                metadata.get("file_size")
                    .and_then(|s| s.parse::<u64>().ok())
                    .map(|size| self.matches_size_filter(size, &filter.value))
                    .unwrap_or(false)
            }
            FilterType::CommitMessage => {
                metadata.get("commit_message")
                    .map(|m| m.to_lowercase().contains(value))
                    .unwrap_or(false)
            }
        }
    }

    /// Simple pattern matching with * wildcard support
    fn matches_pattern(&self, text: &str, pattern: &str) -> bool {
        if pattern.contains('*') {
            // Simple wildcard matching
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 1 {
                return text.contains(pattern);
            }

            let mut current_pos = 0;
            for (i, part) in parts.iter().enumerate() {
                if part.is_empty() {
                    continue;
                }

                if i == 0 {
                    // First part must match from start
                    if !text[current_pos..].starts_with(part) {
                        return false;
                    }
                    current_pos += part.len();
                } else if i == parts.len() - 1 {
                    // Last part must match at end
                    return text[current_pos..].ends_with(part);
                } else {
                    // Middle parts can match anywhere
                    if let Some(pos) = text[current_pos..].find(part) {
                        current_pos += pos + part.len();
                    } else {
                        return false;
                    }
                }
            }
            true
        } else {
            text.contains(pattern)
        }
    }

    /// Match file size filters (e.g., ">1MB", "<100KB", "50KB-2MB")
    fn matches_size_filter(&self, size: u64, filter: &str) -> bool {
        // TODO: Implement size filter parsing and matching
        // This is a placeholder implementation
        true
    }

    /// Render the filter component
    fn render_main_view(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        match self.mode {
            FilterMode::ViewFilters => self.render_filter_list(frame, area, theme),
            FilterMode::SelectType => self.render_type_selection(frame, area, theme),
            FilterMode::EditingFilter => {
                self.render_filter_list(frame, area, theme);
                self.input_modal.render(frame, area, theme);
            }
        }
    }

    /// Render the filter list
    fn render_filter_list(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Split area for header, filters, and help
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Filter list
                Constraint::Length(4), // Help
            ])
            .split(area);

        // Header
        let header_text = format!("Active Filters ({} total, {} active)",
            self.filters.len(),
            self.get_active_filters().len()
        );
        let header = Paragraph::new(header_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Filter Manager")
                    .border_style(Style::default().fg(Color::Blue))
            )
            .style(theme.text_style());
        frame.render_widget(header, chunks[0]);

        // Filter list
        if self.filters.is_empty() {
            let empty_text = vec![
                Line::from("No filters configured"),
                Line::from(""),
                Line::from("Press 'a' to add a filter"),
                Line::from("Press 'p' to apply a preset"),
            ];

            let empty_widget = Paragraph::new(empty_text)
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center);
            frame.render_widget(empty_widget, chunks[1]);
        } else {
            let items: Vec<ListItem> = self.filters.iter().map(|filter| {
                let style = if filter.is_active {
                    Style::default()
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                let color = if filter.is_negated {
                    Color::Red
                } else if filter.is_active {
                    Color::Green
                } else {
                    Color::DarkGray
                };

                ListItem::new(filter.display_text())
                    .style(style.fg(color))
            }).collect();

            let filter_list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Filters"))
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD)
                )
                .highlight_symbol("▶ ");

            frame.render_stateful_widget(filter_list, chunks[1], &mut self.filter_state);
        }

        // Help
        let help_text = vec![
            Line::from("a: Add | d: Delete | e: Edit | Space: Toggle | n: Negate"),
            Line::from("c: Clear All | p: Presets | Enter: Apply | Esc: Close"),
        ];

        let help = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL).title("Help"))
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(help, chunks[2]);
    }

    /// Render type selection
    fn render_type_selection(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items: Vec<ListItem> = self.available_types.iter().map(|filter_type| {
            let line = Line::from(vec![
                Span::styled(filter_type.as_str(), Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" - "),
                Span::styled(filter_type.description(), Style::default().fg(Color::Gray)),
            ]);
            ListItem::new(line)
        }).collect();

        let type_list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Select Filter Type")
                    .border_style(Style::default().fg(Color::Yellow))
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(type_list, area, &mut self.type_state);
    }
}

impl Component for FilterComponent {
    fn render(&mut self, frame: &mut Frame, area: Rect, _state: &AppState, theme: &Theme) {
        if !self.is_open {
            return;
        }

        // Calculate centered area
        let filter_area = Self::centered_rect(80, 60, area);

        // Clear background
        frame.render_widget(Clear, filter_area);

        // Render main content
        self.render_main_view(frame, filter_area, theme);
    }

    fn handle_key_event(&mut self, key: KeyEvent, _state: &mut AppState) -> AppResult<()> {
        if !self.is_open {
            return Ok(());
        }

        // Handle input modal first
        if self.mode == FilterMode::EditingFilter {
            match self.input_modal.handle_key_event(key)? {
                ModalResult::Input(value) => {
                    if let Some(index) = self.current_editing_index {
                        if let Some(filter) = self.filters.get_mut(index) {
                            filter.value = value;
                        }
                    }
                    self.mode = FilterMode::ViewFilters;
                    self.current_editing_index = None;
                }
                ModalResult::Cancelled => {
                    self.mode = FilterMode::ViewFilters;
                    self.current_editing_index = None;
                }
                ModalResult::None => {}
                _ => {}
            }
            return Ok(());
        }

        match self.mode {
            FilterMode::ViewFilters => {
                match key.code {
                    KeyCode::Esc => self.close(),
                    KeyCode::Char('a') => self.add_filter(),
                    KeyCode::Char('d') | KeyCode::Delete => self.remove_selected_filter(),
                    KeyCode::Char('e') | KeyCode::Enter => self.edit_selected_filter(),
                    KeyCode::Char(' ') => self.toggle_selected_filter(),
                    KeyCode::Char('n') => self.toggle_selected_negation(),
                    KeyCode::Char('c') => self.clear_all_filters(),
                    KeyCode::Down => {
                        if !self.filters.is_empty() {
                            let selected = self.filter_state.selected().unwrap_or(0);
                            if selected < self.filters.len() - 1 {
                                self.filter_state.select(Some(selected + 1));
                            }
                        }
                    }
                    KeyCode::Up => {
                        if !self.filters.is_empty() {
                            let selected = self.filter_state.selected().unwrap_or(0);
                            if selected > 0 {
                                self.filter_state.select(Some(selected - 1));
                            }
                        }
                    }
                    _ => {}
                }
            }
            FilterMode::SelectType => {
                match key.code {
                    KeyCode::Esc => self.mode = FilterMode::ViewFilters,
                    KeyCode::Enter => {
                        if let Some(index) = self.type_state.selected() {
                            if let Some(filter_type) = self.available_types.get(index).cloned() {
                                self.input_modal.open_with_placeholder(
                                    &format!("Add {}", filter_type.as_str()),
                                    filter_type.description(),
                                    filter_type.placeholder(),
                                );
                                self.current_editing_index = None; // New filter
                                self.mode = FilterMode::EditingFilter;
                            }
                        }
                    }
                    KeyCode::Down => {
                        let selected = self.type_state.selected().unwrap_or(0);
                        if selected < self.available_types.len() - 1 {
                            self.type_state.select(Some(selected + 1));
                        }
                    }
                    KeyCode::Up => {
                        let selected = self.type_state.selected().unwrap_or(0);
                        if selected > 0 {
                            self.type_state.select(Some(selected - 1));
                        }
                    }
                    _ => {}
                }
            }
            FilterMode::EditingFilter => {
                // Handled above in input modal section
            }
        }

        Ok(())
    }
}

impl FilterComponent {
    /// Calculate centered rectangle
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
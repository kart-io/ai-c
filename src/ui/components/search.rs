//! Global search and filter system

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use std::collections::HashMap;
use tokio::sync::mpsc;

use crate::{
    app::state::AppState,
    error::AppResult,
    git::service::GitService,
    ui::theme::Theme,
};

use super::Component;

/// Search scope types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SearchScope {
    All,
    Files,
    Commits,
    Branches,
    Tags,
    Stash,
    Remotes,
    Content,
}

impl SearchScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            SearchScope::All => "All",
            SearchScope::Files => "Files",
            SearchScope::Commits => "Commits",
            SearchScope::Branches => "Branches",
            SearchScope::Tags => "Tags",
            SearchScope::Stash => "Stash",
            SearchScope::Remotes => "Remotes",
            SearchScope::Content => "Content",
        }
    }
}

/// Search result item
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub title: String,
    pub content: String,
    pub scope: SearchScope,
    pub file_path: Option<String>,
    pub line_number: Option<usize>,
    pub commit_hash: Option<String>,
    pub branch_name: Option<String>,
    pub relevance_score: f32,
}

/// Filter criteria
#[derive(Debug, Clone)]
pub struct FilterCriteria {
    pub author: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub file_extension: Option<String>,
    pub file_path: Option<String>,
    pub content_type: Option<String>,
}

impl Default for FilterCriteria {
    fn default() -> Self {
        Self {
            author: None,
            date_from: None,
            date_to: None,
            file_extension: None,
            file_path: None,
            content_type: None,
        }
    }
}

/// Search and filter component
pub struct SearchComponent {
    is_active: bool,
    query: String,
    cursor_position: usize,
    scope: SearchScope,
    scope_index: usize,
    results: Vec<SearchResult>,
    result_state: ListState,
    filter_criteria: FilterCriteria,
    is_filter_open: bool,
    search_history: Vec<String>,
    history_index: Option<usize>,
    search_sender: Option<mpsc::UnboundedSender<SearchRequest>>,
    is_searching: bool,
}

/// Search request for async processing
#[derive(Debug)]
pub struct SearchRequest {
    pub query: String,
    pub scope: SearchScope,
    pub filter: FilterCriteria,
}

impl SearchComponent {
    pub fn new() -> Self {
        let mut result_state = ListState::default();
        result_state.select(Some(0));

        Self {
            is_active: false,
            query: String::new(),
            cursor_position: 0,
            scope: SearchScope::All,
            scope_index: 0,
            results: Vec::new(),
            result_state,
            filter_criteria: FilterCriteria::default(),
            is_filter_open: false,
            search_history: Vec::new(),
            history_index: None,
            search_sender: None,
            is_searching: false,
        }
    }

    /// Activate search component
    pub fn activate(&mut self) {
        self.is_active = true;
        self.query.clear();
        self.cursor_position = 0;
        self.results.clear();
        self.result_state.select(Some(0));
    }

    /// Deactivate search component
    pub fn deactivate(&mut self) {
        self.is_active = false;
        self.is_filter_open = false;
    }

    /// Check if search is active
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    /// Get current query
    pub fn get_query(&self) -> &str {
        &self.query
    }

    /// Set query programmatically
    pub fn set_query(&mut self, query: &str) {
        self.query = query.to_string();
        self.cursor_position = query.len();
    }

    /// Toggle filter panel
    pub fn toggle_filter(&mut self) {
        self.is_filter_open = !self.is_filter_open;
    }

    /// Perform search with current query and scope
    pub async fn perform_search(&mut self, git_service: &GitService) -> AppResult<()> {
        if self.query.is_empty() {
            self.results.clear();
            self.result_state.select(Some(0));
            return Ok(());
        }

        self.is_searching = true;

        // Add to search history
        if !self.search_history.contains(&self.query) {
            self.search_history.push(self.query.clone());
            if self.search_history.len() > 50 {
                self.search_history.remove(0);
            }
        }

        let mut results = Vec::new();

        match self.scope {
            SearchScope::All => {
                results.extend(self.search_files(git_service).await?);
                results.extend(self.search_commits(git_service).await?);
                results.extend(self.search_branches(git_service).await?);
                results.extend(self.search_tags(git_service).await?);
                results.extend(self.search_content(git_service).await?);
            }
            SearchScope::Files => {
                results.extend(self.search_files(git_service).await?);
            }
            SearchScope::Commits => {
                results.extend(self.search_commits(git_service).await?);
            }
            SearchScope::Branches => {
                results.extend(self.search_branches(git_service).await?);
            }
            SearchScope::Tags => {
                results.extend(self.search_tags(git_service).await?);
            }
            SearchScope::Stash => {
                results.extend(self.search_stash(git_service).await?);
            }
            SearchScope::Remotes => {
                results.extend(self.search_remotes(git_service).await?);
            }
            SearchScope::Content => {
                results.extend(self.search_content(git_service).await?);
            }
        }

        // Sort by relevance score
        results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(std::cmp::Ordering::Equal));

        self.results = results;
        self.result_state.select(Some(0));
        self.is_searching = false;

        Ok(())
    }

    /// Search files
    async fn search_files(&self, git_service: &GitService) -> AppResult<Vec<SearchResult>> {
        let mut results = Vec::new();
        let status = git_service.get_status().await?;

        for file in &status {
            if file.path.to_lowercase().contains(&self.query.to_lowercase()) {
                let relevance = self.calculate_relevance(&file.path, &self.query);
                results.push(SearchResult {
                    title: file.path.clone(),
                    content: format!("Status: {:?}", file.status),
                    scope: SearchScope::Files,
                    file_path: Some(file.path.clone()),
                    line_number: None,
                    commit_hash: None,
                    branch_name: None,
                    relevance_score: relevance,
                });
            }
        }

        Ok(results)
    }

    /// Search commits
    async fn search_commits(&self, git_service: &GitService) -> AppResult<Vec<SearchResult>> {
        let mut results = Vec::new();
        let commits = git_service.get_commits(100).await?;

        for commit in &commits {
            let message_match = commit.message.to_lowercase().contains(&self.query.to_lowercase());
            let author_match = commit.author.to_lowercase().contains(&self.query.to_lowercase());
            let hash_match = commit.hash.to_lowercase().starts_with(&self.query.to_lowercase());

            if message_match || author_match || hash_match {
                let relevance = if hash_match {
                    1.0
                } else if author_match {
                    0.8
                } else {
                    self.calculate_relevance(&commit.message, &self.query)
                };

                results.push(SearchResult {
                    title: format!("{} - {}", &commit.hash[..8], commit.message.lines().next().unwrap_or("")),
                    content: format!("Author: {} | Date: {}", commit.author, commit.date),
                    scope: SearchScope::Commits,
                    file_path: None,
                    line_number: None,
                    commit_hash: Some(commit.hash.clone()),
                    branch_name: None,
                    relevance_score: relevance,
                });
            }
        }

        Ok(results)
    }

    /// Search branches
    async fn search_branches(&self, git_service: &GitService) -> AppResult<Vec<SearchResult>> {
        let mut results = Vec::new();
        let branches = git_service.get_branches().await?;

        for branch in &branches {
            if branch.name.to_lowercase().contains(&self.query.to_lowercase()) {
                let relevance = self.calculate_relevance(&branch.name, &self.query);
                results.push(SearchResult {
                    title: branch.name.clone(),
                    content: format!("Type: {} | Current: {}",
                        if branch.is_local { "Local" } else { "Remote" },
                        if branch.is_current { "Yes" } else { "No" }
                    ),
                    scope: SearchScope::Branches,
                    file_path: None,
                    line_number: None,
                    commit_hash: None,
                    branch_name: Some(branch.name.clone()),
                    relevance_score: relevance,
                });
            }
        }

        Ok(results)
    }

    /// Search tags
    async fn search_tags(&self, git_service: &GitService) -> AppResult<Vec<SearchResult>> {
        let mut results = Vec::new();
        let tags = git_service.get_tags().await?;

        for tag in &tags {
            if tag.name.to_lowercase().contains(&self.query.to_lowercase()) {
                let relevance = self.calculate_relevance(&tag.name, &self.query);
                results.push(SearchResult {
                    title: tag.name.clone(),
                    content: format!("Target: {}", &tag.target[..8]),
                    scope: SearchScope::Tags,
                    file_path: None,
                    line_number: None,
                    commit_hash: Some(tag.target.clone()),
                    branch_name: None,
                    relevance_score: relevance,
                });
            }
        }

        Ok(results)
    }

    /// Search stash
    async fn search_stash(&self, _git_service: &GitService) -> AppResult<Vec<SearchResult>> {
        // TODO: Implement stash search when stash functionality is available
        Ok(Vec::new())
    }

    /// Search remotes
    async fn search_remotes(&self, git_service: &GitService) -> AppResult<Vec<SearchResult>> {
        let mut results = Vec::new();
        let remotes = git_service.get_remotes().await?;

        for remote in &remotes {
            if remote.name.to_lowercase().contains(&self.query.to_lowercase()) ||
               remote.url.to_lowercase().contains(&self.query.to_lowercase()) {
                let relevance = self.calculate_relevance(&remote.name, &self.query)
                    .max(self.calculate_relevance(&remote.url, &self.query));

                results.push(SearchResult {
                    title: remote.name.clone(),
                    content: remote.url.clone(),
                    scope: SearchScope::Remotes,
                    file_path: None,
                    line_number: None,
                    commit_hash: None,
                    branch_name: None,
                    relevance_score: relevance,
                });
            }
        }

        Ok(results)
    }

    /// Search file content
    async fn search_content(&self, _git_service: &GitService) -> AppResult<Vec<SearchResult>> {
        // TODO: Implement content search with ripgrep or similar
        // This would search within file contents for the query
        Ok(Vec::new())
    }

    /// Calculate relevance score for search results
    fn calculate_relevance(&self, text: &str, query: &str) -> f32 {
        let text_lower = text.to_lowercase();
        let query_lower = query.to_lowercase();

        if text_lower == query_lower {
            1.0
        } else if text_lower.starts_with(&query_lower) {
            0.9
        } else if text_lower.contains(&query_lower) {
            0.7
        } else {
            // Fuzzy matching score could be implemented here
            0.5
        }
    }

    /// Get selected search result
    pub fn get_selected_result(&self) -> Option<&SearchResult> {
        if let Some(index) = self.result_state.selected() {
            self.results.get(index)
        } else {
            None
        }
    }

    /// Cycle through search scopes
    fn next_scope(&mut self) {
        self.scope_index = (self.scope_index + 1) % 8;
        self.scope = match self.scope_index {
            0 => SearchScope::All,
            1 => SearchScope::Files,
            2 => SearchScope::Commits,
            3 => SearchScope::Branches,
            4 => SearchScope::Tags,
            5 => SearchScope::Stash,
            6 => SearchScope::Remotes,
            7 => SearchScope::Content,
            _ => SearchScope::All,
        };
    }

    fn previous_scope(&mut self) {
        self.scope_index = if self.scope_index == 0 { 7 } else { self.scope_index - 1 };
        self.scope = match self.scope_index {
            0 => SearchScope::All,
            1 => SearchScope::Files,
            2 => SearchScope::Commits,
            3 => SearchScope::Branches,
            4 => SearchScope::Tags,
            5 => SearchScope::Stash,
            6 => SearchScope::Remotes,
            7 => SearchScope::Content,
            _ => SearchScope::All,
        };
    }

    /// Render search interface
    fn render_search_interface(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Search layout: scope selector + input + results
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Scope selector
                Constraint::Length(3), // Search input
                Constraint::Min(0),    // Results
            ])
            .split(area);

        // Scope selector
        let scope_text = format!("Scope: {}", self.scope.as_str());
        let scope_widget = Paragraph::new(scope_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Search Scope (Space/Shift+Tab to change)")
                    .border_style(Style::default().fg(Color::Blue))
            )
            .style(theme.text_style());
        frame.render_widget(scope_widget, chunks[0]);

        // Search input
        let input_text = if self.query.is_empty() {
            "Type to search..."
        } else {
            &self.query
        };

        let input_style = if self.query.is_empty() {
            Style::default().fg(Color::DarkGray)
        } else {
            theme.text_style()
        };

        let search_widget = Paragraph::new(input_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Search Query (F3: Filter, Enter: Search)")
                    .border_style(Style::default().fg(Color::Green))
            )
            .style(input_style);
        frame.render_widget(search_widget, chunks[1]);

        // Results
        self.render_results(frame, chunks[2], theme);
    }

    /// Render search results
    fn render_results(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if self.is_searching {
            let searching_text = "Searching...";
            let searching_widget = Paragraph::new(searching_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Search Results")
                )
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center);
            frame.render_widget(searching_widget, area);
            return;
        }

        if self.results.is_empty() {
            let no_results_text = if self.query.is_empty() {
                "Enter a search query to begin"
            } else {
                "No results found"
            };

            let no_results_widget = Paragraph::new(no_results_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Search Results")
                )
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center);
            frame.render_widget(no_results_widget, area);
            return;
        }

        let items: Vec<ListItem> = self.results.iter().map(|result| {
            let scope_indicator = match result.scope {
                SearchScope::Files => "📁",
                SearchScope::Commits => "📝",
                SearchScope::Branches => "🌿",
                SearchScope::Tags => "🏷️",
                SearchScope::Stash => "📦",
                SearchScope::Remotes => "🌐",
                SearchScope::Content => "🔍",
                SearchScope::All => "📋",
            };

            let title_line = Line::from(vec![
                Span::styled(scope_indicator, Style::default()),
                Span::raw(" "),
                Span::styled(&result.title, Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" "),
                Span::styled(
                    format!("({:.1})", result.relevance_score),
                    Style::default().fg(Color::DarkGray)
                ),
            ]);

            let content_line = Line::from(vec![
                Span::raw("  "),
                Span::styled(&result.content, Style::default().fg(Color::Gray)),
            ]);

            ListItem::new(vec![title_line, content_line])
        }).collect();

        let results_list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Search Results ({})", self.results.len()))
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(results_list, area, &mut self.result_state);
    }

    /// Render filter panel (when open)
    fn render_filter_panel(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.is_filter_open {
            return;
        }

        // Calculate centered area for filter panel
        let filter_area = Self::centered_rect(60, 40, area);

        // Clear background
        frame.render_widget(ratatui::widgets::Clear, filter_area);

        // Filter panel content
        let filter_text = vec![
            Line::from("Filter Options:"),
            Line::from(""),
            Line::from(format!("Author: {}", self.filter_criteria.author.as_deref().unwrap_or("Any"))),
            Line::from(format!("Date From: {}", self.filter_criteria.date_from.as_deref().unwrap_or("Any"))),
            Line::from(format!("Date To: {}", self.filter_criteria.date_to.as_deref().unwrap_or("Any"))),
            Line::from(format!("File Extension: {}", self.filter_criteria.file_extension.as_deref().unwrap_or("Any"))),
            Line::from(format!("File Path: {}", self.filter_criteria.file_path.as_deref().unwrap_or("Any"))),
            Line::from(""),
            Line::from("Press F3 to close, 1-5 to edit filters"),
        ];

        let filter_widget = Paragraph::new(filter_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Search Filters")
                    .border_style(Style::default().fg(Color::Cyan))
            )
            .style(theme.text_style())
            .wrap(Wrap { trim: true });

        frame.render_widget(filter_widget, filter_area);
    }

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

impl Component for SearchComponent {
    fn render(&mut self, frame: &mut Frame, area: Rect, _state: &AppState, theme: &Theme) {
        if !self.is_active {
            return;
        }

        // Main search interface
        self.render_search_interface(frame, area, theme);

        // Overlay filter panel if open
        if self.is_filter_open {
            self.render_filter_panel(frame, area, theme);
        }
    }

    fn handle_key_event(&mut self, key: KeyEvent, state: &mut AppState) -> AppResult<()> {
        if !self.is_active {
            return Ok(());
        }

        if self.is_filter_open {
            match key.code {
                KeyCode::F(3) => {
                    self.is_filter_open = false;
                }
                KeyCode::Esc => {
                    self.is_filter_open = false;
                }
                _ => {}
            }
            return Ok(());
        }

        match key.code {
            KeyCode::Esc => {
                self.deactivate();
            }
            KeyCode::F(3) => {
                self.toggle_filter();
            }
            KeyCode::Char(' ') => {
                self.next_scope();
            }
            KeyCode::BackTab => {
                self.previous_scope();
            }
            KeyCode::Enter => {
                if let Some(_git_service) = &state.git_service {
                    // TODO: Implement search functionality
                    // Currently disabled due to Send/Sync constraints with git2
                    self.is_searching = true;
                }
            }
            KeyCode::Down => {
                let selected = self.result_state.selected().unwrap_or(0);
                if selected < self.results.len().saturating_sub(1) {
                    self.result_state.select(Some(selected + 1));
                }
            }
            KeyCode::Up => {
                let selected = self.result_state.selected().unwrap_or(0);
                if selected > 0 {
                    self.result_state.select(Some(selected - 1));
                }
            }
            KeyCode::Char(c) => {
                self.query.insert(self.cursor_position, c);
                self.cursor_position += 1;
            }
            KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                    self.query.remove(self.cursor_position);
                }
            }
            KeyCode::Delete => {
                if self.cursor_position < self.query.len() {
                    self.query.remove(self.cursor_position);
                }
            }
            KeyCode::Left => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_position < self.query.len() {
                    self.cursor_position += 1;
                }
            }
            KeyCode::Home => {
                self.cursor_position = 0;
            }
            KeyCode::End => {
                self.cursor_position = self.query.len();
            }
            _ => {}
        }

        Ok(())
    }
}

impl Clone for SearchComponent {
    fn clone(&self) -> Self {
        Self {
            is_active: self.is_active,
            query: self.query.clone(),
            cursor_position: self.cursor_position,
            scope: self.scope.clone(),
            scope_index: self.scope_index,
            results: self.results.clone(),
            result_state: ListState::default(),
            filter_criteria: self.filter_criteria.clone(),
            is_filter_open: self.is_filter_open,
            search_history: self.search_history.clone(),
            history_index: self.history_index,
            search_sender: None,
            is_searching: self.is_searching,
        }
    }
}
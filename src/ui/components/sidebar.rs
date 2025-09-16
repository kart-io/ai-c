//! Sidebar component
//!
//! Displays repository information and dynamic content based on current tab.

use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::{
    app::state::{AppState, FocusArea},
    error::AppResult,
    ui::{components::Component, theme::Theme},
};

/// Sidebar component showing repository info and dynamic lists
pub struct SidebarComponent {
    // Component state can be added here
}

impl SidebarComponent {
    pub fn new() -> Self {
        Self {}
    }
}

impl Component for SidebarComponent {
    fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        state: &AppState,
        theme: &Theme,
    ) {
        // Split sidebar into sections
        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6), // Repository info
                Constraint::Min(0),    // Dynamic content based on current tab
            ])
            .split(area);

        // Repository section
        self.render_repository_info(frame, sections[0], state, theme);

        // Dynamic content section based on current tab
        self.render_dynamic_content(frame, sections[1], state, theme);
    }

    fn handle_key_event(&mut self, _key: KeyEvent, _state: &mut AppState) -> AppResult<()> {
        // Sidebar doesn't handle keys directly in this implementation
        Ok(())
    }
}

impl SidebarComponent {
    fn render_repository_info(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &AppState,
        theme: &Theme,
    ) {
        let repo_info = if state.git_state.is_repository {
            vec![
                format!("📁 Repository: ✓"),
                format!(
                    "📍 Path: {}",
                    state
                        .git_state
                        .repository_path
                        .as_deref()
                        .unwrap_or("Unknown")
                ),
                format!("📊 Files: {}", state.git_state.file_status.len()),
                format!(
                    "🕒 Updated: {}s ago",
                    (chrono::Utc::now() - state.git_state.last_status_update).num_seconds()
                ),
            ]
        } else {
            vec![
                "📁 Repository: ✗".to_string(),
                "Not a Git repository".to_string(),
                "".to_string(),
                "".to_string(),
            ]
        };

        let items: Vec<ListItem> = repo_info
            .into_iter()
            .map(|info| ListItem::new(info))
            .collect();

        let list = List::new(items)
            .style(Style::default().bg(theme.colors.secondary).fg(theme.colors.foreground)) // VS Code sidebar styling
            .block(
                Block::default()
                    .title("Repository")
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()) // Use VS Code border style
                    .title_style(theme.text_style()),
            );

        frame.render_widget(list, area);
    }

    fn render_dynamic_content(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &AppState,
        theme: &Theme,
    ) {
        use crate::app::state::TabType;
        match state.ui_state.current_tab {
            TabType::Branches => self.render_branches_list(frame, area, state, theme),
            TabType::Tags => self.render_tags_list(frame, area, state, theme),
            TabType::Stash => self.render_stash_list(frame, area, state, theme),
            TabType::Status => self.render_status_list(frame, area, state, theme),
            TabType::Remotes => self.render_remotes_list(frame, area, state, theme),
            TabType::GitFlow => self.render_gitflow_list(frame, area, state, theme),
        }
    }

    fn render_branches_list(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &AppState,
        theme: &Theme,
    ) {
        // Get branches from git service if available
        let branch_items = if let Some(git_service) = &state.git_service {
            match git_service.list_branches() {
                Ok(mut branches) => {
                    // Update branch current status based on AppState
                    if let Some(ref current_branch) = state.git_state.current_branch {
                        for branch in &mut branches {
                            branch.is_current = branch.name == current_branch.name;
                        }
                    }

                    branches
                        .into_iter()
                        .enumerate()
                        .map(|(i, branch)| {
                            // Different prefixes for different branch types
                            let prefix = if branch.is_current {
                                "* " // Current branch
                            } else if branch.is_remote {
                                "◦ " // Remote branch (empty circle)
                            } else {
                                "• " // Local branch (filled dot)
                            };

                            let is_focused = state.ui_state.current_focus == FocusArea::Sidebar;
                            let is_selected = i == state.ui_state.sidebar_selected_index;

                            // Create branch display with additional info
                            let branch_display = if branch.is_remote {
                                format!("{}{}", prefix, branch.name) // Simple for remote branches
                            } else if let Some(ref upstream) = branch.upstream {
                                if branch.ahead > 0 || branch.behind > 0 {
                                    format!("{}{} [↑{} ↓{}]", prefix, branch.name, branch.ahead, branch.behind)
                                } else {
                                    format!("{}{} [↔]", prefix, branch.name) // In sync
                                }
                            } else {
                                format!("{}{}", prefix, branch.name) // No upstream
                            };

                            let style = if branch.is_current {
                                theme.success_style() // Current branch in green
                            } else if branch.is_remote {
                                theme.muted_style() // Remote branches muted
                            } else if is_focused && is_selected {
                                theme.highlight_style() // Selected item highlighted
                            } else if is_focused {
                                theme.text_style() // Focused area normal text
                            } else {
                                theme.muted_style() // Unfocused area muted
                            };

                            ListItem::new(branch_display).style(style)
                        })
                        .collect()
                },
                Err(_) => vec![ListItem::new("Error loading branches")],
            }
        } else {
            vec![ListItem::new("Git service not available")]
        };

        let is_focused = state.ui_state.current_focus == FocusArea::Sidebar;
        let branch_count = branch_items.len();
        let title = if is_focused {
            format!("Branches [FOCUSED] ({})", branch_count)
        } else {
            format!("Branches [Space to focus] ({})", branch_count)
        };

        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(state.ui_state.sidebar_selected_index));

        let list = List::new(branch_items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(if is_focused { Style::default().fg(theme.colors.accent) } else { theme.border_style() })
                    .title_style(theme.text_style()),
            )
            .style(Style::default().bg(theme.colors.secondary).fg(theme.colors.foreground))
            .highlight_style(theme.highlight_style())
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, area, &mut list_state);
    }

    fn render_tags_list(&self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        // Get tags from git service if available
        let tag_items = if let Some(git_service) = &state.git_service {
            // Use async runtime to call the async method
            match tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(git_service.list_tags())
            }) {
                Ok(mut tags) => {
                    // Sort tags by name (semantic version sorting would be better, but alphabetical works for now)
                    tags.sort_by(|a, b| {
                        // Try to sort by semantic version, fallback to string comparison
                        use std::cmp::Ordering;

                        // Extract version numbers if possible
                        let parse_version = |tag_name: &str| -> Option<(u32, u32, u32)> {
                            let version_part = tag_name.trim_start_matches('v');
                            let parts: Vec<&str> = version_part.split('.').collect();
                            if parts.len() >= 3 {
                                if let (Ok(major), Ok(minor), Ok(patch)) = (
                                    parts[0].parse::<u32>(),
                                    parts[1].parse::<u32>(),
                                    parts[2].parse::<u32>(),
                                ) {
                                    return Some((major, minor, patch));
                                }
                            }
                            None
                        };

                        match (parse_version(&a.name), parse_version(&b.name)) {
                            (Some(a_ver), Some(b_ver)) => b_ver.cmp(&a_ver), // Reverse order for newest first
                            _ => b.name.cmp(&a.name), // Fallback to reverse alphabetical
                        }
                    });

                    tags.into_iter()
                        .enumerate()
                        .map(|(i, tag)| {
                            let is_focused = state.ui_state.current_focus == FocusArea::Sidebar;
                            let is_selected = i == state.ui_state.sidebar_selected_index;

                            // Format tag display with additional information
                            let tag_display = if let Some(ref message) = tag.message {
                                let short_commit = if tag.target_commit.len() >= 8 {
                                    &tag.target_commit[..8]
                                } else {
                                    &tag.target_commit
                                };
                                format!("🏷️  {} ({}) - {}", tag.name, short_commit,
                                    message.lines().next().unwrap_or("").trim())
                            } else {
                                let short_commit = if tag.target_commit.len() >= 8 {
                                    &tag.target_commit[..8]
                                } else {
                                    &tag.target_commit
                                };
                                format!("🏷️  {} ({})", tag.name, short_commit)
                            };

                            let style = if is_focused && is_selected {
                                theme.highlight_style() // Selected item highlighted
                            } else if is_focused {
                                theme.text_style() // Focused area normal text
                            } else {
                                theme.muted_style() // Unfocused area muted
                            };

                            ListItem::new(tag_display).style(style)
                        })
                        .collect()
                },
                Err(_) => vec![ListItem::new("Error loading tags")],
            }
        } else {
            vec![ListItem::new("Git service not available")]
        };

        let is_focused = state.ui_state.current_focus == FocusArea::Sidebar;
        let tag_count = tag_items.len();
        let title = if is_focused {
            format!("Tags [FOCUSED] ({})", tag_count)
        } else {
            format!("Tags [Space to focus] ({})", tag_count)
        };

        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(state.ui_state.sidebar_selected_index));

        let list = List::new(tag_items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(if is_focused { Style::default().fg(theme.colors.accent) } else { theme.border_style() })
                    .title_style(theme.text_style()),
            )
            .style(Style::default().bg(theme.colors.secondary).fg(theme.colors.foreground))
            .highlight_style(theme.highlight_style())
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, area, &mut list_state);
    }

    fn render_stash_list(&self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        // Get stash entries from git service if available
        let stash_items = if let Some(git_service) = &state.git_service {
            // Use async runtime to call the async method
            match tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(git_service.list_stash())
            }) {
                Ok(mut stashes) => {
                    // Sort stashes by index (newest first, which is how Git stores them)
                    stashes.sort_by(|a, b| a.index.cmp(&b.index));

                    stashes.into_iter()
                        .enumerate()
                        .map(|(i, stash)| {
                            let is_focused = state.ui_state.current_focus == FocusArea::Sidebar;
                            let is_selected = i == state.ui_state.sidebar_selected_index;

                            // Format stash display with detailed information
                            let time_ago = {
                                let now = chrono::Utc::now();
                                let duration = now.signed_duration_since(stash.date);

                                if duration.num_days() > 0 {
                                    format!("{} days ago", duration.num_days())
                                } else if duration.num_hours() > 0 {
                                    format!("{} hours ago", duration.num_hours())
                                } else if duration.num_minutes() > 0 {
                                    format!("{} minutes ago", duration.num_minutes())
                                } else {
                                    "Just now".to_string()
                                }
                            };

                            let stash_display = format!(
                                "📦 stash@{{{}}} on {} - {} ({})",
                                stash.index,
                                stash.branch,
                                stash.message,
                                time_ago
                            );

                            let style = if is_focused && is_selected {
                                theme.highlight_style() // Selected item highlighted
                            } else if is_focused {
                                theme.text_style() // Focused area normal text
                            } else {
                                theme.muted_style() // Unfocused area muted
                            };

                            ListItem::new(stash_display).style(style)
                        })
                        .collect()
                },
                Err(_) => vec![ListItem::new("Error loading stash entries")],
            }
        } else {
            vec![ListItem::new("Git service not available")]
        };

        let is_focused = state.ui_state.current_focus == FocusArea::Sidebar;
        let stash_count = stash_items.len();
        let title = if is_focused {
            format!("Stash [FOCUSED] ({})", stash_count)
        } else {
            format!("Stash [Space to focus] ({})", stash_count)
        };

        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(state.ui_state.sidebar_selected_index));

        let list = List::new(stash_items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(if is_focused { Style::default().fg(theme.colors.accent) } else { theme.border_style() })
                    .title_style(theme.text_style()),
            )
            .style(Style::default().bg(theme.colors.secondary).fg(theme.colors.foreground))
            .highlight_style(theme.highlight_style())
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, area, &mut list_state);
    }

    fn render_status_list(&self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        let file_items = if let Some(git_service) = &state.git_service {
            // Use Git service to get comprehensive status data
            match tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(git_service.get_status())
            }) {
                Ok(mut files) => {
                    // Sort files by status priority and then by path
                    files.sort_by(|a, b| {
                        // Priority order: conflicted, staged, modified, untracked, deleted
                        let priority_a = if a.status.conflicted {
                            0
                        } else if a.status.is_staged() {
                            1
                        } else if a.status.is_modified() {
                            2
                        } else if a.status.is_untracked() {
                            3
                        } else if a.status.wt_deleted || a.status.index_deleted {
                            4
                        } else {
                            5
                        };

                        let priority_b = if b.status.conflicted {
                            0
                        } else if b.status.is_staged() {
                            1
                        } else if b.status.is_modified() {
                            2
                        } else if b.status.is_untracked() {
                            3
                        } else if b.status.wt_deleted || b.status.index_deleted {
                            4
                        } else {
                            5
                        };

                        priority_a.cmp(&priority_b).then_with(|| a.path.cmp(&b.path))
                    });

                    files
                        .iter()
                        .map(|file| {
                            // Enhanced status display with proper Git status symbols
                            let (status_display, color) = if file.status.conflicted {
                                ("🔥 C", Color::Magenta) // Conflicted - highest priority
                            } else if file.status.is_staged() && file.status.is_modified() {
                                ("📋 MM", Color::Yellow) // Staged and modified
                            } else if file.status.index_new {
                                ("📋 A", Color::Green) // Staged - new file
                            } else if file.status.index_modified {
                                ("📋 M", Color::Green) // Staged - modified
                            } else if file.status.index_deleted {
                                ("📋 D", Color::Green) // Staged - deleted
                            } else if file.status.index_renamed {
                                ("📋 R", Color::Green) // Staged - renamed
                            } else if file.status.wt_modified {
                                ("📝 M", theme.colors.warning) // Modified
                            } else if file.status.wt_deleted {
                                ("🗑️ D", Color::Red) // Deleted
                            } else if file.status.wt_renamed {
                                ("📁 R", theme.colors.info) // Renamed
                            } else if file.status.is_untracked() {
                                ("❓ ??", Color::Cyan) // Untracked
                            } else if file.status.ignored {
                                ("🚫 I", Color::DarkGray) // Ignored
                            } else {
                                ("   ", theme.colors.foreground) // No change
                            };

                            // Create display with file size for non-empty files
                            let size_display = if file.size > 0 {
                                if file.size < 1024 {
                                    format!(" ({}B)", file.size)
                                } else if file.size < 1024 * 1024 {
                                    format!(" ({:.1}KB)", file.size as f64 / 1024.0)
                                } else {
                                    format!(" ({:.1}MB)", file.size as f64 / (1024.0 * 1024.0))
                                }
                            } else {
                                "".to_string()
                            };

                            let binary_indicator = if file.is_binary { " 📁" } else { "" };

                            ListItem::new(format!("{} {}{}{}", status_display, file.path, size_display, binary_indicator))
                                .style(Style::default().fg(color))
                        })
                        .collect()
                },
                Err(_) => {
                    // Fallback to state data if Git service fails
                    state
                        .git_state
                        .file_status
                        .iter()
                        .map(|file| {
                            let (status_char, color) = if file.status.conflicted {
                                ("C", Color::Magenta)
                            } else if file.status.wt_modified || file.status.index_modified {
                                ("M", theme.colors.warning)
                            } else if file.status.wt_new || file.status.index_new {
                                ("A", Color::Green)
                            } else if file.status.wt_deleted || file.status.index_deleted {
                                ("D", Color::Red)
                            } else if file.status.is_untracked() {
                                ("??", theme.colors.info)
                            } else {
                                (" ", theme.colors.foreground)
                            };
                            ListItem::new(format!("{} {}", status_char, file.path))
                                .style(Style::default().fg(color))
                        })
                        .collect()
                }
            }
        } else {
            // Fallback when no Git service available
            vec![ListItem::new("No Git service available").style(Style::default().fg(theme.colors.error))]
        };

        let is_focused = state.ui_state.current_focus == FocusArea::Sidebar;
        let title = if is_focused {
            format!("Status [FOCUSED] ({})", file_items.len())
        } else {
            format!("Status [Space to focus] ({})", file_items.len())
        };

        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(state.ui_state.sidebar_selected_index));

        let list = List::new(file_items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(if is_focused { Style::default().fg(theme.colors.accent) } else { theme.border_style() })
                    .title_style(theme.text_style()),
            )
            .style(Style::default().bg(theme.colors.secondary).fg(theme.colors.foreground))
            .highlight_style(theme.highlight_style())
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, area, &mut list_state);
    }

    fn render_remotes_list(&self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        let remote_items = vec![
            ListItem::new("origin (fetch)"),
            ListItem::new("origin (push)"),
            ListItem::new("upstream (fetch)"),
        ];

        let is_focused = state.ui_state.current_focus == FocusArea::Sidebar;
        let title = if is_focused { "Remotes [FOCUSED]" } else { "Remotes [Space to focus]" };

        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(state.ui_state.sidebar_selected_index));

        let list = List::new(remote_items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(if is_focused { Style::default().fg(theme.colors.accent) } else { theme.border_style() }),
            )
            .style(Style::default().bg(theme.colors.secondary).fg(theme.colors.foreground))
            .highlight_style(theme.highlight_style())
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, area, &mut list_state);
    }

    fn render_gitflow_list(&self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        let gitflow_items = vec![
            ListItem::new("feature/ui-improvements"),
            ListItem::new("release/v2.1.0"),
            ListItem::new("hotfix/critical-bug"),
        ];

        let is_focused = state.ui_state.current_focus == FocusArea::Sidebar;
        let title = if is_focused { "Git Flow [FOCUSED]" } else { "Git Flow [Space to focus]" };

        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(state.ui_state.sidebar_selected_index));

        let list = List::new(gitflow_items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(if is_focused { Style::default().fg(theme.colors.accent) } else { theme.border_style() }),
            )
            .style(Style::default().bg(theme.colors.secondary).fg(theme.colors.foreground))
            .highlight_style(theme.highlight_style())
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, area, &mut list_state);
    }

    fn render_empty_list(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let empty_items = vec![ListItem::new("No data available")];

        let list = List::new(empty_items)
            .block(
                Block::default()
                    .title("Content")
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            )
            .style(Style::default().bg(theme.colors.secondary).fg(theme.colors.foreground));

        frame.render_widget(list, area);
    }
}
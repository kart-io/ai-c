//! Agent system UI panel component

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
    Frame,
};
use tracing::debug;

use crate::{
    ai::{AgentStatus, AgentType},
    app::state::AppState,
    error::AppResult,
    ui::theme::Theme,
};

/// Agent system panel component
pub struct AgentPanelComponent {
    selected_index: usize,
    show_details: bool,
}

impl AgentPanelComponent {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            show_details: false,
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        // Split area into agent list and details panel
        let chunks = if self.show_details {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(50), // Agent list
                    Constraint::Percentage(50), // Details panel
                ])
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(100)])
                .split(area)
        };

        // Render agent list
        self.render_agent_list(frame, chunks[0], state, theme);

        // Render details panel if enabled
        if self.show_details && chunks.len() > 1 {
            self.render_agent_details(frame, chunks[1], state, theme);
        }
    }

    fn render_agent_list(&mut self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        let agents = &state.agent_state.agents;

        let items: Vec<ListItem> = agents
            .iter()
            .enumerate()
            .map(|(index, (agent_id, agent_info))| {
                let status_indicator = match agent_info.status {
                    AgentStatus::Uninitialized => "○", // Empty circle
                    AgentStatus::Initializing => "⚪", // White circle
                    AgentStatus::Idle => "●",     // Green dot
                    AgentStatus::Running => "⚡",  // Lightning bolt
                    AgentStatus::Processing(_) => "⚙", // Gear icon
                    AgentStatus::Failed => "✗",   // Red X
                    AgentStatus::Error(_) => "⚠",  // Warning sign
                    AgentStatus::Stopped => "⬜", // Gray square
                    AgentStatus::Shutting => "⏹", // Stop button
                    AgentStatus::Shutdown => "⚫", // Black circle
                };

                let status_color = match agent_info.status {
                    AgentStatus::Uninitialized => Color::Gray,
                    AgentStatus::Initializing => Color::Yellow,
                    AgentStatus::Idle => Color::Green,
                    AgentStatus::Running => Color::Yellow,
                    AgentStatus::Processing(_) => Color::Blue,
                    AgentStatus::Failed => Color::Red,
                    AgentStatus::Error(_) => Color::Red,
                    AgentStatus::Stopped => Color::Gray,
                    AgentStatus::Shutting => Color::DarkGray,
                    AgentStatus::Shutdown => Color::Black,
                };

                let agent_type_name = match &agent_info.agent_type {
                    AgentType::Commit => "Commit Agent",
                    AgentType::Analysis => "Analysis Agent",
                    AgentType::Review => "Review Agent",
                    AgentType::Search => "Search Agent",
                    AgentType::Custom(name) => name.as_str(),
                };

                let style = if index == self.selected_index {
                    theme.highlight_style()
                } else {
                    theme.text_style()
                };

                let line = Line::from(vec![
                    Span::styled(status_indicator, Style::default().fg(status_color)),
                    Span::raw(" "),
                    Span::styled(agent_type_name, style),
                    Span::raw(" "),
                    Span::styled(format!("({})", agent_id), Style::default().fg(Color::Gray)),
                ]);

                ListItem::new(line)
            })
            .collect();

        let title = format!("Agent System ({} agents)", agents.len());
        let list = List::new(items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            )
            .style(theme.text_style());

        frame.render_widget(list, area);
    }

    fn render_agent_details(&mut self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        let agents: Vec<_> = state.agent_state.agents.iter().collect();

        if let Some((agent_id, agent_info)) = agents.get(self.selected_index) {
            // Split details area into sections
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(6),   // Basic info
                    Constraint::Length(4),   // Status and metrics
                    Constraint::Min(0),      // Recent activity
                ])
                .split(area);

            // Basic info section
            let agent_type_name = match &agent_info.agent_type {
                AgentType::Commit => "Commit Message Generator",
                AgentType::Analysis => "Code Analysis",
                AgentType::Review => "Code Review",
                AgentType::Search => "Semantic Search",
                AgentType::Custom(name) => name.as_str(),
            };

            let info_text = vec![
                Line::from(vec![
                    Span::styled("Type: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(agent_type_name),
                ]),
                Line::from(vec![
                    Span::styled("ID: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(agent_id.to_string()),
                ]),
                Line::from(vec![
                    Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(
                        format!("{:?}", agent_info.status),
                        match agent_info.status {
                            AgentStatus::Uninitialized => Style::default().fg(Color::Gray),
                            AgentStatus::Initializing => Style::default().fg(Color::Yellow),
                            AgentStatus::Idle => Style::default().fg(Color::Green),
                            AgentStatus::Running => Style::default().fg(Color::Yellow),
                            AgentStatus::Processing(_) => Style::default().fg(Color::Blue),
                            AgentStatus::Failed => Style::default().fg(Color::Red),
                            AgentStatus::Error(_) => Style::default().fg(Color::Red),
                            AgentStatus::Stopped => Style::default().fg(Color::Gray),
                            AgentStatus::Shutting => Style::default().fg(Color::DarkGray),
                            AgentStatus::Shutdown => Style::default().fg(Color::Black),
                        }
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Last Active: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(agent_info.last_active.format("%H:%M:%S").to_string()),
                ]),
            ];

            let info_paragraph = Paragraph::new(info_text)
                .block(
                    Block::default()
                        .title("Agent Details")
                        .borders(Borders::ALL)
                        .border_style(theme.border_style()),
                )
                .wrap(Wrap { trim: true });

            frame.render_widget(info_paragraph, chunks[0]);

            // Metrics section
            let metrics_text = vec![
                Line::from(vec![
                    Span::styled("Tasks Completed: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(agent_info.tasks_completed.to_string()),
                ]),
                Line::from(vec![
                    Span::styled("Performance: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(format!("{:.1}%", agent_info.performance_score * 100.0)),
                ]),
            ];

            let metrics_paragraph = Paragraph::new(metrics_text)
                .block(
                    Block::default()
                        .title("Metrics")
                        .borders(Borders::ALL)
                        .border_style(theme.border_style()),
                );

            frame.render_widget(metrics_paragraph, chunks[1]);

            // Performance gauge
            let performance_area = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(2), Constraint::Length(1)])
                .split(chunks[1]);

            if performance_area.len() > 1 {
                let gauge = Gauge::default()
                    .gauge_style(Style::default().fg(Color::Green))
                    .percent((agent_info.performance_score * 100.0) as u16);

                frame.render_widget(gauge, performance_area[1]);
            }

            // Recent activity section
            let activity_items: Vec<ListItem> = agent_info.recent_tasks
                .iter()
                .map(|task| {
                    let status_color = if task.success { Color::Green } else { Color::Red };
                    let status_icon = if task.success { "✓" } else { "✗" };

                    let line = Line::from(vec![
                        Span::styled(status_icon, Style::default().fg(status_color)),
                        Span::raw(" "),
                        Span::raw(&task.description),
                        Span::raw(" "),
                        Span::styled(
                            format!("({}ms)", task.duration.as_millis()),
                            Style::default().fg(Color::Gray),
                        ),
                    ]);

                    ListItem::new(line)
                })
                .collect();

            let activity_list = List::new(activity_items)
                .block(
                    Block::default()
                        .title("Recent Activity")
                        .borders(Borders::ALL)
                        .border_style(theme.border_style()),
                );

            frame.render_widget(activity_list, chunks[2]);
        } else {
            // No agent selected
            let placeholder = Paragraph::new("No agent selected")
                .block(
                    Block::default()
                        .title("Agent Details")
                        .borders(Borders::ALL)
                        .border_style(theme.border_style()),
                )
                .style(Style::default().fg(Color::Gray))
                .wrap(Wrap { trim: true });

            frame.render_widget(placeholder, area);
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent, state: &mut AppState) -> AppResult<()> {
        let agent_count = state.agent_state.agents.len();

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_index < agent_count.saturating_sub(1) {
                    self.selected_index += 1;
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                // Toggle details panel
                self.show_details = !self.show_details;
            }
            KeyCode::Char('r') => {
                // Restart selected agent
                if let Some((agent_id, _)) = state.agent_state.agents.iter().nth(self.selected_index) {
                    debug!("Restart agent requested: {}", agent_id);
                    // TODO: Implement agent restart functionality
                }
            }
            KeyCode::Char('s') => {
                // Stop selected agent
                if let Some((agent_id, _)) = state.agent_state.agents.iter().nth(self.selected_index) {
                    debug!("Stop agent requested: {}", agent_id);
                    // TODO: Implement agent stop functionality
                }
            }
            _ => {}
        }

        Ok(())
    }
}
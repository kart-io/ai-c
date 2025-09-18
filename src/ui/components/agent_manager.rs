//! Advanced Agent system management interface

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Tabs, Wrap, BarChart},
    Frame,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::{
    app::state::AppState,
    error::AppResult,
    ui::theme::Theme,
};

use super::{Component, InputModal, ConfirmationModal, Modal, ModalResult};

/// Agent status types
#[derive(Debug, Clone, PartialEq)]
pub enum AgentStatus {
    Idle,
    Running,
    Paused,
    Error,
    Stopping,
    Starting,
}

impl AgentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentStatus::Idle => "Idle",
            AgentStatus::Running => "Running",
            AgentStatus::Paused => "Paused",
            AgentStatus::Error => "Error",
            AgentStatus::Stopping => "Stopping",
            AgentStatus::Starting => "Starting",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            AgentStatus::Idle => Color::Gray,
            AgentStatus::Running => Color::Green,
            AgentStatus::Paused => Color::Yellow,
            AgentStatus::Error => Color::Red,
            AgentStatus::Stopping => Color::Magenta,
            AgentStatus::Starting => Color::Blue,
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            AgentStatus::Idle => "⏸️",
            AgentStatus::Running => "▶️",
            AgentStatus::Paused => "⏸️",
            AgentStatus::Error => "❌",
            AgentStatus::Stopping => "⏹️",
            AgentStatus::Starting => "🔄",
        }
    }
}

/// Agent performance metrics
#[derive(Debug, Clone)]
pub struct AgentMetrics {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub tasks_completed: usize,
    pub tasks_failed: usize,
    pub average_response_time: Duration,
    pub uptime: Duration,
    pub last_activity: Option<Instant>,
}

impl Default for AgentMetrics {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_usage: 0.0,
            tasks_completed: 0,
            tasks_failed: 0,
            average_response_time: Duration::from_millis(0),
            uptime: Duration::from_secs(0),
            last_activity: None,
        }
    }
}

/// Agent information
#[derive(Debug, Clone)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub agent_type: String,
    pub status: AgentStatus,
    pub description: String,
    pub version: String,
    pub capabilities: Vec<String>,
    pub current_task: Option<String>,
    pub metrics: AgentMetrics,
    pub config: HashMap<String, String>,
    pub logs: Vec<AgentLogEntry>,
}

/// Agent log entry
#[derive(Debug, Clone)]
pub struct AgentLogEntry {
    pub timestamp: Instant,
    pub level: LogLevel,
    pub message: String,
    pub context: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warning => "WARN",
            LogLevel::Error => "ERROR",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            LogLevel::Debug => Color::Cyan,
            LogLevel::Info => Color::Green,
            LogLevel::Warning => Color::Yellow,
            LogLevel::Error => Color::Red,
        }
    }
}

/// Agent management interface
pub struct AgentManagerComponent {
    is_open: bool,
    agents: Vec<AgentInfo>,
    selected_agent: usize,
    agent_state: ListState,
    current_tab: usize,
    tab_names: Vec<String>,

    // Modals
    input_modal: InputModal,
    confirmation_modal: ConfirmationModal,

    // View states
    logs_scroll: usize,
    metrics_history: Vec<(Instant, HashMap<String, f64>)>,
    auto_refresh: bool,
    last_refresh: Instant,
}

impl AgentManagerComponent {
    pub fn new() -> Self {
        let mut agent_state = ListState::default();
        agent_state.select(Some(0));

        let tab_names = vec![
            "Agents".to_string(),
            "Details".to_string(),
            "Metrics".to_string(),
            "Logs".to_string(),
            "Config".to_string(),
        ];

        Self {
            is_open: false,
            agents: Vec::new(),
            selected_agent: 0,
            agent_state,
            current_tab: 0,
            tab_names,
            input_modal: InputModal::new(),
            confirmation_modal: ConfirmationModal::new(),
            logs_scroll: 0,
            metrics_history: Vec::new(),
            auto_refresh: true,
            last_refresh: Instant::now(),
        }
    }

    /// Open agent manager
    pub fn open(&mut self) {
        self.is_open = true;
        self.load_agents();
        self.current_tab = 0;
        if !self.agents.is_empty() {
            self.agent_state.select(Some(0));
        }
    }

    /// Close agent manager
    pub fn close(&mut self) {
        self.is_open = false;
        self.input_modal.close();
        self.confirmation_modal.close();
    }

    /// Check if manager is open
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    /// Load agent information
    fn load_agents(&mut self) {
        // TODO: Load real agent data from Agent service
        // For now, create sample data
        self.agents = vec![
            AgentInfo {
                id: "commit-agent-001".to_string(),
                name: "Commit Agent".to_string(),
                agent_type: "CommitAgent".to_string(),
                status: AgentStatus::Running,
                description: "Generates intelligent commit messages from staged changes".to_string(),
                version: "1.2.3".to_string(),
                capabilities: vec![
                    "Commit message generation".to_string(),
                    "Change analysis".to_string(),
                    "Conventional commits".to_string(),
                ],
                current_task: Some("Analyzing staged changes".to_string()),
                metrics: AgentMetrics {
                    cpu_usage: 15.2,
                    memory_usage: 45.8,
                    tasks_completed: 127,
                    tasks_failed: 2,
                    average_response_time: Duration::from_millis(230),
                    uptime: Duration::from_secs(86400),
                    last_activity: Some(Instant::now() - Duration::from_secs(5)),
                },
                config: {
                    let mut config = HashMap::new();
                    config.insert("max_message_length".to_string(), "72".to_string());
                    config.insert("use_conventional_format".to_string(), "true".to_string());
                    config.insert("language_model".to_string(), "gpt-4".to_string());
                    config
                },
                logs: vec![
                    AgentLogEntry {
                        timestamp: Instant::now() - Duration::from_secs(10),
                        level: LogLevel::Info,
                        message: "Generated commit message for 3 files".to_string(),
                        context: Some("files: main.rs, lib.rs, mod.rs".to_string()),
                    },
                    AgentLogEntry {
                        timestamp: Instant::now() - Duration::from_secs(30),
                        level: LogLevel::Debug,
                        message: "Analyzing diff complexity".to_string(),
                        context: None,
                    },
                ],
            },
            AgentInfo {
                id: "analysis-agent-002".to_string(),
                name: "Analysis Agent".to_string(),
                agent_type: "AnalysisAgent".to_string(),
                status: AgentStatus::Idle,
                description: "Performs code analysis and impact assessment".to_string(),
                version: "1.1.0".to_string(),
                capabilities: vec![
                    "Static code analysis".to_string(),
                    "Dependency impact analysis".to_string(),
                    "Security vulnerability detection".to_string(),
                ],
                current_task: None,
                metrics: AgentMetrics {
                    cpu_usage: 0.0,
                    memory_usage: 23.4,
                    tasks_completed: 89,
                    tasks_failed: 0,
                    average_response_time: Duration::from_millis(450),
                    uptime: Duration::from_secs(82800),
                    last_activity: Some(Instant::now() - Duration::from_secs(120)),
                },
                config: {
                    let mut config = HashMap::new();
                    config.insert("analysis_depth".to_string(), "deep".to_string());
                    config.insert("include_dependencies".to_string(), "true".to_string());
                    config.insert("security_checks".to_string(), "enabled".to_string());
                    config
                },
                logs: vec![
                    AgentLogEntry {
                        timestamp: Instant::now() - Duration::from_secs(120),
                        level: LogLevel::Info,
                        message: "Completed analysis of feature branch".to_string(),
                        context: Some("branch: feature/new-ui".to_string()),
                    },
                ],
            },
            AgentInfo {
                id: "review-agent-003".to_string(),
                name: "Review Agent".to_string(),
                agent_type: "ReviewAgent".to_string(),
                status: AgentStatus::Error,
                description: "Automated code review and quality checks".to_string(),
                version: "0.9.5".to_string(),
                capabilities: vec![
                    "Code quality assessment".to_string(),
                    "Best practice validation".to_string(),
                    "Performance analysis".to_string(),
                ],
                current_task: None,
                metrics: AgentMetrics {
                    cpu_usage: 0.0,
                    memory_usage: 12.1,
                    tasks_completed: 45,
                    tasks_failed: 3,
                    average_response_time: Duration::from_millis(890),
                    uptime: Duration::from_secs(12000),
                    last_activity: Some(Instant::now() - Duration::from_secs(300)),
                },
                config: {
                    let mut config = HashMap::new();
                    config.insert("strictness_level".to_string(), "high".to_string());
                    config.insert("performance_checks".to_string(), "enabled".to_string());
                    config.insert("style_guide".to_string(), "rust_standard".to_string());
                    config
                },
                logs: vec![
                    AgentLogEntry {
                        timestamp: Instant::now() - Duration::from_secs(300),
                        level: LogLevel::Error,
                        message: "Failed to connect to review service".to_string(),
                        context: Some("error: connection timeout".to_string()),
                    },
                    AgentLogEntry {
                        timestamp: Instant::now() - Duration::from_secs(400),
                        level: LogLevel::Warning,
                        message: "High memory usage detected".to_string(),
                        context: None,
                    },
                ],
            },
        ];
    }

    /// Switch to next tab
    pub fn next_tab(&mut self) {
        self.current_tab = (self.current_tab + 1) % self.tab_names.len();
    }

    /// Switch to previous tab
    pub fn previous_tab(&mut self) {
        self.current_tab = if self.current_tab == 0 {
            self.tab_names.len() - 1
        } else {
            self.current_tab - 1
        };
    }

    /// Move selection up
    pub fn move_up(&mut self) {
        let len = self.agents.len();
        if len > 0 {
            let selected = self.agent_state.selected().unwrap_or(0);
            let new_selected = if selected == 0 { len - 1 } else { selected - 1 };
            self.agent_state.select(Some(new_selected));
            self.selected_agent = new_selected;
        }
    }

    /// Move selection down
    pub fn move_down(&mut self) {
        let len = self.agents.len();
        if len > 0 {
            let selected = self.agent_state.selected().unwrap_or(0);
            let new_selected = (selected + 1) % len;
            self.agent_state.select(Some(new_selected));
            self.selected_agent = new_selected;
        }
    }

    /// Get currently selected agent
    pub fn selected_agent(&self) -> Option<&AgentInfo> {
        self.agents.get(self.selected_agent)
    }

    /// Start agent
    pub fn start_agent(&mut self) -> AppResult<()> {
        if let Some(agent) = self.agents.get_mut(self.selected_agent) {
            if agent.status == AgentStatus::Idle || agent.status == AgentStatus::Error {
                agent.status = AgentStatus::Starting;
                // TODO: Send start command to agent service
            }
        }
        Ok(())
    }

    /// Stop agent
    pub fn stop_agent(&mut self) -> AppResult<()> {
        if let Some(agent) = self.agents.get_mut(self.selected_agent) {
            if agent.status == AgentStatus::Running {
                agent.status = AgentStatus::Stopping;
                // TODO: Send stop command to agent service
            }
        }
        Ok(())
    }

    /// Restart agent
    pub fn restart_agent(&mut self) -> AppResult<()> {
        self.confirmation_modal.open(
            "Restart Agent",
            "Are you sure you want to restart this agent?",
        );
        Ok(())
    }

    /// Remove agent
    pub fn remove_agent(&mut self) -> AppResult<()> {
        self.confirmation_modal.open(
            "Remove Agent",
            "Are you sure you want to remove this agent? This action cannot be undone.",
        );
        Ok(())
    }

    /// Add new agent
    pub fn add_agent(&mut self) -> AppResult<()> {
        self.input_modal.open(
            "Add Agent",
            "Enter agent type (CommitAgent, AnalysisAgent, ReviewAgent):",
        );
        Ok(())
    }

    /// Toggle auto refresh
    pub fn toggle_auto_refresh(&mut self) {
        self.auto_refresh = !self.auto_refresh;
    }

    /// Refresh agent data
    pub fn refresh(&mut self) {
        self.load_agents();
        self.last_refresh = Instant::now();
    }

    /// Render main interface
    fn render_main_interface(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Calculate centered area
        let modal_area = Self::centered_rect(85, 80, area);

        // Clear background
        frame.render_widget(Clear, modal_area);

        // Split into tabs and content
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tabs
                Constraint::Min(0),    // Content
            ])
            .split(modal_area);

        // Render tabs
        let tab_titles: Vec<Line> = self.tab_names.iter()
            .map(|name| Line::from(name.as_str()))
            .collect();

        let tabs = Tabs::new(tab_titles)
            .block(Block::default().borders(Borders::ALL).title("Agent Manager"))
            .select(self.current_tab)
            .style(Style::default())
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

        frame.render_widget(tabs, chunks[0]);

        // Render content based on current tab
        match self.current_tab {
            0 => self.render_agents_tab(frame, chunks[1], theme),
            1 => self.render_details_tab(frame, chunks[1], theme),
            2 => self.render_metrics_tab(frame, chunks[1], theme),
            3 => self.render_logs_tab(frame, chunks[1], theme),
            4 => self.render_config_tab(frame, chunks[1], theme),
            _ => {}
        }
    }

    /// Render agents list tab
    fn render_agents_tab(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50), // Agent list
                Constraint::Percentage(50), // Agent summary
            ])
            .split(area);

        // Render agent list
        let items: Vec<ListItem> = self.agents.iter().map(|agent| {
            let status_line = Line::from(vec![
                Span::raw(agent.status.icon()),
                Span::raw(" "),
                Span::styled(&agent.name, Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" "),
                Span::styled(
                    format!("({})", agent.status.as_str()),
                    Style::default().fg(agent.status.color())
                ),
            ]);

            let info_line = Line::from(vec![
                Span::raw("  "),
                Span::styled(&agent.agent_type, Style::default().fg(Color::Gray)),
                Span::raw(" v"),
                Span::styled(&agent.version, Style::default().fg(Color::Gray)),
            ]);

            ListItem::new(vec![status_line, info_line])
        }).collect();

        let agent_list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Agents"))
            .highlight_style(
                Style::default()
                    .bg(theme.selection_color())
                    .add_modifier(Modifier::BOLD)
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(agent_list, chunks[0], &mut self.agent_state);

        // Render agent summary
        if let Some(agent) = self.selected_agent() {
            self.render_agent_summary(frame, chunks[1], agent, theme);
        }
    }

    /// Render agent summary
    fn render_agent_summary(&self, frame: &mut Frame, area: Rect, agent: &AgentInfo, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6), // Basic info
                Constraint::Length(4), // Status
                Constraint::Min(0),    // Capabilities
            ])
            .split(area);

        // Basic info
        let info_text = vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&agent.name),
            ]),
            Line::from(vec![
                Span::styled("Type: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&agent.agent_type),
            ]),
            Line::from(vec![
                Span::styled("Version: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&agent.version),
            ]),
            Line::from(vec![
                Span::styled("Description: ", Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(format!("  {}", agent.description)),
        ];

        let info_widget = Paragraph::new(info_text)
            .block(Block::default().borders(Borders::ALL).title("Information"))
            .style(theme.text_style())
            .wrap(Wrap { trim: true });

        frame.render_widget(info_widget, chunks[0]);

        // Status
        let status_text = vec![
            Line::from(vec![
                Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(agent.status.as_str(), Style::default().fg(agent.status.color())),
            ]),
            Line::from(vec![
                Span::styled("Current Task: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(agent.current_task.as_deref().unwrap_or("None")),
            ]),
        ];

        let status_widget = Paragraph::new(status_text)
            .block(Block::default().borders(Borders::ALL).title("Status"))
            .style(theme.text_style());

        frame.render_widget(status_widget, chunks[1]);

        // Capabilities
        let cap_text: Vec<Line> = agent.capabilities.iter()
            .map(|cap| Line::from(format!("• {}", cap)))
            .collect();

        let cap_widget = Paragraph::new(cap_text)
            .block(Block::default().borders(Borders::ALL).title("Capabilities"))
            .style(theme.text_style())
            .wrap(Wrap { trim: true });

        frame.render_widget(cap_widget, chunks[2]);
    }

    /// Render details tab
    fn render_details_tab(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if let Some(agent) = self.selected_agent() {
            let details_text = vec![
                Line::from("Agent Details"),
                Line::from(""),
                Line::from(format!("ID: {}", agent.id)),
                Line::from(format!("Name: {}", agent.name)),
                Line::from(format!("Type: {}", agent.agent_type)),
                Line::from(format!("Version: {}", agent.version)),
                Line::from(format!("Status: {}", agent.status.as_str())),
                Line::from(""),
                Line::from("Description:"),
                Line::from(format!("  {}", agent.description)),
                Line::from(""),
                Line::from("Capabilities:"),
            ];

            let mut all_text = details_text;
            for cap in &agent.capabilities {
                all_text.push(Line::from(format!("  • {}", cap)));
            }

            all_text.extend(vec![
                Line::from(""),
                Line::from("Current Task:"),
                Line::from(format!("  {}", agent.current_task.as_deref().unwrap_or("None"))),
            ]);

            let details_widget = Paragraph::new(all_text)
                .block(Block::default().borders(Borders::ALL).title("Agent Details"))
                .style(theme.text_style())
                .wrap(Wrap { trim: true });

            frame.render_widget(details_widget, area);
        } else {
            let no_agent_text = vec![
                Line::from("No agent selected"),
                Line::from(""),
                Line::from("Select an agent from the Agents tab to view details."),
            ];

            let no_agent_widget = Paragraph::new(no_agent_text)
                .block(Block::default().borders(Borders::ALL).title("Agent Details"))
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center);

            frame.render_widget(no_agent_widget, area);
        }
    }

    /// Render metrics tab
    fn render_metrics_tab(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if let Some(agent) = self.selected_agent() {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(8), // Performance metrics
                    Constraint::Length(6), // Task statistics
                    Constraint::Min(0),    // Resource usage
                ])
                .split(area);

            // Performance metrics
            let perf_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(50), // CPU
                    Constraint::Percentage(50), // Memory
                ])
                .split(chunks[0]);

            // CPU gauge
            let cpu_gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL).title("CPU Usage"))
                .gauge_style(Style::default().fg(Color::Yellow))
                .percent((agent.metrics.cpu_usage * 100.0) as u16)
                .label(format!("{:.1}%", agent.metrics.cpu_usage));

            frame.render_widget(cpu_gauge, perf_chunks[0]);

            // Memory gauge
            let memory_gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL).title("Memory Usage"))
                .gauge_style(Style::default().fg(Color::Blue))
                .percent((agent.metrics.memory_usage * 100.0) as u16)
                .label(format!("{:.1}%", agent.metrics.memory_usage));

            frame.render_widget(memory_gauge, perf_chunks[1]);

            // Task statistics
            let task_text = vec![
                Line::from(format!("Tasks Completed: {}", agent.metrics.tasks_completed)),
                Line::from(format!("Tasks Failed: {}", agent.metrics.tasks_failed)),
                Line::from(format!("Average Response: {:?}", agent.metrics.average_response_time)),
                Line::from(format!("Uptime: {:?}", agent.metrics.uptime)),
            ];

            let task_widget = Paragraph::new(task_text)
                .block(Block::default().borders(Borders::ALL).title("Task Statistics"))
                .style(theme.text_style());

            frame.render_widget(task_widget, chunks[1]);

            // Resource usage chart (placeholder)
            let chart_text = vec![
                Line::from("Resource Usage History"),
                Line::from(""),
                Line::from("CPU, Memory, and Response Time charts"),
                Line::from("would be displayed here."),
                Line::from(""),
                Line::from("This requires time-series data collection"),
                Line::from("and chart rendering capabilities."),
            ];

            let chart_widget = Paragraph::new(chart_text)
                .block(Block::default().borders(Borders::ALL).title("Resource History"))
                .style(theme.text_style())
                .alignment(Alignment::Center);

            frame.render_widget(chart_widget, chunks[2]);
        }
    }

    /// Render logs tab
    fn render_logs_tab(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if let Some(agent) = self.selected_agent() {
            let log_items: Vec<ListItem> = agent.logs.iter().map(|log| {
                let timestamp_str = format!("{:?}", log.timestamp.elapsed());
                let level_style = Style::default().fg(log.level.color());

                let log_line = Line::from(vec![
                    Span::styled(timestamp_str, Style::default().fg(Color::Gray)),
                    Span::raw(" "),
                    Span::styled(log.level.as_str(), level_style.add_modifier(Modifier::BOLD)),
                    Span::raw(" "),
                    Span::raw(&log.message),
                ]);

                let mut lines = vec![log_line];

                if let Some(ref context) = log.context {
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(context, Style::default().fg(Color::DarkGray)),
                    ]));
                }

                ListItem::new(lines)
            }).collect();

            let logs_widget = List::new(log_items)
                .block(Block::default().borders(Borders::ALL).title("Agent Logs"))
                .style(theme.text_style());

            frame.render_widget(logs_widget, area);
        } else {
            let no_logs_text = vec![
                Line::from("No agent selected"),
                Line::from(""),
                Line::from("Select an agent to view its logs."),
            ];

            let no_logs_widget = Paragraph::new(no_logs_text)
                .block(Block::default().borders(Borders::ALL).title("Agent Logs"))
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center);

            frame.render_widget(no_logs_widget, area);
        }
    }

    /// Render config tab
    fn render_config_tab(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if let Some(agent) = self.selected_agent() {
            let config_items: Vec<ListItem> = agent.config.iter().map(|(key, value)| {
                ListItem::new(Line::from(vec![
                    Span::styled(key, Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(": "),
                    Span::raw(value),
                ]))
            }).collect();

            let config_widget = List::new(config_items)
                .block(Block::default().borders(Borders::ALL).title("Agent Configuration"))
                .style(theme.text_style());

            frame.render_widget(config_widget, area);
        } else {
            let no_config_text = vec![
                Line::from("No agent selected"),
                Line::from(""),
                Line::from("Select an agent to view its configuration."),
            ];

            let no_config_widget = Paragraph::new(no_config_text)
                .block(Block::default().borders(Borders::ALL).title("Agent Configuration"))
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center);

            frame.render_widget(no_config_widget, area);
        }
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

impl Component for AgentManagerComponent {
    fn render(&mut self, frame: &mut Frame, area: Rect, _state: &AppState, theme: &Theme) {
        if !self.is_open {
            return;
        }

        // Auto refresh if enabled
        if self.auto_refresh && self.last_refresh.elapsed() > Duration::from_secs(5) {
            // TODO: Refresh agent data from service
            self.last_refresh = Instant::now();
        }

        // Render main interface
        self.render_main_interface(frame, area, theme);

        // Render modals on top if open
        if self.input_modal.is_open() {
            self.input_modal.render(frame, area, theme);
        }

        if self.confirmation_modal.is_open() {
            self.confirmation_modal.render(frame, area, theme);
        }
    }

    fn handle_key_event(&mut self, key: KeyEvent, _state: &mut AppState) -> AppResult<()> {
        if !self.is_open {
            return Ok(());
        }

        // Handle modal events first
        if self.input_modal.is_open() {
            match self.input_modal.handle_key_event(key)? {
                ModalResult::Input(input) => {
                    // Process input for adding new agent
                    println!("Would add agent of type: {}", input);
                }
                ModalResult::Cancelled => {}
                ModalResult::None => {}
                _ => {}
            }
            return Ok(());
        }

        if self.confirmation_modal.is_open() {
            match self.confirmation_modal.handle_key_event(key)? {
                ModalResult::Confirmed => {
                    // Execute confirmed action (restart/remove agent)
                    println!("Action confirmed");
                }
                ModalResult::Cancelled => {}
                ModalResult::None => {}
                _ => {}
            }
            return Ok(());
        }

        // Handle main interface events
        match key.code {
            KeyCode::Esc => self.close(),
            KeyCode::Char(' ') => self.next_tab(),
            KeyCode::BackTab => self.previous_tab(),
            KeyCode::Up => {
                if self.current_tab == 0 {
                    self.move_up();
                }
            }
            KeyCode::Down => {
                if self.current_tab == 0 {
                    self.move_down();
                }
            }
            KeyCode::Char('s') => self.start_agent()?,
            KeyCode::Char('x') => self.stop_agent()?,
            KeyCode::Char('r') => self.restart_agent()?,
            KeyCode::Char('d') => self.remove_agent()?,
            KeyCode::Char('a') => self.add_agent()?,
            KeyCode::Char('t') => self.toggle_auto_refresh(),
            KeyCode::F(5) => self.refresh(),
            _ => {}
        }

        Ok(())
    }
}
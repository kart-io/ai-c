use crate::{
    app::state::AppState,
    error::AppResult,
    ui::{
        diff::{DiffViewer, DiffViewerConfig, DiffProcessorConfig, SyntaxHighlighterConfig, DiffMode},
        theme::Theme,
    },
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, Paragraph, Wrap},
    Frame,
};
use std::path::PathBuf;
use tokio::sync::RwLock;
use std::sync::Arc;
use tracing::{debug, info};

/// 差异查看器UI组件
pub struct DiffViewerComponent {
    /// 差异查看器核心
    viewer: Arc<RwLock<DiffViewer>>,
    /// 是否显示帮助
    show_help: bool,
    /// 是否显示统计信息
    show_stats: bool,
    /// 加载状态
    loading: bool,
    /// 错误信息
    error_message: Option<String>,
}

impl DiffViewerComponent {
    pub fn new() -> Self {
        let config = DiffViewerConfig::default();
        let processor_config = DiffProcessorConfig::default();
        let highlighter_config = SyntaxHighlighterConfig::default();

        let viewer = DiffViewer::new(config, processor_config, highlighter_config);

        Self {
            viewer: Arc::new(RwLock::new(viewer)),
            show_help: false,
            show_stats: false,
            loading: false,
            error_message: None,
        }
    }

    /// 加载文件差异
    pub async fn load_file_diff(&mut self, old_path: &PathBuf, new_path: &PathBuf) -> AppResult<()> {
        self.loading = true;
        self.error_message = None;

        let result: AppResult<()> = async {
            // 读取文件内容
            let old_content = if old_path.exists() {
                tokio::fs::read_to_string(old_path).await.unwrap_or_default()
            } else {
                String::new()
            };

            let new_content = if new_path.exists() {
                tokio::fs::read_to_string(new_path).await.unwrap_or_default()
            } else {
                String::new()
            };

            // 使用新文件路径作为差异标识
            let mut viewer = self.viewer.write().await;
            viewer.load_diff(&old_content, &new_content, new_path).await?;

            info!("Loaded diff for files: {:?} vs {:?}", old_path, new_path);
            Ok(())
        }.await;

        self.loading = false;

        match result {
            Ok(()) => Ok(()),
            Err(e) => {
                self.error_message = Some(e.to_string());
                Err(e)
            }
        }
    }

    /// 加载Git差异
    pub async fn load_git_diff(&mut self, file_path: &PathBuf, old_content: String, new_content: String) -> AppResult<()> {
        self.loading = true;
        self.error_message = None;

        let result: AppResult<()> = async {
            let mut viewer = self.viewer.write().await;
            viewer.load_diff(&old_content, &new_content, file_path).await?;

            info!("Loaded Git diff for file: {:?}", file_path);
            Ok(())
        }.await;

        self.loading = false;

        match result {
            Ok(()) => Ok(()),
            Err(e) => {
                self.error_message = Some(e.to_string());
                Err(e)
            }
        }
    }

    /// 处理键盘输入
    pub async fn handle_key(&mut self, key: KeyEvent) -> AppResult<bool> {
        if self.loading {
            return Ok(false);
        }

        match key {
            // 基础导航
            KeyEvent {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                let mut viewer = self.viewer.write().await;
                viewer.scroll_up(1);
                Ok(true)
            }
            KeyEvent {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                let mut viewer = self.viewer.write().await;
                viewer.scroll_down(1);
                Ok(true)
            }
            KeyEvent {
                code: KeyCode::PageUp,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                let mut viewer = self.viewer.write().await;
                viewer.scroll_up(10);
                Ok(true)
            }
            KeyEvent {
                code: KeyCode::PageDown,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                let mut viewer = self.viewer.write().await;
                viewer.scroll_down(10);
                Ok(true)
            }
            KeyEvent {
                code: KeyCode::Home,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                let mut viewer = self.viewer.write().await;
                viewer.scroll_to(0);
                Ok(true)
            }

            // 差异块导航
            KeyEvent {
                code: KeyCode::Char('n'),
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                let mut viewer = self.viewer.write().await;
                viewer.next_hunk();
                Ok(true)
            }
            KeyEvent {
                code: KeyCode::Char('p'),
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                let mut viewer = self.viewer.write().await;
                viewer.previous_hunk();
                Ok(true)
            }

            // 视图切换
            KeyEvent {
                code: KeyCode::Char('m'),
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                let mut viewer = self.viewer.write().await;
                viewer.toggle_display_mode();
                Ok(true)
            }
            KeyEvent {
                code: KeyCode::Char('l'),
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                let mut viewer = self.viewer.write().await;
                viewer.toggle_line_numbers();
                Ok(true)
            }
            KeyEvent {
                code: KeyCode::Char('w'),
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                let mut viewer = self.viewer.write().await;
                viewer.toggle_whitespace();
                Ok(true)
            }

            // 帮助和统计信息
            KeyEvent {
                code: KeyCode::Char('?'),
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.show_help = !self.show_help;
                Ok(true)
            }
            KeyEvent {
                code: KeyCode::Char('s'),
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.show_stats = !self.show_stats;
                Ok(true)
            }

            // 退出帮助或统计
            KeyEvent {
                code: KeyCode::Esc,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                if self.show_help || self.show_stats {
                    self.show_help = false;
                    self.show_stats = false;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }

            _ => Ok(false),
        }
    }

    /// 渲染组件
    pub async fn render(&mut self, frame: &mut Frame<'_>, area: Rect, _state: &AppState, theme: &Theme) -> AppResult<()> {
        // 如果正在加载，显示加载指示器
        if self.loading {
            self.render_loading(frame, area, theme);
            return Ok(());
        }

        // 如果有错误，显示错误信息
        if let Some(ref error) = self.error_message {
            self.render_error(frame, area, error, theme);
            return Ok(());
        }

        // 主要差异查看器
        let mut viewer = self.viewer.write().await;
        viewer.render(frame, area, theme).await?;

        // 叠加层
        if self.show_help {
            self.render_help_overlay(frame, area, theme);
        } else if self.show_stats {
            self.render_stats_overlay(frame, area, &viewer, theme).await;
        }

        Ok(())
    }

    /// 渲染加载指示器
    fn render_loading(&self, frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Diff Viewer - Loading...");

        let inner_area = block.inner(area);
        frame.render_widget(block, area);

        let gauge = Gauge::default()
            .block(Block::default().title("Computing diff..."))
            .gauge_style(Style::default().fg(Color::Blue))
            .percent(50); // 无法确定的进度

        let gauge_area = Rect {
            x: inner_area.x + inner_area.width / 4,
            y: inner_area.y + inner_area.height / 2,
            width: inner_area.width / 2,
            height: 3,
        };

        frame.render_widget(gauge, gauge_area);
    }

    /// 渲染错误信息
    fn render_error(&self, frame: &mut Frame<'_>, area: Rect, error: &str, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Diff Viewer - Error")
            .border_style(Style::default().fg(Color::Red));

        let error_text = Paragraph::new(error)
            .block(block)
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        frame.render_widget(error_text, area);
    }

    /// 渲染帮助叠加层
    fn render_help_overlay(&self, frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
        let help_area = self.centered_rect(80, 70, area);

        // 清除背景
        frame.render_widget(Clear, help_area);

        let help_text = vec![
            Line::from(vec![Span::styled("Diff Viewer Help", Style::default().add_modifier(Modifier::BOLD))]),
            Line::from(""),
            Line::from("Navigation:"),
            Line::from("  ↑/↓          Scroll up/down"),
            Line::from("  PgUp/PgDn    Page up/down"),
            Line::from("  Home         Go to top"),
            Line::from("  n            Next diff hunk"),
            Line::from("  p            Previous diff hunk"),
            Line::from(""),
            Line::from("View Options:"),
            Line::from("  m            Toggle display mode (side-by-side/unified/inline)"),
            Line::from("  l            Toggle line numbers"),
            Line::from("  w            Toggle whitespace display"),
            Line::from(""),
            Line::from("Other:"),
            Line::from("  s            Show/hide statistics"),
            Line::from("  ?            Show/hide this help"),
            Line::from("  Esc          Close overlays"),
            Line::from(""),
            Line::from(vec![Span::styled("Press Esc to close", Style::default().fg(Color::Yellow))]),
        ];

        let help_paragraph = Paragraph::new(help_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Help")
                    .border_style(Style::default().fg(Color::Blue))
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(help_paragraph, help_area);
    }

    /// 渲染统计信息叠加层
    async fn render_stats_overlay(&self, frame: &mut Frame<'_>, area: Rect, viewer: &DiffViewer, theme: &Theme) {
        let stats_area = self.centered_rect(50, 40, area);

        // 清除背景
        frame.render_widget(Clear, stats_area);

        let mut stats_lines = vec![
            Line::from(vec![Span::styled("Diff Statistics", Style::default().add_modifier(Modifier::BOLD))]),
            Line::from(""),
        ];

        if let Some(stats) = viewer.get_stats() {
            stats_lines.extend(vec![
                Line::from(format!("Files changed: {}", stats.files_changed)),
                Line::from(format!("Lines added: {}", stats.lines_added)),
                Line::from(format!("Lines deleted: {}", stats.lines_deleted)),
                Line::from(format!("Processing time: {:?}", stats.processing_time)),
            ]);
        } else {
            stats_lines.push(Line::from("No diff loaded"));
        }

        stats_lines.push(Line::from(""));
        stats_lines.push(Line::from(vec![Span::styled("Press Esc to close", Style::default().fg(Color::Yellow))]));

        let stats_paragraph = Paragraph::new(stats_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Statistics")
                    .border_style(Style::default().fg(Color::Green))
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(stats_paragraph, stats_area);
    }

    /// 计算居中矩形
    fn centered_rect(&self, percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

    /// 获取当前显示模式
    pub async fn get_display_mode(&self) -> DiffMode {
        let viewer = self.viewer.read().await;
        // 由于DiffViewer的config字段是私有的，我们需要添加一个getter方法
        // 或者在这里使用默认值
        DiffMode::SideBySide // 临时返回默认值
    }

    /// 是否有加载的差异
    pub async fn has_diff(&self) -> bool {
        let viewer = self.viewer.read().await;
        // 类似地，我们需要添加一个方法来检查是否有差异
        true // 临时返回true
    }
}
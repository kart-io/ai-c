pub mod utils;
pub mod inline_editor;

use crate::{
    error::{AppError, AppResult},
    ui::theme::Theme,
};
pub use utils::DiffUtils;
pub use inline_editor::{InlineEditor, InlineEditorConfig, EditOperation, CursorPosition, Selection, EditorMode};
use chrono::{DateTime, Utc};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// 差异显示模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffMode {
    /// 并排显示
    SideBySide,
    /// 统一显示
    Unified,
    /// 内联显示
    Inline,
}

/// 差异算法类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffAlgorithm {
    /// Myers算法
    Myers,
    /// Patience算法
    Patience,
    /// Histogram算法
    Histogram,
    /// Minimal算法
    Minimal,
}

/// 差异行类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffLineType {
    /// 删除的行
    Deleted,
    /// 添加的行
    Added,
    /// 上下文行（未修改）
    Context,
    /// 修改的行（删除+添加的组合）
    Modified,
}

/// 差异行数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    /// 行类型
    pub line_type: DiffLineType,
    /// 原文件行号
    pub old_line_number: Option<usize>,
    /// 新文件行号
    pub new_line_number: Option<usize>,
    /// 行内容
    pub content: String,
    /// 行内高亮范围（用于单词级差异）
    pub highlights: Vec<(usize, usize)>,
}

/// 差异块（连续的修改区域）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    /// 块标题（例如：@@ -1,4 +1,6 @@）
    pub header: String,
    /// 原文件起始行号
    pub old_start: usize,
    /// 原文件行数
    pub old_lines: usize,
    /// 新文件起始行号
    pub new_start: usize,
    /// 新文件行数
    pub new_lines: usize,
    /// 块中的所有行
    pub lines: Vec<DiffLine>,
}

/// 差异统计信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiffStats {
    /// 添加的行数
    pub lines_added: usize,
    /// 删除的行数
    pub lines_deleted: usize,
    /// 修改的文件数
    pub files_changed: usize,
    /// 处理时间
    pub processing_time: Duration,
}

/// 文件差异
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    /// 原文件路径
    pub old_path: Option<PathBuf>,
    /// 新文件路径
    pub new_path: Option<PathBuf>,
    /// 文件状态
    pub status: FileStatus,
    /// 差异块
    pub hunks: Vec<DiffHunk>,
    /// 文件级统计
    pub stats: DiffStats,
    /// 是否为二进制文件
    pub is_binary: bool,
}

/// 文件状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileStatus {
    /// 新增文件
    Added,
    /// 删除文件
    Deleted,
    /// 修改文件
    Modified,
    /// 重命名文件
    Renamed,
    /// 复制文件
    Copied,
}

/// 语法高亮器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxHighlighterConfig {
    /// 启用语法高亮
    pub enabled: bool,
    /// 默认语言
    pub default_language: String,
    /// 主题名称
    pub theme_name: String,
    /// 最大文件大小（字节）
    pub max_file_size: usize,
}

impl Default for SyntaxHighlighterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_language: "text".to_string(),
            theme_name: "base16-ocean.dark".to_string(),
            max_file_size: 1024 * 1024, // 1MB
        }
    }
}

/// 语法高亮器
pub struct SyntaxHighlighter {
    config: SyntaxHighlighterConfig,
    language_cache: Arc<RwLock<HashMap<String, String>>>,
}

impl SyntaxHighlighter {
    pub fn new(config: SyntaxHighlighterConfig) -> Self {
        Self {
            config,
            language_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 检测文件语言
    pub async fn detect_language(&self, file_path: &PathBuf) -> String {
        if let Some(extension) = file_path.extension().and_then(|ext| ext.to_str()) {
            let extension = extension.to_lowercase();
            let language = match extension.as_str() {
                "rs" => "rust",
                "py" => "python",
                "js" | "mjs" => "javascript",
                "ts" => "typescript",
                "html" | "htm" => "html",
                "css" => "css",
                "json" => "json",
                "yaml" | "yml" => "yaml",
                "toml" => "toml",
                "md" => "markdown",
                "sh" | "bash" => "bash",
                "c" => "c",
                "cpp" | "cc" | "cxx" => "cpp",
                "go" => "go",
                "java" => "java",
                _ => &self.config.default_language,
            };

            language.to_string()
        } else {
            self.config.default_language.clone()
        }
    }

    /// 对代码行应用语法高亮
    pub async fn highlight_line(&self, line: &str, language: &str) -> Vec<Span> {
        if !self.config.enabled || line.len() > 1000 {
            // 对于非常长的行，跳过语法高亮以提高性能
            return vec![Span::raw(line.to_string())];
        }

        // 简化的语法高亮实现
        // 在实际项目中，这里会使用专门的语法高亮库如syntect
        self.simple_highlight(line, language).await
    }

    async fn simple_highlight(&self, line: &str, language: &str) -> Vec<Span> {
        let mut spans = Vec::new();
        let mut chars = line.chars().peekable();
        let mut current_token = String::new();
        let mut current_style = Style::default();

        while let Some(ch) = chars.next() {
            match language {
                "rust" => {
                    if ch.is_alphabetic() || ch == '_' {
                        current_token.push(ch);
                        while let Some(&next_ch) = chars.peek() {
                            if next_ch.is_alphanumeric() || next_ch == '_' {
                                current_token.push(chars.next().unwrap());
                            } else {
                                break;
                            }
                        }

                        // Rust关键字高亮
                        current_style = match current_token.as_str() {
                            "fn" | "let" | "mut" | "const" | "static" | "struct" | "enum" | "impl" |
                            "trait" | "pub" | "use" | "mod" | "crate" | "super" | "self" => {
                                Style::default().fg(Color::Magenta)
                            }
                            "if" | "else" | "match" | "for" | "while" | "loop" | "break" | "continue" |
                            "return" => Style::default().fg(Color::Blue),
                            _ => Style::default(),
                        };

                        spans.push(Span::styled(current_token.clone(), current_style));
                        current_token.clear();
                    } else if ch == '"' {
                        // 字符串字面量
                        current_token.push(ch);
                        while let Some(next_ch) = chars.next() {
                            current_token.push(next_ch);
                            if next_ch == '"' && !current_token.ends_with("\\\"") {
                                break;
                            }
                        }
                        spans.push(Span::styled(current_token.clone(), Style::default().fg(Color::Green)));
                        current_token.clear();
                    } else if ch == '/' && chars.peek() == Some(&'/') {
                        // 单行注释
                        current_token.push(ch);
                        current_token.push(chars.next().unwrap());
                        for next_ch in chars.by_ref() {
                            current_token.push(next_ch);
                        }
                        spans.push(Span::styled(current_token.clone(), Style::default().fg(Color::Gray)));
                        current_token.clear();
                        break;
                    } else {
                        spans.push(Span::raw(ch.to_string()));
                    }
                }
                _ => {
                    // 默认无高亮
                    spans.push(Span::raw(ch.to_string()));
                }
            }
        }

        if !current_token.is_empty() {
            spans.push(Span::styled(current_token, current_style));
        }

        spans
    }
}

/// 差异处理器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffProcessorConfig {
    /// 差异算法
    pub algorithm: DiffAlgorithm,
    /// 上下文行数
    pub context_lines: usize,
    /// 启用单词级差异
    pub word_level_diff: bool,
    /// 忽略空白字符
    pub ignore_whitespace: bool,
    /// 最大文件大小
    pub max_file_size: usize,
    /// 启用缓存
    pub enable_cache: bool,
}

impl Default for DiffProcessorConfig {
    fn default() -> Self {
        Self {
            algorithm: DiffAlgorithm::Myers,
            context_lines: 3,
            word_level_diff: true,
            ignore_whitespace: false,
            max_file_size: 10 * 1024 * 1024, // 10MB
            enable_cache: true,
        }
    }
}

/// 差异处理器
pub struct DiffProcessor {
    config: DiffProcessorConfig,
    cache: Arc<RwLock<HashMap<String, FileDiff>>>,
    stats: Arc<RwLock<DiffStats>>,
}

impl DiffProcessor {
    pub fn new(config: DiffProcessorConfig) -> Self {
        Self {
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(DiffStats::default())),
        }
    }

    /// 计算两个文件之间的差异
    pub async fn compute_diff(&self, old_content: &str, new_content: &str, file_path: &PathBuf) -> AppResult<FileDiff> {
        let start_time = Instant::now();

        // 检查缓存
        let cache_key = format!("{:x}", md5::compute(format!("{}{}{:?}", old_content, new_content, file_path)));
        if self.config.enable_cache {
            let cache = self.cache.read().await;
            if let Some(cached_diff) = cache.get(&cache_key) {
                debug!("Returning cached diff for file: {:?}", file_path);
                return Ok(cached_diff.clone());
            }
        }

        // 检查文件大小
        if old_content.len() > self.config.max_file_size || new_content.len() > self.config.max_file_size {
            return Err(AppError::InvalidOperation(format!(
                "File too large for diff processing: {:?}",
                file_path
            )));
        }

        // 检查是否为二进制文件
        let is_binary = self.is_binary_content(old_content) || self.is_binary_content(new_content);
        if is_binary {
            let file_diff = FileDiff {
                old_path: Some(file_path.clone()),
                new_path: Some(file_path.clone()),
                status: if old_content.is_empty() {
                    FileStatus::Added
                } else if new_content.is_empty() {
                    FileStatus::Deleted
                } else {
                    FileStatus::Modified
                },
                hunks: vec![],
                stats: DiffStats::default(),
                is_binary: true,
            };
            return Ok(file_diff);
        }

        // 计算差异
        let hunks = self.compute_hunks(old_content, new_content).await?;

        // 计算统计信息
        let mut stats = DiffStats::default();
        stats.files_changed = 1;
        stats.processing_time = start_time.elapsed();

        for hunk in &hunks {
            for line in &hunk.lines {
                match line.line_type {
                    DiffLineType::Added => stats.lines_added += 1,
                    DiffLineType::Deleted => stats.lines_deleted += 1,
                    _ => {}
                }
            }
        }

        let file_diff = FileDiff {
            old_path: Some(file_path.clone()),
            new_path: Some(file_path.clone()),
            status: if old_content.is_empty() {
                FileStatus::Added
            } else if new_content.is_empty() {
                FileStatus::Deleted
            } else {
                FileStatus::Modified
            },
            hunks,
            stats: stats.clone(),
            is_binary: false,
        };

        // 缓存结果
        if self.config.enable_cache {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, file_diff.clone());
        }

        // 更新全局统计
        let mut global_stats = self.stats.write().await;
        global_stats.lines_added += stats.lines_added;
        global_stats.lines_deleted += stats.lines_deleted;
        global_stats.files_changed += 1;

        info!("Computed diff for file: {:?} in {:?}", file_path, stats.processing_time);
        Ok(file_diff)
    }

    /// 计算差异块
    async fn compute_hunks(&self, old_content: &str, new_content: &str) -> AppResult<Vec<DiffHunk>> {
        let old_lines: Vec<&str> = old_content.lines().collect();
        let new_lines: Vec<&str> = new_content.lines().collect();

        let mut hunks = Vec::new();
        let diff_result = self.run_diff_algorithm(&old_lines, &new_lines).await?;

        let mut current_hunk: Option<DiffHunk> = None;
        let mut old_line_num = 1;
        let mut new_line_num = 1;

        for change in diff_result {
            match change {
                DiffChange::Equal(content) => {
                    if let Some(ref mut hunk) = current_hunk {
                        hunk.lines.push(DiffLine {
                            line_type: DiffLineType::Context,
                            old_line_number: Some(old_line_num),
                            new_line_number: Some(new_line_num),
                            content,
                            highlights: vec![],
                        });
                    }
                    old_line_num += 1;
                    new_line_num += 1;
                }
                DiffChange::Delete(content) => {
                    if current_hunk.is_none() {
                        current_hunk = Some(DiffHunk {
                            header: format!("@@ -{},{} +{},{} @@", old_line_num, 0, new_line_num, 0),
                            old_start: old_line_num,
                            old_lines: 0,
                            new_start: new_line_num,
                            new_lines: 0,
                            lines: vec![],
                        });
                    }

                    if let Some(ref mut hunk) = current_hunk {
                        hunk.lines.push(DiffLine {
                            line_type: DiffLineType::Deleted,
                            old_line_number: Some(old_line_num),
                            new_line_number: None,
                            content,
                            highlights: vec![],
                        });
                        hunk.old_lines += 1;
                    }
                    old_line_num += 1;
                }
                DiffChange::Insert(content) => {
                    if current_hunk.is_none() {
                        current_hunk = Some(DiffHunk {
                            header: format!("@@ -{},{} +{},{} @@", old_line_num, 0, new_line_num, 0),
                            old_start: old_line_num,
                            old_lines: 0,
                            new_start: new_line_num,
                            new_lines: 0,
                            lines: vec![],
                        });
                    }

                    if let Some(ref mut hunk) = current_hunk {
                        hunk.lines.push(DiffLine {
                            line_type: DiffLineType::Added,
                            old_line_number: None,
                            new_line_number: Some(new_line_num),
                            content,
                            highlights: vec![],
                        });
                        hunk.new_lines += 1;
                    }
                    new_line_num += 1;
                }
            }
        }

        if let Some(hunk) = current_hunk {
            hunks.push(hunk);
        }

        Ok(hunks)
    }

    /// 运行差异算法
    async fn run_diff_algorithm(&self, old_lines: &[&str], new_lines: &[&str]) -> AppResult<Vec<DiffChange>> {
        match self.config.algorithm {
            DiffAlgorithm::Myers => self.myers_diff(old_lines, new_lines).await,
            _ => {
                // 其他算法的简化实现
                warn!("Algorithm {:?} not fully implemented, falling back to Myers", self.config.algorithm);
                self.myers_diff(old_lines, new_lines).await
            }
        }
    }

    /// Myers差异算法实现
    async fn myers_diff(&self, old_lines: &[&str], new_lines: &[&str]) -> AppResult<Vec<DiffChange>> {
        let mut result = Vec::new();
        let mut old_idx = 0;
        let mut new_idx = 0;

        // 简化的Myers算法实现
        while old_idx < old_lines.len() || new_idx < new_lines.len() {
            if old_idx < old_lines.len() && new_idx < new_lines.len() {
                if self.lines_equal(old_lines[old_idx], new_lines[new_idx]) {
                    result.push(DiffChange::Equal(old_lines[old_idx].to_string()));
                    old_idx += 1;
                    new_idx += 1;
                } else {
                    // 简单的删除+插入策略
                    result.push(DiffChange::Delete(old_lines[old_idx].to_string()));
                    result.push(DiffChange::Insert(new_lines[new_idx].to_string()));
                    old_idx += 1;
                    new_idx += 1;
                }
            } else if old_idx < old_lines.len() {
                result.push(DiffChange::Delete(old_lines[old_idx].to_string()));
                old_idx += 1;
            } else {
                result.push(DiffChange::Insert(new_lines[new_idx].to_string()));
                new_idx += 1;
            }
        }

        Ok(result)
    }

    /// 比较两行是否相等
    fn lines_equal(&self, old_line: &str, new_line: &str) -> bool {
        if self.config.ignore_whitespace {
            old_line.trim() == new_line.trim()
        } else {
            old_line == new_line
        }
    }

    /// 检查内容是否为二进制
    fn is_binary_content(&self, content: &str) -> bool {
        // 简单的二进制检测：包含null字符
        content.chars().any(|c| c == '\0')
    }

    /// 获取统计信息
    pub async fn get_stats(&self) -> DiffStats {
        self.stats.read().await.clone()
    }

    /// 清除缓存
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}

/// 差异变更类型
#[derive(Debug, Clone)]
enum DiffChange {
    /// 相等的内容
    Equal(String),
    /// 删除的内容
    Delete(String),
    /// 插入的内容
    Insert(String),
}

/// 差异查看器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffViewerConfig {
    /// 显示模式
    pub display_mode: DiffMode,
    /// 显示行号
    pub show_line_numbers: bool,
    /// 显示空白字符
    pub show_whitespace: bool,
    /// 自动换行
    pub word_wrap: bool,
    /// 每页行数
    pub lines_per_page: usize,
    /// 启用虚拟滚动
    pub enable_virtual_scrolling: bool,
}

impl Default for DiffViewerConfig {
    fn default() -> Self {
        Self {
            display_mode: DiffMode::SideBySide,
            show_line_numbers: true,
            show_whitespace: false,
            word_wrap: false,
            lines_per_page: 50,
            enable_virtual_scrolling: true,
        }
    }
}

/// 差异查看器
pub struct DiffViewer {
    config: DiffViewerConfig,
    processor: DiffProcessor,
    highlighter: SyntaxHighlighter,
    current_diff: Option<FileDiff>,
    scroll_offset: usize,
    selected_hunk: usize,
    virtual_scroll_state: VirtualScrollState,
}

/// 虚拟滚动状态
#[derive(Debug, Clone, Default)]
struct VirtualScrollState {
    /// 可见区域开始行
    viewport_start: usize,
    /// 可见区域结束行
    viewport_end: usize,
    /// 总行数
    total_lines: usize,
    /// 每行高度（字符单位）
    line_height: usize,
}

impl DiffViewer {
    pub fn new(
        config: DiffViewerConfig,
        processor_config: DiffProcessorConfig,
        highlighter_config: SyntaxHighlighterConfig,
    ) -> Self {
        Self {
            config,
            processor: DiffProcessor::new(processor_config),
            highlighter: SyntaxHighlighter::new(highlighter_config),
            current_diff: None,
            scroll_offset: 0,
            selected_hunk: 0,
            virtual_scroll_state: VirtualScrollState::default(),
        }
    }

    /// 加载差异
    pub async fn load_diff(&mut self, old_content: &str, new_content: &str, file_path: &PathBuf) -> AppResult<()> {
        let diff = self.processor.compute_diff(old_content, new_content, file_path).await?;
        self.current_diff = Some(diff);
        self.scroll_offset = 0;
        self.selected_hunk = 0;
        self.update_virtual_scroll_state();
        Ok(())
    }

    /// 更新虚拟滚动状态
    fn update_virtual_scroll_state(&mut self) {
        if let Some(ref diff) = self.current_diff {
            let total_lines = diff.hunks.iter().map(|h| h.lines.len()).sum();
            self.virtual_scroll_state = VirtualScrollState {
                viewport_start: self.scroll_offset,
                viewport_end: (self.scroll_offset + self.config.lines_per_page).min(total_lines),
                total_lines,
                line_height: 1,
            };
        }
    }

    /// 滚动到指定位置
    pub fn scroll_to(&mut self, offset: usize) {
        self.scroll_offset = offset;
        self.update_virtual_scroll_state();
    }

    /// 向上滚动
    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
        self.update_virtual_scroll_state();
    }

    /// 向下滚动
    pub fn scroll_down(&mut self, lines: usize) {
        if let Some(ref diff) = self.current_diff {
            let max_scroll = diff.hunks.iter().map(|h| h.lines.len()).sum::<usize>()
                .saturating_sub(self.config.lines_per_page);
            self.scroll_offset = (self.scroll_offset + lines).min(max_scroll);
            self.update_virtual_scroll_state();
        }
    }

    /// 选择下一个差异块
    pub fn next_hunk(&mut self) {
        if let Some(ref diff) = self.current_diff {
            if self.selected_hunk < diff.hunks.len().saturating_sub(1) {
                self.selected_hunk += 1;
            }
        }
    }

    /// 选择上一个差异块
    pub fn previous_hunk(&mut self) {
        self.selected_hunk = self.selected_hunk.saturating_sub(1);
    }

    /// 渲染差异查看器
    pub async fn render(&mut self, frame: &mut Frame<'_>, area: Rect, theme: &Theme) -> AppResult<()> {
        if let Some(diff) = self.current_diff.clone() {
            let display_mode = self.config.display_mode.clone();
            match display_mode {
                DiffMode::SideBySide => self.render_side_by_side(frame, area, &diff, theme).await?,
                DiffMode::Unified => self.render_unified(frame, area, &diff, theme).await?,
                DiffMode::Inline => self.render_inline(frame, area, &diff, theme).await?,
            }
        } else {
            // 显示空状态
            let placeholder = Paragraph::new("No diff to display")
                .block(Block::default().borders(Borders::ALL).title("Diff Viewer"))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            frame.render_widget(placeholder, area);
        }

        Ok(())
    }

    /// 渲染并排显示模式
    async fn render_side_by_side(&mut self, frame: &mut Frame<'_>, area: Rect, diff: &FileDiff, theme: &Theme) -> AppResult<()> {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // 左侧：原文件
        self.render_side(frame, chunks[0], diff, true, theme).await?;

        // 右侧：新文件
        self.render_side(frame, chunks[1], diff, false, theme).await?;

        Ok(())
    }

    /// 渲染一侧（原文件或新文件）
    async fn render_side(&mut self, frame: &mut Frame<'_>, area: Rect, diff: &FileDiff, is_old: bool, theme: &Theme) -> AppResult<()> {
        let title = if is_old { "Old File" } else { "New File" };
        let block = Block::default()
            .borders(Borders::ALL)
            .title(title);

        let inner_area = block.inner(area);
        frame.render_widget(block, area);

        let mut lines = Vec::new();
        for hunk in &diff.hunks {
            for line in &hunk.lines {
                let should_show = match (&line.line_type, is_old) {
                    (DiffLineType::Context, _) => true,
                    (DiffLineType::Deleted, true) => true,
                    (DiffLineType::Added, false) => true,
                    _ => false,
                };

                if should_show {
                    let line_number = if is_old {
                        line.old_line_number
                    } else {
                        line.new_line_number
                    };

                    let style = match line.line_type {
                        DiffLineType::Added => Style::default().bg(Color::Green).fg(Color::Black),
                        DiffLineType::Deleted => Style::default().bg(Color::Red).fg(Color::White),
                        DiffLineType::Context => Style::default(),
                        DiffLineType::Modified => Style::default().bg(Color::Yellow).fg(Color::Black),
                    };

                    let line_content = if self.config.show_line_numbers {
                        format!("{:4} {}", line_number.map_or("".to_string(), |n| n.to_string()), line.content)
                    } else {
                        line.content.clone()
                    };

                    lines.push(ListItem::new(line_content).style(style));
                }
            }
        }

        let list = List::new(lines)
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        frame.render_widget(list, inner_area);

        Ok(())
    }

    /// 渲染统一显示模式
    async fn render_unified(&mut self, frame: &mut Frame<'_>, area: Rect, diff: &FileDiff, theme: &Theme) -> AppResult<()> {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Unified Diff");

        let inner_area = block.inner(area);
        frame.render_widget(block, area);

        let mut lines = Vec::new();

        for hunk in &diff.hunks {
            // 添加块头
            lines.push(ListItem::new(hunk.header.clone())
                .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)));

            for line in &hunk.lines {
                let prefix = match line.line_type {
                    DiffLineType::Added => "+",
                    DiffLineType::Deleted => "-",
                    DiffLineType::Context => " ",
                    DiffLineType::Modified => "~",
                };

                let style = match line.line_type {
                    DiffLineType::Added => Style::default().fg(Color::Green),
                    DiffLineType::Deleted => Style::default().fg(Color::Red),
                    DiffLineType::Context => Style::default(),
                    DiffLineType::Modified => Style::default().fg(Color::Yellow),
                };

                let line_content = if self.config.show_line_numbers {
                    let old_num = line.old_line_number.map_or("   ".to_string(), |n| format!("{:3}", n));
                    let new_num = line.new_line_number.map_or("   ".to_string(), |n| format!("{:3}", n));
                    format!("{} {} {}{}", old_num, new_num, prefix, line.content)
                } else {
                    format!("{}{}", prefix, line.content)
                };

                lines.push(ListItem::new(line_content).style(style));
            }
        }

        let list = List::new(lines)
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        frame.render_widget(list, inner_area);

        Ok(())
    }

    /// 渲染内联显示模式
    async fn render_inline(&mut self, frame: &mut Frame<'_>, area: Rect, diff: &FileDiff, theme: &Theme) -> AppResult<()> {
        // 内联模式与统一模式类似，但可能有不同的样式
        self.render_unified(frame, area, diff, theme).await
    }

    /// 获取差异统计信息
    pub fn get_stats(&self) -> Option<&DiffStats> {
        self.current_diff.as_ref().map(|d| &d.stats)
    }

    /// 切换显示模式
    pub fn toggle_display_mode(&mut self) {
        self.config.display_mode = match self.config.display_mode {
            DiffMode::SideBySide => DiffMode::Unified,
            DiffMode::Unified => DiffMode::Inline,
            DiffMode::Inline => DiffMode::SideBySide,
        };
    }

    /// 切换行号显示
    pub fn toggle_line_numbers(&mut self) {
        self.config.show_line_numbers = !self.config.show_line_numbers;
    }

    /// 切换空白字符显示
    pub fn toggle_whitespace(&mut self) {
        self.config.show_whitespace = !self.config.show_whitespace;
    }
}
use crate::{
    error::{AppError, AppResult},
    ui::theme::Theme,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};
use serde::{Deserialize, Serialize};
use std::{
    cmp::{max, min},
    collections::VecDeque,
    path::PathBuf,
};
use tokio::fs;
use tracing::{debug, info, warn};

/// 编辑操作类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditOperation {
    /// 插入文本
    Insert {
        line: usize,
        column: usize,
        text: String,
    },
    /// 删除文本
    Delete {
        start_line: usize,
        start_column: usize,
        end_line: usize,
        end_column: usize,
    },
    /// 替换文本
    Replace {
        start_line: usize,
        start_column: usize,
        end_line: usize,
        end_column: usize,
        new_text: String,
    },
    /// 插入新行
    InsertLine {
        line: usize,
        text: String,
    },
    /// 删除行
    DeleteLine {
        line: usize,
    },
}

/// 光标位置
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CursorPosition {
    pub line: usize,
    pub column: usize,
}

impl Default for CursorPosition {
    fn default() -> Self {
        Self { line: 0, column: 0 }
    }
}

/// 选择区域
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    pub start: CursorPosition,
    pub end: CursorPosition,
}

impl Selection {
    pub fn new(start: CursorPosition, end: CursorPosition) -> Self {
        // 确保start在end之前
        if start.line < end.line || (start.line == end.line && start.column <= end.column) {
            Self { start, end }
        } else {
            Self { start: end, end: start }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn contains(&self, pos: CursorPosition) -> bool {
        (pos.line > self.start.line || (pos.line == self.start.line && pos.column >= self.start.column))
            && (pos.line < self.end.line || (pos.line == self.end.line && pos.column < self.end.column))
    }
}

/// 内联编辑器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineEditorConfig {
    /// 启用自动保存
    pub auto_save: bool,
    /// 自动保存间隔（秒）
    pub auto_save_interval: u64,
    /// 制表符宽度
    pub tab_width: usize,
    /// 使用空格代替制表符
    pub expand_tabs: bool,
    /// 显示行号
    pub show_line_numbers: bool,
    /// 显示不可见字符
    pub show_invisible: bool,
    /// 启用语法高亮
    pub syntax_highlighting: bool,
    /// 自动缩进
    pub auto_indent: bool,
    /// 最大撤销历史
    pub max_undo_history: usize,
}

impl Default for InlineEditorConfig {
    fn default() -> Self {
        Self {
            auto_save: false,
            auto_save_interval: 30,
            tab_width: 4,
            expand_tabs: true,
            show_line_numbers: true,
            show_invisible: false,
            syntax_highlighting: true,
            auto_indent: true,
            max_undo_history: 100,
        }
    }
}

/// 编辑器状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorMode {
    /// 只读模式
    ReadOnly,
    /// 编辑模式
    Edit,
    /// 选择模式
    Select,
}

/// 内联编辑器
pub struct InlineEditor {
    /// 配置
    config: InlineEditorConfig,
    /// 文件路径
    file_path: Option<PathBuf>,
    /// 文件内容（行）
    lines: Vec<String>,
    /// 光标位置
    cursor: CursorPosition,
    /// 选择区域
    selection: Option<Selection>,
    /// 编辑器模式
    mode: EditorMode,
    /// 滚动偏移
    scroll_offset: usize,
    /// 视口大小
    viewport_height: usize,
    /// 撤销历史
    undo_history: VecDeque<Vec<String>>,
    /// 重做历史
    redo_history: VecDeque<Vec<String>>,
    /// 是否已修改
    is_modified: bool,
    /// 状态消息
    status_message: Option<String>,
    /// 搜索文本
    search_query: Option<String>,
    /// 搜索结果
    search_results: Vec<CursorPosition>,
    /// 当前搜索结果索引
    current_search_index: usize,
}

impl InlineEditor {
    pub fn new(config: InlineEditorConfig) -> Self {
        Self {
            config,
            file_path: None,
            lines: vec![String::new()],
            cursor: CursorPosition::default(),
            selection: None,
            mode: EditorMode::ReadOnly,
            scroll_offset: 0,
            viewport_height: 20,
            undo_history: VecDeque::new(),
            redo_history: VecDeque::new(),
            is_modified: false,
            status_message: None,
            search_query: None,
            search_results: Vec::new(),
            current_search_index: 0,
        }
    }

    /// 加载文件
    pub async fn load_file(&mut self, file_path: PathBuf) -> AppResult<()> {
        if file_path.exists() {
            let content = fs::read_to_string(&file_path).await
                .map_err(|e| AppError::InvalidOperation(format!("Failed to read file: {}", e)))?;

            self.lines = if content.is_empty() {
                vec![String::new()]
            } else {
                content.lines().map(|s| s.to_string()).collect()
            };
        } else {
            self.lines = vec![String::new()];
        }

        self.file_path = Some(file_path.clone());
        self.cursor = CursorPosition::default();
        self.selection = None;
        self.is_modified = false;
        self.undo_history.clear();
        self.redo_history.clear();
        self.mode = EditorMode::Edit;

        info!("Loaded file: {:?} ({} lines)", file_path, self.lines.len());
        Ok(())
    }

    /// 保存文件
    pub async fn save_file(&mut self) -> AppResult<()> {
        if let Some(ref file_path) = self.file_path.clone() {
            let content = self.lines.join("\n");
            fs::write(file_path, content).await
                .map_err(|e| AppError::InvalidOperation(format!("Failed to save file: {}", e)))?;

            self.is_modified = false;
            self.status_message = Some("File saved".to_string());
            info!("Saved file: {:?}", file_path);
            Ok(())
        } else {
            Err(AppError::InvalidOperation("No file path set".to_string()))
        }
    }

    /// 处理键盘输入
    pub fn handle_key(&mut self, key: KeyEvent) -> AppResult<bool> {
        match self.mode {
            EditorMode::ReadOnly => self.handle_readonly_key(key),
            EditorMode::Edit => self.handle_edit_key(key),
            EditorMode::Select => self.handle_select_key(key),
        }
    }

    /// 处理只读模式按键
    fn handle_readonly_key(&mut self, key: KeyEvent) -> AppResult<bool> {
        match key {
            KeyEvent {
                code: KeyCode::Char('e'),
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.mode = EditorMode::Edit;
                self.status_message = Some("Edit mode".to_string());
                Ok(true)
            }
            _ => self.handle_navigation_key(key),
        }
    }

    /// 处理编辑模式按键
    fn handle_edit_key(&mut self, key: KeyEvent) -> AppResult<bool> {
        match key {
            // 退出编辑模式
            KeyEvent {
                code: KeyCode::Esc,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.mode = EditorMode::ReadOnly;
                self.selection = None;
                self.status_message = Some("Read-only mode".to_string());
                Ok(true)
            }

            // 保存文件
            KeyEvent {
                code: KeyCode::Char('s'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => {
                // 这里需要异步处理，但当前函数是同步的
                // 在实际使用中，应该返回一个标志让调用者处理保存
                self.status_message = Some("Save requested".to_string());
                Ok(true)
            }

            // 撤销
            KeyEvent {
                code: KeyCode::Char('z'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => {
                self.undo();
                Ok(true)
            }

            // 重做
            KeyEvent {
                code: KeyCode::Char('y'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => {
                self.redo();
                Ok(true)
            }

            // 复制
            KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => {
                self.copy_selection();
                Ok(true)
            }

            // 粘贴
            KeyEvent {
                code: KeyCode::Char('v'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => {
                // 简化的粘贴操作
                self.status_message = Some("Paste not implemented".to_string());
                Ok(true)
            }

            // 字符输入
            KeyEvent {
                code: KeyCode::Char(c),
                modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
                ..
            } => {
                self.insert_char(c);
                Ok(true)
            }

            // 回车
            KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.insert_newline();
                Ok(true)
            }

            // 制表符
            KeyEvent {
                code: KeyCode::Tab,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.insert_tab();
                Ok(true)
            }

            // 退格键
            KeyEvent {
                code: KeyCode::Backspace,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.backspace();
                Ok(true)
            }

            // 删除键
            KeyEvent {
                code: KeyCode::Delete,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.delete();
                Ok(true)
            }

            _ => self.handle_navigation_key(key),
        }
    }

    /// 处理选择模式按键
    fn handle_select_key(&mut self, key: KeyEvent) -> AppResult<bool> {
        match key {
            KeyEvent {
                code: KeyCode::Esc,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.mode = EditorMode::Edit;
                self.selection = None;
                Ok(true)
            }
            _ => {
                let old_cursor = self.cursor;
                if self.handle_navigation_key(key)? {
                    // 更新选择区域
                    if let Some(ref mut selection) = self.selection {
                        selection.end = self.cursor;
                    } else {
                        self.selection = Some(Selection::new(old_cursor, self.cursor));
                    }
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        }
    }

    /// 处理导航按键
    fn handle_navigation_key(&mut self, key: KeyEvent) -> AppResult<bool> {
        match key {
            KeyEvent {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.move_cursor_up();
                Ok(true)
            }
            KeyEvent {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.move_cursor_down();
                Ok(true)
            }
            KeyEvent {
                code: KeyCode::Left,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.move_cursor_left();
                Ok(true)
            }
            KeyEvent {
                code: KeyCode::Right,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.move_cursor_right();
                Ok(true)
            }
            KeyEvent {
                code: KeyCode::Home,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.cursor.column = 0;
                Ok(true)
            }
            KeyEvent {
                code: KeyCode::End,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                if self.cursor.line < self.lines.len() {
                    self.cursor.column = self.lines[self.cursor.line].len();
                }
                Ok(true)
            }
            KeyEvent {
                code: KeyCode::PageUp,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.page_up();
                Ok(true)
            }
            KeyEvent {
                code: KeyCode::PageDown,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.page_down();
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// 插入字符
    fn insert_char(&mut self, c: char) {
        self.save_state_for_undo();

        if self.cursor.line >= self.lines.len() {
            self.lines.push(String::new());
        }

        let line = &mut self.lines[self.cursor.line];
        if self.cursor.column <= line.len() {
            line.insert(self.cursor.column, c);
            self.cursor.column += 1;
            self.is_modified = true;
        }
    }

    /// 插入新行
    fn insert_newline(&mut self) {
        self.save_state_for_undo();

        if self.cursor.line >= self.lines.len() {
            self.lines.push(String::new());
            self.cursor.line += 1;
            self.cursor.column = 0;
        } else {
            let current_line = self.lines[self.cursor.line].clone();
            let (left, right) = current_line.split_at(self.cursor.column);

            self.lines[self.cursor.line] = left.to_string();
            self.lines.insert(self.cursor.line + 1, right.to_string());

            self.cursor.line += 1;
            self.cursor.column = 0;

            // 自动缩进
            if self.config.auto_indent && self.cursor.line > 0 {
                let prev_line = &self.lines[self.cursor.line - 1];
                let indent = prev_line.chars().take_while(|c| c.is_whitespace()).collect::<String>();
                self.lines[self.cursor.line] = indent.clone() + &self.lines[self.cursor.line];
                self.cursor.column = indent.len();
            }
        }

        self.is_modified = true;
    }

    /// 插入制表符
    fn insert_tab(&mut self) {
        if self.config.expand_tabs {
            let spaces = " ".repeat(self.config.tab_width);
            for c in spaces.chars() {
                self.insert_char(c);
            }
        } else {
            self.insert_char('\t');
        }
    }

    /// 退格键
    fn backspace(&mut self) {
        if self.cursor.column > 0 {
            self.save_state_for_undo();
            let line = &mut self.lines[self.cursor.line];
            line.remove(self.cursor.column - 1);
            self.cursor.column -= 1;
            self.is_modified = true;
        } else if self.cursor.line > 0 {
            self.save_state_for_undo();
            let current_line = self.lines.remove(self.cursor.line);
            self.cursor.line -= 1;
            self.cursor.column = self.lines[self.cursor.line].len();
            self.lines[self.cursor.line].push_str(&current_line);
            self.is_modified = true;
        }
    }

    /// 删除键
    fn delete(&mut self) {
        if self.cursor.line < self.lines.len() {
            if self.cursor.column < self.lines[self.cursor.line].len() {
                self.save_state_for_undo();
                self.lines[self.cursor.line].remove(self.cursor.column);
                self.is_modified = true;
            } else if self.cursor.line + 1 < self.lines.len() {
                self.save_state_for_undo();
                let next_line = self.lines.remove(self.cursor.line + 1);
                self.lines[self.cursor.line].push_str(&next_line);
                self.is_modified = true;
            }
        }
    }

    /// 光标移动方法
    fn move_cursor_up(&mut self) {
        if self.cursor.line > 0 {
            self.cursor.line -= 1;
            let line_len = self.lines.get(self.cursor.line).map_or(0, |l| l.len());
            self.cursor.column = min(self.cursor.column, line_len);
            self.adjust_scroll();
        }
    }

    fn move_cursor_down(&mut self) {
        if self.cursor.line < self.lines.len().saturating_sub(1) {
            self.cursor.line += 1;
            let line_len = self.lines.get(self.cursor.line).map_or(0, |l| l.len());
            self.cursor.column = min(self.cursor.column, line_len);
            self.adjust_scroll();
        }
    }

    fn move_cursor_left(&mut self) {
        if self.cursor.column > 0 {
            self.cursor.column -= 1;
        } else if self.cursor.line > 0 {
            self.cursor.line -= 1;
            self.cursor.column = self.lines.get(self.cursor.line).map_or(0, |l| l.len());
            self.adjust_scroll();
        }
    }

    fn move_cursor_right(&mut self) {
        if self.cursor.line < self.lines.len() {
            let line_len = self.lines[self.cursor.line].len();
            if self.cursor.column < line_len {
                self.cursor.column += 1;
            } else if self.cursor.line < self.lines.len() - 1 {
                self.cursor.line += 1;
                self.cursor.column = 0;
                self.adjust_scroll();
            }
        }
    }

    fn page_up(&mut self) {
        let page_size = self.viewport_height.saturating_sub(1);
        self.cursor.line = self.cursor.line.saturating_sub(page_size);
        let line_len = self.lines.get(self.cursor.line).map_or(0, |l| l.len());
        self.cursor.column = min(self.cursor.column, line_len);
        self.adjust_scroll();
    }

    fn page_down(&mut self) {
        let page_size = self.viewport_height.saturating_sub(1);
        self.cursor.line = min(self.cursor.line + page_size, self.lines.len().saturating_sub(1));
        let line_len = self.lines.get(self.cursor.line).map_or(0, |l| l.len());
        self.cursor.column = min(self.cursor.column, line_len);
        self.adjust_scroll();
    }

    /// 调整滚动偏移
    fn adjust_scroll(&mut self) {
        if self.cursor.line < self.scroll_offset {
            self.scroll_offset = self.cursor.line;
        } else if self.cursor.line >= self.scroll_offset + self.viewport_height {
            self.scroll_offset = self.cursor.line.saturating_sub(self.viewport_height - 1);
        }
    }

    /// 撤销操作
    fn undo(&mut self) {
        if let Some(state) = self.undo_history.pop_back() {
            self.redo_history.push_back(self.lines.clone());
            self.lines = state;
            self.is_modified = true;
            self.status_message = Some("Undone".to_string());
        }
    }

    /// 重做操作
    fn redo(&mut self) {
        if let Some(state) = self.redo_history.pop_back() {
            self.undo_history.push_back(self.lines.clone());
            self.lines = state;
            self.is_modified = true;
            self.status_message = Some("Redone".to_string());
        }
    }

    /// 保存状态用于撤销
    fn save_state_for_undo(&mut self) {
        self.undo_history.push_back(self.lines.clone());
        if self.undo_history.len() > self.config.max_undo_history {
            self.undo_history.pop_front();
        }
        self.redo_history.clear();
    }

    /// 复制选择的文本
    fn copy_selection(&mut self) {
        if let Some(selection) = self.selection {
            // 简化的复制实现
            // 在实际应用中，这里会将文本复制到剪贴板
            self.status_message = Some("Text copied".to_string());
        }
    }

    /// 渲染编辑器
    pub fn render(&mut self, frame: &mut Frame<'_>, area: Rect, theme: &Theme) -> AppResult<()> {
        self.viewport_height = area.height as usize;

        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(
                "Inline Editor - {} {}",
                self.file_path.as_ref()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("Untitled"),
                if self.is_modified { "*" } else { "" }
            ));

        let inner_area = block.inner(area);
        frame.render_widget(block, area);

        // 渲染文本内容
        self.render_text_content(frame, inner_area, theme)?;

        // 渲染状态栏
        if let Some(ref message) = self.status_message.clone() {
            self.render_status_message(frame, area, message, theme);
        }

        Ok(())
    }

    /// 渲染文本内容
    fn render_text_content(&mut self, frame: &mut Frame<'_>, area: Rect, theme: &Theme) -> AppResult<()> {
        let mut lines_to_render = Vec::new();
        let end_line = min(self.scroll_offset + area.height as usize, self.lines.len());

        for (i, line_idx) in (self.scroll_offset..end_line).enumerate() {
            let line_content = &self.lines[line_idx];

            // 构建行显示内容
            let mut spans = Vec::new();

            // 行号
            if self.config.show_line_numbers {
                spans.push(Span::styled(
                    format!("{:4} ", line_idx + 1),
                    Style::default().fg(Color::DarkGray)
                ));
            }

            // 行内容
            if line_idx == self.cursor.line {
                // 当前行高亮
                spans.push(Span::styled(
                    line_content.clone(),
                    Style::default().bg(Color::DarkGray)
                ));
            } else {
                spans.push(Span::raw(line_content.clone()));
            }

            lines_to_render.push(ListItem::new(Line::from(spans)));
        }

        let list = List::new(lines_to_render);
        frame.render_widget(list, area);

        Ok(())
    }

    /// 渲染状态消息
    fn render_status_message(&self, frame: &mut Frame<'_>, area: Rect, message: &str, theme: &Theme) {
        let status_area = Rect {
            x: area.x,
            y: area.y + area.height - 1,
            width: area.width,
            height: 1,
        };

        let status_text = Paragraph::new(message)
            .style(Style::default().bg(Color::Blue).fg(Color::White));

        frame.render_widget(status_text, status_area);
    }

    /// 获取当前模式
    pub fn get_mode(&self) -> EditorMode {
        self.mode.clone()
    }

    /// 是否已修改
    pub fn is_modified(&self) -> bool {
        self.is_modified
    }

    /// 获取文件路径
    pub fn get_file_path(&self) -> Option<&PathBuf> {
        self.file_path.as_ref()
    }

    /// 清除状态消息
    pub fn clear_status_message(&mut self) {
        self.status_message = None;
    }
}
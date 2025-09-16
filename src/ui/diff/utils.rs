use crate::{
    error::{AppError, AppResult},
    ui::diff::{DiffLine, DiffLineType, FileDiff, DiffHunk},
};
use std::path::PathBuf;
use tracing::{debug, warn};

/// 差异工具函数集合
pub struct DiffUtils;

impl DiffUtils {
    /// 从Git差异输出解析FileDiff
    pub fn parse_git_diff(git_diff_output: &str, file_path: &PathBuf) -> AppResult<FileDiff> {
        let lines: Vec<&str> = git_diff_output.lines().collect();
        let mut hunks = Vec::new();
        let mut current_hunk: Option<DiffHunk> = None;
        let mut stats = crate::ui::diff::DiffStats::default();

        let mut i = 0;
        while i < lines.len() {
            let line = lines[i];

            // 检查是否是hunk头部
            if line.starts_with("@@") {
                // 保存当前hunk
                if let Some(hunk) = current_hunk.take() {
                    hunks.push(hunk);
                }

                // 解析hunk头部
                if let Some(hunk) = Self::parse_hunk_header(line)? {
                    current_hunk = Some(hunk);
                }
            } else if line.starts_with('+') && !line.starts_with("+++") {
                // 添加行
                if let Some(ref mut hunk) = current_hunk {
                    hunk.lines.push(DiffLine {
                        line_type: DiffLineType::Added,
                        old_line_number: None,
                        new_line_number: Some(hunk.new_start + hunk.new_lines),
                        content: line[1..].to_string(),
                        highlights: vec![],
                    });
                    hunk.new_lines += 1;
                    stats.lines_added += 1;
                }
            } else if line.starts_with('-') && !line.starts_with("---") {
                // 删除行
                if let Some(ref mut hunk) = current_hunk {
                    hunk.lines.push(DiffLine {
                        line_type: DiffLineType::Deleted,
                        old_line_number: Some(hunk.old_start + hunk.old_lines),
                        new_line_number: None,
                        content: line[1..].to_string(),
                        highlights: vec![],
                    });
                    hunk.old_lines += 1;
                    stats.lines_deleted += 1;
                }
            } else if line.starts_with(' ') {
                // 上下文行
                if let Some(ref mut hunk) = current_hunk {
                    hunk.lines.push(DiffLine {
                        line_type: DiffLineType::Context,
                        old_line_number: Some(hunk.old_start + hunk.old_lines),
                        new_line_number: Some(hunk.new_start + hunk.new_lines),
                        content: line[1..].to_string(),
                        highlights: vec![],
                    });
                    hunk.old_lines += 1;
                    hunk.new_lines += 1;
                }
            }

            i += 1;
        }

        // 保存最后一个hunk
        if let Some(hunk) = current_hunk {
            hunks.push(hunk);
        }

        stats.files_changed = 1;

        Ok(FileDiff {
            old_path: Some(file_path.clone()),
            new_path: Some(file_path.clone()),
            status: crate::ui::diff::FileStatus::Modified,
            hunks,
            stats,
            is_binary: false,
        })
    }

    /// 解析hunk头部
    fn parse_hunk_header(line: &str) -> AppResult<Option<DiffHunk>> {
        // 格式: @@ -old_start,old_count +new_start,new_count @@
        if !line.starts_with("@@") {
            return Ok(None);
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            return Err(AppError::InvalidOperation(format!("Invalid hunk header: {}", line)));
        }

        let old_part = parts[1]; // -old_start,old_count
        let new_part = parts[2]; // +new_start,new_count

        let (old_start, old_lines) = Self::parse_range(old_part.trim_start_matches('-'))?;
        let (new_start, new_lines) = Self::parse_range(new_part.trim_start_matches('+'))?;

        Ok(Some(DiffHunk {
            header: line.to_string(),
            old_start,
            old_lines: 0, // 将在解析过程中填充
            new_start,
            new_lines: 0, // 将在解析过程中填充
            lines: vec![],
        }))
    }

    /// 解析范围字符串 (例如: "1,5" -> (1, 5))
    fn parse_range(range_str: &str) -> AppResult<(usize, usize)> {
        if let Some(comma_pos) = range_str.find(',') {
            let start_str = &range_str[..comma_pos];
            let count_str = &range_str[comma_pos + 1..];

            let start = start_str.parse::<usize>()
                .map_err(|_| AppError::InvalidOperation(format!("Invalid start number: {}", start_str)))?;
            let count = count_str.parse::<usize>()
                .map_err(|_| AppError::InvalidOperation(format!("Invalid count number: {}", count_str)))?;

            Ok((start, count))
        } else {
            let start = range_str.parse::<usize>()
                .map_err(|_| AppError::InvalidOperation(format!("Invalid range: {}", range_str)))?;
            Ok((start, 1))
        }
    }

    /// 计算单词级差异高亮
    pub fn compute_word_highlights(old_line: &str, new_line: &str) -> (Vec<(usize, usize)>, Vec<(usize, usize)>) {
        let old_words: Vec<&str> = old_line.split_whitespace().collect();
        let new_words: Vec<&str> = new_line.split_whitespace().collect();

        let mut old_highlights = Vec::new();
        let mut new_highlights = Vec::new();

        // 简化的单词级差异算法
        let mut old_pos = 0;
        let mut new_pos = 0;

        for (i, old_word) in old_words.iter().enumerate() {
            let old_start = old_pos;
            let old_end = old_pos + old_word.len();

            let mut found_match = false;
            let mut temp_new_pos = 0;

            for (j, new_word) in new_words.iter().enumerate() {
                if i == j && old_word == new_word {
                    found_match = true;
                    break;
                }
                temp_new_pos += new_word.len() + 1; // +1 for space
            }

            if !found_match {
                old_highlights.push((old_start, old_end));
            }

            old_pos = old_end + 1; // +1 for space
        }

        for (i, new_word) in new_words.iter().enumerate() {
            let new_start = new_pos;
            let new_end = new_pos + new_word.len();

            let mut found_match = false;

            for (j, old_word) in old_words.iter().enumerate() {
                if i == j && new_word == old_word {
                    found_match = true;
                    break;
                }
            }

            if !found_match {
                new_highlights.push((new_start, new_end));
            }

            new_pos = new_end + 1; // +1 for space
        }

        (old_highlights, new_highlights)
    }

    /// 检测文件的编程语言
    pub fn detect_language_from_path(file_path: &PathBuf) -> String {
        if let Some(extension) = file_path.extension().and_then(|ext| ext.to_str()) {
            match extension.to_lowercase().as_str() {
                "rs" => "rust",
                "py" => "python",
                "js" | "mjs" => "javascript",
                "ts" => "typescript",
                "jsx" => "jsx",
                "tsx" => "tsx",
                "html" | "htm" => "html",
                "css" => "css",
                "scss" | "sass" => "scss",
                "json" => "json",
                "yaml" | "yml" => "yaml",
                "toml" => "toml",
                "md" | "markdown" => "markdown",
                "sh" | "bash" => "bash",
                "zsh" | "fish" => "shell",
                "c" => "c",
                "cpp" | "cc" | "cxx" | "c++" => "cpp",
                "h" | "hpp" => "c",
                "go" => "go",
                "java" => "java",
                "kt" | "kts" => "kotlin",
                "swift" => "swift",
                "php" => "php",
                "rb" => "ruby",
                "pl" | "pm" => "perl",
                "lua" => "lua",
                "vim" => "vim",
                "xml" => "xml",
                "sql" => "sql",
                "dockerfile" => "dockerfile",
                "tf" => "terraform",
                _ => "text",
            }.to_string()
        } else {
            // 检查特殊文件名
            if let Some(filename) = file_path.file_name().and_then(|name| name.to_str()) {
                match filename.to_lowercase().as_str() {
                    "makefile" | "gnumakefile" => "makefile",
                    "dockerfile" => "dockerfile",
                    "cargo.toml" | "pyproject.toml" => "toml",
                    "package.json" | "tsconfig.json" => "json",
                    ".gitignore" | ".dockerignore" => "gitignore",
                    ".env" | ".env.local" | ".env.example" => "dotenv",
                    _ => "text",
                }.to_string()
            } else {
                "text".to_string()
            }
        }
    }

    /// 格式化差异统计信息为文本
    pub fn format_diff_stats(stats: &crate::ui::diff::DiffStats) -> Vec<String> {
        vec![
            format!("Files changed: {}", stats.files_changed),
            format!("Lines added: +{}", stats.lines_added),
            format!("Lines deleted: -{}", stats.lines_deleted),
            format!("Net change: {}",
                if stats.lines_added >= stats.lines_deleted {
                    format!("+{}", stats.lines_added - stats.lines_deleted)
                } else {
                    format!("-{}", stats.lines_deleted - stats.lines_added)
                }
            ),
            format!("Processing time: {:.2?}", stats.processing_time),
        ]
    }

    /// 生成差异摘要
    pub fn generate_diff_summary(diff: &FileDiff) -> String {
        let mut summary = String::new();

        match diff.status {
            crate::ui::diff::FileStatus::Added => {
                summary.push_str("New file");
            }
            crate::ui::diff::FileStatus::Deleted => {
                summary.push_str("Deleted file");
            }
            crate::ui::diff::FileStatus::Modified => {
                summary.push_str("Modified file");
            }
            crate::ui::diff::FileStatus::Renamed => {
                summary.push_str("Renamed file");
            }
            crate::ui::diff::FileStatus::Copied => {
                summary.push_str("Copied file");
            }
        }

        if diff.is_binary {
            summary.push_str(" (binary)");
        } else {
            summary.push_str(&format!(
                " (+{} -{} lines)",
                diff.stats.lines_added,
                diff.stats.lines_deleted
            ));
        }

        summary
    }

    /// 检查差异是否为空
    pub fn is_empty_diff(diff: &FileDiff) -> bool {
        diff.hunks.is_empty() || diff.hunks.iter().all(|hunk| hunk.lines.is_empty())
    }

    /// 获取差异的复杂度评分
    pub fn get_complexity_score(diff: &FileDiff) -> u32 {
        let mut score = 0;

        // 基于修改行数的评分
        score += diff.stats.lines_added + diff.stats.lines_deleted;

        // 基于修改块数的评分
        score += diff.hunks.len() as usize * 5;

        // 基于文件状态的评分
        score += match diff.status {
            crate::ui::diff::FileStatus::Added | crate::ui::diff::FileStatus::Deleted => 10,
            crate::ui::diff::FileStatus::Renamed | crate::ui::diff::FileStatus::Copied => 15,
            crate::ui::diff::FileStatus::Modified => 5,
        };

        score as u32
    }
}
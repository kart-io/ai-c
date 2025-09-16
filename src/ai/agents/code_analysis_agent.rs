//! Code Analysis Agent for automated code quality and impact assessment
//!
//! This agent provides comprehensive code analysis including:
//! - Code quality metrics and suggestions
//! - Impact assessment for changes
//! - Security vulnerability detection
//! - Performance optimization recommendations

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::fs;
use tracing::{debug, info, instrument, warn};

use crate::{
    ai::{
        client::{AIClient, AIRequest},
        prompt_manager::PromptManager,
        // suggestion_cache::SuggestionCache, // Temporarily disabled
    },
    error::{AppError, AppResult},
    git::GitService,
};

/// Code analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAnalysisConfig {
    /// Enable code quality analysis
    pub enable_quality_analysis: bool,
    /// Enable security analysis
    pub enable_security_analysis: bool,
    /// Enable performance analysis
    pub enable_performance_analysis: bool,
    /// Maximum file size to analyze (in bytes)
    pub max_file_size: usize,
    /// Supported file extensions for analysis
    pub supported_extensions: Vec<String>,
    /// Analysis timeout in seconds
    pub analysis_timeout: Duration,
    /// Cache TTL for analysis results
    pub cache_ttl: Duration,
    /// Maximum cache size for analysis results
    pub max_cache_size: usize,
}

impl Default for CodeAnalysisConfig {
    fn default() -> Self {
        Self {
            enable_quality_analysis: true,
            enable_security_analysis: true,
            enable_performance_analysis: true,
            max_file_size: 1024 * 1024, // 1MB
            supported_extensions: vec![
                "rs".to_string(),
                "py".to_string(),
                "js".to_string(),
                "ts".to_string(),
                "go".to_string(),
                "java".to_string(),
                "cpp".to_string(),
                "c".to_string(),
                "h".to_string(),
                "hpp".to_string(),
            ],
            analysis_timeout: Duration::from_secs(30),
            cache_ttl: Duration::from_secs(3600), // 1 hour
            max_cache_size: 1000,
        }
    }
}

/// Code analysis request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAnalysisRequest {
    /// Files to analyze
    pub files: Vec<PathBuf>,
    /// Analysis type
    pub analysis_type: AnalysisType,
    /// Include context from git history
    pub include_git_context: bool,
    /// Focus areas for analysis
    pub focus_areas: Vec<AnalysisFocus>,
    /// Additional parameters
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Types of code analysis
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AnalysisType {
    /// Full comprehensive analysis
    Full,
    /// Quick quality check
    Quick,
    /// Security-focused analysis
    Security,
    /// Performance-focused analysis
    Performance,
    /// Impact assessment for changes
    Impact,
    /// Custom analysis with specific criteria
    Custom(String),
}

/// Analysis focus areas
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AnalysisFocus {
    CodeQuality,
    Security,
    Performance,
    Maintainability,
    Documentation,
    Testing,
    Architecture,
    Dependencies,
}

/// Code analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAnalysisResult {
    /// Request ID for tracking
    pub request_id: String,
    /// Analysis summary
    pub summary: AnalysisSummary,
    /// Detailed findings
    pub findings: Vec<AnalysisFinding>,
    /// Overall score (0-100)
    pub overall_score: u8,
    /// Recommendations
    pub recommendations: Vec<Recommendation>,
    /// Analysis metadata
    pub metadata: AnalysisMetadata,
}

/// Analysis summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisSummary {
    /// Files analyzed
    pub files_analyzed: usize,
    /// Lines of code analyzed
    pub lines_analyzed: usize,
    /// Issues found by severity
    pub issues_by_severity: HashMap<IssueSeverity, usize>,
    /// Analysis duration
    pub analysis_duration: Duration,
    /// Top issue categories
    pub top_issues: Vec<String>,
}

/// Individual analysis finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisFinding {
    /// Issue ID
    pub id: String,
    /// Issue title
    pub title: String,
    /// Issue description
    pub description: String,
    /// Issue severity
    pub severity: IssueSeverity,
    /// File location
    pub file: PathBuf,
    /// Line number (if applicable)
    pub line: Option<usize>,
    /// Column number (if applicable)
    pub column: Option<usize>,
    /// Issue category
    pub category: IssueCategory,
    /// Suggested fix
    pub suggested_fix: Option<String>,
    /// Code snippet
    pub code_snippet: Option<String>,
}

/// Issue severity levels
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IssueSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Issue categories
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IssueCategory {
    CodeQuality,
    Security,
    Performance,
    Maintainability,
    Documentation,
    Testing,
    Architecture,
    Style,
    Bug,
    Vulnerability,
}

/// Analysis recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    /// Recommendation ID
    pub id: String,
    /// Recommendation title
    pub title: String,
    /// Detailed description
    pub description: String,
    /// Priority level
    pub priority: RecommendationPriority,
    /// Estimated effort
    pub effort: EstimatedEffort,
    /// Related findings
    pub related_findings: Vec<String>,
    /// Implementation steps
    pub implementation_steps: Vec<String>,
}

/// Recommendation priority
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RecommendationPriority {
    Urgent,
    High,
    Medium,
    Low,
    Optional,
}

/// Estimated effort for recommendations
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EstimatedEffort {
    Minutes(u32),
    Hours(u32),
    Days(u32),
    Weeks(u32),
    Unknown,
}

/// Analysis metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisMetadata {
    /// Analysis timestamp
    pub timestamp: DateTime<Utc>,
    /// Agent version
    pub agent_version: String,
    /// Analysis type used
    pub analysis_type: AnalysisType,
    /// Focus areas
    pub focus_areas: Vec<AnalysisFocus>,
    /// Git context included
    pub git_context_included: bool,
    /// Analysis configuration
    pub config_snapshot: serde_json::Value,
}

/// Code Analysis Agent
pub struct CodeAnalysisAgent {
    /// AI client for analysis
    ai_client: Box<dyn AIClient>,
    /// Prompt manager for generating analysis prompts
    prompt_manager: PromptManager,
    /// Cache for analysis results (temporarily disabled)
    // cache: SuggestionCache<CodeAnalysisResult>,
    /// Git repository reference
    git_repo: Option<GitService>,
    /// Agent configuration
    config: CodeAnalysisConfig,
    /// Agent statistics
    stats: AgentStats,
}

/// Agent statistics
#[derive(Debug, Default)]
struct AgentStats {
    total_analyses: u64,
    successful_analyses: u64,
    failed_analyses: u64,
    cache_hits: u64,
    cache_misses: u64,
    total_files_analyzed: u64,
    total_lines_analyzed: u64,
    average_analysis_time: Duration,
}

impl CodeAnalysisAgent {
    /// Create a new Code Analysis Agent
    pub fn new(
        ai_client: Box<dyn AIClient>,
        prompt_manager: PromptManager,
        git_repo: Option<GitService>,
        config: CodeAnalysisConfig,
    ) -> AppResult<Self> {
        // let cache = SuggestionCache::new(config.max_cache_size, config.cache_ttl)?;

        Ok(Self {
            ai_client,
            prompt_manager,
            // cache,
            git_repo,
            config,
            stats: AgentStats::default(),
        })
    }

    /// Perform code analysis
    #[instrument(skip(self, request))]
    pub async fn analyze_code(&mut self, request: CodeAnalysisRequest) -> AppResult<CodeAnalysisResult> {
        let start_time = std::time::Instant::now();
        let request_id = uuid::Uuid::new_v4().to_string();

        info!("Starting code analysis for {} files", request.files.len());

        // Check cache first (disabled)
        // let cache_key = self.generate_cache_key(&request);
        // if let Some(cached_result) = self.cache.get(&cache_key).await {
        //     self.stats.cache_hits += 1;
        //     debug!("Returning cached analysis result");
        //     return Ok(cached_result);
        // }
        self.stats.cache_misses += 1;

        // Validate files
        let valid_files = self.validate_files(&request.files).await?;
        if valid_files.is_empty() {
            return Err(AppError::agent("No valid files to analyze"));
        }

        // Collect file contents and metadata
        let file_data = self.collect_file_data(&valid_files).await?;

        // Generate git context if requested
        let git_context = if request.include_git_context {
            self.collect_git_context(&valid_files).await.unwrap_or_default()
        } else {
            String::new()
        };

        // Generate analysis prompt
        let analysis_prompt = self.generate_analysis_prompt(
            &request,
            &file_data,
            &git_context,
        ).await?;

        // Perform AI analysis
        let ai_request = AIRequest::new(analysis_prompt)
            .with_max_tokens(4000)
            .with_temperature(0.1); // Low temperature for consistent analysis

        let ai_response = self.ai_client.send_request(ai_request).await?;

        // Parse and structure the analysis result
        let analysis_result = self.parse_analysis_response(
            request_id,
            &request,
            &ai_response.text,
            &file_data,
            start_time.elapsed(),
        ).await?;

        // Cache the result (disabled)
        // self.cache.insert(cache_key, analysis_result.clone()).await;

        // Update statistics
        self.update_stats(true, start_time.elapsed(), &analysis_result);

        info!(
            "Code analysis completed in {}ms with {} findings",
            start_time.elapsed().as_millis(),
            analysis_result.findings.len()
        );

        Ok(analysis_result)
    }

    /// Validate files for analysis
    async fn validate_files(&self, files: &[PathBuf]) -> AppResult<Vec<PathBuf>> {
        let mut valid_files = Vec::new();

        for file in files {
            // Check if file exists
            if !file.exists() {
                warn!("File does not exist: {}", file.display());
                continue;
            }

            // Check file extension
            if let Some(extension) = file.extension() {
                if let Some(ext_str) = extension.to_str() {
                    if !self.config.supported_extensions.contains(&ext_str.to_lowercase()) {
                        debug!("Unsupported file extension: {}", ext_str);
                        continue;
                    }
                }
            }

            // Check file size
            if let Ok(metadata) = fs::metadata(file).await {
                if metadata.len() > self.config.max_file_size as u64 {
                    warn!("File too large to analyze: {}", file.display());
                    continue;
                }
            }

            valid_files.push(file.clone());
        }

        Ok(valid_files)
    }

    /// Collect file data for analysis
    async fn collect_file_data(&self, files: &[PathBuf]) -> AppResult<HashMap<PathBuf, FileData>> {
        let mut file_data = HashMap::new();

        for file in files {
            match fs::read_to_string(file).await {
                Ok(content) => {
                    let lines = content.lines().count();
                    let size = content.len();
                    file_data.insert(file.clone(), FileData {
                        content,
                        lines,
                        size,
                    });
                }
                Err(e) => {
                    warn!("Failed to read file {}: {}", file.display(), e);
                }
            }
        }

        Ok(file_data)
    }

    /// Collect git context for files
    async fn collect_git_context(&self, files: &[PathBuf]) -> AppResult<String> {
        if let Some(ref git_repo) = self.git_repo {
            let mut context = String::new();

            for file in files {
                if let Ok(history) = git_repo.get_file_history(file.to_str().unwrap_or(""), Some(5)).await {
                    context.push_str(&format!("\n--- Git History for {} ---\n", file.display()));
                    for commit in history {
                        context.push_str(&format!(
                            "Commit: {} - {} ({})\n",
                            &commit.hash[..8],
                            commit.message.lines().next().unwrap_or(""),
                            commit.author
                        ));
                    }
                }
            }

            Ok(context)
        } else {
            Ok(String::new())
        }
    }

    /// Generate analysis prompt based on request and context
    async fn generate_analysis_prompt(
        &self,
        request: &CodeAnalysisRequest,
        file_data: &HashMap<PathBuf, FileData>,
        git_context: &str,
    ) -> AppResult<String> {
        let mut context = HashMap::new();
        context.insert("analysis_type".to_string(), serde_json::to_value(&request.analysis_type)?);
        context.insert("focus_areas".to_string(), serde_json::to_value(&request.focus_areas)?);
        context.insert("file_count".to_string(), serde_json::Value::Number(file_data.len().into()));

        // Add file contents to context
        let mut files_content = String::new();
        for (path, data) in file_data {
            files_content.push_str(&format!(
                "\n--- File: {} ({} lines) ---\n{}\n",
                path.display(),
                data.lines,
                data.content
            ));
        }
        context.insert("files_content".to_string(), serde_json::Value::String(files_content));

        if !git_context.is_empty() {
            context.insert("git_context".to_string(), serde_json::Value::String(git_context.to_string()));
        }

        let template_name = match request.analysis_type {
            AnalysisType::Security => "security_analysis",
            AnalysisType::Performance => "performance_analysis",
            AnalysisType::Quick => "quick_analysis",
            _ => "full_code_analysis",
        };

        // Convert HashMap<String, Value> to HashMap<String, String>
        let string_context: HashMap<String, String> = context
            .into_iter()
            .map(|(k, v)| (k, v.as_str().unwrap_or(&v.to_string()).to_string()))
            .collect();

        self.prompt_manager.generate_prompt(template_name, &string_context).await
    }

    /// Parse AI response into structured analysis result
    async fn parse_analysis_response(
        &self,
        request_id: String,
        request: &CodeAnalysisRequest,
        response: &str,
        file_data: &HashMap<PathBuf, FileData>,
        duration: Duration,
    ) -> AppResult<CodeAnalysisResult> {
        // This is a simplified parser - in practice, you'd want more sophisticated parsing
        // or structured output from the AI model

        let findings = self.extract_findings_from_response(response, file_data);
        let recommendations = self.extract_recommendations_from_response(response);

        let lines_analyzed: usize = file_data.values().map(|d| d.lines).sum();
        let mut issues_by_severity = HashMap::new();

        for finding in &findings {
            *issues_by_severity.entry(finding.severity.clone()).or_insert(0) += 1;
        }

        let overall_score = self.calculate_overall_score(&findings);

        let summary = AnalysisSummary {
            files_analyzed: file_data.len(),
            lines_analyzed,
            issues_by_severity,
            analysis_duration: duration,
            top_issues: self.extract_top_issues(&findings),
        };

        let metadata = AnalysisMetadata {
            timestamp: Utc::now(),
            agent_version: "1.0.0".to_string(),
            analysis_type: request.analysis_type.clone(),
            focus_areas: request.focus_areas.clone(),
            git_context_included: request.include_git_context,
            config_snapshot: serde_json::to_value(&self.config)?,
        };

        Ok(CodeAnalysisResult {
            request_id,
            summary,
            findings,
            overall_score,
            recommendations,
            metadata,
        })
    }

    /// Extract findings from AI response
    fn extract_findings_from_response(
        &self,
        response: &str,
        file_data: &HashMap<PathBuf, FileData>,
    ) -> Vec<AnalysisFinding> {
        // Simplified extraction - in practice, this would be more sophisticated
        // and might use structured output formats or JSON parsing

        let mut findings = Vec::new();
        let lines: Vec<&str> = response.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            if line.contains("ISSUE:") || line.contains("WARNING:") || line.contains("ERROR:") {
                let finding = AnalysisFinding {
                    id: uuid::Uuid::new_v4().to_string(),
                    title: line.trim_start_matches("ISSUE:").trim_start_matches("WARNING:").trim_start_matches("ERROR:").trim().to_string(),
                    description: lines.get(i + 1).unwrap_or(&"").to_string(),
                    severity: self.determine_severity(line),
                    file: PathBuf::from("unknown"), // Would need better parsing
                    line: None,
                    column: None,
                    category: self.determine_category(line),
                    suggested_fix: None,
                    code_snippet: None,
                };
                findings.push(finding);
            }
        }

        findings
    }

    /// Extract recommendations from AI response
    fn extract_recommendations_from_response(&self, response: &str) -> Vec<Recommendation> {
        let mut recommendations = Vec::new();
        let lines: Vec<&str> = response.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            if line.contains("RECOMMENDATION:") {
                let recommendation = Recommendation {
                    id: uuid::Uuid::new_v4().to_string(),
                    title: line.trim_start_matches("RECOMMENDATION:").trim().to_string(),
                    description: lines.get(i + 1).unwrap_or(&"").to_string(),
                    priority: RecommendationPriority::Medium,
                    effort: EstimatedEffort::Unknown,
                    related_findings: Vec::new(),
                    implementation_steps: Vec::new(),
                };
                recommendations.push(recommendation);
            }
        }

        recommendations
    }

    /// Determine issue severity from text
    fn determine_severity(&self, text: &str) -> IssueSeverity {
        let text_lower = text.to_lowercase();
        if text_lower.contains("critical") || text_lower.contains("security") {
            IssueSeverity::Critical
        } else if text_lower.contains("error") || text_lower.contains("high") {
            IssueSeverity::High
        } else if text_lower.contains("warning") || text_lower.contains("medium") {
            IssueSeverity::Medium
        } else if text_lower.contains("low") {
            IssueSeverity::Low
        } else {
            IssueSeverity::Info
        }
    }

    /// Determine issue category from text
    fn determine_category(&self, text: &str) -> IssueCategory {
        let text_lower = text.to_lowercase();
        if text_lower.contains("security") || text_lower.contains("vulnerability") {
            IssueCategory::Security
        } else if text_lower.contains("performance") {
            IssueCategory::Performance
        } else if text_lower.contains("test") {
            IssueCategory::Testing
        } else if text_lower.contains("doc") {
            IssueCategory::Documentation
        } else if text_lower.contains("style") {
            IssueCategory::Style
        } else if text_lower.contains("bug") {
            IssueCategory::Bug
        } else {
            IssueCategory::CodeQuality
        }
    }

    /// Calculate overall score based on findings
    fn calculate_overall_score(&self, findings: &[AnalysisFinding]) -> u8 {
        if findings.is_empty() {
            return 100;
        }

        let mut penalty = 0;
        for finding in findings {
            penalty += match finding.severity {
                IssueSeverity::Critical => 25,
                IssueSeverity::High => 15,
                IssueSeverity::Medium => 10,
                IssueSeverity::Low => 5,
                IssueSeverity::Info => 1,
            };
        }

        (100 - penalty.min(100)) as u8
    }

    /// Extract top issue categories
    fn extract_top_issues(&self, findings: &[AnalysisFinding]) -> Vec<String> {
        let mut category_counts: HashMap<IssueCategory, usize> = HashMap::new();

        for finding in findings {
            *category_counts.entry(finding.category.clone()).or_insert(0) += 1;
        }

        let mut sorted_categories: Vec<_> = category_counts.into_iter().collect();
        sorted_categories.sort_by(|a, b| b.1.cmp(&a.1));

        sorted_categories.into_iter()
            .take(5)
            .map(|(category, count)| format!("{:?} ({})", category, count))
            .collect()
    }

    /// Generate cache key for analysis request
    fn generate_cache_key(&self, request: &CodeAnalysisRequest) -> String {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        use std::hash::{Hash, Hasher};

        request.files.hash(&mut hasher);
        request.analysis_type.hash(&mut hasher);
        request.focus_areas.hash(&mut hasher);
        request.include_git_context.hash(&mut hasher);

        format!("code_analysis_{:x}", hasher.finish())
    }

    /// Update agent statistics
    fn update_stats(&mut self, success: bool, duration: Duration, result: &CodeAnalysisResult) {
        self.stats.total_analyses += 1;
        if success {
            self.stats.successful_analyses += 1;
        } else {
            self.stats.failed_analyses += 1;
        }

        self.stats.total_files_analyzed += result.summary.files_analyzed as u64;
        self.stats.total_lines_analyzed += result.summary.lines_analyzed as u64;

        // Update average analysis time
        let total_time = self.stats.average_analysis_time.as_millis() as u64 * (self.stats.total_analyses - 1) + duration.as_millis() as u64;
        self.stats.average_analysis_time = Duration::from_millis(total_time / self.stats.total_analyses);
    }

    /// Get agent statistics
    pub fn stats(&self) -> &AgentStats {
        &self.stats
    }

    /// Get agent configuration
    pub fn config(&self) -> &CodeAnalysisConfig {
        &self.config
    }

    /// Update agent configuration
    pub fn update_config(&mut self, config: CodeAnalysisConfig) -> AppResult<()> {
        // Update cache settings if they changed (disabled)
        // if config.max_cache_size != self.config.max_cache_size || config.cache_ttl != self.config.cache_ttl {
        //     self.cache = SuggestionCache::new(config.max_cache_size, config.cache_ttl)?;
        // }

        self.config = config;
        Ok(())
    }
}

/// File data for analysis
#[derive(Debug, Clone)]
struct FileData {
    content: String,
    lines: usize,
    size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_analysis_config_default() {
        let config = CodeAnalysisConfig::default();
        assert!(config.enable_quality_analysis);
        assert!(config.enable_security_analysis);
        assert!(config.enable_performance_analysis);
        assert_eq!(config.max_file_size, 1024 * 1024);
        assert!(!config.supported_extensions.is_empty());
    }

    #[test]
    fn test_cache_key_generation() {
        // This test would need a mock setup, but demonstrates the concept
        let request = CodeAnalysisRequest {
            files: vec![PathBuf::from("test.rs")],
            analysis_type: AnalysisType::Quick,
            include_git_context: false,
            focus_areas: vec![AnalysisFocus::CodeQuality],
            parameters: HashMap::new(),
        };

        // In a real test, we'd create a CodeAnalysisAgent and test cache key generation
        // let agent = CodeAnalysisAgent::new(...);
        // let key = agent.generate_cache_key(&request);
        // assert!(!key.is_empty());
    }

    #[test]
    fn test_severity_determination() {
        // This test would need a mock setup, but demonstrates the concept
        let config = CodeAnalysisConfig::default();
        // let agent = CodeAnalysisAgent::new(...);

        // Test cases for severity determination
        let test_cases = vec![
            ("CRITICAL security vulnerability", IssueSeverity::Critical),
            ("ERROR: undefined behavior", IssueSeverity::High),
            ("WARNING: potential issue", IssueSeverity::Medium),
            ("low priority optimization", IssueSeverity::Low),
            ("info: suggestion", IssueSeverity::Info),
        ];

        // In a real test, we'd verify each case
        // for (text, expected) in test_cases {
        //     assert_eq!(agent.determine_severity(text), expected);
        // }
    }
}
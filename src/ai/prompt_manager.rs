//! Prompt Manager for AI prompt templates and management
//!
//! Provides centralized management of AI prompts with:
//! - Template-based prompt generation
//! - Multi-language support
//! - Prompt optimization and caching
//! - Dynamic parameter substitution

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tracing::{debug, info, warn};

use crate::{
    ai::agents::commit_agent::CommitMessageStyle,
    error::{AppError, AppResult},
};

/// Prompt template structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    /// Template name/identifier
    pub name: String,
    /// Template category (commit, analysis, review, etc.)
    pub category: String,
    /// Template content with placeholders
    pub template: String,
    /// Required parameters
    pub required_params: Vec<String>,
    /// Optional parameters with defaults
    pub optional_params: HashMap<String, String>,
    /// Template description
    pub description: String,
    /// Language/locale
    pub language: String,
    /// Template version
    pub version: String,
    /// Creation date
    pub created_at: DateTime<Utc>,
    /// Last modified date
    pub updated_at: DateTime<Utc>,
    /// Template metadata
    pub metadata: HashMap<String, String>,
}

impl PromptTemplate {
    /// Create a new prompt template
    pub fn new(name: String, category: String, template: String) -> Self {
        Self {
            name,
            category,
            template,
            required_params: Vec::new(),
            optional_params: HashMap::new(),
            description: String::new(),
            language: "en".to_string(),
            version: "1.0.0".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Add a required parameter
    pub fn with_required_param(mut self, param: String) -> Self {
        self.required_params.push(param);
        self
    }

    /// Add an optional parameter with default value
    pub fn with_optional_param(mut self, param: String, default: String) -> Self {
        self.optional_params.insert(param, default);
        self
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    /// Set language
    pub fn with_language(mut self, language: String) -> Self {
        self.language = language;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Render template with parameters
    pub fn render(&self, params: &HashMap<String, String>) -> AppResult<String> {
        let mut result = self.template.clone();

        // Check required parameters
        for param in &self.required_params {
            if !params.contains_key(param) {
                return Err(AppError::agent(format!(
                    "Missing required parameter: {}",
                    param
                )));
            }
        }

        // Substitute parameters
        for (key, value) in params {
            let placeholder = format!("{{{}}}", key);
            result = result.replace(&placeholder, value);
        }

        // Substitute optional parameters with defaults
        for (key, default_value) in &self.optional_params {
            let placeholder = format!("{{{}}}", key);
            if result.contains(&placeholder) {
                let value = params.get(key).unwrap_or(default_value);
                result = result.replace(&placeholder, value);
            }
        }

        Ok(result)
    }

    /// Validate template syntax
    pub fn validate(&self) -> AppResult<Vec<String>> {
        let mut warnings = Vec::new();

        // Check for unused placeholders
        let mut used_params = std::collections::HashSet::new();
        for param in &self.required_params {
            let placeholder = format!("{{{}}}", param);
            if self.template.contains(&placeholder) {
                used_params.insert(param.clone());
            } else {
                warnings.push(format!("Required parameter '{}' not used in template", param));
            }
        }

        // Check for undefined placeholders
        let placeholder_regex = regex::Regex::new(r"\{([^}]+)\}").unwrap();
        for cap in placeholder_regex.captures_iter(&self.template) {
            let param = cap.get(1).unwrap().as_str().to_string();
            if !self.required_params.contains(&param) && !self.optional_params.contains_key(&param) {
                warnings.push(format!("Undefined parameter '{}' in template", param));
            }
        }

        Ok(warnings)
    }
}

/// Prompt category for organizing templates
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PromptCategory {
    /// Commit message generation
    Commit,
    /// Code analysis
    Analysis,
    /// Code review
    Review,
    /// Documentation generation
    Documentation,
    /// Refactoring suggestions
    Refactoring,
    /// Test generation
    Testing,
    /// Custom category
    Custom(String),
}

impl PromptCategory {
    /// Get category name
    pub fn name(&self) -> &str {
        match self {
            PromptCategory::Commit => "commit",
            PromptCategory::Analysis => "analysis",
            PromptCategory::Review => "review",
            PromptCategory::Documentation => "documentation",
            PromptCategory::Refactoring => "refactoring",
            PromptCategory::Testing => "testing",
            PromptCategory::Custom(name) => name,
        }
    }
}

/// Prompt manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptManagerConfig {
    /// Directory to store prompt templates
    pub templates_dir: PathBuf,
    /// Default language for templates
    pub default_language: String,
    /// Enable template caching
    pub enable_caching: bool,
    /// Auto-reload templates on file changes
    pub auto_reload: bool,
    /// Template validation strictness
    pub strict_validation: bool,
}

impl Default for PromptManagerConfig {
    fn default() -> Self {
        Self {
            templates_dir: PathBuf::from("templates/prompts"),
            default_language: "en".to_string(),
            enable_caching: true,
            auto_reload: true,
            strict_validation: false,
        }
    }
}

/// Central prompt manager
pub struct PromptManager {
    /// Configuration
    config: PromptManagerConfig,
    /// Loaded templates by category and name
    templates: HashMap<String, HashMap<String, PromptTemplate>>,
    /// Template cache for performance
    cache: HashMap<String, String>,
    /// Last load time for auto-reload
    last_load: DateTime<Utc>,
}

impl PromptManager {
    /// Create a new prompt manager
    pub fn new(config: PromptManagerConfig) -> AppResult<Self> {
        let mut manager = Self {
            config,
            templates: HashMap::new(),
            cache: HashMap::new(),
            last_load: Utc::now(),
        };

        // Load default templates
        manager.load_default_templates()?;

        // Load templates from files if directory exists
        if manager.config.templates_dir.exists() {
            manager.load_templates_from_dir()?;
        } else {
            info!("Templates directory does not exist, using default templates only");
        }

        Ok(manager)
    }

    /// Load default built-in templates
    fn load_default_templates(&mut self) -> AppResult<()> {
        info!("Loading default prompt templates");

        // Commit message templates
        self.add_template(
            PromptTemplate::new(
                "conventional".to_string(),
                "commit".to_string(),
                "Analyze the following Git changes and generate a conventional commit message:\n\
                 Format: <type>(<scope>): <description>\n\
                 Types: feat, fix, docs, style, refactor, test, chore\n\n\
                 Files changed:\n{files}\n\n\
                 Diff content:\n{diff}\n\n\
                 Context: {context}\n\n\
                 Generate a concise commit message that follows conventional commits format.".to_string(),
            )
            .with_required_param("files".to_string())
            .with_required_param("diff".to_string())
            .with_optional_param("context".to_string(), "No additional context provided".to_string())
            .with_description("Conventional commit message generation".to_string()),
        )?;

        self.add_template(
            PromptTemplate::new(
                "simple".to_string(),
                "commit".to_string(),
                "Generate a simple, clear commit message for the following changes:\n\n\
                 Files changed:\n{files}\n\n\
                 Summary of changes: {summary}\n\n\
                 Write a concise commit message (max 72 characters) that describes what was changed.".to_string(),
            )
            .with_required_param("files".to_string())
            .with_required_param("summary".to_string())
            .with_description("Simple commit message generation".to_string()),
        )?;

        self.add_template(
            PromptTemplate::new(
                "detailed".to_string(),
                "commit".to_string(),
                "Generate a detailed commit message with subject and body for the following changes:\n\n\
                 Files changed:\n{files}\n\n\
                 Diff content:\n{diff}\n\n\
                 Context: {context}\n\n\
                 Create a commit message with:\n\
                 1. Short subject line (max 50 chars)\n\
                 2. Blank line\n\
                 3. Detailed body explaining what and why".to_string(),
            )
            .with_required_param("files".to_string())
            .with_required_param("diff".to_string())
            .with_optional_param("context".to_string(), "No additional context".to_string())
            .with_description("Detailed commit message with subject and body".to_string()),
        )?;

        // Code analysis templates
        self.add_template(
            PromptTemplate::new(
                "code_quality".to_string(),
                "analysis".to_string(),
                "Analyze the following code for quality issues:\n\n\
                 File: {file_path}\n\
                 Language: {language}\n\n\
                 Code:\n{code}\n\n\
                 Provide analysis focusing on:\n\
                 1. Code quality and readability\n\
                 2. Potential bugs or issues\n\
                 3. Performance considerations\n\
                 4. Best practices compliance\n\
                 5. Suggestions for improvement".to_string(),
            )
            .with_required_param("file_path".to_string())
            .with_required_param("code".to_string())
            .with_optional_param("language".to_string(), "auto-detect".to_string())
            .with_description("Code quality analysis".to_string()),
        )?;

        // Code review templates
        self.add_template(
            PromptTemplate::new(
                "pull_request_review".to_string(),
                "review".to_string(),
                "Review the following pull request changes:\n\n\
                 PR Title: {pr_title}\n\
                 PR Description: {pr_description}\n\n\
                 Files changed:\n{files_changed}\n\n\
                 Diff:\n{diff}\n\n\
                 Provide a thorough code review covering:\n\
                 1. Code correctness and logic\n\
                 2. Security considerations\n\
                 3. Performance implications\n\
                 4. Code style and conventions\n\
                 5. Test coverage\n\
                 6. Documentation needs\n\
                 7. Overall recommendations".to_string(),
            )
            .with_required_param("pr_title".to_string())
            .with_required_param("files_changed".to_string())
            .with_required_param("diff".to_string())
            .with_optional_param("pr_description".to_string(), "No description provided".to_string())
            .with_description("Comprehensive pull request review".to_string()),
        )?;

        info!("Loaded {} default templates", self.templates.len());
        Ok(())
    }

    /// Load templates from directory
    fn load_templates_from_dir(&mut self) -> AppResult<()> {
        info!("Loading templates from directory: {:?}", self.config.templates_dir);

        if !self.config.templates_dir.exists() {
            std::fs::create_dir_all(&self.config.templates_dir).map_err(|e| {
                AppError::agent(format!("Failed to create templates directory: {}", e))
            })?;
            return Ok(());
        }

        let entries = std::fs::read_dir(&self.config.templates_dir).map_err(|e| {
            AppError::agent(format!("Failed to read templates directory: {}", e))
        })?;

        let mut loaded_count = 0;
        for entry in entries {
            let entry = entry.map_err(|e| {
                AppError::agent(format!("Failed to read directory entry: {}", e))
            })?;

            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                match self.load_template_from_file(&path) {
                    Ok(template) => {
                        self.add_template(template)?;
                        loaded_count += 1;
                    }
                    Err(e) => {
                        warn!("Failed to load template from {:?}: {}", path, e);
                    }
                }
            }
        }

        info!("Loaded {} templates from files", loaded_count);
        Ok(())
    }

    /// Load a single template from file
    fn load_template_from_file(&self, path: &Path) -> AppResult<PromptTemplate> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            AppError::agent(format!("Failed to read template file {:?}: {}", path, e))
        })?;

        let template: PromptTemplate = serde_json::from_str(&content).map_err(|e| {
            AppError::agent(format!("Failed to parse template file {:?}: {}", path, e))
        })?;

        debug!("Loaded template '{}' from file", template.name);
        Ok(template)
    }

    /// Add a template to the manager
    pub fn add_template(&mut self, template: PromptTemplate) -> AppResult<()> {
        // Validate template if strict validation is enabled
        if self.config.strict_validation {
            let warnings = template.validate()?;
            if !warnings.is_empty() {
                return Err(AppError::agent(format!(
                    "Template validation failed: {}",
                    warnings.join(", ")
                )));
            }
        }

        // Add to templates map
        let category_name = template.category.clone();
        let template_name = template.name.clone();
        let category_map = self.templates.entry(template.category.clone()).or_insert_with(HashMap::new);
        category_map.insert(template.name.clone(), template);

        debug!("Added template '{}' to category '{}'", template_name, category_name);
        Ok(())
    }

    /// Get a template by category and name
    pub fn get_template(&self, category: &str, name: &str) -> Option<&PromptTemplate> {
        self.templates.get(category)?.get(name)
    }

    /// Get all templates in a category
    pub fn get_templates_by_category(&self, category: &str) -> Vec<&PromptTemplate> {
        self.templates
            .get(category)
            .map(|category_map| category_map.values().collect())
            .unwrap_or_default()
    }

    /// Generate a prompt from a template
    pub async fn generate_prompt(&self, template_name: &str, params: &HashMap<String, String>) -> AppResult<String> {
        // Try to find template in any category
        for category_map in self.templates.values() {
            if let Some(template) = category_map.get(template_name) {
                return template.render(params);
            }
        }

        Err(AppError::agent(format!("Template '{}' not found", template_name)))
    }

    /// List all available categories
    pub fn list_categories(&self) -> Vec<&str> {
        self.templates.keys().map(|s| s.as_str()).collect()
    }

    /// List all templates
    pub fn list_all_templates(&self) -> Vec<&PromptTemplate> {
        self.templates
            .values()
            .flat_map(|category_map| category_map.values())
            .collect()
    }

    /// Render a template with parameters
    pub fn render_template(
        &mut self,
        category: &str,
        name: &str,
        params: &HashMap<String, String>,
    ) -> AppResult<String> {
        // Check cache first if enabled
        if self.config.enable_caching {
            let cache_key = format!("{}:{}:{:?}", category, name, params);
            if let Some(cached) = self.cache.get(&cache_key) {
                debug!("Using cached render for {}:{}", category, name);
                return Ok(cached.clone());
            }
        }

        // Get template
        let template = self.get_template(category, name)
            .ok_or_else(|| AppError::agent(format!("Template not found: {}:{}", category, name)))?;

        // Render template
        let result = template.render(params)?;

        // Cache result if enabled
        if self.config.enable_caching {
            let cache_key = format!("{}:{}:{:?}", category, name, params);
            self.cache.insert(cache_key, result.clone());
        }

        debug!("Rendered template {}:{}", category, name);
        Ok(result)
    }

    /// Get template for commit message style
    pub fn get_commit_template(&self, style: &CommitMessageStyle) -> Option<&PromptTemplate> {
        let template_name = match style {
            CommitMessageStyle::Conventional => "conventional",
            CommitMessageStyle::Simple => "simple",
            CommitMessageStyle::Detailed => "detailed",
            CommitMessageStyle::Custom(_) => return None, // Custom templates not supported yet
        };

        self.get_template("commit", template_name)
    }

    /// Clear template cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
        debug!("Cleared template cache");
    }

    /// Reload templates from directory
    pub fn reload_templates(&mut self) -> AppResult<()> {
        info!("Reloading prompt templates");

        // Clear existing templates (except default ones)
        let default_categories = vec!["commit", "analysis", "review"];
        self.templates.retain(|category, _| default_categories.contains(&category.as_str()));

        // Clear cache
        self.clear_cache();

        // Reload from directory
        self.load_templates_from_dir()?;

        self.last_load = Utc::now();
        info!("Templates reloaded successfully");

        Ok(())
    }

    /// Save a template to file
    pub fn save_template(&self, template: &PromptTemplate) -> AppResult<()> {
        let filename = format!("{}.json", template.name);
        let filepath = self.config.templates_dir.join(filename);

        // Ensure directory exists
        if let Some(parent) = filepath.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppError::agent(format!("Failed to create template directory: {}", e))
            })?;
        }

        // Serialize template
        let content = serde_json::to_string_pretty(template).map_err(|e| {
            AppError::agent(format!("Failed to serialize template: {}", e))
        })?;

        // Write to file
        std::fs::write(&filepath, content).map_err(|e| {
            AppError::agent(format!("Failed to write template file: {}", e))
        })?;

        info!("Saved template '{}' to {:?}", template.name, filepath);
        Ok(())
    }

    /// Get manager statistics
    pub fn stats(&self) -> PromptManagerStats {
        let total_templates = self.templates.values()
            .map(|category_map| category_map.len())
            .sum();

        PromptManagerStats {
            total_templates,
            total_categories: self.templates.len(),
            cache_size: self.cache.len(),
            last_load: self.last_load,
        }
    }
}

/// Prompt manager statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptManagerStats {
    /// Total number of templates
    pub total_templates: usize,
    /// Total number of categories
    pub total_categories: usize,
    /// Current cache size
    pub cache_size: usize,
    /// Last template load time
    pub last_load: DateTime<Utc>,
}
//! Search indexing system for fast full-text search

use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::{error::AppResult, git::service::GitService};

/// Document for indexing
#[derive(Debug, Clone)]
pub struct IndexDocument {
    pub id: String,
    pub title: String,
    pub content: String,
    pub file_path: Option<String>,
    pub commit_hash: Option<String>,
    pub branch_name: Option<String>,
    pub author: Option<String>,
    pub timestamp: Option<String>,
    pub doc_type: DocumentType,
}

/// Types of documents that can be indexed
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DocumentType {
    File,
    Commit,
    Branch,
    Tag,
    Remote,
    Stash,
}

/// Term in the inverted index
#[derive(Debug, Clone)]
pub struct IndexTerm {
    pub term: String,
    pub documents: HashMap<String, TermOccurrence>,
}

/// Occurrence of a term in a document
#[derive(Debug, Clone)]
pub struct TermOccurrence {
    pub document_id: String,
    pub frequency: usize,
    pub positions: Vec<usize>,
    pub field: String, // title, content, etc.
}

/// Search index for fast text search
pub struct SearchIndex {
    documents: HashMap<String, IndexDocument>,
    inverted_index: HashMap<String, IndexTerm>,
    last_updated: Instant,
    is_building: RwLock<bool>,
}

impl SearchIndex {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            inverted_index: HashMap::new(),
            last_updated: Instant::now(),
            is_building: RwLock::new(false),
        }
    }

    /// Check if index is currently being built
    pub async fn is_building(&self) -> bool {
        *self.is_building.read().await
    }

    /// Build index from Git repository
    pub async fn build_from_git(&mut self, git_service: &GitService) -> AppResult<()> {
        {
            let mut building = self.is_building.write().await;
            *building = true;
        }

        // Clear existing index
        self.documents.clear();
        self.inverted_index.clear();

        // Index files
        self.index_files(git_service).await?;

        // Index commits
        self.index_commits(git_service).await?;

        // Index branches
        self.index_branches(git_service).await?;

        // Index tags
        self.index_tags(git_service).await?;

        // Index remotes
        self.index_remotes(git_service).await?;

        self.last_updated = Instant::now();

        {
            let mut building = self.is_building.write().await;
            *building = false;
        }

        Ok(())
    }

    /// Index files from repository
    async fn index_files(&mut self, git_service: &GitService) -> AppResult<()> {
        let status = git_service.get_status().await?;

        for file in &status {
            let doc_id = format!("file:{}", file.path);

            // Get file content for indexing
            let content = match git_service.get_file_content_at_head(&std::path::Path::new(&file.path)) {
                Ok(content) => content,
                Err(_) => continue, // Skip files that can't be read
            };

            let document = IndexDocument {
                id: doc_id.clone(),
                title: file.path.clone(),
                content,
                file_path: Some(file.path.clone()),
                commit_hash: None,
                branch_name: None,
                author: None,
                timestamp: None,
                doc_type: DocumentType::File,
            };

            self.add_document_to_index(document);
        }

        Ok(())
    }

    /// Index commits from repository
    async fn index_commits(&mut self, git_service: &GitService) -> AppResult<()> {
        let commits = git_service.get_commits(1000).await?; // Index last 1000 commits

        for commit in &commits {
            let doc_id = format!("commit:{}", commit.hash);

            let document = IndexDocument {
                id: doc_id.clone(),
                title: commit.message.lines().next().unwrap_or("").to_string(),
                content: commit.message.clone(),
                file_path: None,
                commit_hash: Some(commit.hash.clone()),
                branch_name: None,
                author: Some(commit.author.clone()),
                timestamp: Some(commit.date.to_rfc3339()),
                doc_type: DocumentType::Commit,
            };

            self.add_document_to_index(document);
        }

        Ok(())
    }

    /// Index branches from repository
    async fn index_branches(&mut self, git_service: &GitService) -> AppResult<()> {
        let branches = git_service.get_branches().await?;

        for branch in &branches {
            let doc_id = format!("branch:{}", branch.name);

            let content = format!(
                "{} {} {}",
                branch.name,
                if branch.is_local { "local" } else { "remote" },
                if branch.is_current { "current" } else { "" }
            );

            let document = IndexDocument {
                id: doc_id.clone(),
                title: branch.name.clone(),
                content,
                file_path: None,
                commit_hash: None,
                branch_name: Some(branch.name.clone()),
                author: None,
                timestamp: None,
                doc_type: DocumentType::Branch,
            };

            self.add_document_to_index(document);
        }

        Ok(())
    }

    /// Index tags from repository
    async fn index_tags(&mut self, git_service: &GitService) -> AppResult<()> {
        let tags = git_service.get_tags().await?;

        for tag in &tags {
            let doc_id = format!("tag:{}", tag.name);

            let content = format!("{} {}", tag.name, tag.target);

            let document = IndexDocument {
                id: doc_id.clone(),
                title: tag.name.clone(),
                content,
                file_path: None,
                commit_hash: Some(tag.target.clone()),
                branch_name: None,
                author: None,
                timestamp: None,
                doc_type: DocumentType::Tag,
            };

            self.add_document_to_index(document);
        }

        Ok(())
    }

    /// Index remotes from repository
    async fn index_remotes(&mut self, git_service: &GitService) -> AppResult<()> {
        let remotes = git_service.get_remotes().await?;

        for remote in &remotes {
            let doc_id = format!("remote:{}", remote.name);

            let content = format!("{} {}", remote.name, remote.url);

            let document = IndexDocument {
                id: doc_id.clone(),
                title: remote.name.clone(),
                content,
                file_path: None,
                commit_hash: None,
                branch_name: None,
                author: None,
                timestamp: None,
                doc_type: DocumentType::Remote,
            };

            self.add_document_to_index(document);
        }

        Ok(())
    }

    /// Add document to inverted index
    fn add_document_to_index(&mut self, document: IndexDocument) {
        let doc_id = document.id.clone();

        // Tokenize and index title
        self.index_text(&doc_id, &document.title, "title");

        // Tokenize and index content
        self.index_text(&doc_id, &document.content, "content");

        // Index file path if present
        if let Some(ref path) = document.file_path {
            self.index_text(&doc_id, path, "file_path");
        }

        // Index author if present
        if let Some(ref author) = document.author {
            self.index_text(&doc_id, author, "author");
        }

        // Store document
        self.documents.insert(doc_id, document);
    }

    /// Index text content for a document
    fn index_text(&mut self, doc_id: &str, text: &str, field: &str) {
        let tokens = self.tokenize(text);

        for (position, token) in tokens.iter().enumerate() {
            let term = self.inverted_index
                .entry(token.clone())
                .or_insert_with(|| IndexTerm {
                    term: token.clone(),
                    documents: HashMap::new(),
                });

            let occurrence = term.documents
                .entry(doc_id.to_string())
                .or_insert_with(|| TermOccurrence {
                    document_id: doc_id.to_string(),
                    frequency: 0,
                    positions: Vec::new(),
                    field: field.to_string(),
                });

            occurrence.frequency += 1;
            occurrence.positions.push(position);
        }
    }

    /// Tokenize text into searchable terms
    fn tokenize(&self, text: &str) -> Vec<String> {
        text.to_lowercase()
            .split_whitespace()
            .filter(|word| word.len() > 1) // Filter out single characters
            .map(|word| {
                // Remove common punctuation
                word.trim_matches(|c: char| !c.is_alphanumeric())
                    .to_string()
            })
            .filter(|word| !word.is_empty())
            .collect()
    }

    /// Search the index for documents matching query
    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchMatch> {
        let query_terms = self.tokenize(query);
        if query_terms.is_empty() {
            return Vec::new();
        }

        let mut document_scores: HashMap<String, f64> = HashMap::new();

        // Calculate TF-IDF scores for each document
        for term in &query_terms {
            if let Some(index_term) = self.inverted_index.get(term) {
                let idf = self.calculate_idf(index_term.documents.len());

                for (doc_id, occurrence) in &index_term.documents {
                    let tf = occurrence.frequency as f64;
                    let score = tf * idf;

                    // Boost score based on field
                    let field_boost = match occurrence.field.as_str() {
                        "title" => 2.0,
                        "file_path" => 1.5,
                        "author" => 1.2,
                        _ => 1.0,
                    };

                    *document_scores.entry(doc_id.clone()).or_insert(0.0) += score * field_boost;
                }
            }
        }

        // Convert to search matches and sort by score
        let mut matches: Vec<SearchMatch> = document_scores
            .into_iter()
            .filter_map(|(doc_id, score)| {
                self.documents.get(&doc_id).map(|doc| SearchMatch {
                    document: doc.clone(),
                    score,
                    matched_terms: query_terms.clone(),
                })
            })
            .collect();

        matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        matches.truncate(limit);

        matches
    }

    /// Calculate Inverse Document Frequency
    fn calculate_idf(&self, docs_with_term: usize) -> f64 {
        if docs_with_term == 0 {
            0.0
        } else {
            (self.documents.len() as f64 / docs_with_term as f64).ln()
        }
    }

    /// Get index statistics
    pub fn stats(&self) -> IndexStats {
        IndexStats {
            total_documents: self.documents.len(),
            total_terms: self.inverted_index.len(),
            last_updated: self.last_updated,
            document_types: self.count_document_types(),
        }
    }

    /// Count documents by type
    fn count_document_types(&self) -> HashMap<DocumentType, usize> {
        let mut counts = HashMap::new();

        for doc in self.documents.values() {
            *counts.entry(doc.doc_type.clone()).or_insert(0) += 1;
        }

        counts
    }

    /// Get index size in bytes (estimate)
    pub fn estimated_memory_usage(&self) -> usize {
        let mut total = 0;

        // Documents
        for doc in self.documents.values() {
            total += std::mem::size_of::<IndexDocument>();
            total += doc.id.len();
            total += doc.title.len();
            total += doc.content.len();
            if let Some(ref path) = doc.file_path {
                total += path.len();
            }
            if let Some(ref hash) = doc.commit_hash {
                total += hash.len();
            }
            if let Some(ref branch) = doc.branch_name {
                total += branch.len();
            }
            if let Some(ref author) = doc.author {
                total += author.len();
            }
            if let Some(ref timestamp) = doc.timestamp {
                total += timestamp.len();
            }
        }

        // Inverted index
        for term in self.inverted_index.values() {
            total += std::mem::size_of::<IndexTerm>();
            total += term.term.len();

            for occurrence in term.documents.values() {
                total += std::mem::size_of::<TermOccurrence>();
                total += occurrence.document_id.len();
                total += occurrence.field.len();
                total += occurrence.positions.len() * std::mem::size_of::<usize>();
            }
        }

        total
    }
}

/// Search match result
#[derive(Debug, Clone)]
pub struct SearchMatch {
    pub document: IndexDocument,
    pub score: f64,
    pub matched_terms: Vec<String>,
}

/// Index statistics
#[derive(Debug, Clone)]
pub struct IndexStats {
    pub total_documents: usize,
    pub total_terms: usize,
    pub last_updated: Instant,
    pub document_types: HashMap<DocumentType, usize>,
}

impl Default for SearchIndex {
    fn default() -> Self {
        Self::new()
    }
}
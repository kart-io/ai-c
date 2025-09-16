//! Concrete agent implementations
//!
//! This module contains specific agent implementations for various AI tasks:
//! - CommitAgent: Intelligent commit message generation
//! - CodeAnalysisAgent: Code quality and impact analysis
//! - ReviewAgent: Automated code review
//! - SearchAgent: Semantic search and query optimization

pub mod commit_agent;
pub mod code_analysis_agent;

pub use commit_agent::{
    CommitAgent, CommitAgentConfig, CommitGenerationRequest, CommitMessageStyle,
    GeneratedCommitMessage,
};
pub use code_analysis_agent::{
    CodeAnalysisAgent, CodeAnalysisConfig, CodeAnalysisRequest, CodeAnalysisResult,
    AnalysisType, AnalysisFocus, AnalysisFinding, Recommendation, IssueSeverity,
    IssueCategory, RecommendationPriority, EstimatedEffort,
};

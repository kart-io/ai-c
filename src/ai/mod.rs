//! AI Agent system module
//!
//! Provides an Actor-based AI agent system with:
//! - Agent lifecycle management
//! - High-performance message bus
//! - Task scheduling and load balancing
//! - Health monitoring and fault tolerance
//! - MCP protocol integration

pub mod agent;
pub mod agents;
pub mod client;
// pub mod collaboration; // Temporarily disabled due to compilation errors
pub mod config_manager;
pub mod health;
// pub mod hot_swap; // Temporarily disabled due to compilation errors
// pub mod load_balancer; // Temporarily disabled due to compilation errors
pub mod manager;
pub mod message_bus;
pub mod metrics_collector;
pub mod prompt_manager;
pub mod recovery;
pub mod scheduler;
// pub mod suggestion_cache; // Temporarily disabled due to compilation errors
// pub mod timeout_retry; // Temporarily disabled due to compilation errors

pub use agent::{
    Agent, AgentCapability, AgentConfig, AgentError, AgentMessage, AgentResult,
    AgentStatus, AgentTask, HealthStatus, MessageType
};
pub use client::{AIClient, AIClientConfig, AIProvider, AIRequest, AIResponse, HttpAIClient};
// pub use collaboration::{
//     CollaborationOrchestrator, CollaborationRequest, CollaborationResult, CollaborationTaskType,
//     CollaborativeAgent, AgentContribution, QualityRequirements, QualityMetrics
// };
// pub use suggestion_cache::{SuggestionCache, SuggestionCacheConfig, CacheStats};
// pub use load_balancer::{
//     LoadBalancer, LoadBalancingStrategy, TaskRequirements, AgentLoad, ResourceRequirements,
//     RoundRobinStrategy, LeastLoadedStrategy, CapabilityBasedStrategy
// };
// pub use timeout_retry::{
//     TimeoutRetryExecutor, RetryConfig, RetryableOperation, RetryResult, RetryableError
// };
// pub use hot_swap::{
//     HotSwapManager, HotSwapConfig, HotSwapRequest, HotSwapResult, HotSwapOperation,
//     HotSwappableAgent, AgentCreator, AgentFactory, MigrationStrategy, SwapPriority
// };
pub use config_manager::{
    ConfigManager, ConfigStore, FileConfigStore, ConfigError, ConfigValidator, BasicConfigValidator
};
pub use health::{
    HealthMonitor, HealthMonitorConfig, MonitorError, MonitoredAgent, HealthReport,
    HealthCheck, ResponseTimeCheck, MemoryUsageCheck, AlertingService, LoggingAlertingService,
    HealthMonitorStatistics
};
pub use manager::{AgentManager, AgentManagerConfig, SystemStatus};
pub use metrics_collector::{
    MetricsCollector, MetricsRegistry, Metric, MetricsError, SystemMetrics,
    MetricsExporter, PrometheusExporter, JsonFileExporter, MetricsConfig
};
pub use message_bus::MessageBus;
pub use prompt_manager::{PromptManager, PromptManagerConfig, PromptTemplate, PromptCategory};
pub use recovery::{
    FailureRecoveryManager, FailureRecoveryConfig, CircuitBreakerConfig, FailureDetectionConfig,
    FailureType, FailureInfo, FailureSeverity, RecoveryError, CircuitBreakerState, CircuitBreaker,
    RecoveryStrategy, RestartRecoveryStrategy, DegradedModeRecoveryStrategy, FailoverRecoveryStrategy,
    FailureDetector, RecoveryStatistics
};
pub use scheduler::{
    SchedulingStrategy, TaskScheduler, TaskSchedulerConfig, TaskStatus, RunningTask,
    AgentAvailability, PriorityBasedStrategy, CapabilityMatchStrategy,
    SchedulerStatistics
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Agent task priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaskPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Default for TaskPriority {
    fn default() -> Self {
        TaskPriority::Normal
    }
}

/// Agent performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetrics {
    /// Total tasks processed
    pub tasks_processed: u64,
    /// Average response time
    pub average_response_time: Duration,
    /// Error rate (0.0 to 1.0)
    pub error_rate: f64,
    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,
    /// Memory usage in bytes
    pub memory_usage: u64,
    /// CPU usage percentage
    pub cpu_usage: f32,
}

impl Default for AgentMetrics {
    fn default() -> Self {
        Self {
            tasks_processed: 0,
            average_response_time: Duration::ZERO,
            error_rate: 0.0,
            last_activity: Utc::now(),
            memory_usage: 0,
            cpu_usage: 0.0,
        }
    }
}

/// Agent information for management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    /// Agent unique identifier
    pub id: String,
    /// Agent type/name
    pub agent_type: String,
    /// Agent capabilities
    pub capabilities: Vec<AgentCapability>,
    /// Current health status
    pub health_status: HealthStatus,
    /// Performance metrics
    pub metrics: AgentMetrics,
    /// Whether the agent is currently active
    pub is_active: bool,
    /// Agent creation time
    pub created_at: DateTime<Utc>,
    /// Agent last update time
    pub updated_at: DateTime<Utc>,
}

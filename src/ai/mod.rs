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
pub mod health;
pub mod manager;
pub mod message_bus;
pub mod scheduler;

pub use agent::{Agent, AgentCapability, AgentResult, AgentTask, HealthStatus};
pub use health::HealthMonitor;
pub use manager::AgentManager;
pub use message_bus::{AgentMessage, MessageBus};
pub use scheduler::{SchedulingStrategy, TaskScheduler};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

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

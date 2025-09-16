//! Agent Manager for lifecycle management and task distribution
//!
//! The AgentManager is responsible for:
//! - Agent registration and lifecycle management
//! - Task distribution and load balancing
//! - Health monitoring and fault tolerance
//! - System status monitoring

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::Duration,
};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::{
    Agent, AgentError,
    AgentTask, AgentInfo, HealthStatus, MessageBus
};

/// Agent Manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentManagerConfig {
    /// Maximum number of agents that can be registered
    pub max_agents: usize,
    /// Default task timeout
    pub default_task_timeout: Duration,
    /// Health check interval
    pub health_check_interval: Duration,
    /// Maximum concurrent tasks per agent
    pub max_concurrent_tasks_per_agent: usize,
    /// Agent startup timeout
    pub agent_startup_timeout: Duration,
    /// Enable automatic fault recovery
    pub enable_fault_recovery: bool,
}

impl Default for AgentManagerConfig {
    fn default() -> Self {
        Self {
            max_agents: 50,
            default_task_timeout: Duration::from_secs(30),
            health_check_interval: Duration::from_secs(10),
            max_concurrent_tasks_per_agent: 10,
            agent_startup_timeout: Duration::from_secs(5),
            enable_fault_recovery: true,
        }
    }
}

/// System status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    /// Total number of registered agents
    pub total_agents: usize,
    /// Number of healthy agents
    pub healthy_agents: usize,
    /// Number of agents in error state
    pub error_agents: usize,
    /// Total tasks processed
    pub total_tasks_processed: u64,
    /// Average system load
    pub average_load: f32,
    /// System uptime
    pub uptime: Duration,
    /// Last status update timestamp
    pub last_updated: DateTime<Utc>,
}

/// Agent wrapper for internal management
struct ManagedAgent {
    /// The actual agent instance
    agent: Box<dyn Agent>,
    /// Agent metadata
    info: AgentInfo,
    /// Current task assignments
    active_tasks: HashMap<Uuid, AgentTask>,
    /// Message receiver for this agent
    message_receiver: Option<mpsc::UnboundedReceiver<super::message_bus::AgentMessage>>,
    /// Last health check timestamp
    last_health_check: DateTime<Utc>,
}

/// Agent Manager implementation
pub struct AgentManager {
    /// Registered agents
    agents: Arc<RwLock<HashMap<String, ManagedAgent>>>,
    /// Message bus for inter-agent communication
    message_bus: Arc<MessageBus>,
    /// Configuration
    config: AgentManagerConfig,
    /// System startup time
    startup_time: DateTime<Utc>,
    /// Global task counter
    task_counter: Arc<RwLock<u64>>,
    /// System metrics
    metrics: Arc<RwLock<SystemMetrics>>,
}

/// Internal system metrics
#[derive(Debug, Default)]
struct SystemMetrics {
    pub total_tasks_dispatched: u64,
    pub total_tasks_completed: u64,
    pub total_tasks_failed: u64,
    pub agent_registrations: u64,
    pub agent_deregistrations: u64,
}

impl AgentManager {
    /// Create a new Agent Manager
    pub async fn new(config: AgentManagerConfig) -> Result<Self, AgentError> {
        info!("Initializing Agent Manager with config: {:?}", config);

        let message_bus = Arc::new(MessageBus::new());

        Ok(Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            message_bus,
            config,
            startup_time: Utc::now(),
            task_counter: Arc::new(RwLock::new(0)),
            metrics: Arc::new(RwLock::new(SystemMetrics::default())),
        })
    }

    /// Create with default configuration
    pub async fn new_with_defaults() -> Result<Self, AgentError> {
        Self::new(AgentManagerConfig::default()).await
    }

    /// Register an agent with the manager
    pub async fn register_agent(&self, mut agent: Box<dyn Agent>) -> Result<(), AgentError> {
        let agent_id = agent.id().to_string();

        info!("Registering agent: {}", agent_id);

        // Check if agent already exists
        {
            let agents = self.agents.read().await;
            if agents.contains_key(&agent_id) {
                return Err(AgentError::ConfigError(format!(
                    "Agent already registered: {}", agent_id
                )));
            }
        }

        // Check agent limit
        {
            let agents = self.agents.read().await;
            if agents.len() >= self.config.max_agents {
                return Err(AgentError::ResourceUnavailable(format!(
                    "Maximum agent limit reached: {}", self.config.max_agents
                )));
            }
        }

        // Initialize the agent
        agent.initialize().await?;

        // Create agent info
        let agent_info = AgentInfo {
            id: agent_id.clone(),
            agent_type: agent.agent_type().to_string(),
            capabilities: agent.capabilities(),
            health_status: agent.health_check(),
            metrics: agent.metrics(),
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Register with message bus
        let (sender, receiver) = mpsc::unbounded_channel();
        self.message_bus.register_agent(agent_id.clone(), sender).await
            .map_err(|e| AgentError::InitializationFailed(format!("Message bus registration failed: {}", e)))?;

        // Create managed agent
        let managed_agent = ManagedAgent {
            agent,
            info: agent_info,
            active_tasks: HashMap::new(),
            message_receiver: Some(receiver),
            last_health_check: Utc::now(),
        };

        // Store the agent
        {
            let mut agents = self.agents.write().await;
            agents.insert(agent_id.clone(), managed_agent);
        }

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.agent_registrations += 1;
        }

        info!("Agent registered successfully: {}", agent_id);
        Ok(())
    }

    /// Unregister an agent
    pub async fn unregister_agent(&self, agent_id: &str) -> Result<(), AgentError> {
        info!("Unregistering agent: {}", agent_id);

        let mut managed_agent = {
            let mut agents = self.agents.write().await;
            agents.remove(agent_id)
                .ok_or_else(|| AgentError::AgentNotFound(agent_id.to_string()))?
        };

        // Shutdown the agent
        if let Err(e) = managed_agent.agent.shutdown().await {
            warn!("Error during agent shutdown: {}", e);
        }

        // Unregister from message bus
        if let Err(e) = self.message_bus.unregister_agent(agent_id).await {
            warn!("Error unregistering from message bus: {}", e);
        }

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.agent_deregistrations += 1;
        }

        info!("Agent unregistered successfully: {}", agent_id);
        Ok(())
    }

    /// Get agent information
    pub async fn get_agent_info(&self, agent_id: &str) -> Option<AgentInfo> {
        let agents = self.agents.read().await;
        agents.get(agent_id).map(|managed_agent| managed_agent.info.clone())
    }

    /// List all registered agents
    pub async fn list_agents(&self) -> Vec<AgentInfo> {
        let agents = self.agents.read().await;
        agents.values().map(|managed_agent| managed_agent.info.clone()).collect()
    }

    /// Dispatch a task to an appropriate agent
    pub async fn dispatch_task(&self, task: AgentTask) -> Result<String, AgentError> {
        let task_id = task.task_id;
        debug!("Dispatching task: {:?}", task_id);

        // Find suitable agent
        let agent_id = self.find_suitable_agent(&task).await?;

        // Create task assignment message
        let message = super::message_bus::AgentMessage::task_assignment(
            agent_id.clone(),
            "agent_manager".to_string(),
            task.clone(),
        );

        // Send message to agent
        self.message_bus.send_message(message).await
            .map_err(|e| AgentError::TaskProcessingFailed(format!("Failed to dispatch task: {}", e)))?;

        // Update task tracking
        {
            let mut agents = self.agents.write().await;
            if let Some(managed_agent) = agents.get_mut(&agent_id) {
                managed_agent.active_tasks.insert(task_id, task);
            }
        }

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_tasks_dispatched += 1;
        }

        debug!("Task dispatched successfully to agent: {}", agent_id);
        Ok(task_id.to_string())
    }

    /// Get system status
    pub async fn get_system_status(&self) -> SystemStatus {
        let agents = self.agents.read().await;
        let metrics = self.metrics.read().await;

        let total_agents = agents.len();
        let healthy_agents = agents.values()
            .filter(|agent| matches!(agent.agent.health_check(), HealthStatus::Healthy))
            .count();
        let error_agents = agents.values()
            .filter(|agent| matches!(agent.agent.health_check(), HealthStatus::Unhealthy(_)))
            .count();

        let average_load = if total_agents > 0 {
            agents.values()
                .map(|agent| agent.agent.load_factor())
                .sum::<f32>() / total_agents as f32
        } else {
            0.0
        };

        SystemStatus {
            total_agents,
            healthy_agents,
            error_agents,
            total_tasks_processed: metrics.total_tasks_completed,
            average_load,
            uptime: Utc::now().signed_duration_since(self.startup_time)
                .to_std().unwrap_or(Duration::ZERO),
            last_updated: Utc::now(),
        }
    }

    /// Shutdown the agent manager and all agents
    pub async fn shutdown(&self) -> Result<(), AgentError> {
        info!("Shutting down Agent Manager");

        let agent_ids: Vec<String> = {
            let agents = self.agents.read().await;
            agents.keys().cloned().collect()
        };

        // Shutdown all agents
        for agent_id in agent_ids {
            if let Err(e) = self.unregister_agent(&agent_id).await {
                warn!("Error shutting down agent {}: {}", agent_id, e);
            }
        }

        info!("Agent Manager shutdown completed");
        Ok(())
    }

    /// Perform health checks on all agents
    pub async fn perform_health_checks(&self) -> Result<(), AgentError> {
        debug!("Performing health checks on all agents");

        let agent_ids: Vec<String> = {
            let agents = self.agents.read().await;
            agents.keys().cloned().collect()
        };

        for agent_id in agent_ids {
            // Send health check request
            let message = super::message_bus::AgentMessage::health_check_request(
                agent_id.clone(),
                "agent_manager".to_string(),
            );

            if let Err(e) = self.message_bus.send_message(message).await {
                warn!("Failed to send health check to agent {}: {}", agent_id, e);

                // Mark agent as unhealthy
                let mut agents = self.agents.write().await;
                if let Some(managed_agent) = agents.get_mut(&agent_id) {
                    managed_agent.info.health_status = HealthStatus::Unhealthy(
                        "Health check communication failed".to_string()
                    );
                }
            }
        }

        Ok(())
    }

    /// Find the most suitable agent for a task
    async fn find_suitable_agent(&self, task: &AgentTask) -> Result<String, AgentError> {
        let agents = self.agents.read().await;

        let mut suitable_agents = Vec::new();

        for (agent_id, managed_agent) in agents.iter() {
            // Check if agent can handle the task
            if managed_agent.agent.can_handle_task(task) {
                // Check if agent is available
                if managed_agent.agent.is_available() {
                    // Check task limit
                    if managed_agent.active_tasks.len() < self.config.max_concurrent_tasks_per_agent {
                        let load = managed_agent.agent.load_factor();
                        suitable_agents.push((agent_id.clone(), load));
                    }
                }
            }
        }

        if suitable_agents.is_empty() {
            return Err(AgentError::ResourceUnavailable(
                "No suitable agent available for task".to_string()
            ));
        }

        // Select agent with lowest load
        suitable_agents.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(suitable_agents[0].0.clone())
    }
}

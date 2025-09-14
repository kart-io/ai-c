//! Message bus for agent communication
//!
//! High-performance message passing system using tokio channels
//! for inter-agent communication and task distribution.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, warn};
use uuid::Uuid;

use super::{AgentResult, AgentTask};
use crate::error::{AppError, AppResult};

/// Message bus for agent communication
///
/// Performance requirements:
/// - Message routing: < 10ms
/// - Broadcast delivery: < 50ms
/// - Channel capacity: 1000 messages per agent
#[derive(Clone)]
pub struct MessageBus {
    /// Agent message channels
    channels: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<AgentMessage>>>>,
    /// Message routing table
    routing_table: Arc<RwLock<RoutingTable>>,
    /// Broadcast subscribers
    broadcast_subscribers: Arc<RwLock<Vec<mpsc::UnboundedSender<BroadcastMessage>>>>,
}

impl MessageBus {
    /// Create a new message bus
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            routing_table: Arc::new(RwLock::new(RoutingTable::new())),
            broadcast_subscribers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register an agent with the message bus
    pub async fn register_agent(
        &self,
        agent_id: String,
        sender: mpsc::UnboundedSender<AgentMessage>,
    ) -> AppResult<()> {
        debug!("Registering agent with message bus: {}", agent_id);

        {
            let mut channels = self.channels.write().await;
            channels.insert(agent_id.clone(), sender);
        }

        {
            let mut routing_table = self.routing_table.write().await;
            routing_table.add_agent(agent_id);
        }

        Ok(())
    }

    /// Unregister an agent from the message bus
    pub async fn unregister_agent(&self, agent_id: &str) -> AppResult<()> {
        debug!("Unregistering agent from message bus: {}", agent_id);

        {
            let mut channels = self.channels.write().await;
            channels.remove(agent_id);
        }

        {
            let mut routing_table = self.routing_table.write().await;
            routing_table.remove_agent(agent_id);
        }

        Ok(())
    }

    /// Send a message to a specific agent
    ///
    /// Performance requirement: < 10ms
    pub async fn send_message(&self, message: AgentMessage) -> AppResult<()> {
        let target_agent = message.target_agent.clone();

        debug!("Sending message to agent: {}", target_agent);

        let sender = {
            let channels = self.channels.read().await;
            channels.get(&target_agent).cloned()
        };

        if let Some(sender) = sender {
            sender.send(message).map_err(|_| {
                AppError::agent(format!("Failed to send message to agent: {}", target_agent))
            })?;

            debug!("Message sent successfully to agent: {}", target_agent);
        } else {
            warn!("Agent not found for message delivery: {}", target_agent);
            return Err(AppError::agent(format!(
                "Agent not found: {}",
                target_agent
            )));
        }

        Ok(())
    }

    /// Broadcast a message to all registered agents
    ///
    /// Performance requirement: < 50ms
    pub async fn broadcast(&self, message: BroadcastMessage) -> AppResult<()> {
        debug!("Broadcasting message: {:?}", message.message_type);

        let subscribers = {
            let broadcast_subscribers = self.broadcast_subscribers.read().await;
            broadcast_subscribers.clone()
        };

        let mut failed_deliveries = 0;

        for subscriber in subscribers {
            if let Err(_) = subscriber.send(message.clone()) {
                failed_deliveries += 1;
            }
        }

        if failed_deliveries > 0 {
            warn!(
                "Failed to deliver broadcast to {} subscribers",
                failed_deliveries
            );
        }

        debug!("Broadcast completed with {} failures", failed_deliveries);
        Ok(())
    }

    /// Subscribe to broadcast messages
    pub async fn subscribe_to_broadcasts(&self) -> mpsc::UnboundedReceiver<BroadcastMessage> {
        let (sender, receiver) = mpsc::unbounded_channel();

        {
            let mut subscribers = self.broadcast_subscribers.write().await;
            subscribers.push(sender);
        }

        receiver
    }

    /// Get routing statistics
    pub async fn routing_stats(&self) -> RoutingStats {
        let routing_table = self.routing_table.read().await;
        routing_table.stats()
    }

    /// Route a task to the best available agent
    pub async fn route_task(&self, task: AgentTask) -> AppResult<String> {
        let routing_table = self.routing_table.read().await;
        routing_table
            .route_task(&task)
            .ok_or_else(|| AppError::agent("No available agent for task"))
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Message types for agent communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    /// Message unique identifier
    pub message_id: Uuid,
    /// Target agent ID
    pub target_agent: String,
    /// Source agent ID
    pub source_agent: String,
    /// Message type and payload
    pub message_type: MessageType,
    /// Message timestamp
    pub timestamp: DateTime<Utc>,
    /// Message priority
    pub priority: MessagePriority,
}

impl AgentMessage {
    /// Create a new task assignment message
    pub fn task_assignment(target_agent: String, source_agent: String, task: AgentTask) -> Self {
        Self {
            message_id: Uuid::new_v4(),
            target_agent,
            source_agent,
            message_type: MessageType::TaskAssignment(task),
            timestamp: Utc::now(),
            priority: MessagePriority::Normal,
        }
    }

    /// Create a task result message
    pub fn task_result(target_agent: String, source_agent: String, result: AgentResult) -> Self {
        Self {
            message_id: Uuid::new_v4(),
            target_agent,
            source_agent,
            message_type: MessageType::TaskResult(result),
            timestamp: Utc::now(),
            priority: MessagePriority::Normal,
        }
    }

    /// Create a health check request
    pub fn health_check_request(target_agent: String, source_agent: String) -> Self {
        Self {
            message_id: Uuid::new_v4(),
            target_agent,
            source_agent,
            message_type: MessageType::HealthCheckRequest,
            timestamp: Utc::now(),
            priority: MessagePriority::High,
        }
    }
}

/// Message types for inter-agent communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    /// Task assignment to an agent
    TaskAssignment(AgentTask),
    /// Task result from an agent
    TaskResult(AgentResult),
    /// Health check request
    HealthCheckRequest,
    /// Health check response
    HealthCheckResponse(super::HealthStatus),
    /// Agent registration
    AgentRegistration {
        agent_id: String,
        capabilities: Vec<super::AgentCapability>,
    },
    /// Agent shutdown notification
    AgentShutdown(String),
    /// Custom message
    Custom(serde_json::Value),
}

/// Message priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MessagePriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Broadcast message for system-wide notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastMessage {
    /// Message unique identifier
    pub message_id: Uuid,
    /// Broadcast message type
    pub message_type: BroadcastMessageType,
    /// Message timestamp
    pub timestamp: DateTime<Utc>,
    /// Source of the broadcast
    pub source: String,
}

impl BroadcastMessage {
    /// Create system shutdown broadcast
    pub fn system_shutdown(source: String) -> Self {
        Self {
            message_id: Uuid::new_v4(),
            message_type: BroadcastMessageType::SystemShutdown,
            timestamp: Utc::now(),
            source,
        }
    }

    /// Create performance warning broadcast
    pub fn performance_warning(source: String, warning: String) -> Self {
        Self {
            message_id: Uuid::new_v4(),
            message_type: BroadcastMessageType::PerformanceWarning(warning),
            timestamp: Utc::now(),
            source,
        }
    }
}

/// Broadcast message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BroadcastMessageType {
    /// System is shutting down
    SystemShutdown,
    /// Configuration changed
    ConfigurationChanged,
    /// Performance warning
    PerformanceWarning(String),
    /// Agent system health update
    HealthUpdate,
    /// Custom broadcast
    Custom(serde_json::Value),
}

/// Message routing table for intelligent task distribution
#[derive(Debug)]
struct RoutingTable {
    /// Agent capabilities mapping
    agent_capabilities: HashMap<String, Vec<super::AgentCapability>>,
    /// Agent load factors
    agent_loads: HashMap<String, f32>,
    /// Agent health status
    agent_health: HashMap<String, super::HealthStatus>,
    /// Routing statistics
    stats: RoutingStats,
}

impl RoutingTable {
    fn new() -> Self {
        Self {
            agent_capabilities: HashMap::new(),
            agent_loads: HashMap::new(),
            agent_health: HashMap::new(),
            stats: RoutingStats::default(),
        }
    }

    fn add_agent(&mut self, agent_id: String) {
        self.agent_capabilities.insert(agent_id.clone(), Vec::new());
        self.agent_loads.insert(agent_id.clone(), 0.0);
        self.agent_health
            .insert(agent_id, super::HealthStatus::Healthy);
    }

    fn remove_agent(&mut self, agent_id: &str) {
        self.agent_capabilities.remove(agent_id);
        self.agent_loads.remove(agent_id);
        self.agent_health.remove(agent_id);
    }

    fn route_task(&self, task: &AgentTask) -> Option<String> {
        // Find agents that can handle this task
        let mut candidates = Vec::new();

        for (agent_id, capabilities) in &self.agent_capabilities {
            let can_handle = capabilities.iter().any(|cap| cap.matches_task(task));

            if can_handle {
                if let Some(health) = self.agent_health.get(agent_id) {
                    if health.is_operational() {
                        let load = self.agent_loads.get(agent_id).copied().unwrap_or(0.0);
                        candidates.push((agent_id.clone(), load));
                    }
                }
            }
        }

        // Select agent with lowest load
        candidates
            .into_iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(agent_id, _)| agent_id)
    }

    fn stats(&self) -> RoutingStats {
        self.stats.clone()
    }
}

/// Routing statistics
#[derive(Debug, Clone, Default)]
pub struct RoutingStats {
    pub total_messages_routed: u64,
    pub successful_routes: u64,
    pub failed_routes: u64,
    pub average_routing_time_ms: f64,
    pub active_agents: usize,
}

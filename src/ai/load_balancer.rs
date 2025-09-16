//! Load Balancer for Agent Task Distribution
//!
//! Implements intelligent load balancing strategies for distributing tasks
//! across available agents based on their capabilities, current load, and performance.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::error::{AppError, AppResult};
use super::{AgentCapability, AgentInfo, AgentMetrics, TaskPriority};

/// Load balancing strategy trait
#[async_trait]
pub trait LoadBalancingStrategy: Send + Sync {
    /// Select the best agent for a given task
    async fn select_agent(
        &self,
        task_requirements: &TaskRequirements,
        available_agents: &[AgentInfo],
        current_loads: &HashMap<String, AgentLoad>,
    ) -> Option<String>;

    /// Get strategy name
    fn name(&self) -> &str;

    /// Get strategy configuration
    fn config(&self) -> serde_json::Value;
}

/// Task requirements for agent selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequirements {
    /// Required capabilities
    pub required_capabilities: Vec<AgentCapability>,
    /// Estimated processing time
    pub estimated_duration: Option<Duration>,
    /// Task priority
    pub priority: TaskPriority,
    /// Resource requirements
    pub resource_requirements: ResourceRequirements,
    /// Preferred agent types
    pub preferred_agent_types: Vec<String>,
}

/// Resource requirements for tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// Minimum memory required (in bytes)
    pub min_memory: u64,
    /// Minimum CPU percentage required
    pub min_cpu: f32,
    /// Network bandwidth required (if applicable)
    pub network_bandwidth: Option<u64>,
    /// Special hardware requirements
    pub special_hardware: Vec<String>,
}

/// Current load information for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLoad {
    /// Number of currently running tasks
    pub running_tasks: usize,
    /// Number of queued tasks
    pub queued_tasks: usize,
    /// Current CPU usage percentage
    pub cpu_usage: f32,
    /// Current memory usage in bytes
    pub memory_usage: u64,
    /// Last response time
    pub last_response_time: Duration,
    /// Load score (0.0 to 1.0, where 1.0 is fully loaded)
    pub load_score: f32,
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            min_memory: 1024 * 1024, // 1MB
            min_cpu: 0.1, // 10%
            network_bandwidth: None,
            special_hardware: Vec::new(),
        }
    }
}

/// Round Robin load balancing strategy
pub struct RoundRobinStrategy {
    /// Last selected agent index
    last_selected: Arc<RwLock<usize>>,
}

impl RoundRobinStrategy {
    pub fn new() -> Self {
        Self {
            last_selected: Arc::new(RwLock::new(0)),
        }
    }
}

#[async_trait]
impl LoadBalancingStrategy for RoundRobinStrategy {
    async fn select_agent(
        &self,
        _task_requirements: &TaskRequirements,
        available_agents: &[AgentInfo],
        _current_loads: &HashMap<String, AgentLoad>,
    ) -> Option<String> {
        if available_agents.is_empty() {
            return None;
        }

        let mut last_selected = self.last_selected.write().await;
        let selected_index = *last_selected % available_agents.len();
        *last_selected += 1;

        Some(available_agents[selected_index].id.clone())
    }

    fn name(&self) -> &str {
        "round_robin"
    }

    fn config(&self) -> serde_json::Value {
        serde_json::json!({
            "strategy": "round_robin",
            "description": "Distributes tasks evenly across all available agents"
        })
    }
}

/// Least Loaded strategy - selects agent with lowest current load
pub struct LeastLoadedStrategy;

#[async_trait]
impl LoadBalancingStrategy for LeastLoadedStrategy {
    async fn select_agent(
        &self,
        task_requirements: &TaskRequirements,
        available_agents: &[AgentInfo],
        current_loads: &HashMap<String, AgentLoad>,
    ) -> Option<String> {
        if available_agents.is_empty() {
            return None;
        }

        let mut best_agent: Option<(&AgentInfo, f32)> = None;

        for agent in available_agents {
            // Check if agent has required capabilities
            if !self.has_required_capabilities(&agent.capabilities, &task_requirements.required_capabilities) {
                continue;
            }

            // Check resource requirements
            if !self.meets_resource_requirements(agent, &task_requirements.resource_requirements) {
                continue;
            }

            // Calculate load score
            let load_score = if let Some(load) = current_loads.get(&agent.id) {
                load.load_score
            } else {
                0.0 // New agent with no load
            };

            match best_agent {
                None => best_agent = Some((agent, load_score)),
                Some((_, best_score)) if load_score < best_score => {
                    best_agent = Some((agent, load_score));
                }
                _ => {}
            }
        }

        best_agent.map(|(agent, _)| agent.id.clone())
    }

    fn name(&self) -> &str {
        "least_loaded"
    }

    fn config(&self) -> serde_json::Value {
        serde_json::json!({
            "strategy": "least_loaded",
            "description": "Selects the agent with the lowest current load"
        })
    }
}

impl LeastLoadedStrategy {
    fn has_required_capabilities(
        &self,
        agent_capabilities: &[AgentCapability],
        required_capabilities: &[AgentCapability],
    ) -> bool {
        required_capabilities.iter().all(|req_cap| {
            agent_capabilities.iter().any(|agent_cap| agent_cap == req_cap)
        })
    }

    fn meets_resource_requirements(
        &self,
        agent: &AgentInfo,
        requirements: &ResourceRequirements,
    ) -> bool {
        // Check memory requirements
        if agent.metrics.memory_usage > 0 && agent.metrics.memory_usage < requirements.min_memory {
            return false;
        }

        // Check CPU requirements
        if agent.metrics.cpu_usage < requirements.min_cpu {
            return false;
        }

        // Additional resource checks can be added here
        true
    }
}

/// Capability-based strategy - prioritizes agents with best capability match
pub struct CapabilityBasedStrategy;

#[async_trait]
impl LoadBalancingStrategy for CapabilityBasedStrategy {
    async fn select_agent(
        &self,
        task_requirements: &TaskRequirements,
        available_agents: &[AgentInfo],
        current_loads: &HashMap<String, AgentLoad>,
    ) -> Option<String> {
        if available_agents.is_empty() {
            return None;
        }

        let mut scored_agents: Vec<(String, f32)> = Vec::new();

        for agent in available_agents {
            let capability_score = self.calculate_capability_score(
                &agent.capabilities,
                &task_requirements.required_capabilities,
                &task_requirements.preferred_agent_types,
                &agent.agent_type,
            );

            if capability_score == 0.0 {
                continue; // Agent doesn't meet requirements
            }

            let load_score = if let Some(load) = current_loads.get(&agent.id) {
                1.0 - load.load_score // Invert so lower load = higher score
            } else {
                1.0 // New agent
            };

            let performance_score = self.calculate_performance_score(&agent.metrics);

            // Weighted combination of scores
            let total_score = capability_score * 0.5 + load_score * 0.3 + performance_score * 0.2;

            scored_agents.push((agent.id.clone(), total_score));
        }

        // Sort by score descending and return the best agent
        scored_agents.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored_agents.first().map(|(id, _)| id.clone())
    }

    fn name(&self) -> &str {
        "capability_based"
    }

    fn config(&self) -> serde_json::Value {
        serde_json::json!({
            "strategy": "capability_based",
            "description": "Prioritizes agents with best capability match and performance",
            "weights": {
                "capability_match": 0.5,
                "load_score": 0.3,
                "performance_score": 0.2
            }
        })
    }
}

impl CapabilityBasedStrategy {
    fn calculate_capability_score(
        &self,
        agent_capabilities: &[AgentCapability],
        required_capabilities: &[AgentCapability],
        preferred_agent_types: &[String],
        agent_type: &str,
    ) -> f32 {
        // Check if all required capabilities are met
        let required_met = required_capabilities.iter().all(|req_cap| {
            agent_capabilities.iter().any(|agent_cap| agent_cap == req_cap)
        });

        if !required_met {
            return 0.0; // Doesn't meet requirements
        }

        let mut score = 1.0; // Base score for meeting requirements

        // Bonus for preferred agent type
        if preferred_agent_types.contains(&agent_type.to_string()) {
            score += 0.5;
        }

        // Bonus for additional capabilities
        let extra_capabilities = agent_capabilities.len() as f32 - required_capabilities.len() as f32;
        if extra_capabilities > 0.0 {
            score += extra_capabilities * 0.1; // Small bonus for versatility
        }

        score.min(2.0) // Cap at 2.0
    }

    fn calculate_performance_score(&self, metrics: &AgentMetrics) -> f32 {
        let mut score = 1.0;

        // Better score for lower error rate
        score *= 1.0 - metrics.error_rate as f32;

        // Better score for faster response time (normalized)
        let response_time_seconds = metrics.average_response_time.as_secs_f32();
        if response_time_seconds > 0.0 {
            score *= (1.0 / (1.0 + response_time_seconds)).min(1.0);
        }

        // Better score for more tasks processed (experience factor)
        if metrics.tasks_processed > 0 {
            score *= (1.0 + (metrics.tasks_processed as f32).ln() * 0.1).min(2.0);
        }

        score.max(0.0).min(1.0)
    }
}

/// Load balancer that manages task distribution across agents
pub struct LoadBalancer {
    /// Current strategy being used
    strategy: Arc<RwLock<Box<dyn LoadBalancingStrategy>>>,
    /// Agent load tracking
    agent_loads: Arc<RwLock<HashMap<String, AgentLoad>>>,
    /// Load balancer statistics
    stats: Arc<RwLock<LoadBalancerStats>>,
}

/// Load balancer statistics
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LoadBalancerStats {
    /// Total tasks distributed
    pub total_tasks_distributed: u64,
    /// Tasks per agent
    pub tasks_per_agent: HashMap<String, u64>,
    /// Average task distribution time
    pub average_distribution_time: Duration,
    /// Strategy change count
    pub strategy_changes: u64,
    /// Load balancer uptime
    pub uptime_start: Option<Instant>,
}

impl LoadBalancer {
    /// Create a new load balancer with default strategy
    pub fn new() -> Self {
        Self::with_strategy(Box::new(LeastLoadedStrategy))
    }

    /// Create a new load balancer with specified strategy
    pub fn with_strategy(strategy: Box<dyn LoadBalancingStrategy>) -> Self {
        Self {
            strategy: Arc::new(RwLock::new(strategy)),
            agent_loads: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(LoadBalancerStats {
                uptime_start: Some(Instant::now()),
                ..Default::default()
            })),
        }
    }

    /// Select the best agent for a task
    pub async fn select_agent(
        &self,
        task_requirements: TaskRequirements,
        available_agents: Vec<AgentInfo>,
    ) -> AppResult<Option<String>> {
        let start_time = Instant::now();

        let strategy = self.strategy.read().await;
        let agent_loads = self.agent_loads.read().await;

        debug!(
            "Selecting agent using {} strategy for task with {} required capabilities",
            strategy.name(),
            task_requirements.required_capabilities.len()
        );

        let selected_agent = strategy.select_agent(
            &task_requirements,
            &available_agents,
            &agent_loads,
        ).await;

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.total_tasks_distributed += 1;

        if let Some(ref agent_id) = selected_agent {
            *stats.tasks_per_agent.entry(agent_id.clone()).or_insert(0) += 1;
        }

        // Update average distribution time
        let distribution_time = start_time.elapsed();
        let total_time = stats.average_distribution_time.as_nanos() as u64 * (stats.total_tasks_distributed - 1) + distribution_time.as_nanos() as u64;
        stats.average_distribution_time = Duration::from_nanos(total_time / stats.total_tasks_distributed);

        if let Some(ref agent_id) = selected_agent {
            info!("Selected agent {} for task in {:?}", agent_id, distribution_time);
        } else {
            warn!("No suitable agent found for task requirements");
        }

        Ok(selected_agent)
    }

    /// Update agent load information
    pub async fn update_agent_load(&self, agent_id: String, load: AgentLoad) {
        let mut agent_loads = self.agent_loads.write().await;
        agent_loads.insert(agent_id, load);
    }

    /// Remove agent from load tracking
    pub async fn remove_agent(&self, agent_id: &str) {
        let mut agent_loads = self.agent_loads.write().await;
        agent_loads.remove(agent_id);
    }

    /// Change load balancing strategy
    pub async fn set_strategy(&self, strategy: Box<dyn LoadBalancingStrategy>) -> AppResult<()> {
        let mut current_strategy = self.strategy.write().await;
        let old_strategy_name = current_strategy.name().to_string();
        *current_strategy = strategy;

        let mut stats = self.stats.write().await;
        stats.strategy_changes += 1;

        info!(
            "Load balancing strategy changed from {} to {}",
            old_strategy_name,
            current_strategy.name()
        );

        Ok(())
    }

    /// Get current strategy name
    pub async fn current_strategy(&self) -> String {
        let strategy = self.strategy.read().await;
        strategy.name().to_string()
    }

    /// Get load balancer statistics
    pub async fn get_stats(&self) -> LoadBalancerStats {
        self.stats.read().await.clone()
    }

    /// Get current agent loads
    pub async fn get_agent_loads(&self) -> HashMap<String, AgentLoad> {
        self.agent_loads.read().await.clone()
    }

    /// Calculate overall system load
    pub async fn calculate_system_load(&self) -> f32 {
        let agent_loads = self.agent_loads.read().await;

        if agent_loads.is_empty() {
            return 0.0;
        }

        let total_load: f32 = agent_loads.values().map(|load| load.load_score).sum();
        total_load / agent_loads.len() as f32
    }
}

impl Default for LoadBalancer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{AgentCapability, AgentMetrics};
    use chrono::Utc;

    fn create_test_agent(id: &str, capabilities: Vec<AgentCapability>) -> AgentInfo {
        AgentInfo {
            id: id.to_string(),
            agent_type: "test_agent".to_string(),
            capabilities,
            health_status: crate::ai::HealthStatus::Healthy,
            metrics: AgentMetrics::default(),
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_round_robin_strategy() {
        let strategy = RoundRobinStrategy::new();
        let agents = vec![
            create_test_agent("agent1", vec![]),
            create_test_agent("agent2", vec![]),
            create_test_agent("agent3", vec![]),
        ];

        let task_req = TaskRequirements {
            required_capabilities: vec![],
            estimated_duration: None,
            priority: TaskPriority::Normal,
            resource_requirements: ResourceRequirements::default(),
            preferred_agent_types: vec![],
        };

        let loads = HashMap::new();

        // First selection should be agent1
        let selected = strategy.select_agent(&task_req, &agents, &loads).await;
        assert_eq!(selected, Some("agent1".to_string()));

        // Second selection should be agent2
        let selected = strategy.select_agent(&task_req, &agents, &loads).await;
        assert_eq!(selected, Some("agent2".to_string()));

        // Third selection should be agent3
        let selected = strategy.select_agent(&task_req, &agents, &loads).await;
        assert_eq!(selected, Some("agent3".to_string()));

        // Fourth selection should wrap around to agent1
        let selected = strategy.select_agent(&task_req, &agents, &loads).await;
        assert_eq!(selected, Some("agent1".to_string()));
    }

    #[tokio::test]
    async fn test_least_loaded_strategy() {
        let strategy = LeastLoadedStrategy;
        let agents = vec![
            create_test_agent("agent1", vec![]),
            create_test_agent("agent2", vec![]),
        ];

        let mut loads = HashMap::new();
        loads.insert("agent1".to_string(), AgentLoad {
            running_tasks: 5,
            queued_tasks: 2,
            cpu_usage: 80.0,
            memory_usage: 1024 * 1024 * 100, // 100MB
            last_response_time: Duration::from_millis(500),
            load_score: 0.8,
        });
        loads.insert("agent2".to_string(), AgentLoad {
            running_tasks: 1,
            queued_tasks: 0,
            cpu_usage: 20.0,
            memory_usage: 1024 * 1024 * 30, // 30MB
            last_response_time: Duration::from_millis(100),
            load_score: 0.2,
        });

        let task_req = TaskRequirements {
            required_capabilities: vec![],
            estimated_duration: None,
            priority: TaskPriority::Normal,
            resource_requirements: ResourceRequirements::default(),
            preferred_agent_types: vec![],
        };

        // Should select agent2 as it has lower load
        let selected = strategy.select_agent(&task_req, &agents, &loads).await;
        assert_eq!(selected, Some("agent2".to_string()));
    }

    #[tokio::test]
    async fn test_load_balancer() {
        let load_balancer = LoadBalancer::new();

        let agents = vec![
            create_test_agent("agent1", vec![]),
            create_test_agent("agent2", vec![]),
        ];

        let task_req = TaskRequirements {
            required_capabilities: vec![],
            estimated_duration: Some(Duration::from_secs(10)),
            priority: TaskPriority::Normal,
            resource_requirements: ResourceRequirements::default(),
            preferred_agent_types: vec![],
        };

        let selected = load_balancer.select_agent(task_req, agents).await.unwrap();
        assert!(selected.is_some());

        // Check that statistics were updated
        let stats = load_balancer.get_stats().await;
        assert_eq!(stats.total_tasks_distributed, 1);
    }
}
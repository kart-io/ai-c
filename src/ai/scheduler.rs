//! Task scheduler for agent system
//!
//! Provides task queuing, prioritization, and agent selection
//! with pluggable scheduling strategies.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BinaryHeap, HashMap},
    sync::Arc,
    time::SystemTime,
};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, warn};

use super::{AgentError, AgentResult, AgentTask, HealthStatus, TaskPriority};

/// Task scheduler configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSchedulerConfig {
    /// Maximum number of tasks in queue
    pub max_queue_size: usize,
    /// Maximum running tasks per agent
    pub max_tasks_per_agent: usize,
    /// Task timeout in seconds
    pub default_task_timeout: u64,
    /// Queue cleanup interval in seconds
    pub cleanup_interval: u64,
}

impl Default for TaskSchedulerConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 1000,
            max_tasks_per_agent: 5,
            default_task_timeout: 300,
            cleanup_interval: 60,
        }
    }
}

/// Task status enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task is queued and waiting for execution
    Queued,
    /// Task has been assigned to an agent
    Assigned(String),
    /// Task is currently running on an agent
    Running(String),
    /// Task completed successfully
    Completed(AgentResult),
    /// Task failed with error
    Failed(AgentError),
    /// Task was cancelled
    Cancelled,
    /// Task timed out
    TimedOut,
}

/// Running task information
#[derive(Debug, Clone)]
pub struct RunningTask {
    /// The task being executed
    pub task: AgentTask,
    /// Agent executing the task
    pub agent_id: String,
    /// Task start time
    pub started_at: SystemTime,
    /// Current task status
    pub status: TaskStatus,
    /// Expected completion time
    pub expected_completion: Option<SystemTime>,
}

/// Agent availability information
#[derive(Debug, Clone)]
pub struct AgentAvailability {
    /// Agent unique identifier
    pub agent_id: String,
    /// Current number of running tasks
    pub current_load: usize,
    /// Maximum task capacity
    pub max_capacity: usize,
    /// Last activity timestamp
    pub last_activity: SystemTime,
    /// Agent health status
    pub health_status: HealthStatus,
    /// Agent capabilities
    pub capabilities: Vec<super::AgentCapability>,
    /// Current load factor (0.0 to 1.0)
    pub load_factor: f32,
}

/// Task wrapper for priority queue
#[derive(Debug, Clone)]
struct PriorityTask {
    task: AgentTask,
    submitted_at: SystemTime,
}

impl PartialEq for PriorityTask {
    fn eq(&self, other: &Self) -> bool {
        self.task.priority == other.task.priority
    }
}

impl Eq for PriorityTask {}

impl PartialOrd for PriorityTask {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriorityTask {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher priority first, then earlier submission time
        other.task.priority.cmp(&self.task.priority)
            .then_with(|| self.submitted_at.cmp(&other.submitted_at))
    }
}

/// Scheduling strategy trait
#[async_trait]
pub trait SchedulingStrategy: Send + Sync {
    /// Select the best agent for a given task
    async fn select_agent(
        &self,
        task: &AgentTask,
        available_agents: &[AgentAvailability],
    ) -> Option<String>;

    /// Get strategy name
    fn name(&self) -> &str;
}

/// Load balancing scheduling strategy
#[derive(Debug, Default)]
pub struct LoadBalancingStrategy;

#[async_trait]
impl SchedulingStrategy for LoadBalancingStrategy {
    async fn select_agent(
        &self,
        task: &AgentTask,
        available_agents: &[AgentAvailability],
    ) -> Option<String> {
        // Filter agents that can handle the task and are healthy
        let suitable_agents: Vec<_> = available_agents
            .iter()
            .filter(|agent| {
                matches!(agent.health_status, HealthStatus::Healthy | HealthStatus::Degraded(_))
                    && agent.current_load < agent.max_capacity
                    && agent.capabilities.iter().any(|cap| cap.matches_task(task))
            })
            .collect();

        if suitable_agents.is_empty() {
            return None;
        }

        // Select agent with lowest load factor
        suitable_agents
            .into_iter()
            .min_by(|a, b| a.load_factor.partial_cmp(&b.load_factor).unwrap_or(std::cmp::Ordering::Equal))
            .map(|agent| agent.agent_id.clone())
    }

    fn name(&self) -> &str {
        "LoadBalancing"
    }
}

/// Priority-based scheduling strategy
#[derive(Debug, Default)]
pub struct PriorityBasedStrategy;

#[async_trait]
impl SchedulingStrategy for PriorityBasedStrategy {
    async fn select_agent(
        &self,
        task: &AgentTask,
        available_agents: &[AgentAvailability],
    ) -> Option<String> {
        // For high priority tasks, prefer agents with low load
        // For low priority tasks, prefer agents with higher load but still available

        let suitable_agents: Vec<_> = available_agents
            .iter()
            .filter(|agent| {
                matches!(agent.health_status, HealthStatus::Healthy | HealthStatus::Degraded(_))
                    && agent.current_load < agent.max_capacity
                    && agent.capabilities.iter().any(|cap| cap.matches_task(task))
            })
            .collect();

        if suitable_agents.is_empty() {
            return None;
        }

        match task.priority {
            TaskPriority::Critical | TaskPriority::High => {
                // Use least loaded agent for high priority tasks
                suitable_agents
                    .into_iter()
                    .min_by(|a, b| a.load_factor.partial_cmp(&b.load_factor).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|agent| agent.agent_id.clone())
            }
            TaskPriority::Normal | TaskPriority::Low => {
                // Use any available agent for normal/low priority tasks
                suitable_agents
                    .into_iter()
                    .min_by(|a, b| a.current_load.cmp(&b.current_load))
                    .map(|agent| agent.agent_id.clone())
            }
        }
    }

    fn name(&self) -> &str {
        "PriorityBased"
    }
}

/// Capability-matching scheduling strategy
#[derive(Debug, Default)]
pub struct CapabilityMatchStrategy;

#[async_trait]
impl SchedulingStrategy for CapabilityMatchStrategy {
    async fn select_agent(
        &self,
        task: &AgentTask,
        available_agents: &[AgentAvailability],
    ) -> Option<String> {
        // Find agents with exact capability match first
        let exact_match: Vec<_> = available_agents
            .iter()
            .filter(|agent| {
                matches!(agent.health_status, HealthStatus::Healthy)
                    && agent.current_load < agent.max_capacity
                    && agent.capabilities.iter().any(|cap| cap.matches_task(task))
            })
            .collect();

        if !exact_match.is_empty() {
            return exact_match
                .into_iter()
                .min_by(|a, b| a.load_factor.partial_cmp(&b.load_factor).unwrap_or(std::cmp::Ordering::Equal))
                .map(|agent| agent.agent_id.clone());
        }

        // Fallback to any compatible agent
        available_agents
            .iter()
            .filter(|agent| {
                matches!(agent.health_status, HealthStatus::Degraded(_))
                    && agent.current_load < agent.max_capacity
                    && agent.capabilities.iter().any(|cap| cap.matches_task(task))
            })
            .min_by(|a, b| a.load_factor.partial_cmp(&b.load_factor).unwrap_or(std::cmp::Ordering::Equal))
            .map(|agent| agent.agent_id.clone())
    }

    fn name(&self) -> &str {
        "CapabilityMatch"
    }
}

/// Task scheduler implementation
pub struct TaskScheduler {
    /// Priority queue for pending tasks
    task_queue: Arc<Mutex<BinaryHeap<PriorityTask>>>,
    /// Currently running tasks
    running_tasks: Arc<RwLock<HashMap<String, RunningTask>>>,
    /// Agent availability information
    agent_availability: Arc<RwLock<HashMap<String, AgentAvailability>>>,
    /// Scheduling strategy
    scheduling_strategy: Box<dyn SchedulingStrategy>,
    /// Configuration
    config: TaskSchedulerConfig,
    /// Task status tracking
    task_status: Arc<RwLock<HashMap<String, TaskStatus>>>,
}

impl TaskScheduler {
    /// Create a new task scheduler with the specified strategy
    pub fn new(strategy: Box<dyn SchedulingStrategy>) -> Self {
        Self::new_with_config(strategy, TaskSchedulerConfig::default())
    }

    /// Create a new task scheduler with custom configuration
    pub fn new_with_config(
        strategy: Box<dyn SchedulingStrategy>,
        config: TaskSchedulerConfig,
    ) -> Self {
        info!("Creating TaskScheduler with strategy: {}", strategy.name());

        Self {
            task_queue: Arc::new(Mutex::new(BinaryHeap::new())),
            running_tasks: Arc::new(RwLock::new(HashMap::new())),
            agent_availability: Arc::new(RwLock::new(HashMap::new())),
            scheduling_strategy: strategy,
            config,
            task_status: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Submit a task for execution
    pub async fn submit_task(&self, task: AgentTask) -> Result<String, AgentError> {
        let task_id = task.task_id.to_string();

        debug!("Submitting task: {} with priority: {:?}", task_id, task.priority);

        // Check queue capacity
        {
            let queue = self.task_queue.lock().await;
            if queue.len() >= self.config.max_queue_size {
                return Err(AgentError::ResourceUnavailable(
                    "Task queue at maximum capacity".to_string()
                ));
            }
        }

        // Add to task status tracking
        {
            let mut status = self.task_status.write().await;
            status.insert(task_id.clone(), TaskStatus::Queued);
        }

        // Add to priority queue
        {
            let mut queue = self.task_queue.lock().await;
            queue.push(PriorityTask {
                task,
                submitted_at: SystemTime::now(),
            });
        }

        info!("Task submitted successfully: {}", task_id);
        Ok(task_id)
    }

    /// Select the best agent for a task
    pub async fn select_agent(&self, task: &AgentTask) -> Result<String, AgentError> {
        let agents = {
            let availability = self.agent_availability.read().await;
            availability.values().cloned().collect::<Vec<_>>()
        };

        if agents.is_empty() {
            return Err(AgentError::ResourceUnavailable("No agents available".to_string()));
        }

        self.scheduling_strategy
            .select_agent(task, &agents)
            .await
            .ok_or_else(|| AgentError::ResourceUnavailable("No suitable agent found".to_string()))
    }

    /// Execute the next available task
    pub async fn execute_next_task(&self) -> Result<Option<String>, AgentError> {
        // Get next task from queue
        let priority_task = {
            let mut queue = self.task_queue.lock().await;
            queue.pop()
        };

        let priority_task = match priority_task {
            Some(task) => task,
            None => return Ok(None), // No tasks in queue
        };

        let task = priority_task.task;
        let task_id = task.task_id.to_string();

        // Select agent for the task
        let agent_id = self.select_agent(&task).await?;

        // Update task status to assigned
        {
            let mut status = self.task_status.write().await;
            status.insert(task_id.clone(), TaskStatus::Assigned(agent_id.clone()));
        }

        // Create running task entry
        let running_task = RunningTask {
            task: task.clone(),
            agent_id: agent_id.clone(),
            started_at: SystemTime::now(),
            status: TaskStatus::Running(agent_id.clone()),
            expected_completion: Some(
                SystemTime::now() + std::time::Duration::from_secs(self.config.default_task_timeout)
            ),
        };

        // Add to running tasks
        {
            let mut running = self.running_tasks.write().await;
            running.insert(task_id.clone(), running_task);
        }

        // Update agent load
        self.update_agent_load(&agent_id, 1).await?;

        // Update task status to running
        {
            let mut status = self.task_status.write().await;
            status.insert(task_id.clone(), TaskStatus::Running(agent_id.clone()));
        }

        info!("Task {} assigned to agent {}", task_id, agent_id);
        Ok(Some(task_id))
    }

    /// Mark a task as completed
    pub async fn complete_task(&self, task_id: &str, result: AgentResult) -> Result<(), AgentError> {
        debug!("Completing task: {}", task_id);

        // Get the running task
        let running_task = {
            let mut running = self.running_tasks.write().await;
            running.remove(task_id)
        };

        if let Some(running_task) = running_task {
            // Update agent load
            self.update_agent_load(&running_task.agent_id, -1).await?;

            // Update task status
            {
                let mut status = self.task_status.write().await;
                status.insert(task_id.to_string(), TaskStatus::Completed(result));
            }

            info!("Task {} completed successfully", task_id);
        } else {
            warn!("Attempted to complete unknown task: {}", task_id);
        }

        Ok(())
    }

    /// Mark a task as failed
    pub async fn fail_task(&self, task_id: &str, error: AgentError) -> Result<(), AgentError> {
        debug!("Marking task as failed: {}", task_id);

        // Get the running task
        let running_task = {
            let mut running = self.running_tasks.write().await;
            running.remove(task_id)
        };

        if let Some(running_task) = running_task {
            // Update agent load
            self.update_agent_load(&running_task.agent_id, -1).await?;

            // Update task status
            {
                let mut status = self.task_status.write().await;
                status.insert(task_id.to_string(), TaskStatus::Failed(error));
            }

            info!("Task {} marked as failed", task_id);
        } else {
            warn!("Attempted to fail unknown task: {}", task_id);
        }

        Ok(())
    }

    /// Cancel a task
    pub async fn cancel_task(&self, task_id: &str) -> Result<(), AgentError> {
        debug!("Cancelling task: {}", task_id);

        // Check if task is running
        let running_task = {
            let mut running = self.running_tasks.write().await;
            running.remove(task_id)
        };

        if let Some(running_task) = running_task {
            // Update agent load
            self.update_agent_load(&running_task.agent_id, -1).await?;
        }

        // Update task status
        {
            let mut status = self.task_status.write().await;
            status.insert(task_id.to_string(), TaskStatus::Cancelled);
        }

        info!("Task {} cancelled", task_id);
        Ok(())
    }

    /// Get task status
    pub async fn get_task_status(&self, task_id: &str) -> Option<TaskStatus> {
        let status = self.task_status.read().await;
        status.get(task_id).cloned()
    }

    /// Update agent availability
    pub async fn update_agent_availability(&self, availability: AgentAvailability) -> Result<(), AgentError> {
        let mut agents = self.agent_availability.write().await;
        agents.insert(availability.agent_id.clone(), availability);
        Ok(())
    }

    /// Remove agent availability
    pub async fn remove_agent(&self, agent_id: &str) -> Result<(), AgentError> {
        let mut agents = self.agent_availability.write().await;
        agents.remove(agent_id);
        Ok(())
    }

    /// Get queue size
    pub async fn queue_size(&self) -> usize {
        let queue = self.task_queue.lock().await;
        queue.len()
    }

    /// Get running tasks count
    pub async fn running_tasks_count(&self) -> usize {
        let running = self.running_tasks.read().await;
        running.len()
    }

    /// Get scheduler statistics
    pub async fn get_statistics(&self) -> SchedulerStatistics {
        let queue_size = self.queue_size().await;
        let running_count = self.running_tasks_count().await;
        let agent_count = {
            let agents = self.agent_availability.read().await;
            agents.len()
        };

        SchedulerStatistics {
            queued_tasks: queue_size,
            running_tasks: running_count,
            available_agents: agent_count,
            strategy_name: self.scheduling_strategy.name().to_string(),
        }
    }

    /// Update agent load (internal helper)
    async fn update_agent_load(&self, agent_id: &str, delta: i32) -> Result<(), AgentError> {
        let mut agents = self.agent_availability.write().await;
        if let Some(agent) = agents.get_mut(agent_id) {
            if delta > 0 {
                agent.current_load += delta as usize;
            } else if delta < 0 && agent.current_load > 0 {
                agent.current_load -= (-delta) as usize;
            }

            // Update load factor
            agent.load_factor = if agent.max_capacity > 0 {
                agent.current_load as f32 / agent.max_capacity as f32
            } else {
                0.0
            };

            agent.last_activity = SystemTime::now();
        }
        Ok(())
    }
}

impl Default for TaskScheduler {
    fn default() -> Self {
        Self::new(Box::new(LoadBalancingStrategy::default()))
    }
}

/// Scheduler statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerStatistics {
    pub queued_tasks: usize,
    pub running_tasks: usize,
    pub available_agents: usize,
    pub strategy_name: String,
}

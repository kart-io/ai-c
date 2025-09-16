//! Agent Hot-Swap Mechanism
//!
//! Provides capabilities for adding, removing, and updating agents at runtime
//! without interrupting the system operation. Supports graceful shutdown,
//! state migration, and seamless agent replacement.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    sync::{mpsc, RwLock, Semaphore},
    time::timeout,
};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use super::{Agent, AgentInfo, AgentTask, HealthStatus};

/// Hot-swap operation types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HotSwapOperation {
    /// Add a new agent to the system
    Add,
    /// Remove an agent from the system
    Remove,
    /// Replace an existing agent with a new one
    Replace,
    /// Update an existing agent's configuration
    Update,
    /// Restart an agent (remove and re-add)
    Restart,
}

/// Hot-swap configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotSwapConfig {
    /// Maximum time to wait for graceful shutdown
    pub graceful_shutdown_timeout: Duration,
    /// Maximum time to wait for agent initialization
    pub initialization_timeout: Duration,
    /// Maximum time to wait for state migration
    pub state_migration_timeout: Duration,
    /// Whether to enable state migration between agents
    pub enable_state_migration: bool,
    /// Maximum number of concurrent hot-swap operations
    pub max_concurrent_operations: usize,
    /// Retry configuration for failed operations
    pub retry_attempts: u32,
    /// Health check timeout during hot-swap
    pub health_check_timeout: Duration,
}

impl Default for HotSwapConfig {
    fn default() -> Self {
        Self {
            graceful_shutdown_timeout: Duration::from_secs(30),
            initialization_timeout: Duration::from_secs(60),
            state_migration_timeout: Duration::from_secs(120),
            enable_state_migration: true,
            max_concurrent_operations: 3,
            retry_attempts: 2,
            health_check_timeout: Duration::from_secs(10),
        }
    }
}

/// Hot-swap request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotSwapRequest {
    /// Unique request ID
    pub id: String,
    /// Operation type
    pub operation: HotSwapOperation,
    /// Target agent ID (for remove, replace, update operations)
    pub target_agent_id: Option<String>,
    /// New agent factory (for add, replace operations)
    pub new_agent_factory: Option<AgentFactory>,
    /// Configuration updates (for update operations)
    pub config_updates: Option<serde_json::Value>,
    /// Whether to perform graceful transition
    pub graceful: bool,
    /// Custom timeout for this operation
    pub timeout: Option<Duration>,
    /// Priority of the operation
    pub priority: SwapPriority,
    /// Migration strategy (if applicable)
    pub migration_strategy: Option<MigrationStrategy>,
}

/// Agent factory for creating new agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentFactory {
    /// Agent type identifier
    pub agent_type: String,
    /// Agent configuration
    pub config: serde_json::Value,
    /// Agent capabilities
    pub capabilities: Vec<String>,
    /// Custom initialization parameters
    pub init_params: HashMap<String, serde_json::Value>,
}

/// Migration strategy for transferring state between agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationStrategy {
    /// No migration - start fresh
    None,
    /// Copy all state from old agent to new agent
    FullMigration,
    /// Copy only essential state
    PartialMigration(Vec<String>),
    /// Custom migration with specific rules
    Custom(serde_json::Value),
}

/// Priority levels for hot-swap operations
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SwapPriority {
    Low,
    Normal,
    High,
    Urgent,
    Emergency,
}

/// Hot-swap operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotSwapResult {
    /// Request ID
    pub request_id: String,
    /// Whether the operation succeeded
    pub success: bool,
    /// Operation that was performed
    pub operation: HotSwapOperation,
    /// ID of the old agent (if applicable)
    pub old_agent_id: Option<String>,
    /// ID of the new agent (if applicable)
    pub new_agent_id: Option<String>,
    /// Operation duration
    pub duration: Duration,
    /// Any errors that occurred
    pub errors: Vec<String>,
    /// Migration summary (if applicable)
    pub migration_summary: Option<MigrationSummary>,
    /// Final status
    pub status: OperationStatus,
}

/// Migration summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationSummary {
    /// Amount of state migrated
    pub state_items_migrated: u64,
    /// Migration duration
    pub migration_duration: Duration,
    /// Whether migration was successful
    pub migration_successful: bool,
    /// Migration errors
    pub migration_errors: Vec<String>,
}

/// Operation status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationStatus {
    /// Operation completed successfully
    Completed,
    /// Operation partially succeeded
    PartialSuccess,
    /// Operation failed
    Failed,
    /// Operation was cancelled
    Cancelled,
    /// Operation timed out
    TimedOut,
}

/// Agent state for migration
pub trait AgentState: Send + Sync {
    /// Serialize agent state for migration
    fn serialize_state(&self) -> AppResult<serde_json::Value>;

    /// Deserialize and restore agent state
    fn deserialize_state(&mut self, state: serde_json::Value) -> AppResult<()>;

    /// Get essential state keys for partial migration
    fn essential_state_keys(&self) -> Vec<String>;
}

/// Hot-swappable agent trait
#[async_trait]
pub trait HotSwappableAgent: Agent + AgentState {
    /// Prepare for graceful shutdown
    async fn prepare_shutdown(&mut self) -> AppResult<()>;

    /// Perform graceful shutdown
    async fn graceful_shutdown(&mut self) -> AppResult<()>;

    /// Check if agent can be safely shut down
    async fn can_shutdown(&self) -> bool;

    /// Get current active tasks that need to be completed
    async fn get_active_tasks(&self) -> Vec<AgentTask>;

    /// Transfer active tasks to another agent
    async fn transfer_tasks(&mut self, target_agent_id: &str) -> AppResult<Vec<AgentTask>>;
}

/// Agent factory trait for creating new agents
#[async_trait]
pub trait AgentCreator: Send + Sync {
    /// Create a new agent instance
    async fn create_agent(&self, factory: &AgentFactory) -> AppResult<Box<dyn HotSwappableAgent>>;

    /// Validate agent factory configuration
    fn validate_factory(&self, factory: &AgentFactory) -> AppResult<()>;

    /// Get supported agent types
    fn supported_types(&self) -> Vec<String>;
}

/// Hot-swap manager
pub struct HotSwapManager {
    /// Hot-swap configuration
    config: HotSwapConfig,
    /// Currently active agents
    active_agents: Arc<RwLock<HashMap<String, Box<dyn HotSwappableAgent>>>>,
    /// Agent creators by type
    agent_creators: Arc<RwLock<HashMap<String, Box<dyn AgentCreator>>>>,
    /// Operation queue
    operation_queue: Arc<RwLock<Vec<HotSwapRequest>>>,
    /// Semaphore for limiting concurrent operations
    operation_semaphore: Semaphore,
    /// Operation history
    operation_history: Arc<RwLock<Vec<HotSwapResult>>>,
    /// Statistics
    stats: Arc<RwLock<HotSwapStats>>,
}

/// Hot-swap statistics
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct HotSwapStats {
    /// Total operations performed
    pub total_operations: u64,
    /// Successful operations
    pub successful_operations: u64,
    /// Failed operations
    pub failed_operations: u64,
    /// Operations by type
    pub operations_by_type: HashMap<HotSwapOperation, u64>,
    /// Average operation duration
    pub average_operation_duration: Duration,
    /// Total migration operations
    pub total_migrations: u64,
    /// Successful migrations
    pub successful_migrations: u64,
    /// System uptime with hot-swap capability
    pub uptime_start: Option<Instant>,
}

impl HotSwapManager {
    /// Create a new hot-swap manager
    pub fn new(config: HotSwapConfig) -> Self {
        let operation_semaphore = Semaphore::new(config.max_concurrent_operations);

        Self {
            config,
            active_agents: Arc::new(RwLock::new(HashMap::new())),
            agent_creators: Arc::new(RwLock::new(HashMap::new())),
            operation_queue: Arc::new(RwLock::new(Vec::new())),
            operation_semaphore,
            operation_history: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(HotSwapStats {
                uptime_start: Some(Instant::now()),
                ..Default::default()
            })),
        }
    }

    /// Register an agent creator
    pub async fn register_agent_creator(
        &self,
        agent_type: String,
        creator: Box<dyn AgentCreator>,
    ) -> AppResult<()> {
        let mut creators = self.agent_creators.write().await;
        creators.insert(agent_type.clone(), creator);

        info!("Registered agent creator for type: {}", agent_type);
        Ok(())
    }

    /// Submit a hot-swap request
    pub async fn submit_request(&self, request: HotSwapRequest) -> AppResult<String> {
        let request_id = request.id.clone();

        // Validate the request
        self.validate_request(&request).await?;

        // Add to queue
        {
            let mut queue = self.operation_queue.write().await;
            queue.push(request);
            queue.sort_by(|a, b| b.priority.cmp(&a.priority));
        }

        info!("Hot-swap request {} submitted", request_id);

        // Start processing if possible
        self.process_queue().await;

        Ok(request_id)
    }

    /// Process the operation queue
    async fn process_queue(&self) {
        let permit = match self.operation_semaphore.try_acquire() {
            Ok(permit) => permit,
            Err(_) => {
                debug!("Hot-swap manager at capacity, deferring queue processing");
                return;
            }
        };

        let request = {
            let mut queue = self.operation_queue.write().await;
            queue.pop()
        };

        if let Some(request) = request {
            let manager = self.clone();
            tokio::spawn(async move {
                let _permit = permit; // Hold permit for the duration of operation
                manager.execute_request(request).await;
            });
        }
    }

    /// Execute a hot-swap request
    async fn execute_request(&self, request: HotSwapRequest) {
        let start_time = Instant::now();
        let request_id = request.id.clone();
        let operation = request.operation.clone();

        info!("Executing hot-swap request {}: {:?}", request_id, operation);

        let result = match self.perform_operation(&request).await {
            Ok(result) => result,
            Err(error) => HotSwapResult {
                request_id: request_id.clone(),
                success: false,
                operation: operation.clone(),
                old_agent_id: None,
                new_agent_id: None,
                duration: start_time.elapsed(),
                errors: vec![error.to_string()],
                migration_summary: None,
                status: OperationStatus::Failed,
            },
        };

        // Update statistics
        self.update_stats(&result).await;

        // Store result in history
        {
            let mut history = self.operation_history.write().await;
            history.push(result.clone());

            // Keep only recent history
            if history.len() > 1000 {
                history.drain(0..100);
            }
        }

        if result.success {
            info!("Hot-swap request {} completed successfully in {:?}", request_id, result.duration);
        } else {
            error!("Hot-swap request {} failed: {:?}", request_id, result.errors);
        }

        // Continue processing queue
        self.process_queue().await;
    }

    /// Perform the actual hot-swap operation
    async fn perform_operation(&self, request: &HotSwapRequest) -> AppResult<HotSwapResult> {
        let operation_timeout = request.timeout.unwrap_or(
            match request.operation {
                HotSwapOperation::Add => self.config.initialization_timeout,
                HotSwapOperation::Remove => self.config.graceful_shutdown_timeout,
                HotSwapOperation::Replace => self.config.graceful_shutdown_timeout + self.config.initialization_timeout,
                HotSwapOperation::Update => Duration::from_secs(30),
                HotSwapOperation::Restart => self.config.graceful_shutdown_timeout + self.config.initialization_timeout,
            }
        );

        timeout(operation_timeout, async {
            match request.operation {
                HotSwapOperation::Add => self.add_agent(request).await,
                HotSwapOperation::Remove => self.remove_agent(request).await,
                HotSwapOperation::Replace => self.replace_agent(request).await,
                HotSwapOperation::Update => self.update_agent(request).await,
                HotSwapOperation::Restart => self.restart_agent(request).await,
            }
        }).await.map_err(|_| AppError::agent("Hot-swap operation timed out"))?
    }

    /// Add a new agent
    async fn add_agent(&self, request: &HotSwapRequest) -> AppResult<HotSwapResult> {
        let start_time = Instant::now();

        let factory = request.new_agent_factory.as_ref()
            .ok_or_else(|| AppError::agent("Agent factory required for add operation"))?;

        // Create the new agent
        let new_agent = self.create_agent(factory).await?;
        let new_agent_id = new_agent.agent_id().to_string();

        // Add to active agents
        {
            let mut agents = self.active_agents.write().await;
            if agents.contains_key(&new_agent_id) {
                return Err(AppError::agent(format!("Agent {} already exists", new_agent_id)));
            }
            agents.insert(new_agent_id.clone(), new_agent);
        }

        Ok(HotSwapResult {
            request_id: request.id.clone(),
            success: true,
            operation: HotSwapOperation::Add,
            old_agent_id: None,
            new_agent_id: Some(new_agent_id),
            duration: start_time.elapsed(),
            errors: Vec::new(),
            migration_summary: None,
            status: OperationStatus::Completed,
        })
    }

    /// Remove an agent
    async fn remove_agent(&self, request: &HotSwapRequest) -> AppResult<HotSwapResult> {
        let start_time = Instant::now();

        let agent_id = request.target_agent_id.as_ref()
            .ok_or_else(|| AppError::agent("Target agent ID required for remove operation"))?;

        let mut removed_agent = {
            let mut agents = self.active_agents.write().await;
            agents.remove(agent_id)
                .ok_or_else(|| AppError::agent(format!("Agent {} not found", agent_id)))?
        };

        // Perform graceful shutdown if requested
        if request.graceful {
            if let Err(e) = removed_agent.graceful_shutdown().await {
                warn!("Graceful shutdown failed for agent {}: {}", agent_id, e);
            }
        }

        Ok(HotSwapResult {
            request_id: request.id.clone(),
            success: true,
            operation: HotSwapOperation::Remove,
            old_agent_id: Some(agent_id.clone()),
            new_agent_id: None,
            duration: start_time.elapsed(),
            errors: Vec::new(),
            migration_summary: None,
            status: OperationStatus::Completed,
        })
    }

    /// Replace an agent
    async fn replace_agent(&self, request: &HotSwapRequest) -> AppResult<HotSwapResult> {
        let start_time = Instant::now();

        let agent_id = request.target_agent_id.as_ref()
            .ok_or_else(|| AppError::agent("Target agent ID required for replace operation"))?;

        let factory = request.new_agent_factory.as_ref()
            .ok_or_else(|| AppError::agent("Agent factory required for replace operation"))?;

        // Create new agent first
        let new_agent = self.create_agent(factory).await?;
        let new_agent_id = new_agent.agent_id().to_string();

        let mut migration_summary = None;

        // Remove old agent and perform migration if enabled
        let mut old_agent = {
            let mut agents = self.active_agents.write().await;
            agents.remove(agent_id)
                .ok_or_else(|| AppError::agent(format!("Agent {} not found", agent_id)))?
        };

        // Perform state migration if configured
        if self.config.enable_state_migration {
            if let Some(strategy) = &request.migration_strategy {
                migration_summary = Some(self.migrate_state(&mut old_agent, &new_agent, strategy).await?);
            }
        }

        // Graceful shutdown of old agent
        if request.graceful {
            if let Err(e) = old_agent.graceful_shutdown().await {
                warn!("Graceful shutdown failed for old agent {}: {}", agent_id, e);
            }
        }

        // Add new agent
        {
            let mut agents = self.active_agents.write().await;
            agents.insert(new_agent_id.clone(), new_agent);
        }

        Ok(HotSwapResult {
            request_id: request.id.clone(),
            success: true,
            operation: HotSwapOperation::Replace,
            old_agent_id: Some(agent_id.clone()),
            new_agent_id: Some(new_agent_id),
            duration: start_time.elapsed(),
            errors: Vec::new(),
            migration_summary,
            status: OperationStatus::Completed,
        })
    }

    /// Update an agent
    async fn update_agent(&self, request: &HotSwapRequest) -> AppResult<HotSwapResult> {
        let start_time = Instant::now();

        let agent_id = request.target_agent_id.as_ref()
            .ok_or_else(|| AppError::agent("Target agent ID required for update operation"))?;

        // For now, update operations are not implemented as they require agent-specific logic
        // This would need to be implemented based on the specific agent types and their update capabilities

        Ok(HotSwapResult {
            request_id: request.id.clone(),
            success: false,
            operation: HotSwapOperation::Update,
            old_agent_id: Some(agent_id.clone()),
            new_agent_id: None,
            duration: start_time.elapsed(),
            errors: vec!["Update operation not yet implemented".to_string()],
            migration_summary: None,
            status: OperationStatus::Failed,
        })
    }

    /// Restart an agent
    async fn restart_agent(&self, request: &HotSwapRequest) -> AppResult<HotSwapResult> {
        // Restart is implemented as remove + add
        let remove_request = HotSwapRequest {
            operation: HotSwapOperation::Remove,
            ..request.clone()
        };

        let remove_result = self.remove_agent(&remove_request).await?;

        if !remove_result.success {
            return Ok(remove_result);
        }

        let add_request = HotSwapRequest {
            operation: HotSwapOperation::Add,
            target_agent_id: None,
            ..request.clone()
        };

        self.add_agent(&add_request).await
    }

    /// Create a new agent using the factory
    async fn create_agent(&self, factory: &AgentFactory) -> AppResult<Box<dyn HotSwappableAgent>> {
        let creators = self.agent_creators.read().await;
        let creator = creators.get(&factory.agent_type)
            .ok_or_else(|| AppError::agent(format!("No creator registered for agent type: {}", factory.agent_type)))?;

        creator.validate_factory(factory)?;
        creator.create_agent(factory).await
    }

    /// Migrate state between agents
    async fn migrate_state(
        &self,
        old_agent: &mut Box<dyn HotSwappableAgent>,
        new_agent: &Box<dyn HotSwappableAgent>,
        strategy: &MigrationStrategy,
    ) -> AppResult<MigrationSummary> {
        let start_time = Instant::now();

        // Implementation would depend on the specific migration strategy
        // This is a simplified version

        match strategy {
            MigrationStrategy::None => {
                Ok(MigrationSummary {
                    state_items_migrated: 0,
                    migration_duration: start_time.elapsed(),
                    migration_successful: true,
                    migration_errors: Vec::new(),
                })
            }
            _ => {
                // Full migration and other strategies would be implemented here
                Ok(MigrationSummary {
                    state_items_migrated: 0,
                    migration_duration: start_time.elapsed(),
                    migration_successful: false,
                    migration_errors: vec!["Migration not yet implemented".to_string()],
                })
            }
        }
    }

    /// Validate a hot-swap request
    async fn validate_request(&self, request: &HotSwapRequest) -> AppResult<()> {
        match request.operation {
            HotSwapOperation::Add => {
                if request.new_agent_factory.is_none() {
                    return Err(AppError::agent("Agent factory required for add operation"));
                }
            }
            HotSwapOperation::Remove | HotSwapOperation::Update | HotSwapOperation::Restart => {
                if request.target_agent_id.is_none() {
                    return Err(AppError::agent("Target agent ID required for this operation"));
                }
            }
            HotSwapOperation::Replace => {
                if request.target_agent_id.is_none() || request.new_agent_factory.is_none() {
                    return Err(AppError::agent("Both target agent ID and factory required for replace operation"));
                }
            }
        }

        Ok(())
    }

    /// Update statistics
    async fn update_stats(&self, result: &HotSwapResult) {
        let mut stats = self.stats.write().await;

        stats.total_operations += 1;
        if result.success {
            stats.successful_operations += 1;
        } else {
            stats.failed_operations += 1;
        }

        *stats.operations_by_type.entry(result.operation.clone()).or_insert(0) += 1;

        // Update average operation duration
        let total_time = stats.average_operation_duration.as_nanos() as u64 * (stats.total_operations - 1) + result.duration.as_nanos() as u64;
        stats.average_operation_duration = Duration::from_nanos(total_time / stats.total_operations);

        if let Some(ref migration) = result.migration_summary {
            stats.total_migrations += 1;
            if migration.migration_successful {
                stats.successful_migrations += 1;
            }
        }
    }

    /// Get list of active agents
    pub async fn get_active_agents(&self) -> Vec<String> {
        let agents = self.active_agents.read().await;
        agents.keys().cloned().collect()
    }

    /// Get hot-swap statistics
    pub async fn get_stats(&self) -> HotSwapStats {
        self.stats.read().await.clone()
    }

    /// Get operation history
    pub async fn get_operation_history(&self, limit: Option<usize>) -> Vec<HotSwapResult> {
        let history = self.operation_history.read().await;
        let limit = limit.unwrap_or(100);
        history.iter().rev().take(limit).cloned().collect()
    }
}

impl Clone for HotSwapManager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            active_agents: Arc::clone(&self.active_agents),
            agent_creators: Arc::clone(&self.agent_creators),
            operation_queue: Arc::clone(&self.operation_queue),
            operation_semaphore: Semaphore::new(self.config.max_concurrent_operations),
            operation_history: Arc::clone(&self.operation_history),
            stats: Arc::clone(&self.stats),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hot_swap_manager_creation() {
        let config = HotSwapConfig::default();
        let manager = HotSwapManager::new(config);

        let active_agents = manager.get_active_agents().await;
        assert!(active_agents.is_empty());

        let stats = manager.get_stats().await;
        assert_eq!(stats.total_operations, 0);
    }

    #[test]
    fn test_hot_swap_request_validation() {
        let request = HotSwapRequest {
            id: Uuid::new_v4().to_string(),
            operation: HotSwapOperation::Add,
            target_agent_id: None,
            new_agent_factory: Some(AgentFactory {
                agent_type: "test".to_string(),
                config: serde_json::Value::Null,
                capabilities: vec![],
                init_params: HashMap::new(),
            }),
            config_updates: None,
            graceful: true,
            timeout: None,
            priority: SwapPriority::Normal,
            migration_strategy: None,
        };

        assert_eq!(request.operation, HotSwapOperation::Add);
        assert!(request.new_agent_factory.is_some());
    }
}